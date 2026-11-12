//! Modules ECMAScript : chargement, liaison et évaluation.
//!
//! Chaque module possède une portée, un objet dont le prototype est l'objet
//! global : ses déclarations de premier niveau en sont les propriétés. Une
//! liaison importée est un accesseur qui lit la propriété correspondante dans
//! la portée du module exportateur ; elle reste donc « vivante », comme
//! l'exige le langage.
//!
//! L'hôte fournit le texte des modules par un [`ModuleLoader`].
//!
//! `import()` charge le module dans une microtâche : la promesse rendue
//! n'est jamais réglée pendant l'appel.
//!
//! LIMITES CONNUES : pas d'attributs
//! d'importation (`with { type: "json" }`) ; une déclaration
//! `export default function () {}` anonyme n'est pas hissée ; affecter une
//! liaison importée est ignoré au lieu de lever `TypeError` ; deux
//! `export *` qui fournissent le même nom ne sont pas signalés comme ambigus.

use std::sync::Arc;

use crate::bytecode::FunctionProto;
use crate::heap::{Kind, Object, PromiseState, Property, Slot};
use crate::lexer::SyntaxError;
use crate::value::{JsStr, ObjId, PropertyKey, Value};
use crate::vm::{JsResult, Vm};
use crate::{EvalError, compiler, parser};

/// Fournit un module à la machine : reçoit le spécificateur écrit dans
/// `import … from "spécificateur"` et l'identifiant du module qui l'importe ;
/// rend l'identifiant du module importé (son adresse résolue) et son texte.
pub type ModuleLoader = fn(&mut Vm, &str, &str) -> Result<(String, String), String>;

/// Nombre maximal de modules chargés dans une machine.
const MAX_MODULES: usize = 10_000;
/// Profondeur maximale d'une chaîne d'importations.
const MAX_DEPTH: usize = 400;

/// Sépare, dans une demande de module, le spécificateur de son type.
const TYPE_MARK: char = '\u{1}';

/// Demande du module `specifier` avec l'attribut `type: kind`.
pub(crate) fn typed_request(specifier: &str, kind: &str) -> String {
    format!("{specifier}{TYPE_MARK}{kind}")
}

/// Spécificateur et type (attribut `type`) d'une demande de module.
fn split_request(request: &str) -> (&str, Option<&str>) {
    match request.split_once(TYPE_MARK) {
        Some((specifier, kind)) => (specifier, Some(kind)),
        None => (request, None),
    }
}

/// Liaison importée par un module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Import {
    /// Index du module source dans [`ModuleInfo::requests`].
    pub(crate) request: usize,
    /// Nom exporté par la source ; `None` : l'espace de noms entier (`* as`).
    pub(crate) name: Option<JsStr>,
    pub(crate) local: JsStr,
    /// 0 : importation ordinaire ; 1 : `import source` ; 2 : `import defer`.
    pub(crate) phase: u8,
}

/// Ce qu'un module importe et exporte, relevé à l'analyse.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleInfo {
    /// Spécificateurs des modules demandés, sans doublon, dans l'ordre du texte.
    pub(crate) requests: Vec<JsStr>,
    /// Pour chaque demande, le nombre de fois où elle est faite, et parmi
    /// elles le nombre d'`import defer`.
    pub(crate) uses: Vec<u32>,
    pub(crate) deferred: Vec<u32>,
    pub(crate) imports: Vec<Import>,
    /// (nom exporté, liaison locale).
    pub(crate) local_exports: Vec<(JsStr, JsStr)>,
    /// (nom exporté, module source, nom dans la source ; `None` : espace de noms).
    pub(crate) indirect_exports: Vec<(JsStr, usize, Option<JsStr>)>,
    /// Modules réexportés en bloc (`export * from`).
    pub(crate) star_exports: Vec<usize>,
    /// Le module emploie `await` au premier niveau : son corps s'exécute
    /// comme une fonction async.
    pub(crate) has_top_level_await: bool,
}

