//! `Atomics` : opérations atomiques sur les tableaux typés d'entiers, et
//! mise en sommeil d'un agent sur un élément d'un `SharedArrayBuffer`.
//!
//! Sur un tampon partagé, chaque opération se fait sous le verrou du tampon :
//! aucune autre opération d'`Atomics` ne s'intercale entre sa lecture et son
//! écriture, d'où qu'elle vienne. Sur un tampon ordinaire, qu'un seul agent
//! voit, l'opération est atomique par construction.
//!
//! `Atomics.waitAsync` n'endort personne : il inscrit une attente et rend
//! une promesse. `notify` dépose un jeton dans la boîte de l'agent qui
//! attend ; cet agent règle la promesse entre deux tâches, en vidant sa
//! file de micro-tâches ([`Vm::settle_async_waits`]). Un hôte qui n'a plus
//! rien à faire peut patienter avec [`Vm::park_for_async_waits`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::heap::SharedBytes;

use super::arg;
use super::typed_array::{convert, decode_value, encode_value, number, to_index, validate};
use crate::heap::ElementKind;
use crate::value::{ObjId, Value};
use crate::vm::{JsResult, NativeFn, Vm};

/// Vérifie que `value` est un tableau typé d'entiers vivant ; `waitable` :
/// seulement ceux sur lesquels on peut s'endormir (`Int32Array`,
/// `BigInt64Array`). Renvoie le tableau, son type et sa longueur.
fn integer_array(
    vm: &mut Vm,
    value: &Value,
    waitable: bool,
) -> JsResult<(ObjId, ElementKind, usize)> {
    let (id, length) = validate(vm, value)?;
    let Some(view) = vm.typed_array(id) else {
        return Err(vm.type_error("un tableau typé est attendu"));
    };
    let accepted = if waitable {
        matches!(view.kind, ElementKind::Int32 | ElementKind::BigInt64)
    } else {
        matches!(
            view.kind,
            ElementKind::Int8
                | ElementKind::Uint8
                | ElementKind::Int16
                | ElementKind::Uint16
                | ElementKind::Int32
                | ElementKind::Uint32
                | ElementKind::BigInt64
                | ElementKind::BigUint64
        )
    };
    if !accepted {
        return Err(vm.type_error("Atomics : ce type de tableau n'est pas accepté"));
    }
    Ok((id, view.kind, length))
}

/// Comme [`integer_array`], pour une écriture : `TypeError` aussi si le
/// tampon du tableau est immuable.
fn writable_array(vm: &mut Vm, value: &Value) -> JsResult<(ObjId, ElementKind, usize)> {
    if value.as_object().is_some_and(|id| vm.typed_immutable(id)) {
        return Err(vm.type_error("Atomics : le tampon du tableau est immuable"));
    }
    integer_array(vm, value, false)
}

/// Index d'élément demandé, vérifié contre la longueur du tableau.
fn access_index(vm: &mut Vm, request: &Value, length: usize) -> JsResult<usize> {
    let index = to_index(vm, request)?;
    if index >= length {
        return Err(vm.error("RangeError", "Atomics : index hors du tableau"));
    }
    Ok(index)
}

/// Opérande converti au type des éléments : un BigInt, ou un nombre entier.
fn operand(vm: &mut Vm, kind: ElementKind, value: &Value) -> JsResult<Value> {
    match convert(vm, kind, value)? {
        Value::Number(n) => {
            // `ToIntegerOrInfinity` : NaN vaut 0, la partie fractionnaire tombe.
            let integer = if n.is_nan() { 0.0 } else { n.trunc() };
            Ok(Value::Number(integer + 0.0))
        }
        other => Ok(other),
    }
}

/// Décalage en octets, dans son tampon, de l'élément `index` du tableau
/// `id` ; l'accès est revérifié, les conversions d'arguments ayant pu
/// détacher ou rétrécir le tampon.
fn element_offset(vm: &mut Vm, id: ObjId, index: usize) -> JsResult<(ObjId, usize)> {
    let Some(view) = vm.typed_array(id) else {
        return Err(vm.type_error("un tableau typé est attendu"));
    };
    match vm.typed_length(id) {
        None => Err(vm.type_error("le tableau typé est détaché ou hors des bornes de son tampon")),
        Some(length) if index >= length => {
            Err(vm.error("RangeError", "Atomics : index hors du tableau"))
        }
        Some(_) => Ok((view.buffer, view.offset + index * view.kind.size())),
    }
}

