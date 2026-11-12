//! Lanceur de la suite Web Platform Tests pour `oxyde_content`.
//!
//! Chaque fichier de test est chargé comme une page (`Page::from_html`), ses
//! scripts sont servis depuis la copie locale de la suite, et
//! `testharnessreport.js` est remplacé par un rapporteur qui rend les
//! résultats par la console. Un fichier s'exécute dans un processus fils :
//! un plantage ou une boucle sans fin n'emporte que lui.
//!
//! ```text
//! oxyde_wpt [--suite vendor/wpt] [--filtre texte] [--publier] [--echecs N]
//!           [--journal fichier] [dossier…]
//! oxyde_wpt --un dom/nodes/Node-appendChild.html
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use oxyde_content::{FetchResponse, Page, Url, Viewport};

/// Origine fictive sous laquelle les tests sont servis.
const ORIGIN: &str = "https://web-platform.test";
/// Rapporteur substitué à `/resources/testharnessreport.js`.
const REPORT_JS: &str = include_str!("report.js");
/// Temps réel accordé à un fichier dans le processus fils.
const CHILD_BUDGET: Duration = Duration::from_secs(10);
/// Temps au bout duquel le processus père tue un fils muet.
const KILL_AFTER: Duration = Duration::from_secs(20);
/// Instructions accordées à chaque tâche de script : certains fichiers
/// enchaînent des dizaines de milliers de sous-tests dans un seul script.
const TASK_BUDGET: u64 = 2_000_000_000;
/// Nombre maximal de réveils de minuteurs par fichier.
const MAX_TICKS: usize = 50_000;

/// Résultat d'un sous-test.
#[derive(Clone, Debug)]
struct Subtest {
    /// Réussi (`PASS`).
    pass: bool,
    /// Nom donné par le test.
    name: String,
    /// Message d'échec, vide en cas de réussite.
    message: String,
}

/// Issue d'un fichier de test.
#[derive(Clone, Debug)]
enum Outcome {
    /// Le harnais est allé au bout (état `OK`).
    Done(Vec<Subtest>),
    /// Le harnais a signalé une erreur globale ou un délai dépassé.
    HarnessError(Vec<Subtest>, String),
    /// Aucun résultat : plantage, boucle sans fin ou harnais jamais terminé.
    NoResult(String),
}

fn main() -> ExitCode {
    let mut suite = PathBuf::from("vendor/wpt");
    let mut filter: Option<String> = None;
    let mut single: Option<String> = None;
    let mut publish = false;
    let mut shown = 40usize;
    let mut journal: Option<PathBuf> = None;
    let mut dirs: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--suite" => suite = PathBuf::from(args.next().unwrap_or_default()),
            "--filtre" => filter = args.next(),
            "--un" => single = args.next(),
            "--publier" => publish = true,
            "--journal" => journal = args.next().map(PathBuf::from),
            "--echecs" => shown = args.next().and_then(|n| n.parse().ok()).unwrap_or(shown),
            _ => dirs.push(arg),
        }
    }
    if let Some(path) = single {
        return run_child(&suite, &path);
    }
    if !suite.join("resources/testharness.js").is_file() {
        eprintln!(
            "suite introuvable dans {} (attendu : resources/testharness.js)",
            suite.display()
        );
        return ExitCode::FAILURE;
    }
    if dirs.is_empty() {
        dirs.push("dom".to_owned());
    }
    let mut tests = Vec::new();
    for dir in &dirs {
        collect(&suite, &suite.join(dir), &mut tests);
    }
    tests.sort();
    if let Some(f) = &filter {
        tests.retain(|t| t.contains(f.as_str()));
    }
    let results = run_all(&suite, &tests);
    if let Some(path) = journal
        && let Err(e) = std::fs::write(&path, journal_text(&results))
    {
        eprintln!("journal non écrit dans {} : {e}", path.display());
    }
    print!("{}", render(&dirs, &results, publish, shown));
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// Recensement des tests
// ---------------------------------------------------------------------------