impl ModuleInfo {
    /// Index de `specifier` dans les modules demandés, ajouté au besoin.
    pub(crate) fn request(&mut self, specifier: &str) -> usize {
        if let Some(index) = self.requests.iter().position(|r| &**r == specifier) {
            if let Some(count) = self.uses.get_mut(index) {
                *count += 1;
            }
            return index;
        }
        self.requests.push(Arc::from(specifier));
        self.uses.push(1);
        self.deferred.push(0);
        self.requests.len() - 1
    }

    /// Vrai si la demande `index` n'est faite que par des `import defer` :
    /// le module n'est alors évalué qu'à son premier usage.
    pub(crate) fn defer_only(&self, index: usize) -> bool {
        let deferred = self.deferred.get(index).copied().unwrap_or(0);
        deferred > 0 && self.uses.get(index).copied() == Some(deferred)
    }

    /// Vrai si `name` est déjà exporté par ce module.
    pub(crate) fn exports(&self, name: &str) -> bool {
        self.local_exports.iter().any(|(n, _)| &**n == name)
            || self.indirect_exports.iter().any(|(n, _, _)| &**n == name)
    }
}

#[derive(Clone, Debug)]
enum ModuleState {
    Loaded,
    Linked,
    Evaluating,
    Evaluated,
    Failed(Value),
}

/// Module chargé dans une machine.
#[derive(Debug)]
pub(crate) struct ModuleRecord {
    scope: ObjId,
    info: Arc<ModuleInfo>,
    /// Code du module ; consommé par son évaluation.
    proto: Option<Arc<FunctionProto>>,
    /// Identifiants des modules demandés, dans l'ordre de `info.requests`.
    deps: Vec<JsStr>,
    state: ModuleState,
    namespace: Option<ObjId>,
    /// Espace de noms différé (`import defer`), distinct de l'ordinaire.
    deferred_namespace: Option<ObjId>,
}

impl ModuleRecord {
    /// Ajoute à `out` les valeurs que le module garde en vie.
    pub(crate) fn trace(&self, out: &mut Vec<Value>) {
        out.push(Value::Object(self.scope));
        out.extend(self.namespace.map(Value::Object));
        out.extend(self.deferred_namespace.map(Value::Object));
        if let ModuleState::Failed(value) = &self.state {
            out.push(value.clone());
        }
    }
}

/// Spécificateurs des modules qu'importe statiquement `source`, dans l'ordre
/// du texte et sans doublon. Sert à l'hôte pour précharger un graphe de modules.
///
/// # Errors
/// L'erreur de syntaxe de `source`.
pub fn module_requests(source: &str) -> Result<Vec<String>, SyntaxError> {
    let (_, info) = parser::parse_module(source)?;
    // Sans doublon : un même fichier peut être demandé sous deux types.
    let mut requests: Vec<String> = Vec::new();
    for request in &info.requests {
        let specifier = split_request(request).0.to_owned();
        if !requests.contains(&specifier) {
            requests.push(specifier);
        }
    }
    Ok(requests)
}

/// Accesseur d'une liaison importée : lit `data[1]` dans la portée `data[0]`.
fn import_getter(vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    let (Some(scope), Some(Value::String(name))) =
        (data.first().and_then(Value::as_object), data.get(1))
    else {
        return Ok(Value::Undefined);
    };
    let dead = matches!(
        vm.heap.get(scope).props.get(&**name).map(|p| &p.slot),
        None | Some(Slot::Data(Value::Hole))
    );
    if dead {
        return Err(vm.error(
            "ReferenceError",
            &format!("{name} est utilisé avant son initialisation"),
        ));
    }
    vm.get(&Value::Object(scope), name)
}

impl Vm {
    /// Fixe l'identifiant des scripts classiques : c'est le référent passé au
    /// chargeur de modules pour leurs `import()`.
    pub fn set_script_url(&mut self, url: &str) {
        self.script_url = Arc::from(url);
    }