/// Lit l'élément, lui applique `update` et écrit le résultat s'il y en a
/// un ; renvoie l'ancienne valeur. Rien ne s'intercale sur un tampon partagé.
fn read_modify_write(
    vm: &mut Vm,
    buffer: ObjId,
    start: usize,
    kind: ElementKind,
    update: impl FnOnce(u64) -> Option<u64>,
) -> Value {
    let size = kind.size();
    let shared = vm.buffer(buffer).and_then(|b| b.shared.clone());
    let run = |vm: &mut Vm| {
        let old = vm
            .buffer(buffer)
            .and_then(|b| b.read(start, size))
            .unwrap_or([0; 8]);
        if let Some(new) = update(u64::from_le_bytes(old))
            && let Some(target) = vm.buffer_mut(buffer)
        {
            target.write(start, &new.to_le_bytes()[..size]);
        }
        decode_value(kind, &old[..size])
    };
    match shared {
        Some(shared) => shared.atomically(|| run(vm)),
        None => run(vm),
    }
}

/// Octets, en entier petit-boutiste, d'un opérande déjà converti.
fn bits(kind: ElementKind, value: &Value) -> u64 {
    let (bytes, size) = encode_value(kind, value);
    let mut raw = [0u8; 8];
    raw[..size].copy_from_slice(&bytes[..size]);
    u64::from_le_bytes(raw)
}

/// Opération de lecture-modification-écriture `Atomics.<nom>(tableau, index,
/// valeur)`.
fn modify(vm: &mut Vm, args: &[Value], combine: impl FnOnce(u64, u64) -> u64) -> JsResult {
    let (id, kind, length) = writable_array(vm, &arg(args, 0))?;
    let index = access_index(vm, &arg(args, 1), length)?;
    let value = operand(vm, kind, &arg(args, 2))?;
    let (buffer, start) = element_offset(vm, id, index)?;
    let operand = bits(kind, &value);
    Ok(read_modify_write(vm, buffer, start, kind, |old| {
        Some(combine(old, operand))
    }))
}

fn atomics_add(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, u64::wrapping_add)
}

fn atomics_sub(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, u64::wrapping_sub)
}

fn atomics_and(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, |old, value| old & value)
}

fn atomics_or(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, |old, value| old | value)
}

fn atomics_xor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, |old, value| old ^ value)
}

fn atomics_exchange(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    modify(vm, args, |_, value| value)
}

fn atomics_compare_exchange(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (id, kind, length) = writable_array(vm, &arg(args, 0))?;
    let index = access_index(vm, &arg(args, 1), length)?;
    let expected = operand(vm, kind, &arg(args, 2))?;
    let replacement = operand(vm, kind, &arg(args, 3))?;
    let (buffer, start) = element_offset(vm, id, index)?;
    // La comparaison porte sur les octets : seuls ceux de l'élément comptent.
    let mask = match kind.size() {
        8 => u64::MAX,
        size => (1u64 << (size * 8)) - 1,
    };
    let (expected, replacement) = (bits(kind, &expected), bits(kind, &replacement));
    Ok(read_modify_write(vm, buffer, start, kind, |old| {
        (old & mask == expected & mask).then_some(replacement)
    }))
}

fn atomics_load(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (id, kind, length) = integer_array(vm, &arg(args, 0), false)?;
    let index = access_index(vm, &arg(args, 1), length)?;
    let (buffer, start) = element_offset(vm, id, index)?;
    Ok(read_modify_write(vm, buffer, start, kind, |_| None))
}

fn atomics_store(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (id, kind, length) = writable_array(vm, &arg(args, 0))?;
    let index = access_index(vm, &arg(args, 1), length)?;
    let value = operand(vm, kind, &arg(args, 2))?;
    let (buffer, start) = element_offset(vm, id, index)?;
    let new = bits(kind, &value);
    read_modify_write(vm, buffer, start, kind, |_| Some(new));
    // C'est la valeur convertie qui est rendue, pas celle relue.
    Ok(value)
}

fn atomics_is_lock_free(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let size = vm.to_number(&arg(args, 0))?;
    let size = if size.is_nan() { 0.0 } else { size.trunc() };
    Ok(Value::Bool(
        size == 1.0 || size == 2.0 || size == 4.0 || size == 8.0,
    ))
}

/// Attente d'`Atomics.waitAsync` encore en cours.
pub(crate) struct AsyncWait {
    /// Jeton que `notify` dépose dans la boîte de l'agent.
    token: u64,
    /// Numéro de l'attente dans la liste du tampon.
    waiter: u64,
    shared: Arc<SharedBytes>,
    /// Terme du délai ; `None` : attente sans limite.
    deadline: Option<Instant>,
    /// Promesse rendue au script.
    pub(crate) promise: ObjId,
}

