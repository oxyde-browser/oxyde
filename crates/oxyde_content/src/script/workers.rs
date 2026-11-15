//! Workers et adresses `blob:` : ce que le moteur fournit aux scripts
//! `workers.js`, `worker_scope.js` et `fetch.js`.
//!
//! Un worker s'exécute dans un royaume JavaScript à lui, sur le fil de la
//! page : voir les limites en tête de `worker_scope.js`.

use oxyde_js::{JsResult, Value, Vm};
use url::Url;

use super::{arg, frames, host, install};

/// Nombre maximal de workers créés par une page : leurs royaumes ne sont
/// pas libérés.
const MAX_WORKERS: usize = 32;
/// Nombre maximal d'adresses `blob:` vivantes, et taille totale de leurs
/// contenus.
const MAX_BLOB_URLS: usize = 4096;
const MAX_BLOB_BYTES: usize = 256 << 20;

/// `createWorker(adresse, nom, versCréateur, erreurVersCréateur)` : crée le
/// royaume d'un worker, l'équipe et rend l'objet qui le pilote ; `null` si
/// l'adresse est invalide ou si la page a déjà trop de workers.
pub(super) fn create_worker(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let address = vm.to_string(&arg(args, 0))?;
    let Ok(script_url) = Url::parse(&address) else {
        return Ok(Value::Null);
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    if h.workers >= MAX_WORKERS {
        return Ok(Value::Null);
    }
    // Un script `blob:` ou `data:` n'a pas d'adresse contre laquelle résoudre
    // ses requêtes : le worker garde celle de son créateur.
    let url = if matches!(script_url.scheme(), "blob" | "data") {
        h.url.clone()
    } else {
        script_url
    };
    let Some(realm) = vm.create_realm() else {
        return Ok(Value::Null);
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    h.workers += 1;
    // Un worker n'a pas de document : celui-ci, vide et détaché, ne sert
    // qu'aux fonctions natives du royaume qui en attendent un.
    let document = frames::blank_document(&mut h.doc);
    h.doc_realms
        .insert(u32::try_from(document.index()).unwrap_or(u32::MAX), realm);
    let previous = vm.realm();
    vm.set_realm(realm);
    let built = (|| -> JsResult {
        install(vm, &url)?;
        if let Some(h) = host(vm) {
            h.document = document;
        }
        let setup = vm.eval_script(include_str!("worker_scope.js"))?;
        let run = Value::Object(vm.new_function("runScript", 1, run_script));
        vm.call(
            &setup,
            &Value::Undefined,
            &[arg(args, 1), arg(args, 2), arg(args, 3), run],
        )
    })();
    vm.set_realm(previous);
    built
}

/// `runScript(source)` : exécute `source` comme un script classique du
/// royaume courant.
fn run_script(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let source = vm.to_string(&arg(args, 0))?;
    vm.eval_script(&source)
}

/// Octets d'une « chaîne binaire » : un caractère de code 0 à 255 par octet.
fn binary_bytes(text: &str) -> Vec<u8> {
    text.chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect()
}

/// `blobUrlPut(adresse, octets, type)` : enregistre le contenu d'une adresse
/// `blob:` (`octets` : une chaîne binaire). Rend `false` si la page en a
/// déjà trop.
pub(super) fn blob_url_put(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let address = vm.to_string(&arg(args, 0))?.to_string();
    let bytes = binary_bytes(&vm.to_string(&arg(args, 1))?);
    let kind = vm.to_string(&arg(args, 2))?.to_string();
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let held: usize = h.blob_urls.values().map(|(bytes, _)| bytes.len()).sum();
    if h.blob_urls.len() >= MAX_BLOB_URLS || held.saturating_add(bytes.len()) > MAX_BLOB_BYTES {
        return Ok(Value::Bool(false));
    }
    h.blob_urls.insert(address, (bytes, kind));
    Ok(Value::Bool(true))
}

/// `blobUrlGet(adresse)` : `[octets, type]` de l'adresse `blob:`, ou `null`
/// si elle est inconnue ou révoquée.
pub(super) fn blob_url_get(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let address = vm.to_string(&arg(args, 0))?;
    // Un fragment ne change pas la ressource désignée.
    let key = address.split('#').next().unwrap_or_default().to_owned();
    let found = host(vm).and_then(|h| h.blob_urls.get(&key).cloned());
    Ok(match found {
        Some((bytes, kind)) => {
            let binary: String = bytes.iter().map(|byte| char::from(*byte)).collect();
            vm.new_array(vec![Value::string(&binary), Value::string(&kind)])
        }
        None => Value::Null,
    })
}

/// `blobUrlDelete(adresse)` : révoque une adresse `blob:`.
pub(super) fn blob_url_delete(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let address = vm.to_string(&arg(args, 0))?.to_string();
    if let Some(h) = host(vm) {
        h.blob_urls.remove(&address);
    }
    Ok(Value::Undefined)
}