/// Ajoute à `out` les tests `testharness` trouvés sous `dir`, en chemins
/// relatifs à la suite, séparés par `/`.
fn collect(suite: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if !matches!(
                name.as_str(),
                "support" | "resources" | "crashtests" | "tools" | "reference"
            ) {
                collect(suite, &path, out);
            }
            continue;
        }
        let Ok(rel) = path.strip_prefix(suite) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if is_test(&path, &name) {
            out.push(rel);
        }
    }
}

/// Vrai si le fichier est un test `testharness` exécutable dans une fenêtre.
fn is_test(path: &Path, name: &str) -> bool {
    if [
        "-manual.",
        "-ref.",
        "-notref.",
        "-crash.",
        ".tentative-ref.",
    ]
    .iter()
    .any(|m| name.contains(m))
    {
        return false;
    }
    if name.ends_with(".any.js") {
        // Portée par défaut : fenêtre et worker dédié.
        return std::fs::read_to_string(path).is_ok_and(|s| {
            meta(&s, "global").is_none_or(|g| g.split(',').any(|p| p.trim() == "window"))
        });
    }
    if name.ends_with(".window.js") {
        return true;
    }
    if name.ends_with(".html") || name.ends_with(".htm") {
        return std::fs::read(path)
            .is_ok_and(|b| String::from_utf8_lossy(&b).contains("testharness.js"));
    }
    false
}

/// Valeur de la première ligne `// META: key=…` de l'en-tête d'un test.
fn meta<'a>(source: &'a str, key: &'a str) -> Option<&'a str> {
    metas(source, key).next()
}

/// Valeurs des lignes `// META: key=…` de l'en-tête d'un test.
fn metas<'a>(source: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> {
    source
        .lines()
        .take_while(|l| l.starts_with("//") || l.trim().is_empty())
        .filter_map(move |l| {
            let rest = l.strip_prefix("// META:")?.trim();
            let (k, v) = rest.split_once('=')?;
            (k.trim() == key).then_some(v.trim())
        })
}

// ---------------------------------------------------------------------------
// Processus fils : un fichier
// ---------------------------------------------------------------------------

/// Exécute le test `rel` et écrit ses lignes de résultat sur la sortie.
fn run_child(suite: &Path, rel: &str) -> ExitCode {
    let Some((url, html)) = load_test(suite, rel) else {
        println!("WPT-X\tfichier illisible");
        return ExitCode::FAILURE;
    };
    let mut page = Page::from_html(url, &html);
    page.set_task_budget(TASK_BUDGET);
    page.set_viewport(Viewport {
        width: 800.0,
        height: 600.0,
    });
    for url in page.pending_stylesheets() {
        let css = read_url(suite, &url).unwrap_or_default();
        page.set_stylesheet(&url, &css);
    }
    for url in page.pending_images() {
        page.set_image(&url, None);
    }
    for url in page.pending_frames() {
        let html = read_url(suite, &url);
        page.set_frame(&url, html.as_deref());
    }
    for url in page.pending_scripts() {
        let source = read_url(suite, &url);
        page.set_script(&url, source);
    }
    // Les modules importés se découvrent de proche en proche.
    for _ in 0..64 {
        let pending = page.pending_modules();
        if pending.is_empty() {
            break;
        }
        for url in pending {
            let source = read_url(suite, &url);
            page.set_module(&url, source);
        }
    }
    page.run_scripts();

    let started = Instant::now();
    let mut finished = false;
    for _ in 0..MAX_TICKS {
        for request in page.take_fetches() {
            let result = match read_url_bytes(suite, &request.url) {
                Some(body) => Ok(FetchResponse {
                    url: request.url.clone(),
                    status: 200,
                    headers: vec![(
                        "content-type".to_owned(),
                        content_type(request.url.path()).to_owned(),
                    )],
                    body,
                }),
                None => Ok(FetchResponse {
                    url: request.url.clone(),
                    status: 404,
                    headers: Vec::new(),
                    body: Vec::new(),
                }),
            };
            page.complete_fetch(request.id, result);
        }
        // Cadres créés par les scripts : documents et scripts servis de même.
        for url in page.pending_frames() {
            let html = read_url(suite, &url);
            page.set_frame(&url, html.as_deref());
        }
        for url in page.pending_scripts() {
            let source = read_url(suite, &url);
            page.set_script(&url, source);
        }
        let loaded = page.load_ready_frames();
        for message in page.take_console() {
            if message.text.starts_with("WPT-") {
                println!("{}", message.text);
                finished |= message.text.starts_with("WPT-H");
            }
        }
        if finished || started.elapsed() > CHILD_BUDGET {
            break;
        }
        // Temps virtuel : on saute directement à l'échéance suivante.
        match page.next_wake() {
            Some(due) => {
                page.tick(due);
            }
            // Un cadre vient d'être chargé : ses scripts ont pu en demander d'autres.
            None if loaded => {}
            None => break,
        }
    }
    if finished {
        ExitCode::SUCCESS
    } else {
        println!("WPT-X\tle harnais n'a pas terminé");
        ExitCode::FAILURE
    }
}