    /// `import(specifier)` depuis du code dont la portée de module est
    /// `scope` (`None` : script classique). Rend la promesse de l'espace de
    /// noms ; le chargement se fait dans une microtâche.
    pub(crate) fn dynamic_import(
        &mut self,
        specifier: &Value,
        scope: Option<ObjId>,
        source: u8,
        options: &Value,
    ) -> Value {
        let promise = self.new_promise();
        let meta = scope.and_then(|scope| {
            match self
                .heap
                .get(scope)
                .props
                .get("import.meta")
                .map(|p| &p.slot)
            {
                Some(Slot::Data(meta)) => Some(meta.clone()),
                _ => None,
            }
        });
        let url = match meta {
            Some(meta) => self.get(&meta, "url").ok(),
            None => None,
        };
        let referrer = match url {
            Some(Value::String(url)) => url,
            _ => Arc::clone(&self.script_url),
        };
        let request = self.to_string(specifier).and_then(|specifier| {
            Ok(match self.import_type(options)? {
                Some(kind) => Arc::from(typed_request(&specifier, &kind)),
                None => specifier,
            })
        });
        match request {
            Ok(specifier) => self.jobs.push_back(crate::builtins::promise::Job::Import {
                promise,
                specifier,
                referrer,
                source,
            }),
            Err(thrown) => self.settle(promise, true, thrown),
        }
        Value::Object(promise)
    }

    /// Attribut `type` du second argument d'`import()` : `{ with: { type } }`.
    /// Tout y est vérifié : les options et `with` sont des objets, les
    /// valeurs des chaînes, et `type` est le seul attribut reconnu.
    fn import_type(&mut self, options: &Value) -> JsResult<Option<JsStr>> {
        if *options == Value::Undefined {
            return Ok(None);
        }
        if options.as_object().is_none() {
            return Err(self.type_error("import() : les options doivent être un objet"));
        }
        let attributes = self.get(options, "with")?;
        if attributes == Value::Undefined {
            return Ok(None);
        }
        let Some(id) = attributes.as_object() else {
            return Err(self.type_error("import() : « with » doit être un objet"));
        };
        let mut kind = None;
        let mut unsupported = None;
        for key in self.enumerable_own_keys(id)? {
            let Value::String(value) = self.get(&attributes, &key)? else {
                return Err(self.type_error(
                    "import() : la valeur d'un attribut d'importation doit être une chaîne",
                ));
            };
            if &*key == "type" {
                kind = Some(value);
            } else {
                unsupported.get_or_insert(key);
            }
        }
        if let Some(key) = unsupported {
            return Err(self.type_error(&format!(
                "import() : attribut d'importation « {key} » non pris en charge"
            )));
        }
        Ok(kind)
    }

    /// Microtâche d'un `import()` : règle `promise` avec l'espace de noms du
    /// module, ou la rejette avec l'erreur de chargement ou d'évaluation.
    ///
    /// `source` : `import.source(…)`. Le module est chargé, puis la promesse
    /// est rejetée : un module écrit en JavaScript n'a pas de source à
    /// rendre (ECMA-262, `GetModuleSource`).
    pub(crate) fn run_import(
        &mut self,
        promise: ObjId,
        specifier: &JsStr,
        referrer: &JsStr,
        source: u8,
    ) {
        if source == 2 {
            // `import.defer(…)` : chargé et lié, évalué au premier usage.
            let before = self.modules.len();
            let loaded = self
                .load_request(specifier, referrer, 0)
                .and_then(|id| self.link_modules().map(|()| id))
                .and_then(|id| {
                    self.evaluate_async_parts(&id, &mut Vec::new(), 0)?;
                    self.module_namespace_of(&id, true)
                });
            match loaded {
                Ok(namespace) => self.resolve_promise(promise, Value::Object(namespace)),
                Err(thrown) => {
                    self.modules.truncate(before);
                    self.settle(promise, true, thrown);
                }
            }
            return;
        }
        if source == 1 {
            let thrown = match self.load_only(specifier, referrer) {
                Ok(()) => self.error(
                    "SyntaxError",
                    "import.source : ce module n'a pas de source à fournir",
                ),
                Err(thrown) => thrown,
            };
            self.settle(promise, true, thrown);
            return;
        }
        match self.import_namespace(specifier, referrer) {
            Ok(namespace) => self.resolve_promise(promise, Value::Object(namespace)),
            Err(thrown) => self.settle(promise, true, thrown),
        }
    }

