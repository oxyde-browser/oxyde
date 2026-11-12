//! Événements du DOM (DOM Standard §2) : `Event`, `CustomEvent`,
//! `EventTarget` et l'algorithme de distribution, avec ses phases de capture,
//! de cible et de bouillonnement.
//!
//! L'état d'un événement vit dans des emplacements internes de son objet
//! JavaScript, que le ramasse-miettes libère avec lui.
//!
//! LIMITES CONNUES : pas d'arbre fantôme (ni reciblage ni `composed`
//! effectif), pas de comportement d'activation, pas d'`AbortSignal` dans les
//! options d'écoute ; un gestionnaire `on<type>` est appelé après les
//! écouteurs de sa cible, quel que soit le moment où il a été posé.

use oxyde_dom::NodeId;
use oxyde_js::{JsResult, NativeFn, ObjId, Value, Vm};

use super::{arg, host, node_of, report, wrap};

/// Cible d'un écouteur ou d'une distribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// L'objet global.
    Window,
    /// Un nœud du document, par son index.
    Node(u32),
    /// Un objet `EventTarget` créé par un script.
    Object(ObjId),
}

/// Écouteur enregistré par `addEventListener`.
pub(super) struct Listener {
    /// Identité de l'enregistrement, pour savoir s'il a été retiré pendant
    /// une distribution.
    id: u64,
    target: Target,
    kind: String,
    callback: Value,
    capture: bool,
    once: bool,
    /// Écouteur passif : son `preventDefault()` est sans effet.
    passive: bool,
}

const NONE: f64 = 0.0;
const CAPTURING_PHASE: f64 = 1.0;
const AT_TARGET: f64 = 2.0;
const BUBBLING_PHASE: f64 = 3.0;

// Emplacements internes d'un événement.
const TYPE: &str = "événement.type";
const TARGET: &str = "événement.cible";
const CURRENT: &str = "événement.cible courante";
const PHASE: &str = "événement.phase";
const BUBBLES: &str = "événement.bouillonne";
const CANCELABLE: &str = "événement.annulable";
const COMPOSED: &str = "événement.composé";
const CANCELED: &str = "événement.annulé";
const STOP: &str = "événement.arrêt";
const IN_PASSIVE: &str = "événement.écouteur passif";
const STOP_NOW: &str = "événement.arrêt immédiat";
const DISPATCHING: &str = "événement.en distribution";
const TRUSTED: &str = "événement.de confiance";
const TIME: &str = "événement.instant";
const PATH: &str = "événement.chemin";
const DETAIL: &str = "événement.détail";

fn target_of(vm: &mut Vm, value: &Value) -> Target {
    if let Some(node) = node_of(vm, value) {
        return Target::Node(u32::try_from(node.index()).unwrap_or(u32::MAX));
    }
    // La fenêtre de la page est `Target::Window` ; celle d'un cadre est
    // désignée par son objet global.
    let object = match value {
        Value::Object(id) => *id,
        _ => vm.global(),
    };
    if vm.realm_global(0) == Some(object) {
        Target::Window
    } else {
        Target::Object(object)
    }
}

fn target_value(vm: &mut Vm, target: Target) -> Value {
    match target {
        Target::Window => vm.realm_global(0).map_or(Value::Null, Value::Object),
        Target::Object(id) => Value::Object(id),
        Target::Node(index) => match host(vm).and_then(|h| h.doc.id_from_index(index as usize)) {
            Some(node) => wrap(vm, node),
            None => Value::Null,
        },
    }
}

// ---- Écouteurs ---------------------------------------------------------------------------

/// Option booléenne `name` d'un troisième argument d'écoute : un booléen
/// vaut `capture`, un objet porte ses options.
fn listen_option(vm: &mut Vm, options: &Value, name: &str) -> JsResult<bool> {
    match options {
        Value::Object(_) => Ok(vm.get(options, name)?.truthy()),
        other => Ok(name == "capture" && other.truthy()),
    }
}