/// Adresse et source HTML du test `rel` ; les tests `.any.js` et
/// `.window.js` sont enveloppés dans une page générée.
fn load_test(suite: &Path, rel: &str) -> Option<(Url, String)> {
    let bytes = std::fs::read(suite.join(rel)).ok()?;
    let source = String::from_utf8_lossy(&bytes).into_owned();
    let wrapped = rel
        .strip_suffix(".any.js")
        .map(|stem| format!("{stem}.any.html"))
        .or_else(|| {
            rel.strip_suffix(".window.js")
                .map(|stem| format!("{stem}.window.html"))
        });
    let Some(page_path) = wrapped else {
        let url = Url::parse(&format!("{ORIGIN}/{rel}")).ok()?;
        return Some((url, source));
    };
    let mut html = String::from(
        "<!doctype html>\n<meta charset=utf-8>\n\
         <script src=\"/resources/testharness.js\"></script>\n\
         <script src=\"/resources/testharnessreport.js\"></script>\n",
    );
    if let Some(title) = meta(&source, "title") {
        let _ = writeln!(html, "<title>{}</title>", title.replace('<', "&lt;"));
    }
    for script in metas(&source, "script") {
        let _ = writeln!(
            html,
            "<script src=\"{}\"></script>",
            script.replace('"', "&quot;")
        );
    }
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let _ = writeln!(html, "<div id=log></div>\n<script src=\"{file}\"></script>");
    let url = Url::parse(&format!("{ORIGIN}/{page_path}")).ok()?;
    Some((url, html))
}

/// Contenu texte de la ressource `url`, servie depuis la suite.
fn read_url(suite: &Path, url: &Url) -> Option<String> {
    read_url_bytes(suite, url).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Contenu de la ressource `url`, servie depuis la suite. Seule l'origine des
/// tests est servie, et jamais un chemin qui sort de la suite.
fn read_url_bytes(suite: &Path, url: &Url) -> Option<Vec<u8>> {
    if !url.as_str().starts_with(ORIGIN) {
        return None;
    }
    let path = url.path();
    if path == "/resources/testharnessreport.js" {
        return Some(REPORT_JS.as_bytes().to_vec());
    }
    let rel = path.trim_start_matches('/');
    if rel
        .split('/')
        .any(|part| part == ".." || part.contains(':') || part.contains('\\'))
    {
        return None;
    }
    std::fs::read(suite.join(rel)).ok()
}

/// Type MIME d'après l'extension du chemin.
fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" | "htm" => "text/html",
        "js" => "text/javascript",
        "json" => "application/json",
        "css" => "text/css",
        "xml" | "xhtml" => "application/xml",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// Processus père : toute la liste
// ---------------------------------------------------------------------------

/// Exécute tous les tests, plusieurs à la fois, chacun dans un processus fils.
fn run_all(suite: &Path, tests: &[String]) -> Vec<(String, Outcome)> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("exécutable introuvable : {e}");
            return Vec::new();
        }
    };
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, Outcome)>> = Mutex::new(Vec::with_capacity(tests.len()));
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(test) = tests.get(i) else {
                        break;
                    };
                    let outcome = run_one(&exe, suite, test);
                    if let Ok(mut all) = results.lock() {
                        all.push((i, outcome));
                    }
                }
            });
        }
    });
    let mut all = results.into_inner().unwrap_or_default();
    all.sort_by_key(|(i, _)| *i);
    all.into_iter()
        .filter_map(|(i, outcome)| Some((tests.get(i)?.clone(), outcome)))
        .collect()
}

