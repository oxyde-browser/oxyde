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
//! LIMITES CONNUES : pas d'`await` de premier niveau ni d'attributs
//! d'importation (`with { type: "json" }`) ; une déclaration
//! `export default function () {}` anonyme n'est pas hissée ; affecter une
//! liaison importée est ignoré au lieu de lever `TypeError` ; deux
//! `export *` qui fournissent le même nom ne sont pas signalés comme ambigus.

use std::sync::Arc;

use crate::bytecode::FunctionProto;
use crate::heap::{Kind, Object, Property, Slot};
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

/// Liaison importée par un module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Import {
    /// Index du module source dans [`ModuleInfo::requests`].
    pub(crate) request: usize,
    /// Nom exporté par la source ; `None` : l'espace de noms entier (`* as`).
    pub(crate) name: Option<JsStr>,
    pub(crate) local: JsStr,
}

/// Ce qu'un module importe et exporte, relevé à l'analyse.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleInfo {
    /// Spécificateurs des modules demandés, sans doublon, dans l'ordre du texte.
    pub(crate) requests: Vec<JsStr>,
    pub(crate) imports: Vec<Import>,
    /// (nom exporté, liaison locale).
    pub(crate) local_exports: Vec<(JsStr, JsStr)>,
    /// (nom exporté, module source, nom dans la source ; `None` : espace de noms).
    pub(crate) indirect_exports: Vec<(JsStr, usize, Option<JsStr>)>,
    /// Modules réexportés en bloc (`export * from`).
    pub(crate) star_exports: Vec<usize>,
}

impl ModuleInfo {
    /// Index de `specifier` dans les modules demandés, ajouté au besoin.
    pub(crate) fn request(&mut self, specifier: &str) -> usize {
        if let Some(index) = self.requests.iter().position(|r| &**r == specifier) {
            return index;
        }
        self.requests.push(Arc::from(specifier));
        self.requests.len() - 1
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
}

impl ModuleRecord {
    /// Ajoute à `out` les valeurs que le module garde en vie.
    pub(crate) fn trace(&self, out: &mut Vec<Value>) {
        out.push(Value::Object(self.scope));
        out.extend(self.namespace.map(Value::Object));
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
    Ok(info.requests.iter().map(|r| r.to_string()).collect())
}

/// Accesseur d'une liaison importée : lit `data[1]` dans la portée `data[0]`.
fn import_getter(vm: &mut Vm, data: &[Value], _this: &Value, _args: &[Value]) -> JsResult {
    let (Some(scope), Some(Value::String(name))) =
        (data.first().and_then(Value::as_object), data.get(1))
    else {
        return Ok(Value::Undefined);
    };
    if !vm.has_own(scope, name) {
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
    pub(crate) fn dynamic_import(&mut self, specifier: &Value, scope: Option<ObjId>) -> Value {
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
        match self.to_string(specifier) {
            Ok(specifier) => self.jobs.push_back(crate::builtins::promise::Job::Import {
                promise,
                specifier,
                referrer,
            }),
            Err(thrown) => self.settle(promise, true, thrown),
        }
        Value::Object(promise)
    }

    /// Microtâche d'un `import()` : règle `promise` avec l'espace de noms du
    /// module, ou la rejette avec l'erreur de chargement ou d'évaluation.
    pub(crate) fn run_import(&mut self, promise: ObjId, specifier: &JsStr, referrer: &JsStr) {
        match self.import_namespace(specifier, referrer) {
            Ok(namespace) => self.resolve_promise(promise, Value::Object(namespace)),
            Err(thrown) => self.settle(promise, true, thrown),
        }
    }

    fn import_namespace(&mut self, specifier: &JsStr, referrer: &JsStr) -> JsResult<ObjId> {
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
        let id: JsStr = Arc::from(id);
        // Un chargement ou une liaison qui échoue ne laisse aucun module à
        // moitié chargé : les importations suivantes repartent d'un état sain.
        let before = self.modules.len();
        let loaded = self
            .load_module(&id, &source, 0)
            .and_then(|()| self.link_modules());
        if let Err(thrown) = loaded {
            self.modules.truncate(before);
            return Err(thrown);
        }
        self.evaluate_module(&id, 0)?;
        self.module_namespace(&id)
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
            compiler::compile_script(&body, true).map(|proto| (proto, info))
        });
        let (proto, info) = match parsed {
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
            },
        );
        let mut deps = Vec::with_capacity(info.requests.len());
        for specifier in &info.requests {
            let Some(loader) = self.module_loader else {
                return Err(self.type_error(&format!(
                    "module « {specifier} » introuvable : aucun chargeur de modules"
                )));
            };
            let (dep_id, dep_source) = match loader(self, specifier, id) {
                Ok(loaded) => loaded,
                Err(message) => {
                    return Err(
                        self.type_error(&format!("module « {specifier} » introuvable : {message}"))
                    );
                }
            };
            let dep_id: JsStr = Arc::from(dep_id);
            self.load_module(&dep_id, &dep_source, depth + 1)?;
            deps.push(dep_id);
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
                let property = match &import.name {
                    None => Property {
                        slot: Slot::Data(Value::Object(self.module_namespace(dep)?)),
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
        if let Some(namespace) = self.modules.get(id).and_then(|r| r.namespace) {
            return Ok(namespace);
        }
        let namespace = self.heap.alloc(Object::new(None, Kind::Ordinary));
        // Enregistré avant de résoudre les exports : un cycle d'importations
        // retrouve ce même objet.
        match self.modules.get_mut(id) {
            Some(record) => record.namespace = Some(namespace),
            None => return Err(self.type_error(&format!("module « {id} » non chargé"))),
        }
        let mut names = Vec::new();
        self.export_names(id, &mut Vec::new(), &mut names);
        names.sort();
        for name in names {
            if let Some((scope, local)) = self.resolve_export(id, &name, &mut Vec::new())? {
                let property = self.live_binding(scope, &local);
                self.heap.get_mut(namespace).props.insert(name, property);
            }
        }
        let tag = self.well_known.to_string_tag.clone();
        self.heap.get_mut(namespace).insert_prop(
            PropertyKey::Symbol(tag),
            Property {
                slot: Slot::Data(Value::string("Module")),
                writable: false,
                enumerable: false,
                configurable: false,
            },
        );
        self.heap.get_mut(namespace).extensible = false;
        Ok(namespace)
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
        let mut result = Ok(());
        for dep in &deps {
            result = self.evaluate_module(dep, depth + 1);
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
                .map(|_| ());
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
