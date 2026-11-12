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
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use oxyde_js::{EvalError, JsResult, SharedStorage, Value, Vm};

/// Dossiers de la suite pris en compte (ECMA-262 et son annexe B).
/// `intl402` (ECMA-402) et `staging` (propositions) sont hors périmètre.
const SCOPES: [&str; 3] = ["language", "built-ins", "annexB"];
/// Budget d'instructions d'un test : borne les boucles infinies.
const BUDGET: u64 = 50_000_000;

/// Temps qu'un test asynchrone peut passer à attendre ses minuteurs et ses
/// attentes d'`Atomics.waitAsync` avant d'être déclaré inachevé.
const LOOP_LIMIT: Duration = Duration::from_secs(20);

/// Pause la plus longue de la boucle d'événements entre deux vérifications.
const LOOP_PAUSE: Duration = Duration::from_millis(50);
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

/// Message diffusé aux agents : un tampon partagé et un nombre.
type Broadcast = (SharedStorage, f64);

/// Ce que les agents d'un test ont en commun (`$262.agent`).
struct Hub {
    /// Comptes rendus des agents, dans l'ordre d'arrivée.
    reports: Mutex<VecDeque<String>>,
    /// Boîte de réception de chaque agent lancé.
    agents: Mutex<Vec<mpsc::Sender<Broadcast>>>,
    /// Agents qui n'ont pas encore retiré la diffusion en cours.
    pending: Mutex<usize>,
    retrieved: Condvar,
    /// Origine de l'horloge commune.
    epoch: Instant,
}

/// Verrou repris même empoisonné : un agent qui panique ne doit pas bloquer
/// les autres.
fn relock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// État d'un agent lancé par `$262.agent.start`.
struct AgentState {
    /// Fonction donnée à `receiveBroadcast`.
    callback: Option<Value>,
    /// Vrai une fois `leaving()` appelé.
    leaving: bool,
}

/// Minuteur posé par `setTimeout`.
struct Timer {
    callback: Value,
    due: Instant,
}

/// État de l'hôte attaché à un moteur.
#[derive(Default)]
struct Host {
    /// Lignes écrites par `print`.
    lines: Vec<String>,
    /// Minuteurs en attente.
    timers: Vec<Timer>,
    hub: Option<Arc<Hub>>,
    /// Présent dans un agent, absent dans l'agent principal.
    agent: Option<AgentState>,
}

fn host(vm: &mut Vm) -> Option<&mut Host> {
    vm.host.as_mut().and_then(|h| h.downcast_mut::<Host>())
}

/// Nœud commun aux agents du test, créé au premier besoin.
fn hub(vm: &mut Vm) -> Option<Arc<Hub>> {
    let host = host(vm)?;
    Some(Arc::clone(host.hub.get_or_insert_with(|| {
        Arc::new(Hub {
            reports: Mutex::new(VecDeque::new()),
            agents: Mutex::new(Vec::new()),
            pending: Mutex::new(0),
            retrieved: Condvar::new(),
            epoch: Instant::now(),
        })
    })))
}

/// `print` : les tests asynchrones annoncent leur issue par cette fonction.
fn host_print(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = args
        .iter()
        .map(|a| vm.display(a))
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(host) = host(vm) {
        host.lines.push(text);
    }
    Ok(Value::Undefined)
}

/// Objet `$262` d'un agent.
const AGENT_OBJECT: &str = "var $262 = { agent: { receiveBroadcast: __agentReceive, \
     report: __agentReport, leaving: __agentLeaving, sleep: __agentSleep, \
     monotonicNow: __agentNow } };";

/// Durée maximale d'attente d'une diffusion par l'agent principal.
const BROADCAST_LIMIT: Duration = Duration::from_secs(20);

/// Corps d'un agent : exécute `source`, puis sert les diffusions jusqu'à
/// `leaving()` ou la fin du test.
fn run_agent(source: &str, hub: Arc<Hub>, inbox: &mpsc::Receiver<Broadcast>) {
    let mut vm = Vm::new();
    vm.set_budget(Some(BUDGET));
    vm.set_can_block(true);
    vm.host = Some(Box::new(Host {
        lines: Vec::new(),
        timers: Vec::new(),
        hub: Some(hub),
        agent: Some(AgentState {
            callback: None,
            leaving: false,
        }),
    }));
    let global = vm.global();
    vm.define_method(global, "__agentReceive", 1, agent_receive);
    vm.define_method(global, "__agentReport", 1, agent_report);
    vm.define_method(global, "__agentLeaving", 0, agent_leaving);
    vm.define_method(global, "__agentSleep", 1, agent_sleep);
    vm.define_method(global, "__agentNow", 0, agent_now);
    // Une erreur de l'agent se voit à l'absence de ses comptes rendus.
    let _ = vm.eval(&format!("{AGENT_OBJECT}\n{source}"));
    loop {
        if host(&mut vm)
            .and_then(|h| h.agent.as_ref())
            .is_none_or(|a| a.leaving)
        {
            return;
        }
        // Tant qu'une attente d'`Atomics.waitAsync` est en cours, l'agent
        // guette à la fois son réveil et les diffusions.
        let received = if vm.pending_async_waits() > 0 {
            match inbox.try_recv() {
                Ok(message) => Ok(message),
                Err(mpsc::TryRecvError::Empty) => {
                    vm.park_for_async_waits(LOOP_PAUSE);
                    vm.run_microtasks();
                    continue;
                }
                Err(mpsc::TryRecvError::Disconnected) => Err(()),
            }
        } else {
            inbox.recv().map_err(|_| ())
        };
        let Ok((storage, number)) = received else {
            // L'agent principal a terminé.
            return;
        };
        // La diffusion est retirée : l'agent principal peut repartir.
        if let Some(hub) = host(&mut vm).and_then(|h| h.hub.clone()) {
            let mut pending = relock(&hub.pending);
            *pending = pending.saturating_sub(1);
            drop(pending);
            hub.retrieved.notify_all();
        }
        let callback = host(&mut vm)
            .and_then(|h| h.agent.as_ref())
            .and_then(|a| a.callback.clone());
        if let Some(callback) = callback {
            let buffer = vm.shared_buffer_from_storage(&storage);
            vm.set_budget(Some(BUDGET));
            let _ = vm.call(
                &callback,
                &Value::Undefined,
                &[buffer, Value::Number(number)],
            );
            vm.run_microtasks();
        }
    }
}

