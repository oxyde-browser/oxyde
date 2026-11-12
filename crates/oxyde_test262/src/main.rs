//! Lanceur de test262, la suite de conformité officielle d'ECMAScript.
//!
//! ```text
//! cargo run -p oxyde_test262 --release -- [--suite DOSSIER] [--filtre TEXTE]
//!                                         [--echecs N] [--publier]
//! cargo run -p oxyde_test262 --release -- --evaluer "1 + 1" [--evaluer …]
//! ```
//!
//! Chaque test s'exécute dans une machine neuve, précédé des fichiers du
//! harnais qu'il demande, en mode non strict puis strict comme le prescrit
//! `INTERPRETING.md`. Un test n'est compté réussi que si tous ses modes
//! passent. Rien n'est écarté d'avance : un test qui demande une
//! fonctionnalité absente d'Oxyde (`BigInt`, tableaux typés…) échoue et pèse dans le
//! taux.
//!
//! La suite n'est pas versionnée avec Oxyde : la cloner dans `vendor/test262`
//! (`git clone --depth 1 https://github.com/tc39/test262 vendor/test262`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use oxyde_js::{EvalError, JsResult, Value, Vm};

/// Dossiers de la suite pris en compte (ECMA-262 et son annexe B).
/// `intl402` (ECMA-402) et `staging` (propositions) sont hors périmètre.
const SCOPES: [&str; 3] = ["language", "built-ins", "annexB"];
/// Budget d'instructions d'un test : borne les boucles infinies.
const BUDGET: u64 = 50_000_000;
/// Pile des fils d'exécution des tests (récursions profondes).
const STACK_BYTES: usize = 256 * 1024 * 1024;

/// Métadonnées d'un test (bloc `/*--- … ---*/`).
#[derive(Debug, Default, PartialEq, Eq)]
struct Metadata {
    includes: Vec<String>,
    flags: Vec<String>,
    /// Type d'erreur attendu (`SyntaxError`…) pour un test négatif.
    negative: Option<String>,
}

impl Metadata {
    fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}

/// Éléments d'une liste YAML en ligne : `[a, b]`.
fn inline_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Lit les métadonnées utiles au lanceur. Le format est un petit sous-ensemble
/// de YAML : clés de premier niveau, listes en ligne ou à tirets.
fn parse_metadata(source: &str) -> Metadata {
    let mut meta = Metadata::default();
    let Some(start) = source.find("/*---") else {
        return meta;
    };
    let Some(len) = source.get(start..).and_then(|rest| rest.find("---*/")) else {
        return meta;
    };
    let block = source.get(start + 5..start + len).unwrap_or("");
    let mut section = "";
    for line in block.lines() {
        let indented = line.starts_with([' ', '\t']);
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        if !indented {
            let (key, value) = text.split_once(':').unwrap_or((text, ""));
            section = match key {
                "includes" | "flags" | "negative" => key,
                _ => "",
            };
            match key {
                "includes" => meta.includes = inline_list(value),
                "flags" => meta.flags = inline_list(value),
                _ => {}
            }
            continue;
        }
        match section {
            "includes" | "flags" => {
                if let Some(item) = text.strip_prefix('-') {
                    let list = if section == "includes" {
                        &mut meta.includes
                    } else {
                        &mut meta.flags
                    };
                    list.push(item.trim().to_owned());
                }
            }
            "negative" => {
                if let Some(kind) = text.strip_prefix("type:") {
                    meta.negative = Some(kind.trim().to_owned());
                }
            }
            _ => {}
        }
    }
    meta
}

/// Issue d'un test.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail(String),
}

/// Fichiers du harnais, lus une fois.
struct Harness {
    files: BTreeMap<String, String>,
}

impl Harness {
    fn load(suite: &Path) -> std::io::Result<Self> {
        let mut files = BTreeMap::new();
        for entry in std::fs::read_dir(suite.join("harness"))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "js")
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                files.insert(name.to_owned(), std::fs::read_to_string(&path)?);
            }
        }
        Ok(Self { files })
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.files.get(name).map(String::as_str)
    }
}