impl Vm {
    /// Règle les promesses d'`Atomics.waitAsync` dont l'attente est finie :
    /// `"ok"` pour celles qu'un `notify` a réveillées, `"timed-out"` pour
    /// celles dont le délai est passé. Renvoie vrai s'il y en avait.
    ///
    /// Appelé au début de chaque vidage des micro-tâches ; un hôte dont la
    /// boucle d'événements tourne à vide peut aussi l'appeler lui-même.
    pub fn settle_async_waits(&mut self) -> bool {
        if self.async_waits.is_empty() {
            return false;
        }
        let woken = self.async_mailbox.take();
        let now = Instant::now();
        let mut settled = Vec::new();
        let waits = std::mem::take(&mut self.async_waits);
        for wait in waits {
            if woken.contains(&wait.token) {
                settled.push((wait.promise, "ok"));
            } else if wait.deadline.is_some_and(|deadline| now >= deadline)
                && wait.shared.cancel_async(wait.waiter)
            {
                // Retirée de la liste avant tout `notify` : le délai l'emporte.
                settled.push((wait.promise, "timed-out"));
            } else {
                self.async_waits.push(wait);
            }
        }
        let any = !settled.is_empty();
        for (promise, outcome) in settled {
            self.resolve_promise(promise, Value::string(outcome));
        }
        any
    }

    /// Nombre d'attentes d'`Atomics.waitAsync` encore en cours.
    #[must_use]
    pub fn pending_async_waits(&self) -> usize {
        self.async_waits.len()
    }

    /// Patiente, `max` au plus, jusqu'au réveil d'une attente asynchrone ou
    /// au terme du plus proche de leurs délais.
    pub fn park_for_async_waits(&self, max: Duration) {
        let now = Instant::now();
        let nearest = self
            .async_waits
            .iter()
            .filter_map(|wait| wait.deadline)
            .map(|deadline| deadline.saturating_duration_since(now))
            .min();
        self.async_mailbox
            .park(nearest.map_or(max, |nearest| nearest.min(max)));
    }
}

/// Demande d'attente validée, commune à `wait` et `waitAsync`.
struct WaitRequest {
    shared: Arc<SharedBytes>,
    /// Décalage de l'élément dans le tampon.
    start: usize,
    /// Délai ; `None` : sans limite.
    timeout: Option<Duration>,
    /// Vrai tant que l'élément vaut la valeur attendue.
    still_expected: Box<dyn Fn() -> bool>,
}

/// Valide les arguments de `wait` (`blocking`) ou de `waitAsync`, dans
/// l'ordre de la spécification.
fn wait_request(vm: &mut Vm, args: &[Value], blocking: bool) -> JsResult<WaitRequest> {
    let (id, kind, length) = integer_array(vm, &arg(args, 0), true)?;
    let buffer = vm.typed_array(id).map(|view| view.buffer);
    let shared = buffer
        .and_then(|buffer| vm.buffer(buffer))
        .and_then(|b| b.shared.clone());
    let Some(shared) = shared else {
        return Err(vm.type_error("Atomics : le tampon doit être partagé pour y attendre"));
    };
    let index = access_index(vm, &arg(args, 1), length)?;
    let expected = operand(vm, kind, &arg(args, 2))?;
    let timeout = vm.to_number(&arg(args, 3))?;
    // Délai : NaN vaut l'infini, un délai négatif vaut zéro.
    let timeout = if timeout.is_nan() || timeout == f64::INFINITY {
        None
    } else {
        Some(Duration::from_secs_f64(
            (timeout.max(0.0) / 1000.0).min(1.0e9),
        ))
    };
    if blocking && !vm.can_block {
        return Err(vm.type_error("Atomics.wait : cet agent ne peut pas s'endormir"));
    }
    let (buffer, start) = element_offset(vm, id, index)?;
    let size = kind.size();
    let mask = match size {
        8 => u64::MAX,
        size => (1u64 << (size * 8)) - 1,
    };
    let expected = bits(kind, &expected) & mask;
    let Some(data) = vm.buffer(buffer).cloned() else {
        return Err(vm.type_error("un tableau typé est attendu"));
    };
    Ok(WaitRequest {
        shared,
        start,
        timeout,
        still_expected: Box::new(move || {
            data.read(start, size)
                .is_some_and(|bytes| u64::from_le_bytes(bytes) & mask == expected)
        }),
    })
}