    /// Charge le module que désigne `request` (un spécificateur, suivi le cas
    /// échéant de son type) depuis `referrer`, et rend sa clé. Avec le type
    /// `json` ou `text`, le module est synthétique : son seul export,
    /// `default`, est la valeur du fichier.
    fn load_request(&mut self, request: &str, referrer: &str, depth: usize) -> JsResult<JsStr> {
        let (specifier, kind) = split_request(request);
        let Some(loader) = self.module_loader else {
            return Err(self.type_error(&format!(
                "module « {specifier} » introuvable : aucun chargeur de modules"
            )));
        };
        let (id, source) = match loader(self, specifier, referrer) {
            Ok(loaded) => loaded,
            Err(message) => {
                return Err(
                    self.type_error(&format!("module « {specifier} » introuvable : {message}"))
                );
            }
        };
        let Some(kind) = kind else {
            let id: JsStr = Arc::from(id);
            self.load_module(&id, &source, depth)?;
            return Ok(id);
        };
        let key: JsStr = Arc::from(typed_request(&id, kind));
        if self.modules.contains_key(&key) {
            return Ok(key);
        }
        if self.modules.len() >= MAX_MODULES {
            return Err(self.error("RangeError", "trop de modules imbriqués ou chargés"));
        }
        let value = match kind {
            "json" => self.parse_json_text(&source)?,
            "text" => Value::string(&source),
            other => {
                return Err(self.type_error(&format!(
                    "module « {specifier} » : type « {other} » non pris en charge"
                )));
            }
        };
        let scope = self.heap.alloc(Object::new(None, Kind::Ordinary));
        self.define_hidden(scope, parser::DEFAULT_EXPORT, value);
        let info = ModuleInfo {
            local_exports: vec![(Arc::from("default"), Arc::from(parser::DEFAULT_EXPORT))],
            ..ModuleInfo::default()
        };
        self.modules.insert(
            Arc::clone(&key),
            ModuleRecord {
                scope,
                info: Arc::new(info),
                proto: None,
                deps: Vec::new(),
                state: ModuleState::Loaded,
                namespace: None,
                deferred_namespace: None,
            },
        );
        Ok(key)
    }

    /// Charge et lie le module `specifier`, sans l'évaluer.
    fn load_only(&mut self, specifier: &JsStr, referrer: &JsStr) -> JsResult<()> {
        let before = self.modules.len();
        let loaded = self
            .load_request(specifier, referrer, 0)
            .and_then(|_| self.link_modules());
        if loaded.is_err() {
            self.modules.truncate(before);
        }
        loaded
    }

    pub(crate) fn import_namespace(
        &mut self,
        specifier: &JsStr,
        referrer: &JsStr,
    ) -> JsResult<ObjId> {
        // Un chargement ou une liaison qui échoue ne laisse aucun module à
        // moitié chargé : les importations suivantes repartent d'un état sain.
        let before = self.modules.len();
        let loaded = self.load_request(specifier, referrer, 0);
        let id = match loaded.and_then(|id| self.link_modules().map(|()| id)) {
            Ok(id) => id,
            Err(thrown) => {
                self.modules.truncate(before);
                return Err(thrown);
            }
        };
        self.evaluate_module(&id, 0)?;
        self.module_namespace(&id)
    }

    /// Attend la fin d'un module qui emploie `await` au premier niveau :
    /// `completion` est alors la promesse de son corps, et les microtâches
    /// sont exécutées jusqu'à ce qu'elle soit réglée.
    ///
    /// LIMITE : les modules qui l'importent attendent sa fin, mais ses
    /// modules frères aussi, alors que le standard les laisse s'exécuter.
    fn await_module(&mut self, completion: &Value) -> JsResult<()> {
        let Some(promise) = completion.as_object() else {
            return Ok(());
        };
        if let Kind::Promise(data) = &mut self.heap.get_mut(promise).kind {
            data.handled = true;
        }
        loop {
            match &self.heap.get(promise).kind {
                Kind::Promise(data) => match &data.state {
                    PromiseState::Pending => {}
                    PromiseState::Fulfilled(_) => return Ok(()),
                    PromiseState::Rejected(reason) => return Err(reason.clone()),
                },
                _ => return Ok(()),
            }
            if self.is_interrupted() {
                return Ok(());
            }
            let Some(job) = self.jobs.pop_front() else {
                return Ok(());
            };
            self.without_gc(|vm| vm.run_job(job));
        }
    }