/// `print` : les tests asynchrones annoncent leur issue par cette fonction.
fn host_print(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = args
        .iter()
        .map(|a| vm.display(a))
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(lines) = vm
        .host
        .as_mut()
        .and_then(|h| h.downcast_mut::<Vec<String>>())
    {
        lines.push(text);
    }
    Ok(Value::Undefined)
}

/// `$262.detachArrayBuffer(tampon)`.
fn host_detach(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let buffer = args.first().cloned().unwrap_or(Value::Undefined);
    vm.detach_array_buffer(&buffer)?;
    Ok(Value::Null)
}

/// `$262.evalScript(source)` : évalue `source` comme un script distinct.
fn host_eval_script(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    match args.first() {
        Some(Value::String(source)) => vm.eval_script(source),
        _ => Ok(Value::Undefined),
    }
}

/// Objet `$262` minimal exposé aux tests.
const HOST_OBJECT: &str = "var $262 = { global: this, gc() {}, agent: {}, detachArrayBuffer: __detach, \
     evalScript: __evalScript };";

/// Chargeur de modules : lit le fichier désigné par `specifier`, relatif au
/// module qui l'importe. L'identifiant d'un module est son chemin.
fn file_loader(_vm: &mut Vm, specifier: &str, referrer: &str) -> Result<(String, String), String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return Err("seuls les chemins relatifs sont permis".to_owned());
    }
    let directory = Path::new(referrer).parent().unwrap_or(Path::new("."));
    // `components()` retire les « ./ » : un même fichier a un seul identifiant.
    let path: PathBuf = directory.join(specifier).components().collect();
    let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok((path.to_string_lossy().into_owned(), source))
}

/// Exécute `source` une fois, dans le mode demandé. `path` : chemin du test,
/// qui sert d'identifiant à son module.
fn run_once(
    harness: &Harness,
    meta: &Metadata,
    source: &str,
    path: &Path,
    strict: bool,
) -> Outcome {
    let mut vm = Vm::new();
    vm.set_budget(Some(BUDGET));
    vm.host = Some(Box::new(Vec::<String>::new()));
    let global = vm.global();
    vm.define_method(global, "print", 1, host_print);
    vm.define_method(global, "__detach", 1, host_detach);
    vm.define_method(global, "__evalScript", 1, host_eval_script);

    if !meta.has("raw") {
        let mut prelude = String::from(HOST_OBJECT);
        let mut names = vec!["assert.js", "sta.js"];
        if meta.has("async") {
            names.push("doneprintHandle.js");
        }
        names.extend(meta.includes.iter().map(String::as_str));
        for name in names {
            let Some(text) = harness.get(name) else {
                return Outcome::Fail(format!("fichier de harnais {name} introuvable"));
            };
            prelude.push('\n');
            prelude.push_str(text);
        }
        if let Err(error) = vm.eval(&prelude) {
            return Outcome::Fail(format!("harnais : {error}"));
        }
        // Le budget est celui du test, pas du harnais.
        vm.set_budget(Some(BUDGET));
    }

    // Les scripts aussi peuvent importer, par `import()`.
    vm.set_module_loader(file_loader);
    vm.set_script_url(&path.to_string_lossy());
    let result = if meta.has("module") {
        vm.eval_module(source, &path.to_string_lossy())
            .map(|()| Value::Undefined)
    } else if strict {
        vm.eval(&format!("\"use strict\";\n{source}"))
    } else {
        vm.eval(source)
    };
    if vm.was_interrupted() {
        return Outcome::Fail("budget d'instructions épuisé".to_owned());
    }
    if let Some(expected) = &meta.negative {
        return match result {
            Ok(_) => Outcome::Fail(format!("{expected} attendue, aucune erreur levée")),
            Err(EvalError::Syntax(_)) if expected == "SyntaxError" => Outcome::Pass,
            Err(EvalError::Thrown { message, .. }) if message.starts_with(expected.as_str()) => {
                Outcome::Pass
            }
            Err(error) => Outcome::Fail(format!("{expected} attendue, obtenu : {error}")),
        };
    }
    if let Err(error) = result {
        return Outcome::Fail(error.to_string());
    }
    if meta.has("async") {
        let lines = vm
            .host
            .as_ref()
            .and_then(|h| h.downcast_ref::<Vec<String>>())
            .cloned()
            .unwrap_or_default();
        if let Some(failure) = lines
            .iter()
            .find(|l| l.starts_with("Test262:AsyncTestFailure"))
        {
            return Outcome::Fail(failure.clone());
        }
        if !lines.iter().any(|l| l == "Test262:AsyncTestComplete") {
            return Outcome::Fail("le test asynchrone ne s'est pas terminé".to_owned());
        }
    }
    Outcome::Pass
}

