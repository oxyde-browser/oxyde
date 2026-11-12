//! `MutationObserver` (DOM Standard §4.3) : le document consigne ses
//! modifications, et la fin de chaque tâche les remet aux observateurs.
//!
//! LIMITES CONNUES : un enregistrement par nœud inséré ou retiré (le standard
//! en regroupe plusieurs dans un même enregistrement) ; un nœud retiré cesse
//! aussitôt d'être suivi par les observations de sous-arbre.

use oxyde_dom::{Mutation, NodeId};
use oxyde_js::{JsResult, ObjId, Value, Vm};

use super::{arg, host, node_arg, report, wrap, wrap_all, wrap_opt};

/// Nombre maximal de tours de remise par tâche : un observateur qui modifie
/// le document à chaque appel ne bloque pas la page.
const MAX_ROUNDS: usize = 64;

/// Ce qu'une observation retient.
#[derive(Clone, Debug, Default)]
struct Options {
    child_list: bool,
    attributes: bool,
    character_data: bool,
    subtree: bool,
    attribute_old_value: bool,
    character_data_old_value: bool,
    /// Noms d'attributs retenus ; `None` : tous.
    attribute_filter: Option<Vec<String>>,
}

/// Un observateur et ses cibles.
pub(super) struct Observer {
    /// L'objet `MutationObserver`.
    object: ObjId,
    callback: Value,
    /// Cibles observées : index du nœud et options.
    targets: Vec<(u32, Options)>,
    /// Enregistrements en attente de remise.
    queue: Vec<Value>,
}

fn observer_index(vm: &mut Vm, observer: &Value) -> Option<usize> {
    let id = observer.as_object()?;
    host(vm)?.observers.iter().position(|o| o.object == id)
}

fn flag(vm: &mut Vm, options: &Value, name: &str) -> JsResult<Option<bool>> {
    Ok(match vm.get(options, name)? {
        Value::Undefined => None,
        other => Some(other.truthy()),
    })
}

/// `observe(observateur, rappel, cible, options)`.
pub(super) fn observe(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (observer, callback) = (arg(args, 0), arg(args, 1));
    let target = node_arg(vm, &arg(args, 2))?;
    let options = arg(args, 3);
    let mut parsed = Options::default();
    let mut attributes = None;
    let mut character_data = None;
    if options.as_object().is_some() {
        parsed.child_list = flag(vm, &options, "childList")?.unwrap_or(false);
        parsed.subtree = flag(vm, &options, "subtree")?.unwrap_or(false);
        attributes = flag(vm, &options, "attributes")?;
        character_data = flag(vm, &options, "characterData")?;
        let attribute_old = flag(vm, &options, "attributeOldValue")?;
        let data_old = flag(vm, &options, "characterDataOldValue")?;
        parsed.attribute_old_value = attribute_old.unwrap_or(false);
        parsed.character_data_old_value = data_old.unwrap_or(false);
        let filter = vm.get(&options, "attributeFilter")?;
        if filter.as_object().is_some() {
            let length = vm.get(&filter, "length")?;
            let count = vm.to_number(&length)?;
            let mut names = Vec::new();
            let mut index = 0.0;
            while index < count {
                let name = vm.get(&filter, &index.to_string())?;
                names.push(vm.to_string(&name)?.to_string());
                index += 1.0;
            }
            parsed.attribute_filter = Some(names);
        }
        // Demander une ancienne valeur ou un filtre, c'est observer la chose.
        if attributes.is_none() && (attribute_old.is_some() || parsed.attribute_filter.is_some()) {
            attributes = Some(true);
        }
        if character_data.is_none() && data_old.is_some() {
            character_data = Some(true);
        }
    }
    parsed.attributes = attributes.unwrap_or(false);
    parsed.character_data = character_data.unwrap_or(false);
    if !(parsed.child_list || parsed.attributes || parsed.character_data) {
        return Err(vm.error(
            "TypeError",
            "MutationObserver.observe : il faut observer childList, attributes ou characterData",
        ));
    }
    if (parsed.attribute_old_value || parsed.attribute_filter.is_some()) && !parsed.attributes {
        return Err(vm.error(
            "TypeError",
            "MutationObserver.observe : attributeOldValue et attributeFilter exigent attributes",
        ));
    }
    if parsed.character_data_old_value && !parsed.character_data {
        return Err(vm.error(
            "TypeError",
            "MutationObserver.observe : characterDataOldValue exige characterData",
        ));
    }
    let Some(object) = observer.as_object() else {
        return Ok(Value::Undefined);
    };
    let node = u32::try_from(target.index()).unwrap_or(u32::MAX);
    let index = match observer_index(vm, &observer) {
        Some(index) => index,
        None => {
            // L'observateur vit tant qu'il observe quelque chose.
            vm.add_root(observer.clone());
            vm.add_root(callback.clone());
            let Some(h) = host(vm) else {
                return Ok(Value::Undefined);
            };
            h.observers.push(Observer {
                object,
                callback,
                targets: Vec::new(),
                queue: Vec::new(),
            });
            h.observers.len() - 1
        }
    };
    if let Some(h) = host(vm) {
        h.doc.record_mutations(true);
        if let Some(entry) = h.observers.get_mut(index) {
            // Observer deux fois la même cible remplace les options.
            entry.targets.retain(|(observed, _)| *observed != node);
            entry.targets.push((node, parsed));
        }
    }
    Ok(Value::Undefined)
}