    /// Installe la fonction qui fournit le texte des modules importés.
    pub fn set_module_loader(&mut self, loader: ModuleLoader) {
        self.module_loader = Some(loader);
    }

    /// Charge `source` comme le module d'identifiant `id`, avec tous les
    /// modules qu'il importe, puis les évalue dans l'ordre de leurs
    /// dépendances. Un module déjà évalué ne l'est pas une seconde fois.
    ///
    /// # Errors
    /// [`EvalError::Syntax`] si `source` est invalide, [`EvalError::Thrown`]
    /// pour toute autre erreur (module introuvable, export manquant,
    /// exception levée par un module).
    pub fn eval_module(&mut self, source: &str, id: &str) -> Result<(), EvalError> {
        let id: JsStr = Arc::from(id);
        let result = match parser::parse_module(source) {
            Err(error) if !self.modules.contains_key(&id) => Err(EvalError::Syntax(error)),
            _ => self
                .load_module(&id, source, 0)
                .and_then(|()| self.link_modules())
                .and_then(|()| self.evaluate_module(&id, 0))
                .map_err(|value| {
                    let message = self.display(&value);
                    EvalError::Thrown { value, message }
                }),
        };
        // Fin de la tâche : les réactions de promesses en attente s'exécutent.
        self.run_microtasks();
        result
    }