/// Exécute un test dans tous les modes que ses métadonnées demandent.
fn run_test(harness: &Harness, source: &str, path: &Path) -> Outcome {
    let meta = parse_metadata(source);
    // Un module est toujours strict : un seul mode.
    let modes: &[bool] = if meta.has("module") || meta.has("raw") || meta.has("noStrict") {
        &[false]
    } else if meta.has("onlyStrict") {
        &[true]
    } else {
        &[false, true]
    };
    for strict in modes {
        if let Outcome::Fail(reason) = run_once(harness, &meta, source, path, *strict) {
            let mode = if *strict { "strict" } else { "non strict" };
            return Outcome::Fail(format!("[{mode}] {reason}"));
        }
    }
    Outcome::Pass
}

/// Tous les fichiers de test sous `dir`, triés.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "js")
            && !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("_FIXTURE"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Chemin relatif à la suite, avec des barres obliques.
fn relative(suite: &Path, path: &Path) -> String {
    path.strip_prefix(suite.join("test"))
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Groupe d'un test dans le rapport : ses deux premiers dossiers.
fn group_of(relative: &str) -> String {
    let mut parts = relative.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), Some(_)) => format!("{a}/{b}"),
        (Some(a), _, _) => a.to_owned(),
        _ => String::new(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Count {
    total: usize,
    passed: usize,
}

struct Args {
    suite: PathBuf,
    filter: Option<String>,
    failures: usize,
    publish: bool,
    /// Sources à évaluer une à une au lieu de lancer la suite (diagnostic).
    evaluate: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        suite: PathBuf::from("vendor/test262"),
        filter: None,
        failures: 0,
        publish: false,
        evaluate: Vec::new(),
    };
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| iter.next().ok_or(format!("{name} attend une valeur"));
        match arg.as_str() {
            "--suite" => args.suite = PathBuf::from(value("--suite")?),
            "--filtre" => args.filter = Some(value("--filtre")?),
            "--echecs" => {
                args.failures = value("--echecs")?
                    .parse()
                    .map_err(|_| "--echecs attend un nombre".to_owned())?;
            }
            "--publier" => args.publish = true,
            "--evaluer" => args.evaluate.push(value("--evaluer")?),
            other => return Err(format!("option inconnue : {other}")),
        }
    }
    Ok(args)
}

fn percent(count: Count) -> String {
    if count.total == 0 {
        return "—".to_owned();
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombres de tests petits devant 2^53"
    )]
    let rate = count.passed as f64 * 100.0 / count.total as f64;
    format!("{rate:.1} %")
}

/// Révision de la suite (contenu de `.git/HEAD` résolu), si elle est disponible.
fn suite_revision(suite: &Path) -> Option<String> {
    let head = std::fs::read_to_string(suite.join(".git/HEAD")).ok()?;
    let head = head.trim();
    let hash = match head.strip_prefix("ref: ") {
        Some(reference) => std::fs::read_to_string(suite.join(".git").join(reference)).ok()?,
        None => head.to_owned(),
    };
    Some(hash.trim().chars().take(12).collect())
}