pub(super) fn add_event_listener(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?.to_string();
    let callback = arg(args, 1);
    let options = arg(args, 2);
    let capture = listen_option(vm, &options, "capture")?;
    let once = listen_option(vm, &options, "once")?;
    let passive = match &options {
        Value::Object(_) => match vm.get(&options, "passive")? {
            Value::Undefined => None,
            other => Some(other.truthy()),
        },
        _ => None,
    };
    if callback.as_object().is_none() {
        return Ok(Value::Undefined);
    }
    let target = target_of(vm, this);
    // Par défaut, les écouteurs de défilement tactile posés sur la fenêtre,
    // le document, son élément racine ou son corps sont passifs.
    let passive = passive.unwrap_or_else(|| {
        matches!(
            kind.as_str(),
            "touchstart" | "touchmove" | "wheel" | "mousewheel"
        ) && match target {
            Target::Window => true,
            Target::Node(index) => host(vm).is_some_and(|h| {
                let doc = &h.doc;
                doc.id_from_index(index as usize).is_some_and(|node| {
                    node == doc.root()
                        || doc.node(node).parent == Some(doc.root())
                        || doc.element(node).is_some_and(|e| e.is_html("body"))
                })
            }),
            Target::Object(_) => false,
        }
    });
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let exists = h.listeners.iter().any(|l| {
        l.target == target && l.kind == kind && l.capture == capture && l.callback == callback
    });
    if !exists {
        let id = h.next_listener;
        h.next_listener = h.next_listener.saturating_add(1);
        h.listeners.push(Listener {
            id,
            target,
            kind,
            callback: callback.clone(),
            capture,
            once,
            passive,
        });
        vm.add_root(callback);
    }
    Ok(Value::Undefined)
}

pub(super) fn remove_event_listener(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?.to_string();
    let callback = arg(args, 1);
    let capture = listen_option(vm, &arg(args, 2), "capture")?;
    let target = target_of(vm, this);
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let before = h.listeners.len();
    h.listeners.retain(|l| {
        !(l.target == target && l.kind == kind && l.capture == capture && l.callback == callback)
    });
    if h.listeners.len() != before {
        vm.remove_root(&callback);
    }
    Ok(Value::Undefined)
}

// ---- Objet Event ---------------------------------------------------------------------------

fn flag(vm: &Vm, event: ObjId, name: &'static str) -> bool {
    vm.slot(event, name).is_some_and(|v| v.truthy())
}

fn set_flag(vm: &mut Vm, event: ObjId, name: &'static str, value: bool) {
    vm.set_slot(event, name, Value::Bool(value));
}

/// L'événement désigné par `value` : un objet créé par `new Event(…)` ou par
/// le navigateur.
fn event_of(vm: &mut Vm, value: &Value) -> JsResult<ObjId> {
    match value.as_object().filter(|id| vm.slot(*id, TYPE).is_some()) {
        Some(id) => Ok(id),
        None => Err(vm.error("TypeError", "un objet Event est attendu")),
    }
}

/// Crée un événement de type `kind`. `interface` : `Event` ou `CustomEvent`.
fn new_event(
    vm: &mut Vm,
    interface: &'static str,
    kind: &str,
    bubbles: bool,
    cancelable: bool,
    trusted: bool,
) -> ObjId {
    let event = vm.new_object();
    let (proto, elapsed) = match host(vm) {
        Some(h) => (h.protos.get(interface).copied(), h.elapsed_ms()),
        None => (None, 0.0),
    };
    if let Some(proto) = proto {
        // Objet neuf et ordinaire : son prototype se change toujours.
        let _ = vm.set_prototype(event, Some(proto));
    }
    vm.set_slot(event, TYPE, Value::string(kind));
    vm.set_slot(event, TARGET, Value::Null);
    vm.set_slot(event, CURRENT, Value::Null);
    vm.set_slot(event, PHASE, Value::Number(NONE));
    vm.set_slot(event, TIME, Value::Number(elapsed));
    for (name, value) in [
        (BUBBLES, bubbles),
        (CANCELABLE, cancelable),
        (COMPOSED, false),
        (CANCELED, false),
        (STOP, false),
        (STOP_NOW, false),
        (DISPATCHING, false),
        (TRUSTED, trusted),
    ] {
        set_flag(vm, event, name, value);
    }
    event
}

/// Membre booléen `name` d'un dictionnaire d'initialisation.
fn init_flag(vm: &mut Vm, init: &Value, name: &str) -> JsResult<bool> {
    if init.as_object().is_none() {
        return Ok(false);
    }
    Ok(vm.get(init, name)?.truthy())
}