fn atomics_wait(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let request = wait_request(vm, args, true)?;
    let outcome = request
        .shared
        .wait(request.start, request.timeout, &*request.still_expected);
    Ok(Value::string(match outcome {
        None => "not-equal",
        Some(true) => "ok",
        Some(false) => "timed-out",
    }))
}

/// `Atomics.waitAsync(tableau, index, valeur, délai)` : rend
/// `{ async: false, value: "not-equal" | "timed-out" }` quand la réponse est
/// immédiate, sinon `{ async: true, value: promesse }`.
fn atomics_wait_async(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let request = wait_request(vm, args, false)?;
    let immediate = |vm: &mut Vm, outcome: &str| -> JsResult {
        let result = Value::Object(vm.new_object());
        vm.set(&result, "async", Value::Bool(false))?;
        vm.set(&result, "value", Value::string(outcome))?;
        Ok(result)
    };
    if request.timeout == Some(Duration::ZERO) {
        // Délai nul : rien n'est inscrit, la réponse ne dépend que de la valeur.
        let same = request.shared.atomically(&*request.still_expected);
        return immediate(vm, if same { "timed-out" } else { "not-equal" });
    }
    vm.wait_tokens = vm.wait_tokens.wrapping_add(1);
    let token = vm.wait_tokens;
    let mailbox = Arc::clone(&vm.async_mailbox);
    let Some(waiter) =
        request
            .shared
            .wait_async(request.start, &mailbox, token, &*request.still_expected)
    else {
        return immediate(vm, "not-equal");
    };
    let promise = vm.new_promise();
    vm.async_waits.push(AsyncWait {
        token,
        waiter,
        shared: request.shared,
        deadline: request.timeout.map(|timeout| Instant::now() + timeout),
        promise,
    });
    let result = Value::Object(vm.new_object());
    vm.set(&result, "async", Value::Bool(true))?;
    vm.set(&result, "value", Value::Object(promise))?;
    Ok(result)
}

fn atomics_notify(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (id, _, length) = integer_array(vm, &arg(args, 0), true)?;
    let index = access_index(vm, &arg(args, 1), length)?;
    let count = match arg(args, 2) {
        Value::Undefined => usize::MAX,
        other => {
            let count = vm.to_number(&other)?;
            if count.is_nan() || count <= 0.0 {
                0
            } else if count >= 4_294_967_296.0 {
                usize::MAX
            } else {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "valeur positive bornée ci-dessus"
                )]
                {
                    count.trunc() as usize
                }
            }
        }
    };
    // Personne ne dort sur un tampon qu'un seul agent voit, même si la
    // conversion de l'index vient de le détacher ou de le rétrécir.
    let is_shared = vm
        .typed_array(id)
        .and_then(|view| vm.buffer(view.buffer))
        .is_some_and(|b| b.shared.is_some());
    if !is_shared {
        return Ok(number(0));
    }
    let (buffer, start) = element_offset(vm, id, index)?;
    let shared = vm.buffer(buffer).and_then(|b| b.shared.clone());
    Ok(number(
        shared.map_or(0, |shared| shared.notify(start, count)),
    ))
}

fn atomics_pause(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    match arg(args, 0) {
        Value::Undefined => {}
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 => {}
        _ => return Err(vm.type_error("Atomics.pause : un entier est attendu")),
    }
    std::hint::spin_loop();
    Ok(Value::Undefined)
}

pub(super) fn install(vm: &mut Vm) {
    let atomics = vm.new_object();
    let methods: &[(&str, u32, NativeFn)] = &[
        ("add", 3, atomics_add),
        ("and", 3, atomics_and),
        ("compareExchange", 4, atomics_compare_exchange),
        ("exchange", 3, atomics_exchange),
        ("isLockFree", 1, atomics_is_lock_free),
        ("load", 2, atomics_load),
        ("notify", 3, atomics_notify),
        ("or", 3, atomics_or),
        ("pause", 0, atomics_pause),
        ("store", 3, atomics_store),
        ("sub", 3, atomics_sub),
        ("wait", 4, atomics_wait),
        ("waitAsync", 4, atomics_wait_async),
        ("xor", 3, atomics_xor),
    ];
    for (name, arity, func) in methods {
        vm.define_method(atomics, name, *arity, *func);
    }
    let tag = vm.well_known.to_string_tag.clone();
    vm.define_hidden_symbol(atomics, &tag, Value::string("Atomics"));
    vm.define_hidden(vm.global, "Atomics", Value::Object(atomics));
}