fn report(groups: &BTreeMap<String, Count>, total: Count, revision: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Résultats test262\n");
    let _ = writeln!(
        out,
        "Suite [tc39/test262](https://github.com/tc39/test262), révision `{revision}`, dossiers \
         `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.\n"
    );
    let _ = writeln!(
        out,
        "Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test \
         n'est écarté : ceux qui demandent une fonctionnalité absente échouent.\n"
    );
    let _ = writeln!(
        out,
        "**Total : {} réussis sur {} ({}).**\n",
        total.passed,
        total.total,
        percent(total)
    );
    let _ = writeln!(out, "| Dossier | Tests | Réussis | Taux |");
    let _ = writeln!(out, "|---|---:|---:|---:|");
    for (group, count) in groups {
        let _ = writeln!(
            out,
            "| `{group}` | {} | {} | {} |",
            count.total,
            count.passed,
            percent(*count)
        );
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    if !args.evaluate.is_empty() {
        #[expect(
            clippy::print_stdout,
            reason = "sortie de l'outil en ligne de commande"
        )]
        for source in &args.evaluate {
            let mut vm = Vm::new();
            vm.set_budget(Some(BUDGET));
            match vm.eval(source) {
                Ok(value) => println!(
                    "{source}
    => {}",
                    vm.display(&value)
                ),
                Err(error) => println!(
                    "{source}
    !! {error}"
                ),
            }
        }
        return Ok(());
    }
    if !args.suite.join("test").is_dir() {
        return Err(format!(
            "suite introuvable dans {} : la cloner avec « git clone --depth 1 \
             https://github.com/tc39/test262 vendor/test262 »",
            args.suite.display()
        )
        .into());
    }
    let harness = Harness::load(&args.suite)?;
    let mut paths = Vec::new();
    for scope in SCOPES {
        collect(&args.suite.join("test").join(scope), &mut paths)?;
    }
    if let Some(filter) = &args.filter {
        paths.retain(|p| relative(&args.suite, p).contains(filter.as_str()));
    }

    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(String, Outcome)>> = Mutex::new(Vec::with_capacity(paths.len()));
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    std::thread::scope(|scope| -> std::io::Result<()> {
        for _ in 0..workers {
            std::thread::Builder::new()
                .name("test262".to_owned())
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, || {
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(index) else { break };
                        let name = relative(&args.suite, path);
                        let outcome = match std::fs::read_to_string(path) {
                            Ok(source) => {
                                // Une panique du moteur est un défaut d'Oxyde : elle
                                // est comptée comme un échec et signalée.
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    run_test(&harness, &source, path)
                                }))
                                .unwrap_or_else(|_| Outcome::Fail("PANIQUE du moteur".to_owned()))
                            }
                            Err(error) => Outcome::Fail(format!("lecture impossible : {error}")),
                        };
                        if let Ok(mut results) = results.lock() {
                            results.push((name, outcome));
                        }
                    }
                })?;
        }
        Ok(())
    })?;

    let mut results = results.into_inner().unwrap_or_default();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    let mut groups: BTreeMap<String, Count> = BTreeMap::new();
    let mut total = Count::default();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for (name, outcome) in &results {
        let count = groups.entry(group_of(name)).or_default();
        count.total += 1;
        total.total += 1;
        match outcome {
            Outcome::Pass => {
                count.passed += 1;
                total.passed += 1;
            }
            Outcome::Fail(reason) => {
                // Regroupe les échecs par début de message, pour repérer les
                // causes les plus fréquentes.
                let key: String = reason.chars().take(70).collect();
                *reasons.entry(key).or_default() += 1;
            }
        }
    }

    let revision = suite_revision(&args.suite).unwrap_or_else(|| "inconnue".to_owned());
    let text = report(&groups, total, &revision);
    #[expect(
        clippy::print_stdout,
        reason = "sortie de l'outil en ligne de commande"
    )]
    {
        print!("{text}");
        if args.failures > 0 {
            let mut frequent: Vec<(&String, &usize)> = reasons.iter().collect();
            frequent.sort_by(|a, b| b.1.cmp(a.1));
            println!("\n## Causes d'échec les plus fréquentes\n");
            for (reason, count) in frequent.iter().take(args.failures) {
                println!("{count:>6}  {reason}");
            }
            println!("\n## Premiers échecs\n");
            for (name, outcome) in results
                .iter()
                .filter(|(_, o)| *o != Outcome::Pass)
                .take(args.failures)
            {
                if let Outcome::Fail(reason) = outcome {
                    println!("{name}\n    {reason}");
                }
            }
        }
    }
    if args.publish {
        if args.filter.is_some() {
            return Err("--publier exige la suite entière (sans --filtre)".into());
        }
        std::fs::write("conformance/test262.md", text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_read_from_both_list_styles() {
        let meta = parse_metadata(
            "// Copyright\n/*---\ndescription: >\n  texte: avec deux-points\nincludes: [a.js, b.js]\nflags:\n  - onlyStrict\n  - async\nnegative:\n  phase: parse\n  type: SyntaxError\nfeatures: [x]\n---*/\ncode();",
        );
        assert_eq!(meta.includes, ["a.js", "b.js"]);
        assert_eq!(meta.flags, ["onlyStrict", "async"]);
        assert_eq!(meta.negative.as_deref(), Some("SyntaxError"));
        assert!(meta.has("async") && !meta.has("module"));
        assert_eq!(parse_metadata("sans métadonnées"), Metadata::default());
    }

    #[test]
    fn groups_and_rates() {
        assert_eq!(group_of("built-ins/Array/from/x.js"), "built-ins/Array");
        assert_eq!(group_of("language/x.js"), "language");
        assert_eq!(
            percent(Count {
                total: 0,
                passed: 0
            }),
            "—"
        );
        assert_eq!(
            percent(Count {
                total: 8,
                passed: 2
            }),
            "25.0 %"
        );
    }

    fn run_test_at(harness: &Harness, source: &str) -> Outcome {
        run_test(harness, source, Path::new("test.js"))
    }

    #[test]
    fn outcomes_follow_the_metadata() {
        let harness = Harness {
            files: BTreeMap::from([
                (
                    "assert.js".to_owned(),
                    "function assert(v) { if (!v) throw new Error('assertion'); }".to_owned(),
                ),
                ("sta.js".to_owned(), "var chargé = true;".to_owned()),
                (
                    "doneprintHandle.js".to_owned(),
                    "function $DONE(e) { print(e ? 'Test262:AsyncTestFailure:' + e : 'Test262:AsyncTestComplete'); }"
                        .to_owned(),
                ),
            ]),
        };
        assert_eq!(
            run_test_at(&harness, "assert(chargé && 1 + 1 === 2);"),
            Outcome::Pass
        );
        assert!(matches!(
            run_test_at(&harness, "assert(false);"),
            Outcome::Fail(_)
        ));
        assert_eq!(
            run_test_at(
                &harness,
                "/*---\nnegative:\n  phase: parse\n  type: SyntaxError\n---*/\nvar = ;"
            ),
            Outcome::Pass
        );
        assert!(matches!(
            run_test_at(
                &harness,
                "/*---\nnegative:\n  phase: runtime\n  type: TypeError\n---*/\n1;"
            ),
            Outcome::Fail(_)
        ));
        assert_eq!(
            run_test_at(
                &harness,
                "/*---\nflags: [async]\n---*/\nPromise.resolve().then(() => $DONE());"
            ),
            Outcome::Pass
        );
        assert!(matches!(
            run_test_at(&harness, "/*---\nflags: [async]\n---*/\n1;"),
            Outcome::Fail(_)
        ));
        assert_eq!(
            run_test_at(
                &harness,
                "/*---\nflags: [module]\n---*/\nexport const a = typeof this; assert(a === 'undefined');"
            ),
            Outcome::Pass
        );
        assert!(matches!(
            run_test_at(&harness, "/*---\nflags: [module]\n---*/\nimport 'fs';"),
            Outcome::Fail(_)
        ));
        assert!(matches!(
            run_test_at(&harness, "/*---\nflags: [noStrict]\n---*/\nwhile (true) {}"),
            Outcome::Fail(_)
        ));
        assert!(matches!(
            run_test_at(&harness, "/*---\nincludes: [absent.js]\n---*/\n1;"),
            Outcome::Fail(_)
        ));
    }
}