/// `$262.agent.start(source)` : lance un agent sur son propre fil.
fn agent_start(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let Some(Value::String(source)) = args.first() else {
        return Err(vm.error("TypeError", "$262.agent.start : une source est attendue"));
    };
    let source = source.to_string();
    let Some(hub) = hub(vm) else {
        return Ok(Value::Undefined);
    };
    let (sender, inbox) = mpsc::channel();
    relock(&hub.agents).push(sender);
    let shared = Arc::clone(&hub);
    let spawned = std::thread::Builder::new()
        .name("test262-agent".to_owned())
        .stack_size(STACK_BYTES)
        .spawn(move || run_agent(&source, shared, &inbox));
    if spawned.is_err() {
        relock(&hub.agents).pop();
        return Err(vm.error("RangeError", "impossible de lancer un agent"));
    }
    Ok(Value::Undefined)
}

/// `$262.agent.broadcast(tampon, nombre)` : remet le tampon partagé à tous
/// les agents et attend qu'ils l'aient retiré.
fn agent_broadcast(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let buffer = args.first().cloned().unwrap_or(Value::Undefined);
    let Some(storage) = vm.shared_buffer_storage(&buffer) else {
        return Err(vm.error(
            "TypeError",
            "$262.agent.broadcast : un SharedArrayBuffer est attendu",
        ));
    };
    let number = match args.get(1) {
        Some(Value::Number(n)) => *n,
        _ => 0.0,
    };
    let Some(hub) = hub(vm) else {
        return Ok(Value::Undefined);
    };
    let delivered = {
        let mut agents = relock(&hub.agents);
        // Un agent parti ne reçoit plus rien.
        agents.retain(|agent| agent.send((storage.clone(), number)).is_ok());
        agents.len()
    };
    let deadline = Instant::now() + BROADCAST_LIMIT;
    let mut pending = relock(&hub.pending);
    *pending += delivered;
    while *pending > 0 {
        let now = Instant::now();
        if now >= deadline {
            *pending = 0;
            break;
        }
        pending = hub
            .retrieved
            .wait_timeout(pending, deadline - now)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
    Ok(Value::Undefined)
}

/// `$262.agent.getReport()` : le plus ancien compte rendu, ou `null`.
fn agent_get_report(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let report = hub(vm).and_then(|hub| relock(&hub.reports).pop_front());
    Ok(report.map_or(Value::Null, |text| Value::string(&text)))
}

/// `$262.agent.sleep(ms)`.
fn agent_sleep(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let ms = match args.first() {
        Some(value) => vm.to_number(value)?,
        None => 0.0,
    };
    if ms.is_finite() && ms > 0.0 {
        std::thread::sleep(Duration::from_secs_f64((ms / 1000.0).min(60.0)));
    }
    Ok(Value::Undefined)
}

/// `setTimeout(fonction, délai)` : le harnais s'en sert pour patienter sans
/// occuper le moteur.
fn host_set_timeout(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let callback = args.first().cloned().unwrap_or(Value::Undefined);
    let delay = match args.get(1) {
        Some(value) => vm.to_number(value)?,
        None => 0.0,
    };
    let delay = if delay.is_finite() && delay > 0.0 {
        Duration::from_secs_f64((delay / 1000.0).min(60.0))
    } else {
        Duration::ZERO
    };
    // La fonction reste vivante jusqu'à son appel.
    vm.add_root(callback.clone());
    if let Some(host) = host(vm) {
        host.timers.push(Timer {
            callback,
            due: Instant::now() + delay,
        });
    }
    Ok(Value::Undefined)
}

/// Boucle d'événements d'un test asynchrone : appelle les minuteurs échus et
/// règle les attentes d'`Atomics.waitAsync`, jusqu'à ce que `finished` soit
/// vrai, qu'il n'y ait plus rien à attendre ou que le temps imparti soit
/// écoulé.
fn run_event_loop(vm: &mut Vm, finished: impl Fn(&mut Vm) -> bool) {
    let started = Instant::now();
    loop {
        // Règle aussi les attentes asynchrones arrivées à leur terme.
        vm.run_microtasks();
        if finished(vm) || vm.was_interrupted() || started.elapsed() > LOOP_LIMIT {
            return;
        }
        let now = Instant::now();
        let due = host(vm).and_then(|h| {
            let (position, _) = h
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.due <= now)
                .min_by_key(|(_, timer)| timer.due)?;
            Some(h.timers.remove(position))
        });
        if let Some(timer) = due {
            vm.set_budget(Some(BUDGET));
            // Une erreur du minuteur se voit à l'absence de `$DONE`.
            let _ = vm.call(&timer.callback, &Value::Undefined, &[]);
            vm.remove_root(&timer.callback);
            continue;
        }
        let next = host(vm).and_then(|h| h.timers.iter().map(|timer| timer.due).min());
        let waiting = vm.pending_async_waits() > 0;
        if next.is_none() && !waiting {
            return;
        }
        let pause = next.map_or(LOOP_PAUSE, |due| {
            due.saturating_duration_since(now).min(LOOP_PAUSE)
        });
        if waiting {
            vm.park_for_async_waits(pause);
        } else {
            std::thread::sleep(pause);
        }
    }
}