fn construct_event(vm: &mut Vm, interface: &'static str, args: &[Value]) -> JsResult<ObjId> {
    if !vm.is_constructing() {
        return Err(vm.error(
            "TypeError",
            &format!("{interface} doit être appelé avec « new »"),
        ));
    }
    if args.is_empty() {
        return Err(vm.error(
            "TypeError",
            &format!("{interface} : le type de l'événement est requis"),
        ));
    }
    let kind = vm.to_string(&arg(args, 0))?;
    let init = arg(args, 1);
    if !init.is_nullish() && init.as_object().is_none() {
        return Err(vm.error(
            "TypeError",
            &format!("{interface} : le second argument doit être un objet"),
        ));
    }
    let bubbles = init_flag(vm, &init, "bubbles")?;
    let cancelable = init_flag(vm, &init, "cancelable")?;
    let composed = init_flag(vm, &init, "composed")?;
    let event = new_event(vm, interface, &kind, bubbles, cancelable, false);
    set_flag(vm, event, COMPOSED, composed);
    Ok(event)
}

pub(super) fn event_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(Value::Object(construct_event(vm, "Event", args)?))
}

pub(super) fn custom_event_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let event = construct_event(vm, "CustomEvent", args)?;
    let init = arg(args, 1);
    let detail = if init.as_object().is_some() {
        match vm.get(&init, "detail")? {
            Value::Undefined => Value::Null,
            other => other,
        }
    } else {
        Value::Null
    };
    vm.set_slot(event, DETAIL, detail);
    Ok(Value::Object(event))
}

/// `new EventTarget()` : un objet ordinaire qui hérite de `EventTarget.prototype`.
pub(super) fn event_target_ctor(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    if !vm.is_constructing() {
        return Err(vm.error("TypeError", "EventTarget doit être appelé avec « new »"));
    }
    let object = vm.new_object();
    if let Some(proto) = host(vm).and_then(|h| h.protos.get("EventTarget").copied()) {
        let _ = vm.set_prototype(object, Some(proto));
    }
    Ok(Value::Object(object))
}

// ---- Attributs et méthodes d'Event -----------------------------------------------------------

/// Fabrique l'accesseur d'un attribut rangé tel quel dans un emplacement.
macro_rules! slot_getter {
    ($name:ident, $slot:expr) => {
        fn $name(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
            let event = event_of(vm, this)?;
            Ok(vm.slot(event, $slot).unwrap_or(Value::Undefined))
        }
    };
}

slot_getter!(get_type, TYPE);
slot_getter!(get_target, TARGET);
slot_getter!(get_current_target, CURRENT);
slot_getter!(get_event_phase, PHASE);
slot_getter!(get_bubbles, BUBBLES);
slot_getter!(get_cancelable, CANCELABLE);
slot_getter!(get_composed, COMPOSED);
slot_getter!(get_default_prevented, CANCELED);
slot_getter!(get_is_trusted, TRUSTED);
slot_getter!(get_time_stamp, TIME);
slot_getter!(get_cancel_bubble, STOP);

fn get_detail(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    Ok(vm.slot(event, DETAIL).unwrap_or(Value::Null))
}

fn get_return_value(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    Ok(Value::Bool(!flag(vm, event, CANCELED)))
}

/// Annule l'événement, s'il est annulable.
fn cancel(vm: &mut Vm, event: ObjId) {
    if flag(vm, event, CANCELABLE) && !flag(vm, event, IN_PASSIVE) {
        set_flag(vm, event, CANCELED, true);
    }
}

fn set_return_value(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    if !arg(args, 0).truthy() {
        cancel(vm, event);
    }
    Ok(Value::Undefined)
}

fn set_cancel_bubble(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    if arg(args, 0).truthy() {
        set_flag(vm, event, STOP, true);
    }
    Ok(Value::Undefined)
}

pub(super) fn prevent_default(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    cancel(vm, event);
    Ok(Value::Undefined)
}

pub(super) fn stop_propagation(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    set_flag(vm, event, STOP, true);
    Ok(Value::Undefined)
}

pub(super) fn stop_immediate_propagation(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    set_flag(vm, event, STOP, true);
    set_flag(vm, event, STOP_NOW, true);
    Ok(Value::Undefined)
}