/// Lance le processus fils d'un test et interprète sa sortie.
fn run_one(exe: &Path, suite: &Path, test: &str) -> Outcome {
    let child = Command::new(exe)
        .arg("--suite")
        .arg(suite)
        .arg("--un")
        .arg(test)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(e) => return Outcome::NoResult(format!("lancement impossible : {e}")),
    };
    // La sortie est lue à part : un tube plein bloquerait le fils.
    let reader = child.stdout.take().map(|mut out| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = out.read_to_end(&mut bytes);
            bytes
        })
    });
    let started = Instant::now();
    let mut killed = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > KILL_AFTER => {
                let _ = child.kill();
                let _ = child.wait();
                killed = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => break None,
        }
    };
    let bytes = reader.and_then(|r| r.join().ok()).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let mut subtests = Vec::new();
    let mut harness: Option<(u32, String)> = None;
    let mut note = String::new();
    for line in text.lines() {
        let mut parts = line.splitn(4, '\t');
        match parts.next() {
            Some("WPT-T") => {
                let status = parts.next().and_then(|s| s.parse::<u32>().ok());
                subtests.push(Subtest {
                    pass: status == Some(0),
                    name: parts.next().unwrap_or("").to_owned(),
                    message: parts.next().unwrap_or("").to_owned(),
                });
            }
            Some("WPT-H") => {
                let status = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                harness = Some((status, parts.next().unwrap_or("").to_owned()));
            }
            Some("WPT-X") => note = parts.next().unwrap_or("").to_owned(),
            _ => {}
        }
    }
    match harness {
        Some((0, _)) => Outcome::Done(subtests),
        Some((2, _)) => Outcome::HarnessError(subtests, "délai du harnais dépassé".to_owned()),
        Some((_, message)) => Outcome::HarnessError(subtests, message),
        None if killed => Outcome::NoResult("boucle sans fin (processus tué)".to_owned()),
        None if !note.is_empty() => Outcome::NoResult(note),
        None => Outcome::NoResult(match status.and_then(|s| s.code()) {
            Some(code) => format!("plantage (code {code:#x})"),
            None => "plantage".to_owned(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Rapport
// ---------------------------------------------------------------------------

/// Compteurs d'un dossier.
#[derive(Default)]
struct Tally {
    files: usize,
    files_ok: usize,
    subtests: usize,
    passed: usize,
}

impl Tally {
    fn add(&mut self, outcome: &Outcome) {
        self.files += 1;
        let (subtests, clean) = match outcome {
            Outcome::Done(s) => (s.as_slice(), true),
            Outcome::HarnessError(s, _) => (s.as_slice(), false),
            Outcome::NoResult(_) => (&[][..], false),
        };
        let passed = subtests.iter().filter(|s| s.pass).count();
        self.subtests += subtests.len();
        self.passed += passed;
        if clean && !subtests.is_empty() && passed == subtests.len() {
            self.files_ok += 1;
        }
    }
}

/// Pourcentage à une décimale, virgule française.
fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "—".to_owned();
    }
    // Arrondi au dixième en entiers : pas de flottant, pas de surprise.
    let tenths = (part * 1000 + whole / 2) / whole;
    format!("{},{} %", tenths / 10, tenths % 10)
}

/// Rapport en Markdown. `publish` donne la forme stable, destinée à
/// `conformance/wpt.md` ; sinon les `shown` premiers échecs sont détaillés.
fn render(dirs: &[String], results: &[(String, Outcome)], publish: bool, shown: usize) -> String {
    let mut total = Tally::default();
    let mut by_dir: BTreeMap<String, Tally> = BTreeMap::new();
    for (test, outcome) in results {
        total.add(outcome);
        let dir = test.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
        by_dir.entry(dir).or_default().add(outcome);
    }
    let mut out = String::new();
    let _ = writeln!(out, "# Web Platform Tests\n");
    let _ = writeln!(
        out,
        "Produit par `cargo run --release -p oxyde_wpt -- --publier {}`, sur la suite\n\
         `web-platform-tests/wpt` déposée dans `vendor/wpt`. Seuls les tests `testharness.js`\n\
         destinés à une fenêtre sont exécutés ; un fichier est « entièrement réussi » quand le\n\
         harnais va au bout et que tous ses sous-tests passent.\n",
        dirs.join(" ")
    );
    let _ = writeln!(
        out,
        "**Sous-tests réussis : {} / {} ({})** — fichiers entièrement réussis : {} / {} ({}).\n",
        total.passed,
        total.subtests,
        percent(total.passed, total.subtests),
        total.files_ok,
        total.files,
        percent(total.files_ok, total.files)
    );
    let _ = writeln!(
        out,
        "Le taux en sous-tests surestime la conformité : un fichier qui plante ou que le harnais\n\
         abandonne ne rapporte aucun sous-test, donc n'en fait échouer aucun. Le taux en fichiers\n\
         est le plus honnête des deux.\n"
    );
    let _ = writeln!(out, "| Dossier | Fichiers réussis | Sous-tests réussis |");
    let _ = writeln!(out, "|---|---:|---:|");
    for (dir, t) in &by_dir {
        let _ = writeln!(
            out,
            "| `{dir}` | {} / {} | {} / {} ({}) |",
            t.files_ok,
            t.files,
            t.passed,
            t.subtests,
            percent(t.passed, t.subtests)
        );
    }
    let _ = writeln!(out, "\n## Fichiers non entièrement réussis\n");
    let _ = writeln!(out, "| Fichier | Sous-tests | Cause |");
    let _ = writeln!(out, "|---|---:|---|");
    let mut details = String::new();
    let mut detailed = 0usize;
    for (test, outcome) in results {
        let (subtests, cause): (&[Subtest], String) = match outcome {
            Outcome::Done(s) if !s.is_empty() && s.iter().all(|t| t.pass) => continue,
            Outcome::Done(s) if s.is_empty() => (s, "aucun sous-test".to_owned()),
            Outcome::Done(s) => (s, String::new()),
            Outcome::HarnessError(s, m) => (s, format!("harnais : {}", cell(m))),
            Outcome::NoResult(m) => (&[], cell(m)),
        };
        let passed = subtests.iter().filter(|s| s.pass).count();
        let _ = writeln!(
            out,
            "| `{test}` | {passed} / {} | {cause} |",
            subtests.len()
        );
        for t in subtests.iter().filter(|t| !t.pass) {
            if detailed >= shown {
                break;
            }
            detailed += 1;
            let _ = writeln!(details, "- `{test}` — {} : {}", t.name, t.message);
        }
    }
    if !publish && !details.is_empty() {
        let _ = writeln!(out, "\n## Premiers échecs\n\n{details}");
    }
    out
}

/// Journal : une ligne par sous-test (`PASS` ou `FAIL`, fichier, nom) et une
/// par fichier (`FILE-OK` ou `FILE-KO`), pour comparer deux exécutions.
fn journal_text(results: &[(String, Outcome)]) -> String {
    let mut out = String::new();
    for (test, outcome) in results {
        let (subtests, clean) = match outcome {
            Outcome::Done(s) => (s.as_slice(), true),
            Outcome::HarnessError(s, _) => (s.as_slice(), false),
            Outcome::NoResult(_) => (&[][..], false),
        };
        let all = clean && !subtests.is_empty() && subtests.iter().all(|t| t.pass);
        let _ = writeln!(out, "{}\t{test}", if all { "FILE-OK" } else { "FILE-KO" });
        for t in subtests {
            let _ = writeln!(
                out,
                "{}\t{test}\t{}",
                if t.pass { "PASS" } else { "FAIL" },
                t.name
            );
        }
    }
    out
}

/// Texte borné et sans barre verticale, pour une cellule de tableau.
fn cell(text: &str) -> String {
    let mut s: String = text.chars().take(120).collect();
    s = s.replace('|', "\\|");
    s
}