/// `$262.agent.monotonicNow()` : millisecondes depuis le début du test.
fn agent_now(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let elapsed = hub(vm).map_or(0.0, |hub| hub.epoch.elapsed().as_secs_f64() * 1000.0);
    Ok(Value::Number(elapsed))
}

/// `$262.agent.receiveBroadcast(fonction)`, côté agent.
fn agent_receive(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let callback = args.first().cloned().unwrap_or(Value::Undefined);
    // La fonction doit survivre au ramasse-miettes jusqu'à la diffusion.
    vm.add_root(callback.clone());
    if let Some(agent) = host(vm).and_then(|h| h.agent.as_mut()) {
        agent.callback = Some(callback);
    }
    Ok(Value::Undefined)
}

/// `$262.agent.report(valeur)`, côté agent.
fn agent_report(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let value = args.first().cloned().unwrap_or(Value::Undefined);
    let text = vm.to_string(&value)?.to_string();
    if let Some(hub) = hub(vm) {
        relock(&hub.reports).push_back(text);
    }
    Ok(Value::Undefined)
}

/// `$262.agent.leaving()`, côté agent.
fn agent_leaving(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    if let Some(agent) = host(vm).and_then(|h| h.agent.as_mut()) {
        agent.leaving = true;
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

/// `$262.createRealm()` : un royaume neuf, rendu par son propre objet `$262`.
fn host_create_realm(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let Some(realm) = vm.create_realm() else {
        return Err(vm.error("RangeError", "trop de royaumes"));
    };
    let previous = vm.realm();
    vm.set_realm(realm);
    install_host(vm);
    let object = vm
        .eval_script(HOST_OBJECT)
        .and_then(|_| vm.eval_script("$262"));
    vm.set_realm(previous);
    object
}

/// Installe dans le royaume courant les fonctions sur lesquelles `$262`
/// s'appuie.
fn install_host(vm: &mut Vm) {
    let global = vm.global();
    vm.define_method(global, "print", 1, host_print);
    vm.define_method(global, "setTimeout", 2, host_set_timeout);
    vm.define_method(global, "__detach", 1, host_detach);
    vm.define_method(global, "__evalScript", 1, host_eval_script);
    vm.define_method(global, "__createRealm", 0, host_create_realm);
    vm.define_method(global, "__agentStart", 1, agent_start);
    vm.define_method(global, "__agentBroadcast", 2, agent_broadcast);
    vm.define_method(global, "__agentGetReport", 0, agent_get_report);
    vm.define_method(global, "__agentSleep", 1, agent_sleep);
    vm.define_method(global, "__agentNow", 0, agent_now);
}

/// Objet `$262` minimal exposé aux tests.
const HOST_OBJECT: &str = "var $262 = { global: this, gc() {}, detachArrayBuffer: __detach, \
     evalScript: __evalScript, createRealm: __createRealm, \
     agent: { start: __agentStart, broadcast: __agentBroadcast, getReport: __agentGetReport, \
     sleep: __agentSleep, monotonicNow: __agentNow } };";

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
    // L'agent principal d'un lanceur de tests peut s'endormir, sauf pour les
    // tests qui vérifient ce qui se passe quand il ne le peut pas.
    vm.set_can_block(!meta.has("CanBlockIsFalse"));
    vm.host = Some(Box::new(Host::default()));
    install_host(&mut vm);

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
        run_event_loop(&mut vm, |vm| {
            host(vm).is_some_and(|h| h.lines.iter().any(|l| l.starts_with("Test262:Async")))
        });
        let lines = host(&mut vm).map(|h| h.lines.clone()).unwrap_or_default();
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
