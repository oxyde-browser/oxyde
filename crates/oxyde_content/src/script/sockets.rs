//! `WebSocket` : ce que le moteur fournit à `websocket.js`.
//!
//! Le moteur n'ouvre aucune connexion : il range les demandes des scripts
//! ([`crate::SocketCommand`]) pour l'embarqueur, qui lui rend ce qui arrive
//! ([`crate::SocketEvent`]).

use oxyde_js::{JsResult, Value, Vm};
use url::Url;

use super::{arg, host};

/// Dernier identifiant attribué. Les identifiants ne se répètent pas dans
/// un processus : un événement arrivé pour une page déjà quittée ne peut
/// pas être pris pour celui d'une connexion de la page suivante.
static NEXT_SOCKET: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Nombre maximal de connexions WebSocket ouvertes à la fois par une page.
const MAX_SOCKETS: usize = 64;

/// Octets d'une « chaîne binaire » : un caractère de code 0 à 255 par octet.
fn binary_bytes(text: &str) -> Vec<u8> {
    text.chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect()
}

/// Identifiant de connexion passé par le script.
fn socket_id(vm: &mut Vm, value: &Value) -> JsResult<u64> {
    let number = vm.to_number(value)?;
    // Les identifiants sont de petits entiers positifs.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée juste au-dessus"
    )]
    Ok(if number.is_finite() && (0.0..9.0e15).contains(&number) {
        number as u64
    } else {
        u64::MAX
    })
}

/// `socketOpen(adresse, protocoles, rappel)` : demande l'ouverture d'une
/// connexion vers `adresse` (`ws:` ou `wss:`), en proposant les
/// sous-protocoles de `protocoles` (séparés par des virgules). `rappel(nature,
/// a, b, c)` sera appelé pour chaque événement. Rend l'identifiant de la
/// connexion, ou `null` si l'adresse est invalide ou si la page a déjà trop
/// de connexions.
pub(super) fn socket_open(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let address = vm.to_string(&arg(args, 0))?;
    let protocols = vm.to_string(&arg(args, 1))?;
    let callback = arg(args, 2);
    let Ok(url) = Url::parse(&address) else {
        return Ok(Value::Null);
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    if h.sockets.len() >= MAX_SOCKETS {
        return Ok(Value::Null);
    }
    let id = NEXT_SOCKET.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    h.sockets.insert(id, callback.clone());
    h.socket_commands.push(crate::SocketCommand::Open {
        id,
        url,
        document: h.url.clone(),
        protocols: protocols
            .split(',')
            .map(str::trim)
            .filter(|protocol| !protocol.is_empty())
            .map(str::to_owned)
            .collect(),
    });
    // Le rappel tient l'objet `WebSocket` en vie tant que la connexion dure.
    vm.add_root(callback);
    #[expect(
        clippy::cast_precision_loss,
        reason = "les identifiants restent très en deçà de 2^53"
    )]
    Ok(Value::Number(id as f64))
}

/// `socketSend(id, données, binaire)` : envoie un message ; `données` est le
/// texte, ou une chaîne binaire si `binaire`.
pub(super) fn socket_send(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = socket_id(vm, &arg(args, 0))?;
    let data = vm.to_string(&arg(args, 1))?;
    let binary = arg(args, 2).truthy();
    if let Some(h) = host(vm)
        && h.sockets.contains_key(&id)
    {
        h.socket_commands.push(if binary {
            crate::SocketCommand::Binary {
                id,
                bytes: binary_bytes(&data),
            }
        } else {
            crate::SocketCommand::Text {
                id,
                text: data.to_string(),
            }
        });
    }
    Ok(Value::Undefined)
}

/// `socketClose(id, code, raison)` : demande la fermeture ; `code` vaut
/// `undefined` s'il n'y en a pas.
pub(super) fn socket_close(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = socket_id(vm, &arg(args, 0))?;
    let code = match arg(args, 1) {
        Value::Undefined | Value::Null => None,
        other => {
            let number = vm.to_number(&other)?;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "valeur bornée par le filtre"
            )]
            Some(number)
                .filter(|n| (0.0..=65535.0).contains(n))
                .map(|n| n as u16)
        }
    };
    let reason = vm.to_string(&arg(args, 2))?.to_string();
    if let Some(h) = host(vm)
        && h.sockets.contains_key(&id)
    {
        h.socket_commands
            .push(crate::SocketCommand::Close { id, code, reason });
    }
    Ok(Value::Undefined)
}

/// Vrai si la connexion `id` appartient à cette page et n'est pas finie.
pub(super) fn known(vm: &mut Vm, id: u64) -> bool {
    host(vm).is_some_and(|h| h.sockets.contains_key(&id))
}

/// Remet au script l'événement `event` de la connexion `id`.
pub(super) fn deliver(vm: &mut Vm, id: u64, event: crate::SocketEvent) {
    let Some(callback) = host(vm).and_then(|h| h.sockets.get(&id).cloned()) else {
        return;
    };
    let binary = |bytes: &[u8]| -> String { bytes.iter().map(|byte| char::from(*byte)).collect() };
    let closing = matches!(event, crate::SocketEvent::Close { .. });
    let arguments = match event {
        crate::SocketEvent::Open { protocol } => {
            vec![Value::string("open"), Value::string(&protocol)]
        }
        crate::SocketEvent::Text(text) => vec![Value::string("text"), Value::string(&text)],
        crate::SocketEvent::Binary(bytes) => {
            vec![Value::string("binary"), Value::string(&binary(&bytes))]
        }
        crate::SocketEvent::Close {
            code,
            reason,
            clean,
        } => vec![
            Value::string("close"),
            Value::Number(f64::from(code)),
            Value::string(&reason),
            Value::Bool(clean),
        ],
    };
    if closing {
        // Dernier événement : la connexion est oubliée.
        if let Some(h) = host(vm) {
            h.sockets.remove(&id);
        }
    }
    if let Err(thrown) = vm.call(&callback, &Value::Undefined, &arguments) {
        super::report(vm, &thrown);
    }
    vm.run_microtasks();
    if closing {
        vm.remove_root(&callback);
    }
}