/// `composedPath()` : les cibles que l'événement parcourt, pendant sa
/// distribution ; un tableau vide en dehors.
pub(super) fn composed_path(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    let path = vm.slot(event, PATH).unwrap_or(Value::Undefined);
    if !flag(vm, event, DISPATCHING) || path.as_object().is_none() {
        return Ok(vm.new_array(Vec::new()));
    }
    // Une copie : le script ne doit pas modifier le chemin en cours.
    let length = vm.get(&path, "length")?;
    let count = vm.to_number(&length)?;
    let mut items = Vec::new();
    let mut index = 0.0;
    while index < count {
        items.push(vm.get(&path, &index.to_string())?);
        index += 1.0;
    }
    Ok(vm.new_array(items))
}

/// `initEvent(type, bubbles, cancelable)` : sans effet pendant la distribution.
pub(super) fn init_event(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    if args.is_empty() {
        return Err(vm.error("TypeError", "initEvent : le type est requis"));
    }
    let kind = vm.to_string(&arg(args, 0))?;
    if flag(vm, event, DISPATCHING) {
        return Ok(Value::Undefined);
    }
    vm.set_slot(event, TYPE, Value::String(kind));
    vm.set_slot(event, TARGET, Value::Null);
    set_flag(vm, event, BUBBLES, arg(args, 1).truthy());
    set_flag(vm, event, CANCELABLE, arg(args, 2).truthy());
    for name in [CANCELED, STOP, STOP_NOW, TRUSTED] {
        set_flag(vm, event, name, false);
    }
    Ok(Value::Undefined)
}

pub(super) fn init_custom_event(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let event = event_of(vm, this)?;
    let dispatching = flag(vm, event, DISPATCHING);
    init_event(vm, this, args)?;
    if !dispatching {
        let detail = match arg(args, 3) {
            Value::Undefined => Value::Null,
            other => other,
        };
        vm.set_slot(event, DETAIL, detail);
    }
    Ok(Value::Undefined)
}