/// `disconnect(observateur)` : plus aucune cible, file vidée.
pub(super) fn disconnect(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let observer = arg(args, 0);
    if let Some(index) = observer_index(vm, &observer) {
        let removed = host(vm).map(|h| {
            let entry = h.observers.remove(index);
            if h.observers.is_empty() {
                h.doc.record_mutations(false);
            }
            entry
        });
        if let Some(entry) = removed {
            vm.remove_root(&observer);
            vm.remove_root(&entry.callback);
            for record in &entry.queue {
                vm.remove_root(record);
            }
        }
    }
    Ok(Value::Undefined)
}

/// `takeRecords(observateur)` : les enregistrements en attente.
pub(super) fn take_records(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    // Les modifications de la tâche en cours deviennent d'abord des
    // enregistrements.
    collect(vm);
    let observer = arg(args, 0);
    let records = observer_index(vm, &observer)
        .and_then(|index| host(vm).map(|h| std::mem::take(&mut h.observers[index].queue)))
        .unwrap_or_default();
    for record in &records {
        vm.remove_root(record);
    }
    Ok(vm.new_array(records))
}

/// Contenu d'un `MutationRecord`.
#[derive(Default)]
struct Change<'a> {
    kind: &'a str,
    added: Option<NodeId>,
    removed: Option<NodeId>,
    previous: Option<NodeId>,
    next: Option<NodeId>,
    attribute: Option<&'a str>,
    old: Option<&'a str>,
}

/// Objet `MutationRecord` décrivant `change` sur `target`.
fn record(vm: &mut Vm, target: NodeId, change: &Change<'_>) -> JsResult {
    let object = Value::Object(vm.new_object());
    let target = wrap(vm, target);
    let added = wrap_all(vm, change.added.into_iter().collect());
    let removed = wrap_all(vm, change.removed.into_iter().collect());
    let previous = wrap_opt(vm, change.previous);
    let next = wrap_opt(vm, change.next);
    let text = |value: Option<&str>| value.map_or(Value::Null, Value::string);
    for (name, value) in [
        ("type", Value::string(change.kind)),
        ("target", target),
        ("addedNodes", added),
        ("removedNodes", removed),
        ("previousSibling", previous),
        ("nextSibling", next),
        ("attributeName", text(change.attribute)),
        ("attributeNamespace", Value::Null),
        ("oldValue", text(change.old)),
    ] {
        vm.set(&object, name, value)?;
    }
    Ok(object)
}