    /// Analyse et enregistre le module `id` et, récursivement, ses dépendances.
    fn load_module(&mut self, id: &JsStr, source: &str, depth: usize) -> JsResult<()> {
        if self.modules.contains_key(id) {
            return Ok(());
        }
        if self.modules.len() >= MAX_MODULES || depth > MAX_DEPTH {
            return Err(self.error("RangeError", "trop de modules imbriqués ou chargés"));
        }
        let parsed = parser::parse_module(source).and_then(|(body, info)| {
            // Liaisons de premier niveau, créées dès le chargement : les
            // `let`, `const` et classes en zone morte, les `var` à `undefined`.
            let mut lexical = Vec::new();
            compiler::lexical_names(&body, &mut lexical);
            let mut vars = Vec::new();
            compiler::var_names(&body, &mut vars);
            compiler::compile_script(&body, true, info.has_top_level_await, true, false)
                .map(|proto| (proto, info, lexical, vars))
        });
        let (proto, info, lexical, vars) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                return Err(self.error("SyntaxError", &format!("module {id} : {}", error.message)));
            }
        };
        // La portée hérite de l'objet global : les noms libres y sont trouvés.
        let scope = self
            .heap
            .alloc(Object::new(Some(self.global), Kind::Ordinary));
        let meta = Value::Object(self.heap.alloc(Object::new(None, Kind::Ordinary)));
        self.set(&meta, "url", Value::String(Arc::clone(id)))?;
        self.define_hidden(scope, "import.meta", meta);
        let props = &mut self.heap.get_mut(scope).props;
        for name in vars {
            props.insert(name, Property::data(Value::Undefined));
        }
        for name in lexical {
            props.insert(name, Property::data(Value::Hole));
        }
        let info = Arc::new(info);
        self.modules.insert(
            Arc::clone(id),
            ModuleRecord {
                scope,
                info: Arc::clone(&info),
                proto: Some(proto),
                deps: Vec::new(),
                state: ModuleState::Loaded,
                namespace: None,
                deferred_namespace: None,
            },
        );
        let mut deps = Vec::with_capacity(info.requests.len());
        for request in &info.requests {
            deps.push(self.load_request(request, id, depth + 1)?);
        }
        if let Some(record) = self.modules.get_mut(id) {
            record.deps = deps;
        }
        Ok(())
    }

    /// Installe les liaisons importées de tous les modules chargés mais pas
    /// encore liés.
    fn link_modules(&mut self) -> JsResult<()> {
        let pending: Vec<JsStr> = self
            .modules
            .iter()
            .filter(|(_, record)| matches!(record.state, ModuleState::Loaded))
            .map(|(id, _)| Arc::clone(id))
            .collect();
        for id in &pending {
            let Some(record) = self.modules.get(id) else {
                continue;
            };
            let (scope, info, deps) = (record.scope, Arc::clone(&record.info), record.deps.clone());
            for import in &info.imports {
                let Some(dep) = deps.get(import.request) else {
                    return Err(self.type_error(&format!("module {id} : dépendance non chargée")));
                };
                if import.phase == 1 {
                    return Err(self.error(
                        "SyntaxError",
                        &format!("import source : le module « {dep} » n'a pas de source à fournir"),
                    ));
                }
                let property = match &import.name {
                    None => Property {
                        slot: Slot::Data(Value::Object(
                            self.module_namespace_of(dep, import.phase == 2)?,
                        )),
                        writable: false,
                        enumerable: true,
                        configurable: false,
                    },
                    Some(name) => {
                        let Some((source, local)) =
                            self.resolve_export(dep, name, &mut Vec::new())?
                        else {
                            return Err(self.error(
                                "SyntaxError",
                                &format!("le module « {dep} » n'exporte pas « {name} »"),
                            ));
                        };
                        self.live_binding(source, &local)
                    }
                };
                self.heap
                    .get_mut(scope)
                    .props
                    .insert(Arc::clone(&import.local), property);
            }
            // Les réexportations doivent aussi désigner un export existant.
            for (exported, _, _) in &info.indirect_exports {
                if self
                    .resolve_export(id, exported, &mut Vec::new())?
                    .is_none()
                {
                    return Err(self.error(
                        "SyntaxError",
                        &format!("le module « {id} » réexporte « {exported} », qui n'existe pas"),
                    ));
                }
            }
        }
        for id in &pending {
            if let Some(record) = self.modules.get_mut(id) {
                record.state = ModuleState::Linked;
            }
        }
        Ok(())
    }

    /// Propriété accesseur qui lit `local` dans la portée `scope`.
    fn live_binding(&mut self, scope: ObjId, local: &JsStr) -> Property {
        let getter = self.new_native_data(
            import_getter,
            vec![Value::Object(scope), Value::String(Arc::clone(local))],
        );
        Property {
            slot: Slot::Accessor {
                get: getter.as_object(),
                set: None,
            },
            writable: true,
            enumerable: true,
            configurable: false,
        }
    }

    /// Liaison que désigne l'export `name` du module `id` : portée et nom
    /// local, après avoir suivi les réexportations. `seen` coupe les cycles.
    fn resolve_export(
        &mut self,
        id: &JsStr,
        name: &JsStr,
        seen: &mut Vec<(JsStr, JsStr)>,
    ) -> JsResult<Option<(ObjId, JsStr)>> {
        if seen.iter().any(|(i, n)| i == id && n == name) {
            return Ok(None);
        }
        seen.push((Arc::clone(id), Arc::clone(name)));
        let Some(record) = self.modules.get(id) else {
            return Ok(None);
        };
        let (scope, info, deps) = (record.scope, Arc::clone(&record.info), record.deps.clone());
        if let Some((_, local)) = info.local_exports.iter().find(|(n, _)| n == name) {
            return Ok(Some((scope, Arc::clone(local))));
        }
        if let Some((_, request, imported)) =
            info.indirect_exports.iter().find(|(n, _, _)| n == name)
        {
            let Some(dep) = deps.get(*request) else {
                return Ok(None);
            };
            return match imported {
                Some(imported) => self.resolve_export(dep, imported, seen),
                None => {
                    // `export * as ns from` : l'espace de noms est rangé dans
                    // la portée du module sous un nom qu'un script ne peut écrire.
                    let namespace = self.module_namespace(dep)?;
                    let key: JsStr = Arc::from(format!("*ns*{name}"));
                    self.define_hidden(scope, &key, Value::Object(namespace));
                    Ok(Some((scope, key)))
                }
            };
        }
        if &**name == "default" {
            return Ok(None);
        }
        for request in &info.star_exports {
            if let Some(dep) = deps.get(*request)
                && let Some(found) = self.resolve_export(dep, name, seen)?
            {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    /// Noms exportés par le module `id`, réexportations en bloc comprises.
    fn export_names(&self, id: &JsStr, seen: &mut Vec<JsStr>, out: &mut Vec<JsStr>) {
        if seen.contains(id) {
            return;
        }
        seen.push(Arc::clone(id));
        let Some(record) = self.modules.get(id) else {
            return;
        };
        let names = record
            .info
            .local_exports
            .iter()
            .map(|(name, _)| name)
            .chain(record.info.indirect_exports.iter().map(|(name, _, _)| name));
        for name in names {
            if !out.contains(name) {
                out.push(Arc::clone(name));
            }
        }
        for request in &record.info.star_exports {
            if let Some(dep) = record.deps.get(*request) {
                let mut inner = Vec::new();
                self.export_names(dep, seen, &mut inner);
                for name in inner {
                    if &*name != "default" && !out.contains(&name) {
                        out.push(name);
                    }
                }
            }
        }
    }

    /// Objet espace de noms du module `id` (`import * as ns`), créé au
    /// premier besoin.
    fn module_namespace(&mut self, id: &JsStr) -> JsResult<ObjId> {
        self.module_namespace_of(id, false)
    }

    /// Espace de noms du module `id`. `deferred` : celui d'`import defer`,
    /// objet distinct, qui n'expose pas `then` et dont le premier usage
    /// d'une propriété évalue le module.
    fn module_namespace_of(&mut self, id: &JsStr, deferred: bool) -> JsResult<ObjId> {
        let known = self.modules.get(id).and_then(|r| {
            if deferred {
                r.deferred_namespace
            } else {
                r.namespace
            }
        });
        if let Some(namespace) = known {
            return Ok(namespace);
        }
        let namespace = self.heap.alloc(Object::new(None, Kind::Ordinary));
        // Enregistré avant de résoudre les exports : un cycle d'importations
        // retrouve ce même objet.
        match self.modules.get_mut(id) {
            Some(record) if deferred => record.deferred_namespace = Some(namespace),
            Some(record) => record.namespace = Some(namespace),
            None => return Err(self.type_error(&format!("module « {id} » non chargé"))),
        }
        if deferred {
            self.deferred_namespaces.insert(namespace, Arc::clone(id));
        }
        let mut names = Vec::new();
        self.export_names(id, &mut Vec::new(), &mut names);
        names.sort();
        for name in names {
            if deferred && &*name == "then" {
                continue;
            }
            if let Some((scope, local)) = self.resolve_export(id, &name, &mut Vec::new())? {
                let property = self.live_binding(scope, &local);
                self.heap.get_mut(namespace).props.insert(name, property);
            }
        }
        let tag = self.well_known.to_string_tag.clone();
        self.heap.get_mut(namespace).insert_prop(
            PropertyKey::Symbol(tag),
            Property {
                slot: Slot::Data(Value::string(if deferred {
                    "Deferred Module"
                } else {
                    "Module"
                })),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        );
        self.heap.get_mut(namespace).extensible = false;
        Ok(namespace)
    }

    /// Évalue, dans le graphe d'un module différé, ce qui ne peut pas
    /// attendre : les modules qui emploient `await` au premier niveau, que
    /// l'usage synchrone d'un espace de noms ne saurait lancer.
    fn evaluate_async_parts(
        &mut self,
        id: &JsStr,
        seen: &mut Vec<JsStr>,
        depth: usize,
    ) -> JsResult<()> {
        if seen.contains(id) {
            return Ok(());
        }
        seen.push(Arc::clone(id));
        let Some(record) = self.modules.get(id) else {
            return Ok(());
        };
        if !matches!(record.state, ModuleState::Loaded | ModuleState::Linked) {
            return Ok(());
        }
        let (info, deps) = (Arc::clone(&record.info), record.deps.clone());
        if info.has_top_level_await {
            return self.evaluate_module(id, depth);
        }
        for (index, dep) in deps.iter().enumerate() {
            if !info.defer_only(index) {
                self.evaluate_async_parts(dep, seen, depth + 1)?;
            }
        }
        Ok(())
    }

    /// Vrai si le module `id` ou l'une de ses dépendances est en cours
    /// d'évaluation.
    fn evaluating(&self, id: &JsStr, seen: &mut Vec<JsStr>) -> bool {
        if seen.contains(id) {
            return false;
        }
        seen.push(Arc::clone(id));
        let Some(record) = self.modules.get(id) else {
            return false;
        };
        match record.state {
            ModuleState::Evaluating => true,
            ModuleState::Evaluated | ModuleState::Failed(_) => false,
            ModuleState::Loaded | ModuleState::Linked => {
                let deps = record.deps.clone();
                deps.iter().any(|dep| self.evaluating(dep, seen))
            }
        }
    }

    /// Usage d'un espace de noms différé : `object` en est-il un, et `key`
    /// (`None` : l'énumération des clés) déclenche-t-elle l'évaluation de
    /// son module ? Ni un symbole ni `then` ne la déclenchent.
    ///
    /// # Errors
    /// `TypeError` si le module est en cours d'évaluation ; ce que lève le
    /// module.
    pub(crate) fn touch_deferred(&mut self, object: ObjId, key: Option<&str>) -> JsResult<()> {
        // `Temporal` se construit au premier usage de la propriété globale.
        if !self.lazy_temporal.is_empty()
            && matches!(key, None | Some("Temporal"))
            && self.lazy_temporal.contains(&object)
        {
            self.materialize_temporal(object)?;
        }
        if self.deferred_namespaces.is_empty() || key == Some("then") {
            return Ok(());
        }
        let Some(id) = self.deferred_namespaces.get(&object).cloned() else {
            return Ok(());
        };
        // Un identifiant d'objet réemployé n'est plus cet espace de noms.
        let live = self
            .modules
            .get(&id)
            .is_some_and(|record| record.deferred_namespace == Some(object));
        if !live {
            self.deferred_namespaces.remove(&object);
            return Ok(());
        }
        if self.evaluating(&id, &mut Vec::new()) {
            return Err(self.type_error(&format!(
                "le module différé « {id} » est en cours d'évaluation"
            )));
        }
        self.evaluate_module(&id, 0)?;
        self.deferred_namespaces.remove(&object);
        Ok(())
    }

    /// Évalue le module `id` après ses dépendances.
    fn evaluate_module(&mut self, id: &JsStr, depth: usize) -> JsResult<()> {
        let Some(record) = self.modules.get_mut(id) else {
            return Err(self.type_error(&format!("module « {id} » non chargé")));
        };
        match &record.state {
            // Déjà évalué, ou en cours (cycle d'importations) : rien à faire.
            ModuleState::Evaluated | ModuleState::Evaluating => return Ok(()),
            ModuleState::Failed(thrown) => return Err(thrown.clone()),
            ModuleState::Loaded | ModuleState::Linked => {}
        }
        if depth > MAX_DEPTH {
            return Err(self.error("RangeError", "modules trop imbriqués"));
        }
        record.state = ModuleState::Evaluating;
        let (scope, deps, proto) = (record.scope, record.deps.clone(), record.proto.take());
        let info = Arc::clone(&record.info);
        let mut result = Ok(());
        for (index, dep) in deps.iter().enumerate() {
            // Demandée seulement par `import defer` : évaluée à son premier usage.
            result = if info.defer_only(index) {
                self.evaluate_async_parts(dep, &mut Vec::new(), depth + 1)
            } else {
                self.evaluate_module(dep, depth + 1)
            };
            if result.is_err() {
                break;
            }
        }
        if result.is_ok()
            && let Some(proto) = proto
        {
            // Dans un module, `this` vaut `undefined` au premier niveau.
            result = self
                .run_top_level(proto, Some(scope), Value::Undefined)
                .and_then(|completion| self.await_module(&completion));
        }
        if let Some(record) = self.modules.get_mut(id) {
            record.state = match &result {
                Ok(()) => ModuleState::Evaluated,
                Err(thrown) => ModuleState::Failed(thrown.clone()),
            };
        }
        result
    }
}
