//! `localStorage` et `sessionStorage` (HTML Standard §12.2), ainsi que
//! `dataset` et les méthodes des collections de nœuds.
//!
//! LIMITES CONNUES : les données de `localStorage` vivent avec la page et ne
//! sont pas encore conservées d'une navigation à l'autre ; `delete` et `in`
//! ne voient pas les entrées ; les collections sont des tableaux figés au
//! moment de leur création.

use oxyde_dom::NodeId;
use oxyde_js::{JsResult, Value, Vm};

use super::{CLASS_STORAGE, arg, host, node_of, number};

/// Taille maximale d'une zone de stockage, en octets de clés et de valeurs.
const QUOTA: usize = 5 * 1024 * 1024;

/// Zone désignée par le receveur : 0 pour `localStorage`, 1 pour `sessionStorage`.
fn area(vm: &mut Vm, this: &Value) -> JsResult<usize> {
    match vm.host_data(this) {
        Some((CLASS_STORAGE, data)) => Ok(usize::from(data != 0)),
        _ => Err(vm.error("TypeError", "cette méthode attend un objet Storage")),
    }
}

/// Valeur de `key` dans la zone `area`.
pub(super) fn stored(vm: &mut Vm, area: usize, key: &str) -> Option<String> {
    host(vm)?
        .storage
        .get(area)?
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.clone())
}

pub(super) fn stored_count(vm: &mut Vm, area: usize) -> usize {
    host(vm)
        .and_then(|h| h.storage.get(area))
        .map_or(0, Vec::len)
}

/// Écrit `key` dans la zone `area` ; `QuotaExceededError` si elle déborde.
pub(super) fn store(vm: &mut Vm, area: usize, key: &str, value: &str) -> JsResult<()> {
    let Some(entries) = host(vm).and_then(|h| h.storage.get_mut(area)) else {
        return Ok(());
    };
    let used: usize = entries
        .iter()
        .filter(|(name, _)| name != key)
        .map(|(name, value)| name.len() + value.len())
        .sum();
    if used.saturating_add(key.len()).saturating_add(value.len()) > QUOTA {
        return Err(vm.error(
            "Error",
            "QuotaExceededError : la zone de stockage est pleine",
        ));
    }
    match entries.iter_mut().find(|(name, _)| name == key) {
        Some(entry) => value.clone_into(&mut entry.1),
        None => entries.push((key.to_owned(), value.to_owned())),
    }
    Ok(())
}

pub(super) fn get_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let area = area(vm, this)?;
    let key = vm.to_string(&arg(args, 0))?;
    Ok(stored(vm, area, &key).map_or(Value::Null, |v| Value::string(&v)))
}

pub(super) fn set_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let area = area(vm, this)?;
    let key = vm.to_string(&arg(args, 0))?;
    let value = vm.to_string(&arg(args, 1))?;
    store(vm, area, &key, &value)?;
    Ok(Value::Undefined)
}

pub(super) fn remove_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let area = area(vm, this)?;
    let key = vm.to_string(&arg(args, 0))?;
    if let Some(entries) = host(vm).and_then(|h| h.storage.get_mut(area)) {
        entries.retain(|(name, _)| **name != *key);
    }
    Ok(Value::Undefined)
}

pub(super) fn clear(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let area = area(vm, this)?;
    if let Some(entries) = host(vm).and_then(|h| h.storage.get_mut(area)) {
        entries.clear();
    }
    Ok(Value::Undefined)
}

pub(super) fn key(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let area = area(vm, this)?;
    let index = vm.to_number(&arg(args, 0))?;
    if !index.is_finite() || index < 0.0 {
        return Ok(Value::Null);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positif vérifié ; un index démesuré ne désigne rien"
    )]
    let index = index.trunc() as usize;
    let name = host(vm)
        .and_then(|h| h.storage.get(area))
        .and_then(|entries| entries.get(index))
        .map(|(name, _)| name.clone());
    Ok(name.map_or(Value::Null, |n| Value::string(&n)))
}

// ---- dataset ---------------------------------------------------------------------------

/// Nom de l'attribut `data-*` que désigne la propriété `key` de `dataset`
/// (`fooBar` → `data-foo-bar`).
pub(super) fn data_attribute(key: &str) -> String {
    let mut name = String::from("data-");
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            name.push('-');
            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }
    name
}

/// Valeur de la propriété `key` du `dataset` de `node`.
pub(super) fn dataset_get(vm: &mut Vm, node: NodeId, key: &str) -> Option<String> {
    let name = data_attribute(key);
    host(vm)?.doc.element(node)?.attr(&name).map(str::to_owned)
}

pub(super) fn dataset_owner(vm: &mut Vm, data: u64) -> Option<NodeId> {
    host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)
}

// ---- Collections -----------------------------------------------------------------------

/// `item(index)` de `NodeList` et `HTMLCollection` : le receveur est un tableau.
pub(super) fn list_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let index = vm.to_number(&arg(args, 0))?;
    if !index.is_finite() || index < 0.0 {
        return Ok(Value::Null);
    }
    let value = vm.get(this, &index.trunc().to_string())?;
    Ok(match value {
        Value::Undefined => Value::Null,
        other => other,
    })
}

/// `namedItem(name)` : le premier élément dont `id` ou `name` vaut `name`.
pub(super) fn list_named_item(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?;
    if name.is_empty() {
        return Ok(Value::Null);
    }
    let length = vm.get(this, "length")?;
    let count = vm.to_number(&length)?;
    let mut index = 0.0;
    while index < count {
        let item = vm.get(this, &index.to_string())?;
        if let Some(node) = node_of(vm, &item) {
            let matches = host(vm).is_some_and(|h| {
                h.doc
                    .element(node)
                    .is_some_and(|e| e.attr("id") == Some(&*name) || e.attr("name") == Some(&*name))
            });
            if matches {
                return Ok(item);
            }
        }
        index += 1.0;
    }
    Ok(Value::Null)
}

/// Nombre d'entrées d'une zone, pour l'attribut `length`.
pub(super) fn length(vm: &mut Vm, area: usize) -> Value {
    number(stored_count(vm, area))
}