/// Transforme les modifications consignées en enregistrements, rangés dans
/// la file des observateurs intéressés.
fn collect(vm: &mut Vm) {
    let Some(h) = host(vm) else { return };
    let log = h.doc.take_mutations();
    if log.is_empty() || h.observers.is_empty() {
        return;
    }
    for (mutation, path) in log {
        // Le nœud touché, puis ses ancêtres au moment de la modification.
        let Some(target) = path.first().copied() else {
            continue;
        };
        let target_index = u32::try_from(target.index()).unwrap_or(u32::MAX);
        let path: Vec<u32> = path
            .iter()
            .map(|n| u32::try_from(n.index()).unwrap_or(u32::MAX))
            .collect();
        // Observateurs intéressés, et s'ils veulent l'ancienne valeur.
        let Some(h) = host(vm) else { return };
        let mut interested: Vec<(usize, bool)> = Vec::new();
        for (index, observer) in h.observers.iter().enumerate() {
            let mut wants = None;
            for (node, options) in &observer.targets {
                let in_scope = *node == target_index || (options.subtree && path.contains(node));
                if !in_scope {
                    continue;
                }
                let old_value = match &mutation {
                    Mutation::Attribute { name, .. } => {
                        let listed = options
                            .attribute_filter
                            .as_ref()
                            .is_none_or(|filter| filter.iter().any(|f| f == name));
                        (options.attributes && listed).then_some(options.attribute_old_value)
                    }
                    Mutation::Text { .. } => options
                        .character_data
                        .then_some(options.character_data_old_value),
                    _ => options.child_list.then_some(false),
                };
                if let Some(old_value) = old_value {
                    wants = Some(wants.unwrap_or(false) || old_value);
                }
            }
            if let Some(old_value) = wants {
                interested.push((index, old_value));
            }
        }
        for (index, with_old) in interested {
            let change = match &mutation {
                Mutation::Attribute { name, old, .. } => Change {
                    kind: "attributes",
                    attribute: Some(name),
                    old: old.as_deref().filter(|_| with_old),
                    ..Change::default()
                },
                Mutation::Text { old, .. } => Change {
                    kind: "characterData",
                    old: with_old.then_some(old.as_str()),
                    ..Change::default()
                },
                Mutation::Inserted {
                    node,
                    previous,
                    next,
                    ..
                } => Change {
                    kind: "childList",
                    added: Some(*node),
                    previous: *previous,
                    next: *next,
                    ..Change::default()
                },
                Mutation::Removed {
                    node,
                    previous,
                    next,
                    ..
                } => Change {
                    kind: "childList",
                    removed: Some(*node),
                    previous: *previous,
                    next: *next,
                    ..Change::default()
                },
            };
            let built = record(vm, target, &change);
            if let Ok(built) = built {
                vm.add_root(built.clone());
                if let Some(entry) = host(vm).and_then(|h| h.observers.get_mut(index)) {
                    entry.queue.push(built);
                }
            }
        }
    }
}

/// Fin de tâche : remet aux observateurs leurs enregistrements, puis
/// recommence tant que leurs rappels modifient le document.
pub(super) fn deliver(vm: &mut Vm) {
    for _ in 0..MAX_ROUNDS {
        collect(vm);
        let Some(h) = host(vm) else { return };
        let pending: Vec<(ObjId, Value, Vec<Value>)> = h
            .observers
            .iter_mut()
            .filter(|observer| !observer.queue.is_empty())
            .map(|observer| {
                (
                    observer.object,
                    observer.callback.clone(),
                    std::mem::take(&mut observer.queue),
                )
            })
            .collect();
        if pending.is_empty() {
            return;
        }
        for (object, callback, records) in pending {
            for record in &records {
                vm.remove_root(record);
            }
            let observer = Value::Object(object);
            let list = vm.new_array(records);
            if let Err(thrown) = vm.call(&callback, &observer, &[list, observer.clone()]) {
                report(vm, &thrown);
            }
            if vm.was_interrupted() {
                return;
            }
        }
        vm.run_microtasks();
    }
}