/// Attributs d'`Event` et de `CustomEvent` : nom, lecture, écriture éventuelle.
type Accessor = (&'static str, NativeFn, Option<NativeFn>);

const EVENT_ATTRIBUTES: &[Accessor] = &[
    ("type", get_type, None),
    ("target", get_target, None),
    ("srcElement", get_target, None),
    ("currentTarget", get_current_target, None),
    ("eventPhase", get_event_phase, None),
    ("bubbles", get_bubbles, None),
    ("cancelable", get_cancelable, None),
    ("composed", get_composed, None),
    ("defaultPrevented", get_default_prevented, None),
    ("isTrusted", get_is_trusted, None),
    ("timeStamp", get_time_stamp, None),
    ("cancelBubble", get_cancel_bubble, Some(set_cancel_bubble)),
    ("returnValue", get_return_value, Some(set_return_value)),
];

const CUSTOM_EVENT_ATTRIBUTES: &[Accessor] = &[("detail", get_detail, None)];

/// Installe les accesseurs des attributs sur les prototypes d'`Event` et de
/// `CustomEvent`.
pub(super) fn install_attributes(vm: &mut Vm) -> JsResult<()> {
    let global = Value::Object(vm.global());
    let object = vm.get(&global, "Object")?;
    let define = vm.get(&object, "defineProperty")?;
    for (interface, attributes) in [
        ("Event", EVENT_ATTRIBUTES),
        ("CustomEvent", CUSTOM_EVENT_ATTRIBUTES),
    ] {
        let Some(proto) = host(vm).and_then(|h| h.protos.get(interface).copied()) else {
            continue;
        };
        for (name, getter, setter) in attributes {
            let descriptor = Value::Object(vm.new_object());
            let get = Value::Object(vm.new_function(&format!("get {name}"), 0, *getter));
            vm.set(&descriptor, "get", get)?;
            if let Some(setter) = setter {
                let set = Value::Object(vm.new_function(&format!("set {name}"), 1, *setter));
                vm.set(&descriptor, "set", set)?;
            }
            vm.set(&descriptor, "enumerable", Value::Bool(true))?;
            vm.set(&descriptor, "configurable", Value::Bool(true))?;
            vm.call(
                &define,
                &object,
                &[Value::Object(proto), Value::string(name), descriptor],
            )?;
        }
    }
    Ok(())
}

/// Gestionnaire écrit dans un attribut (`<p onclick="…">`), compilé à la
/// demande en fonction de paramètre `event`. Pour une fenêtre, c'est
/// l'attribut de l'élément `<body>` de son document (`<body onload="…">`).
fn attribute_handler(vm: &mut Vm, item: Target, kind: &str) -> Option<Value> {
    let name = format!("on{kind}");
    // Un objet cible n'a de gestionnaire en attribut que s'il est la
    // fenêtre d'un cadre.
    let frame_realm = match item {
        Target::Object(id) => {
            let realms: Vec<usize> = host(vm)?.frames.values().map(|f| f.realm).collect();
            Some(
                realms
                    .into_iter()
                    .find(|realm| vm.realm_global(*realm) == Some(id))?,
            )
        }
        _ => None,
    };
    let (source, realm) = {
        let h = host(vm)?;
        let doc = &h.doc;
        let node = match item {
            Target::Node(index) => doc.id_from_index(index as usize)?,
            Target::Window => doc.find_html_element(doc.root(), "body")?,
            Target::Object(_) => {
                // Fenêtre d'un cadre : le corps de son document.
                let frame = h.frames.values().find(|f| Some(f.realm) == frame_realm)?;
                doc.find_html_element(frame.document, "body")?
            }
        };
        let source = doc.element(node)?.attr(&name)?.to_owned();
        let owner = if doc.is_document(node) {
            node
        } else {
            doc.owner(node)
        };
        let realm = u32::try_from(owner.index())
            .ok()
            .and_then(|i| h.doc_realms.get(&i))
            .copied()
            .unwrap_or(0);
        (source, realm)
    };
    let global = Value::Object(vm.realm_global(realm)?);
    let function = vm.get(&global, "Function").ok()?;
    match vm.construct(
        &function,
        &[Value::string("event"), Value::string(&source)],
        &function,
    ) {
        Ok(handler) => Some(handler),
        Err(thrown) => {
            report(vm, &thrown);
            None
        }
    }
}

// ---- Distribution ------------------------------------------------------------------------------

/// Appelle les écouteurs de `item` pour la phase en cours. Renvoie `false`
/// si la propagation doit s'arrêter là.
fn invoke(vm: &mut Vm, event: ObjId, item: Target, kind: &str, phase: f64, capture: bool) -> bool {
    if flag(vm, event, STOP) {
        return false;
    }
    let current = target_value(vm, item);
    vm.set_slot(event, CURRENT, current.clone());
    vm.set_slot(event, PHASE, Value::Number(phase));
    // Copie de la liste : un écouteur peut en ajouter ou en retirer.
    let mut calls: Vec<(Option<u64>, Value, bool, bool)> = host(vm)
        .map(|h| {
            h.listeners
                .iter()
                .filter(|l| l.target == item && l.kind == kind && l.capture == capture)
                .map(|l| (Some(l.id), l.callback.clone(), l.once, l.passive))
                .collect()
        })
        .unwrap_or_default();
    // Gestionnaire posé par affectation : `element.onclick = …`.
    if !capture
        && let Ok(handler) = vm.get(&current, &format!("on{kind}"))
        && handler.as_object().is_some()
    {
        calls.push((None, handler, false, false));
    } else if !capture && let Some(handler) = attribute_handler(vm, item, kind) {
        calls.push((None, handler, false, false));
    }
    let event_value = Value::Object(event);
    for (id, callback, once, passive) in calls {
        if let Some(id) = id {
            // Retiré par un écouteur précédent : il n'est plus appelé.
            let still_there = host(vm).is_some_and(|h| h.listeners.iter().any(|l| l.id == id));
            if !still_there {
                continue;
            }
            if once {
                if let Some(h) = host(vm) {
                    h.listeners.retain(|l| l.id != id);
                }
                vm.remove_root(&callback);
            }
        }
        set_flag(vm, event, IN_PASSIVE, passive);
        // Une fonction, ou un objet muni d'une méthode `handleEvent`.
        let outcome = match vm.get(&callback, "handleEvent") {
            Ok(method) if vm.type_of(&callback) != "function" => {
                vm.call(&method, &callback, std::slice::from_ref(&event_value))
            }
            Ok(_) => vm.call(&callback, &current, std::slice::from_ref(&event_value)),
            Err(thrown) => Err(thrown),
        };
        set_flag(vm, event, IN_PASSIVE, false);
        if let Err(thrown) = outcome {
            report(vm, &thrown);
        }
        if vm.was_interrupted() || flag(vm, event, STOP_NOW) {
            return false;
        }
    }
    !flag(vm, event, STOP)
}

/// Distribue `event` sur `target` (DOM Standard §2.9). Renvoie `true` si
/// l'événement a été annulé par `preventDefault()`.
fn fire(vm: &mut Vm, target: Target, event: ObjId) -> bool {
    let kind = match vm.slot(event, TYPE) {
        Some(Value::String(kind)) => kind.to_string(),
        _ => String::new(),
    };
    // Chemin de propagation : la cible, ses ancêtres, puis la fenêtre.
    let mut path = vec![target];
    let mut frame_realm = None;
    if let Target::Node(index) = target
        && let Some(h) = host(vm)
        && let Some(node) = h.doc.id_from_index(index as usize)
    {
        let ancestors: Vec<NodeId> = h.doc.ancestors(node).collect();
        let root = ancestors.last().copied().unwrap_or(node);
        let connected = root == h.doc.root();
        frame_realm = h
            .frames
            .values()
            .find(|frame| frame.document == root)
            .map(|frame| frame.realm);
        path.extend(
            ancestors
                .into_iter()
                .map(|n| Target::Node(u32::try_from(n.index()).unwrap_or(u32::MAX))),
        );
        if connected {
            path.push(Target::Window);
        }
    }
    // Un événement né dans un cadre remonte jusqu'à la fenêtre du cadre.
    if let Some(window) = frame_realm.and_then(|realm| vm.realm_global(realm)) {
        path.push(Target::Object(window));
    }
    let target_object = target_value(vm, target);
    vm.set_slot(event, TARGET, target_object);
    let mut path_values = Vec::with_capacity(path.len());
    for item in &path {
        path_values.push(target_value(vm, *item));
    }
    let path_array = vm.new_array(path_values);
    vm.set_slot(event, PATH, path_array);
    set_flag(vm, event, DISPATCHING, true);

    // Capture : de la fenêtre vers la cible.
    let mut running = true;
    for item in path.iter().rev() {
        let phase = if *item == target {
            AT_TARGET
        } else {
            CAPTURING_PHASE
        };
        if !invoke(vm, event, *item, &kind, phase, true) {
            running = false;
            break;
        }
    }
    // Cible, puis bouillonnement vers la fenêtre.
    if running {
        let bubbles = flag(vm, event, BUBBLES);
        for item in &path {
            let phase = if *item == target {
                AT_TARGET
            } else if bubbles {
                BUBBLING_PHASE
            } else {
                break;
            };
            if !invoke(vm, event, *item, &kind, phase, false) {
                break;
            }
        }
    }
    vm.set_slot(event, PHASE, Value::Number(NONE));
    vm.set_slot(event, CURRENT, Value::Null);
    vm.set_slot(event, PATH, Value::Undefined);
    for name in [DISPATCHING, STOP, STOP_NOW] {
        set_flag(vm, event, name, false);
    }
    flag(vm, event, CANCELED)
}

/// `EventTarget.prototype.dispatchEvent(event)`.
pub(super) fn dispatch_event(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let event = event_of(vm, &arg(args, 0))?;
    if flag(vm, event, DISPATCHING) {
        return Err(super::dom_exception(
            vm,
            "InvalidStateError",
            "l'événement est déjà en cours de distribution",
        ));
    }
    set_flag(vm, event, TRUSTED, false);
    let target = target_of(vm, this);
    let canceled = fire(vm, target, event);
    Ok(Value::Bool(!canceled))
}

/// Distribue un événement du navigateur, de type `kind`, sur `target` (la
/// fenêtre si `None`). Renvoie `true` si `preventDefault()` a été appelé.
pub(super) fn dispatch(vm: &mut Vm, target: Option<NodeId>, kind: &str) -> bool {
    let event = new_event(vm, "Event", kind, true, true, true);
    let target = match target {
        Some(node) => Target::Node(u32::try_from(node.index()).unwrap_or(u32::MAX)),
        // La fenêtre du royaume courant.
        None => target_of(vm, &Value::Undefined),
    };
    fire(vm, target, event)
}
