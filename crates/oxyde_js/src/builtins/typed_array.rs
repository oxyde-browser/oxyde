//! `ArrayBuffer`, tableaux typés (`Int8Array`… `Float64Array`) et `DataView`.
//!
//! Un tableau typé est une vue sur les octets d'un `ArrayBuffer` : ses
//! propriétés indexées lisent et écrivent directement ces octets (voir
//! `object.rs`), sans jamais consulter le prototype.
//!
//! Un `SharedArrayBuffer` est un tampon dont les octets se partagent entre
//! agents (voir `heap::SharedBytes`) ; les opérations d'`Atomics` sont dans
//! `atomics.rs`.
//!
//! LIMITES CONNUES : `Object.defineProperty` sur un index n'est pas
//! spécialisé ; un tampon est limité à 1 Gio.

use std::sync::Arc;

use super::{arg, array, callable, constructor, getter, is_callable};
use crate::bigint::BigInt;
use crate::heap::{BufferData, ElementKind, IndexedKind, IterState, Kind, Object, TypedArrayData};
use crate::value::{ObjId, PropertyKey, Value, to_int32, to_uint32};
use crate::vm::{JsResult, NativeFn, Vm};

/// Taille maximale d'un tampon, en octets.
const MAX_BUFFER: usize = 1 << 30;

/// Types d'éléments, dans l'ordre de `Vm::typed_protos`.
const KINDS: [(ElementKind, &str); 12] = [
    (ElementKind::Int8, "Int8Array"),
    (ElementKind::Uint8, "Uint8Array"),
    (ElementKind::Uint8Clamped, "Uint8ClampedArray"),
    (ElementKind::Int16, "Int16Array"),
    (ElementKind::Uint16, "Uint16Array"),
    (ElementKind::Int32, "Int32Array"),
    (ElementKind::Uint32, "Uint32Array"),
    (ElementKind::Float16, "Float16Array"),
    (ElementKind::Float32, "Float32Array"),
    (ElementKind::Float64, "Float64Array"),
    (ElementKind::BigInt64, "BigInt64Array"),
    (ElementKind::BigUint64, "BigUint64Array"),
];

impl ElementKind {
    /// Taille d'un élément, en octets.
    pub(crate) fn size(self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 | Self::Uint8Clamped => 1,
            Self::Int16 | Self::Uint16 | Self::Float16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 | Self::BigInt64 | Self::BigUint64 => 8,
        }
    }

    fn name(self) -> &'static str {
        KINDS
            .iter()
            .find(|(kind, _)| *kind == self)
            .map_or("TypedArray", |(_, name)| name)
    }

    fn index(self) -> usize {
        KINDS
            .iter()
            .position(|(kind, _)| *kind == self)
            .unwrap_or(0)
    }
}

// ---- Conversions des éléments ----------------------------------------------------

/// Arrondi au plus proche, les demis vers le pair, borné à `[0, 255]`.
fn clamp_u8(n: f64) -> u8 {
    if n.is_nan() {
        return 0;
    }
    let clamped = n.clamp(0.0, 255.0);
    let floor = clamped.floor();
    let fraction = clamped - floor;
    let rounded = if fraction < 0.5 {
        floor
    } else if fraction > 0.5 || floor % 2.0 != 0.0 {
        floor + 1.0
    } else {
        floor
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur ramenée dans [0, 255]"
    )]
    let byte = rounded as u8;
    byte
}

/// Bits du demi-flottant (binary16) le plus proche de `value`, les demis
/// arrondis vers le pair.
pub(super) fn f64_to_f16_bits(value: f64) -> u16 {
    let bits = value.to_bits();
    let sign = if bits >> 63 == 1 { 0x8000_u16 } else { 0 };
    if value.is_nan() {
        return 0x7e00;
    }
    let magnitude = value.abs();
    // 65520 est à mi-chemin entre le plus grand fini (65504) et 2^16.
    if magnitude >= 65520.0 {
        return sign | 0x7c00;
    }
    let exponent = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0) - 1023;
    // En dessous de 2^-25 (moitié du plus petit dénormalisé) : zéro.
    if magnitude == 0.0 || exponent < -25 {
        return sign;
    }
    let significand = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    // Normal : 10 bits de fraction. Dénormalisé : multiples de 2^-24.
    let shift = if exponent >= -14 {
        42
    } else {
        u32::try_from(28 - exponent).unwrap_or(53)
    };
    let mut quotient = significand >> shift;
    let remainder = significand & ((1_u64 << shift) - 1);
    let half = 1_u64 << (shift - 1);
    if remainder > half || (remainder == half && quotient & 1 == 1) {
        quotient += 1;
    }
    if exponent >= -14 {
        let mut biased = exponent + 15;
        if quotient == 2048 {
            quotient = 1024;
            biased += 1;
        }
        if biased >= 31 {
            return sign | 0x7c00;
        }
        let fraction = u16::try_from(quotient & 0x3ff).unwrap_or(0);
        sign | (u16::try_from(biased).unwrap_or(0) << 10) | fraction
    } else {
        // `quotient` vaut au plus 1024, qui est le plus petit nombre normal.
        sign | u16::try_from(quotient).unwrap_or(0)
    }
}

pub(super) fn f16_bits_to_f64(bits: u16) -> f64 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f64::from(bits & 0x3ff);
    let magnitude = match exponent {
        0 => fraction * 2.0_f64.powi(-24),
        31 if fraction == 0.0 => f64::INFINITY,
        31 => return f64::NAN,
        _ => (1.0 + fraction / 1024.0) * 2.0_f64.powi(exponent - 15),
    };
    sign * magnitude
}

/// Octets (petit-boutistes) de `value` converti au type `kind`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "troncature modulo 2^n voulue par ECMA-262"
)]
fn encode(kind: ElementKind, value: f64) -> ([u8; 8], usize) {
    let mut out = [0_u8; 8];
    let size = kind.size();
    match kind {
        ElementKind::Int8 => out[0] = (to_int32(value) as i8).to_le_bytes()[0],
        ElementKind::Uint8 => out[0] = to_uint32(value) as u8,
        ElementKind::Uint8Clamped => out[0] = clamp_u8(value),
        ElementKind::Int16 => out[..2].copy_from_slice(&(to_int32(value) as i16).to_le_bytes()),
        ElementKind::Uint16 => out[..2].copy_from_slice(&(to_uint32(value) as u16).to_le_bytes()),
        ElementKind::Int32 => out[..4].copy_from_slice(&to_int32(value).to_le_bytes()),
        ElementKind::Uint32 => out[..4].copy_from_slice(&to_uint32(value).to_le_bytes()),
        ElementKind::Float16 => out[..2].copy_from_slice(&f64_to_f16_bits(value).to_le_bytes()),
        ElementKind::Float32 => out[..4].copy_from_slice(&(value as f32).to_le_bytes()),
        ElementKind::Float64 => out.copy_from_slice(&value.to_le_bytes()),
        // Les éléments BigInt passent par `encode_value`.
        ElementKind::BigInt64 | ElementKind::BigUint64 => {}
    }
    (out, size)
}

/// Octets d'une valeur déjà convertie au type des éléments (nombre ou BigInt).
pub(super) fn encode_value(kind: ElementKind, value: &Value) -> ([u8; 8], usize) {
    match value {
        Value::BigInt(big) => (big.low_u64().to_le_bytes(), 8),
        Value::Number(n) => encode(kind, *n),
        _ => ([0; 8], kind.size()),
    }
}

/// Valeur lue dans `bytes` : nombre, ou BigInt pour les types 64 bits entiers.
pub(super) fn decode_value(kind: ElementKind, bytes: &[u8]) -> Value {
    let mut raw = [0_u8; 8];
    for (slot, byte) in raw.iter_mut().zip(bytes) {
        *slot = *byte;
    }
    match kind {
        ElementKind::BigInt64 => Value::BigInt(Arc::new(BigInt::from_i64(i64::from_le_bytes(raw)))),
        ElementKind::BigUint64 => {
            Value::BigInt(Arc::new(BigInt::from_u64(u64::from_le_bytes(raw))))
        }
        _ => Value::Number(decode(kind, bytes)),
    }
}

/// Convertit `value` au type des éléments : `ToBigInt` pour les types 64 bits
/// entiers, `ToNumber` pour les autres.
pub(super) fn convert(vm: &mut Vm, kind: ElementKind, value: &Value) -> JsResult<Value> {
    if matches!(kind, ElementKind::BigInt64 | ElementKind::BigUint64) {
        super::bigint::to_bigint(vm, value).map(Value::BigInt)
    } else {
        vm.to_number(value).map(Value::Number)
    }
}

/// Valeur lue dans `bytes` (petit-boutiste, au moins `kind.size()` octets).
fn decode(kind: ElementKind, bytes: &[u8]) -> f64 {
    let take = |n: usize| -> [u8; 8] {
        let mut buffer = [0_u8; 8];
        for (slot, byte) in buffer.iter_mut().zip(bytes.iter().take(n)) {
            *slot = *byte;
        }
        buffer
    };
    let b = take(kind.size());
    match kind {
        ElementKind::Int8 => f64::from(i8::from_le_bytes([b[0]])),
        ElementKind::Uint8 | ElementKind::Uint8Clamped => f64::from(b[0]),
        ElementKind::Int16 => f64::from(i16::from_le_bytes([b[0], b[1]])),
        ElementKind::Uint16 => f64::from(u16::from_le_bytes([b[0], b[1]])),
        ElementKind::Int32 => f64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        ElementKind::Uint32 => f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        ElementKind::Float16 => f16_bits_to_f64(u16::from_le_bytes([b[0], b[1]])),
        ElementKind::Float32 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        ElementKind::Float64 => f64::from_le_bytes(b),
        ElementKind::BigInt64 | ElementKind::BigUint64 => 0.0,
    }
}

#[expect(clippy::cast_precision_loss, reason = "tailles petites devant 2^53")]
pub(super) fn number(n: usize) -> Value {
    Value::Number(n as f64)
}

/// `ToIndex` : entier dans `[0, 2^53 - 1]`, sinon `RangeError`.
pub(super) fn to_index(vm: &mut Vm, value: &Value) -> JsResult<usize> {
    if *value == Value::Undefined {
        return Ok(0);
    }
    let n = vm.to_number(value)?;
    let n = if n.is_nan() { 0.0 } else { n.trunc() };
    if !(0.0..=9_007_199_254_740_991.0).contains(&n) {
        return Err(vm.error("RangeError", "index ou longueur invalide"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "entier vérifié dans [0, 2^53 - 1]"
    )]
    Ok(n as usize)
}

// ---- Accès aux tampons et aux vues -----------------------------------------------

impl Vm {
    pub(super) fn buffer(&self, id: ObjId) -> Option<&BufferData> {
        match &self.heap.get(id).kind {
            Kind::ArrayBuffer(data) => Some(data),
            _ => None,
        }
    }

    pub(super) fn buffer_mut(&mut self, id: ObjId) -> Option<&mut BufferData> {
        match &mut self.heap.get_mut(id).kind {
            Kind::ArrayBuffer(data) => Some(data),
            _ => None,
        }
    }

    /// Nouveau tampon de `len` octets nuls.
    fn new_buffer(&mut self, len: usize, max_len: Option<usize>) -> JsResult<ObjId> {
        if len > MAX_BUFFER || max_len.is_some_and(|max| max > MAX_BUFFER) {
            return Err(vm_range(self, "tampon trop grand"));
        }
        Ok(self.heap.alloc(Object::new(
            Some(self.intrinsics.array_buffer_proto),
            Kind::ArrayBuffer(Box::new(BufferData::new(len, max_len))),
        )))
    }

    /// Prototype de `SharedArrayBuffer` : rangé à la suite de ceux des
    /// tableaux typés, pour suivre le royaume courant comme eux.
    fn shared_buffer_proto(&self) -> Option<ObjId> {
        self.typed_protos.get(KINDS.len()).copied()
    }

    /// Constructeur `Uint8Array` d'origine du royaume courant.
    pub(super) fn uint8_array_constructor(&self) -> Option<Value> {
        self.typed_protos
            .get(KINDS.len() + 1 + ElementKind::Uint8.index())
            .map(|id| Value::Object(*id))
    }

    /// Nouveau tampon partagé de `len` octets nuls.
    fn new_shared_buffer(&mut self, len: usize, max_len: Option<usize>) -> JsResult<ObjId> {
        if len > MAX_BUFFER || max_len.is_some_and(|max| max > MAX_BUFFER) {
            return Err(vm_range(self, "tampon trop grand"));
        }
        Ok(self.heap.alloc(Object::new(
            self.shared_buffer_proto(),
            Kind::ArrayBuffer(Box::new(BufferData::new_shared(len, max_len))),
        )))
    }

    /// Stockage du tampon partagé `value`, pour le confier à un autre agent.
    #[must_use]
    pub fn shared_buffer_storage(&self, value: &Value) -> Option<SharedStorage> {
        let buffer = self.buffer(value.as_object()?)?;
        Some(SharedStorage {
            bytes: Arc::clone(buffer.shared.as_ref()?),
            max_len: buffer.max_len,
        })
    }

    /// `SharedArrayBuffer` de cet agent sur un stockage reçu d'un autre.
    pub fn shared_buffer_from_storage(&mut self, storage: &SharedStorage) -> Value {
        let data = BufferData::from_shared(Arc::clone(&storage.bytes), storage.max_len);
        Value::Object(self.heap.alloc(Object::new(
            self.shared_buffer_proto(),
            Kind::ArrayBuffer(Box::new(data)),
        )))
    }

    /// Détache le tampon `value` : ses octets sont libérés et toutes ses vues
    /// deviennent vides. Sert à l'hôte (transfert de tampons) et aux tests.
    ///
    /// # Errors
    /// `TypeError` si `value` n'est pas un `ArrayBuffer`.
    pub fn detach_array_buffer(&mut self, value: &Value) -> JsResult<()> {
        match value.as_object().and_then(|id| self.buffer_mut(id)) {
            Some(data) => {
                if data.shared.is_some() {
                    return Err(self.type_error("un SharedArrayBuffer ne se détache pas"));
                }
                if data.immutable {
                    return Err(self.type_error("un ArrayBuffer immuable ne se détache pas"));
                }
                data.take_bytes();
                data.detached = true;
                Ok(())
            }
            None => Err(self.type_error("un ArrayBuffer est attendu")),
        }
    }

    /// Vrai si `id` est un tableau typé posé sur un tampon immuable.
    pub(crate) fn typed_immutable(&self, id: ObjId) -> bool {
        self.typed_array(id)
            .and_then(|view| self.buffer(view.buffer))
            .is_some_and(|buffer| buffer.immutable)
    }

    pub(crate) fn typed_array(&self, id: ObjId) -> Option<TypedArrayData> {
        match &self.heap.get(id).kind {
            Kind::TypedArray(data) => Some(*data),
            _ => None,
        }
    }

    /// Longueur courante du tableau typé `id`, en éléments ; `None` s'il est
    /// hors des bornes de son tampon (tampon détaché ou rétréci).
    pub(crate) fn typed_length(&self, id: ObjId) -> Option<usize> {
        let view = self.typed_array(id)?;
        let buffer = self.buffer(view.buffer)?;
        if buffer.detached {
            return None;
        }
        let size = view.kind.size();
        let available = buffer.len().checked_sub(view.offset)?;
        match view.length {
            Some(length) => (length.checked_mul(size)? <= available).then_some(length),
            None => Some(available / size),
        }
    }

    /// Élément `index` du tableau typé `id`, s'il existe.
    pub(crate) fn typed_get(&self, id: ObjId, index: usize) -> Option<Value> {
        if index >= self.typed_length(id)? {
            return None;
        }
        let view = self.typed_array(id)?;
        let start = view.offset + index * view.kind.size();
        let size = view.kind.size();
        let bytes = self.buffer(view.buffer)?.read(start, size)?;
        Some(decode_value(view.kind, &bytes[..size]))
    }

    /// Convertit `value` au type des éléments, puis l'écrit dans l'élément
    /// `index` s'il existe. La conversion a lieu même sans élément à écrire.
    pub(crate) fn typed_store(
        &mut self,
        id: ObjId,
        index: Option<usize>,
        value: &Value,
    ) -> JsResult<()> {
        let Some(view) = self.typed_array(id) else {
            return Ok(());
        };
        let converted = convert(self, view.kind, value)?;
        if let Some(index) = index {
            self.typed_set(id, index, &converted);
        }
        Ok(())
    }

    /// Écrit dans l'élément `index` une valeur déjà convertie au type des
    /// éléments ; sans effet hors bornes.
    pub(crate) fn typed_set(&mut self, id: ObjId, index: usize, value: &Value) {
        let Some(view) = self.typed_array(id) else {
            return;
        };
        if self.typed_length(id).is_none_or(|len| index >= len) || self.typed_immutable(id) {
            return;
        }
        let (bytes, size) = encode_value(view.kind, value);
        let start = view.offset + index * size;
        if let Some(buffer) = self.buffer_mut(view.buffer) {
            buffer.write(start, &bytes[..size]);
        }
    }

    /// Nouveau tableau typé de type `kind` et de `len` éléments nuls.
    fn new_typed(&mut self, kind: ElementKind, len: usize) -> JsResult<ObjId> {
        let Some(bytes) = len.checked_mul(kind.size()) else {
            return Err(vm_range(self, "tableau typé trop grand"));
        };
        let buffer = self.new_buffer(bytes, None)?;
        Ok(self.typed_view(kind, buffer, 0, Some(len)))
    }

    fn typed_view(
        &mut self,
        kind: ElementKind,
        buffer: ObjId,
        offset: usize,
        length: Option<usize>,
    ) -> ObjId {
        let proto = self.typed_protos.get(kind.index()).copied();
        self.heap.alloc(Object::new(
            proto,
            Kind::TypedArray(TypedArrayData {
                buffer,
                kind,
                offset,
                length,
            }),
        ))
    }
}

fn vm_range(vm: &mut Vm, message: &str) -> Value {
    vm.error("RangeError", message)
}

/// Tableau typé désigné par `this`, avec sa longueur ; `TypeError` si ce n'en
/// est pas un ou s'il est hors des bornes de son tampon.
pub(super) fn validate(vm: &mut Vm, this: &Value) -> JsResult<(ObjId, usize)> {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    match vm.typed_length(id) {
        Some(len) => Ok((id, len)),
        None => Err(vm.type_error("le tableau typé est détaché ou hors des bornes de son tampon")),
    }
}

/// Comme [`validate`], pour une écriture : `TypeError` aussi si le tampon du
/// tableau est immuable.
pub(super) fn validate_write(vm: &mut Vm, this: &Value) -> JsResult<(ObjId, usize)> {
    if this.as_object().is_some_and(|id| vm.typed_immutable(id)) {
        return Err(vm.type_error("le tampon du tableau typé est immuable"));
    }
    validate(vm, this)
}

// ---- ArrayBuffer -----------------------------------------------------------------

fn array_buffer_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("ArrayBuffer doit être appelé avec « new »"));
    }
    let len = to_index(vm, &arg(args, 0))?;
    let max_len = max_byte_length(vm, &arg(args, 1))?;
    if max_len.is_some_and(|max| len > max) {
        return Err(vm_range(vm, "byteLength dépasse maxByteLength"));
    }
    Ok(Value::Object(vm.new_buffer(len, max_len)?))
}

/// Stockage d'un `SharedArrayBuffer`, à confier à un autre agent : c'est la
/// seule donnée d'un moteur qui se partage entre fils.
#[derive(Clone, Debug)]
pub struct SharedStorage {
    bytes: Arc<crate::heap::SharedBytes>,
    max_len: Option<usize>,
}

/// Tampon ordinaire désigné par `this` ; un `SharedArrayBuffer` n'en est pas un.
fn this_buffer(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this
        .as_object()
        .filter(|id| vm.buffer(*id).is_some_and(|b| b.shared.is_none()))
    {
        Some(id) => Ok(id),
        None => Err(vm.type_error("le receveur n'est pas un ArrayBuffer")),
    }
}

/// Tampon partagé désigné par `this`.
fn this_shared(vm: &mut Vm, this: &Value) -> JsResult<ObjId> {
    match this
        .as_object()
        .filter(|id| vm.buffer(*id).is_some_and(|b| b.shared.is_some()))
    {
        Some(id) => Ok(id),
        None => Err(vm.type_error("le receveur n'est pas un SharedArrayBuffer")),
    }
}

/// Taille maximale demandée par l'option `maxByteLength`, s'il y en a une.
fn max_byte_length(vm: &mut Vm, options: &Value) -> JsResult<Option<usize>> {
    if options.as_object().is_none() {
        return Ok(None);
    }
    match vm.get(options, "maxByteLength")? {
        Value::Undefined => Ok(None),
        other => to_index(vm, &other).map(Some),
    }
}

fn shared_buffer_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("SharedArrayBuffer doit être appelé avec « new »"));
    }
    let len = to_index(vm, &arg(args, 0))?;
    let max_len = max_byte_length(vm, &arg(args, 1))?;
    if max_len.is_some_and(|max| len > max) {
        return Err(vm_range(vm, "byteLength dépasse maxByteLength"));
    }
    Ok(Value::Object(vm.new_shared_buffer(len, max_len)?))
}

fn shared_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_shared(vm, this)?;
    Ok(number(vm.buffer(id).map_or(0, BufferData::len)))
}

fn shared_max_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_shared(vm, this)?;
    Ok(number(
        vm.buffer(id).map_or(0, |b| b.max_len.unwrap_or(b.len())),
    ))
}

fn shared_growable(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_shared(vm, this)?;
    Ok(Value::Bool(
        vm.buffer(id).is_some_and(|b| b.max_len.is_some()),
    ))
}

fn shared_grow(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_shared(vm, this)?;
    let Some(max) = vm.buffer(id).and_then(|b| b.max_len) else {
        return Err(vm.type_error("ce SharedArrayBuffer n'est pas extensible"));
    };
    let len = to_index(vm, &arg(args, 0))?;
    if len > max {
        return Err(vm_range(vm, "taille au-delà de maxByteLength"));
    }
    let grown = vm
        .buffer(id)
        .and_then(|b| b.shared.as_ref())
        .is_some_and(|shared| shared.grow(len));
    if !grown {
        return Err(vm_range(vm, "un SharedArrayBuffer ne rétrécit pas"));
    }
    Ok(Value::Undefined)
}

/// `SpeciesConstructor` : le constructeur que désigne `Symbol.species` sur
/// le constructeur de `object`, sinon `default`.
fn species_constructor(vm: &mut Vm, object: &Value, default: Value) -> JsResult {
    let constructor = vm.get(object, "constructor")?;
    if constructor == Value::Undefined {
        return Ok(default);
    }
    if constructor.as_object().is_none() {
        return Err(vm.type_error("« constructor » doit être un objet"));
    }
    let key = PropertyKey::Symbol(vm.well_known.species.clone());
    let species = vm.get_key(&constructor, &key)?;
    if species.is_nullish() {
        return Ok(default);
    }
    if !vm.is_constructor(&species) {
        return Err(vm.type_error("« Symbol.species » doit être un constructeur"));
    }
    Ok(species)
}

/// Constructeur à employer pour la copie rendue par `slice` d'un tampon.
fn buffer_species(vm: &mut Vm, this: &Value, default: &str) -> JsResult {
    let global = Value::Object(vm.global);
    let default = vm.get(&global, default)?;
    species_constructor(vm, this, default)
}

/// Vrai pour les types dont les éléments sont des BigInt.
fn is_bigint_kind(kind: ElementKind) -> bool {
    matches!(kind, ElementKind::BigInt64 | ElementKind::BigUint64)
}

/// `TypedArrayCreateFromCtor` : construit par `constructor` et vérifie que
/// le résultat est un tableau typé vivant, assez long si une longueur était
/// demandée ; `write` : et qu'on peut y écrire.
fn create_from_constructor(
    vm: &mut Vm,
    constructor: &Value,
    args: &[Value],
    write: bool,
) -> JsResult<ObjId> {
    let created = vm.construct(constructor, args, constructor)?;
    let (id, length) = if write {
        validate_write(vm, &created)?
    } else {
        validate(vm, &created)?
    };
    #[expect(clippy::cast_precision_loss, reason = "longueurs petites devant 2^53")]
    if let [Value::Number(wanted)] = args
        && (length as f64) < *wanted
    {
        return Err(vm.type_error("le tableau typé construit est trop court"));
    }
    Ok(id)
}

/// `TypedArraySpeciesCreate` : un tableau du constructeur que désigne
/// l'espèce d'`exemplar`, de même nature d'éléments (nombres ou BigInt).
fn species_create(vm: &mut Vm, exemplar: ObjId, args: &[Value], write: bool) -> JsResult<ObjId> {
    let kind = kind_of(vm, exemplar);
    let default = vm
        .typed_protos
        .get(KINDS.len() + 1 + kind.index())
        .copied()
        .map_or(Value::Undefined, Value::Object);
    let constructor = species_constructor(vm, &Value::Object(exemplar), default)?;
    let created = create_from_constructor(vm, &constructor, args, write)?;
    if is_bigint_kind(kind_of(vm, created)) != is_bigint_kind(kind) {
        return Err(vm.type_error("le tableau typé construit n'a pas le même type de contenu"));
    }
    Ok(created)
}

fn typed_species(_vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this.clone())
}

/// `slice` commun aux deux sortes de tampon (`shared` dit laquelle).
fn slice_buffer(vm: &mut Vm, this: &Value, args: &[Value], shared: bool) -> JsResult {
    let id = if shared {
        this_shared(vm, this)?
    } else {
        let id = this_buffer(vm, this)?;
        live_buffer(vm, id)?;
        id
    };
    let len = vm.buffer(id).map_or(0, BufferData::len);
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 1), len, len)?;
    let count = end.saturating_sub(start);
    let default = if shared {
        "SharedArrayBuffer"
    } else {
        "ArrayBuffer"
    };
    let constructor = buffer_species(vm, this, default)?;
    let created = vm.construct(&constructor, &[number(count)], &constructor)?;
    // Le constructeur vient du script : ce qu'il rend est vérifié.
    let Some(copy) = created.as_object().filter(|copy| {
        vm.buffer(*copy)
            .is_some_and(|b| b.shared.is_some() == shared)
    }) else {
        return Err(vm.type_error("le constructeur n'a pas rendu un tampon de la bonne sorte"));
    };
    if !shared {
        live_buffer(vm, copy)?;
        mutable_buffer(vm, copy)?;
    }
    if copy == id {
        return Err(vm.type_error("le constructeur a rendu le tampon lui-même"));
    }
    if vm.buffer(copy).map_or(0, BufferData::len) < count {
        return Err(vm.type_error("le tampon rendu par le constructeur est trop petit"));
    }
    // Les conversions et le constructeur ont pu détacher ou rétrécir le tampon.
    if !shared {
        live_buffer(vm, id)?;
    }
    let current = vm.buffer(id).map_or(0, BufferData::len);
    let available = current.saturating_sub(start).min(count);
    let bytes: Vec<u8> = vm
        .buffer(id)
        .map(|b| b.copy_range(start, available))
        .unwrap_or_default();
    if let Some(target) = vm.buffer_mut(copy) {
        target.write(0, &bytes);
    }
    Ok(created)
}

fn shared_slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    slice_buffer(vm, this, args, true)
}

fn array_buffer_is_view(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let is_view = arg(args, 0).as_object().is_some_and(|id| {
        matches!(
            vm.heap.get(id).kind,
            Kind::TypedArray(_) | Kind::DataView { .. }
        )
    });
    Ok(Value::Bool(is_view))
}

fn buffer_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    Ok(number(vm.buffer(id).map_or(0, BufferData::len)))
}

fn buffer_max_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    let buffer = vm.buffer(id);
    Ok(number(buffer.map_or(0, |b| {
        if b.detached {
            0
        } else {
            b.max_len.unwrap_or(b.len())
        }
    })))
}

fn buffer_resizable(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    Ok(Value::Bool(
        vm.buffer(id).is_some_and(|b| b.max_len.is_some()),
    ))
}

fn buffer_detached(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    Ok(Value::Bool(vm.buffer(id).is_some_and(|b| b.detached)))
}

/// Tampon non détaché désigné par `id`, sinon `TypeError`.
fn live_buffer(vm: &mut Vm, id: ObjId) -> JsResult<()> {
    if vm.buffer(id).is_none_or(|b| b.detached) {
        return Err(vm.type_error("l'ArrayBuffer est détaché"));
    }
    Ok(())
}

/// Tampon modifiable désigné par `id`, sinon `TypeError`.
fn mutable_buffer(vm: &mut Vm, id: ObjId) -> JsResult<()> {
    if vm.buffer(id).is_some_and(|b| b.immutable) {
        return Err(vm.type_error("l'ArrayBuffer est immuable"));
    }
    Ok(())
}

fn buffer_immutable(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    Ok(Value::Bool(vm.buffer(id).is_some_and(|b| b.immutable)))
}

/// Nouveau tampon immuable contenant `bytes`.
fn new_immutable_buffer(vm: &mut Vm, bytes: Vec<u8>) -> JsResult<ObjId> {
    let copy = vm.new_buffer(bytes.len(), None)?;
    if let Some(target) = vm.buffer_mut(copy) {
        target.set_bytes(bytes);
        target.immutable = true;
    }
    Ok(copy)
}

/// `ArrayBuffer.prototype.transferToImmutable(longueur)` : le contenu passe
/// dans un tampon immuable, le receveur est détaché.
fn buffer_transfer_immutable(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    let len = match arg(args, 0) {
        Value::Undefined => vm.buffer(id).map_or(0, BufferData::len),
        other => to_index(vm, &other)?,
    };
    live_buffer(vm, id)?;
    mutable_buffer(vm, id)?;
    // La taille est vérifiée avant que le receveur ne perde ses octets.
    if u32::try_from(len).is_err() {
        return Err(vm_range(vm, "tampon trop grand"));
    }
    let mut bytes = vm
        .buffer_mut(id)
        .map(BufferData::take_bytes)
        .unwrap_or_default();
    bytes.resize(len, 0);
    let copy = new_immutable_buffer(vm, bytes)?;
    vm.detach_array_buffer(this)?;
    Ok(Value::Object(copy))
}

/// `ArrayBuffer.prototype.sliceToImmutable(début, fin)` : copie immuable
/// d'une tranche, sans passer par l'espèce.
fn buffer_slice_immutable(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    live_buffer(vm, id)?;
    let len = vm.buffer(id).map_or(0, BufferData::len);
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 1), len, len)?;
    let count = end.saturating_sub(start);
    // Les conversions ont pu détacher ou rétrécir le tampon.
    live_buffer(vm, id)?;
    let current = vm.buffer(id).map_or(0, BufferData::len);
    if count > 0 && current < end {
        return Err(vm_range(
            vm,
            "le tampon a rétréci en deçà de la tranche demandée",
        ));
    }
    let bytes = vm
        .buffer(id)
        .map(|b| b.copy_range(start, count))
        .unwrap_or_default();
    new_immutable_buffer(vm, bytes).map(Value::Object)
}

fn buffer_slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    slice_buffer(vm, this, args, false)
}

fn buffer_resize(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let id = this_buffer(vm, this)?;
    mutable_buffer(vm, id)?;
    let Some(max) = vm.buffer(id).and_then(|b| b.max_len) else {
        return Err(vm.type_error("cet ArrayBuffer n'est pas redimensionnable"));
    };
    let len = to_index(vm, &arg(args, 0))?;
    live_buffer(vm, id)?;
    if len > max {
        return Err(vm_range(vm, "taille au-delà de maxByteLength"));
    }
    if let Some(buffer) = vm.buffer_mut(id) {
        buffer.resize(len);
    }
    Ok(Value::Undefined)
}

fn transfer(vm: &mut Vm, this: &Value, args: &[Value], keep_resizable: bool) -> JsResult {
    let id = this_buffer(vm, this)?;
    let len = match arg(args, 0) {
        Value::Undefined => vm.buffer(id).map_or(0, BufferData::len),
        other => to_index(vm, &other)?,
    };
    live_buffer(vm, id)?;
    mutable_buffer(vm, id)?;
    let max_len = vm
        .buffer(id)
        .and_then(|b| b.max_len)
        .filter(|_| keep_resizable);
    if max_len.is_some_and(|max| len > max) {
        return Err(vm_range(vm, "taille au-delà de maxByteLength"));
    }
    let copy = vm.new_buffer(len, max_len)?;
    let mut bytes = vm
        .buffer_mut(id)
        .map(BufferData::take_bytes)
        .unwrap_or_default();
    bytes.resize(len, 0);
    if let Some(target) = vm.buffer_mut(copy) {
        target.set_bytes(bytes);
    }
    vm.detach_array_buffer(this)?;
    Ok(Value::Object(copy))
}

fn buffer_transfer(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    transfer(vm, this, args, true)
}

fn buffer_transfer_fixed(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    transfer(vm, this, args, false)
}

// ---- Construction des tableaux typés ---------------------------------------------

fn construct(vm: &mut Vm, kind: ElementKind, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error(&format!("{} doit être appelé avec « new »", kind.name())));
    }
    let source = arg(args, 0);
    let Some(source_id) = source.as_object() else {
        let len = to_index(vm, &source)?;
        return Ok(Value::Object(vm.new_typed(kind, len)?));
    };
    let size = kind.size();
    if vm.buffer(source_id).is_some() {
        let offset = to_index(vm, &arg(args, 1))?;
        if offset % size != 0 {
            return Err(vm_range(
                vm,
                "décalage non multiple de la taille des éléments",
            ));
        }
        let length = match arg(args, 2) {
            Value::Undefined => None,
            other => Some(to_index(vm, &other)?),
        };
        live_buffer(vm, source_id)?;
        let (buffer_len, resizable) = vm
            .buffer(source_id)
            .map_or((0, false), |b| (b.len(), b.max_len.is_some()));
        let length = match length {
            Some(length) => {
                if length
                    .checked_mul(size)
                    .and_then(|bytes| bytes.checked_add(offset))
                    .is_none_or(|end| end > buffer_len)
                {
                    return Err(vm_range(vm, "la vue dépasse la fin du tampon"));
                }
                Some(length)
            }
            // Sur un tampon redimensionnable, la vue suit sa longueur.
            None if resizable => {
                if offset > buffer_len {
                    return Err(vm_range(vm, "décalage au-delà de la fin du tampon"));
                }
                None
            }
            None => {
                if buffer_len % size != 0 || offset > buffer_len {
                    return Err(vm_range(vm, "taille du tampon incompatible avec ce type"));
                }
                Some((buffer_len - offset) / size)
            }
        };
        return Ok(Value::Object(
            vm.typed_view(kind, source_id, offset, length),
        ));
    }
    // Autre tableau typé, itérable ou objet de type tableau : copie des valeurs.
    let values: Vec<Value> = if vm.typed_array(source_id).is_some() {
        let (_, len) = validate(vm, &source)?;
        (0..len)
            .filter_map(|i| vm.typed_get(source_id, i))
            .collect()
    } else {
        let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
        if vm.get_key(&source, &key)?.is_nullish() {
            super::array_like_to_vec(vm, &source)?
        } else {
            vm.iterate_to_vec(&source)?
        }
    };
    let id = vm.new_typed(kind, values.len())?;
    for (index, value) in values.iter().enumerate() {
        vm.typed_store(id, Some(index), value)?;
    }
    Ok(Value::Object(id))
}

macro_rules! typed_constructors {
    ($($name:ident => $kind:ident),* $(,)?) => {
        $(
            fn $name(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
                construct(vm, ElementKind::$kind, args)
            }
        )*
        const CONSTRUCTORS: [NativeFn; 12] = [$($name),*];
    };
}
typed_constructors! {
    ctor_int8 => Int8, ctor_uint8 => Uint8, ctor_uint8_clamped => Uint8Clamped,
    ctor_int16 => Int16, ctor_uint16 => Uint16, ctor_int32 => Int32, ctor_uint32 => Uint32,
    ctor_float16 => Float16, ctor_float32 => Float32, ctor_float64 => Float64,
    ctor_bigint64 => BigInt64, ctor_biguint64 => BigUint64,
}

fn typed_array_abstract(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Err(vm.type_error("TypedArray est abstrait : utiliser Int8Array, Float64Array…"))
}

/// `%TypedArray%.from(source, transformation, receveur)`.
fn typed_from(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if !vm.is_constructor(this) {
        return Err(vm.type_error("le receveur n'est pas un constructeur"));
    }
    let (source, map, this_arg) = (arg(args, 0), arg(args, 1), arg(args, 2));
    if map != Value::Undefined && !is_callable(vm, &map) {
        return Err(vm.type_error("la fonction de transformation n'est pas appelable"));
    }
    let key = PropertyKey::Symbol(vm.well_known.iterator.clone());
    // Un itérable est parcouru en entier avant la création du tableau ; un
    // objet à indices est lu élément par élément, au fil de l'écriture.
    // La méthode d'itération n'est lue qu'une fois.
    let method = vm.get_key(&source, &key)?;
    let iterated = if method.is_nullish() {
        None
    } else {
        let iterator = vm.iterator_from_method(&source, &method)?;
        let mut values = Vec::new();
        while let Some(value) = vm.iterator_next(&iterator)? {
            vm.tick(1)?;
            values.push(value);
        }
        Some(values)
    };
    let object = match iterated {
        Some(_) => Value::Undefined,
        None => vm.to_object(&source)?,
    };
    let len = match &iterated {
        Some(values) => values.len(),
        None => super::array::length_of(vm, &object)?,
    };
    let target = create_from_constructor(vm, this, &[number(len)], true)?;
    for index in 0..len {
        vm.tick(1)?;
        let value = match &iterated {
            Some(values) => values.get(index).cloned().unwrap_or(Value::Undefined),
            None => vm.get(&object, &index.to_string())?,
        };
        let value = if map == Value::Undefined {
            value
        } else {
            vm.call(&map, &this_arg, &[value, number(index)])?
        };
        vm.typed_store(target, Some(index), &value)?;
    }
    Ok(Value::Object(target))
}

fn typed_of(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if !vm.is_constructor(this) {
        return Err(vm.type_error("le receveur n'est pas un constructeur"));
    }
    let target = create_from_constructor(vm, this, &[number(args.len())], true)?;
    for (index, value) in args.iter().enumerate() {
        vm.typed_store(target, Some(index), value)?;
    }
    Ok(Value::Object(target))
}

// ---- Prototype des tableaux typés ------------------------------------------------

fn typed_buffer(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    match this.as_object().and_then(|id| vm.typed_array(id)) {
        Some(view) => Ok(Value::Object(view.buffer)),
        None => Err(vm.type_error("le receveur n'est pas un tableau typé")),
    }
}

fn typed_byte_offset(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    let offset = vm.typed_array(id).map_or(0, |v| v.offset);
    Ok(number(if vm.typed_length(id).is_some() {
        offset
    } else {
        0
    }))
}

fn typed_length_getter(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    Ok(number(vm.typed_length(id).unwrap_or(0)))
}

fn typed_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    let size = vm.typed_array(id).map_or(1, |v| v.kind.size());
    Ok(number(vm.typed_length(id).unwrap_or(0) * size))
}

fn typed_tag(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    Ok(this
        .as_object()
        .and_then(|id| vm.typed_array(id))
        .map_or(Value::Undefined, |view| Value::string(view.kind.name())))
}

/// Méthodes dont l'algorithme est celui de `Array.prototype`, une fois le
/// receveur validé (longueur interne, pas de trous).
macro_rules! shared_methods {
    ($($name:ident => $array:ident),* $(,)?) => {
        $(
            fn $name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
                validate(vm, this)?;
                array::$array(vm, this, args)
            }
        )*
    };
}
shared_methods! {
    typed_at => at,
    typed_find => find, typed_find_index => find_index, typed_find_last => find_last,
    typed_find_last_index => find_last_index, typed_includes => includes,
    typed_index_of => index_of, typed_last_index_of => last_index_of, typed_join => join,
}

fn typed_reverse(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    validate_write(vm, this)?;
    array::reverse(vm, this, args)
}

fn typed_copy_within(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    validate_write(vm, this)?;
    array::copy_within(vm, this, args)
}

/// Parcourt les `len` éléments que le tableau avait à l'appel, dans l'ordre
/// (`backwards` : à rebours), en appelant `callback(élément, index, tableau)`.
/// Un élément disparu en route — tampon rétréci ou détaché par le rappel —
/// est lu `undefined`. `visit` reçoit chaque résultat et rend `false` pour
/// arrêter le parcours.
fn each_element(
    vm: &mut Vm,
    this: &Value,
    args: &[Value],
    mut visit: impl FnMut(&Value) -> bool,
) -> JsResult<()> {
    let (id, len) = validate(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let this_arg = arg(args, 1);
    for index in 0..len {
        vm.tick(1)?;
        let value = vm.typed_get(id, index).unwrap_or(Value::Undefined);
        let result = vm.call(&callback, &this_arg, &[value, number(index), this.clone()])?;
        if !visit(&result) {
            break;
        }
    }
    Ok(())
}

fn typed_every(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut all = true;
    each_element(vm, this, args, |result| {
        all = result.truthy();
        all
    })?;
    Ok(Value::Bool(all))
}

fn typed_some(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let mut any = false;
    each_element(vm, this, args, |result| {
        any = result.truthy();
        !any
    })?;
    Ok(Value::Bool(any))
}

fn typed_for_each(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    each_element(vm, this, args, |_| true)?;
    Ok(Value::Undefined)
}

/// `reduce` et `reduceRight` (`backwards`).
fn reduce_elements(vm: &mut Vm, this: &Value, args: &[Value], backwards: bool) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let mut order: Vec<usize> = (0..len).collect();
    if backwards {
        order.reverse();
    }
    let mut indices = order.into_iter();
    let mut accumulator = if args.len() >= 2 {
        arg(args, 1)
    } else {
        match indices.next() {
            Some(first) => vm.typed_get(id, first).unwrap_or(Value::Undefined),
            None => return Err(vm.type_error("reduce d'un tableau vide sans valeur initiale")),
        }
    };
    for index in indices {
        vm.tick(1)?;
        let value = vm.typed_get(id, index).unwrap_or(Value::Undefined);
        accumulator = vm.call(
            &callback,
            &Value::Undefined,
            &[accumulator, value, number(index), this.clone()],
        )?;
    }
    Ok(accumulator)
}

fn typed_reduce(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_elements(vm, this, args, false)
}

fn typed_reduce_right(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    reduce_elements(vm, this, args, true)
}

/// `toLocaleString` : chaque élément par sa propre méthode `toLocaleString`.
fn typed_to_locale_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let mut out = String::new();
    for index in 0..len {
        vm.tick(1)?;
        if index > 0 {
            out.push(',');
        }
        let value = vm.typed_get(id, index).unwrap_or(Value::Undefined);
        if value.is_nullish() {
            continue;
        }
        let method = vm.get(&value, "toLocaleString")?;
        let text = vm.call(&method, &value, &[])?;
        out.push_str(&vm.to_string(&text)?);
    }
    Ok(Value::string(&out))
}

fn typed_iterator(vm: &mut Vm, this: &Value, kind: IndexedKind) -> JsResult {
    validate(vm, this)?;
    Ok(vm.new_iterator(IterState::Indexed {
        target: this.clone(),
        index: 0,
        kind,
    }))
}

fn typed_keys(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    typed_iterator(vm, this, IndexedKind::Keys)
}

fn typed_values(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    typed_iterator(vm, this, IndexedKind::Values)
}

fn typed_entries(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    typed_iterator(vm, this, IndexedKind::Entries)
}

/// Copie des éléments du tableau typé `id`.
fn elements(vm: &Vm, id: ObjId) -> Vec<Value> {
    let len = vm.typed_length(id).unwrap_or(0);
    (0..len).filter_map(|i| vm.typed_get(id, i)).collect()
}

fn kind_of(vm: &Vm, id: ObjId) -> ElementKind {
    vm.typed_array(id).map_or(ElementKind::Uint8, |v| v.kind)
}

fn typed_fill(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, len) = validate_write(vm, this)?;
    let value = convert(vm, kind_of(vm, id), &arg(args, 0))?;
    let start = super::relative_index(vm, &arg(args, 1), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 2), len, len)?;
    // Les conversions ont pu rétrécir le tableau.
    let (_, len) = validate(vm, this)?;
    for index in start..end.min(len) {
        vm.typed_set(id, index, &value);
    }
    Ok(this.clone())
}

fn typed_map(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let this_arg = arg(args, 1);
    let out = species_create(vm, id, &[number(len)], true)?;
    for index in 0..len {
        vm.tick(1)?;
        let value = vm.typed_get(id, index).unwrap_or(Value::Undefined);
        let mapped = vm.call(&callback, &this_arg, &[value, number(index), this.clone()])?;
        vm.typed_store(out, Some(index), &mapped)?;
    }
    Ok(Value::Object(out))
}

fn typed_filter(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let callback = callable(vm, &arg(args, 0))?;
    let this_arg = arg(args, 1);
    let mut kept = Vec::new();
    for index in 0..len {
        vm.tick(1)?;
        let value = vm.typed_get(id, index).unwrap_or(Value::Undefined);
        let keep = vm.call(
            &callback,
            &this_arg,
            &[value.clone(), number(index), this.clone()],
        )?;
        if keep.truthy() {
            kept.push(value);
        }
    }
    let out = species_create(vm, id, &[number(kept.len())], true)?;
    for (index, value) in kept.iter().enumerate() {
        vm.typed_store(out, Some(index), value)?;
    }
    Ok(Value::Object(out))
}

fn typed_slice(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 1), len, len)?;
    let count = end.saturating_sub(start);
    let out = species_create(vm, id, &[number(count)], true)?;
    if count == 0 {
        return Ok(Value::Object(out));
    }
    // Les conversions et le constructeur ont pu détacher ou rétrécir le tableau.
    let (_, len) = validate(vm, this)?;
    let end = end.min(len);
    let count = end.saturating_sub(start);
    let (Some(source), Some(target)) = (vm.typed_array(id), vm.typed_array(out)) else {
        return Ok(Value::Object(out));
    };
    if source.kind == target.kind {
        // Même type : les octets sont copiés tels quels, sans conversion.
        let size = source.kind.size();
        let available = vm.typed_length(out).unwrap_or(0).min(count);
        let bytes = vm
            .buffer(source.buffer)
            .map(|b| b.copy_range(source.offset + start * size, available * size))
            .unwrap_or_default();
        if let Some(buffer) = vm.buffer_mut(target.buffer) {
            buffer.write(target.offset, &bytes);
        }
    } else {
        for offset in 0..count {
            let value = vm.typed_get(id, start + offset).unwrap_or(Value::Undefined);
            vm.typed_store(out, Some(offset), &value)?;
        }
    }
    Ok(Value::Object(out))
}

fn typed_subarray(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    let view = vm
        .typed_array(id)
        .ok_or_else(|| vm.type_error("le receveur n'est pas un tableau typé"))?;
    let len = vm.typed_length(id).unwrap_or(0);
    let start = super::relative_index(vm, &arg(args, 0), len, 0)?;
    let end = super::relative_index(vm, &arg(args, 1), len, len)?;
    let offset = view.offset + start * view.kind.size();
    let mut arguments = vec![Value::Object(view.buffer), number(offset)];
    // Une vue qui suit la longueur de son tampon engendre une vue qui la suit aussi.
    if view.length.is_some() || arg(args, 1) != Value::Undefined {
        arguments.push(number(end.saturating_sub(start)));
    }
    species_create(vm, id, &arguments, false).map(Value::Object)
}

fn typed_set_method(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let Some(id) = this.as_object().filter(|id| vm.typed_array(*id).is_some()) else {
        return Err(vm.type_error("le receveur n'est pas un tableau typé"));
    };
    if vm.typed_immutable(id) {
        return Err(vm.type_error("le tampon du tableau typé est immuable"));
    }
    let offset = vm.to_number(&arg(args, 1))?;
    let offset = if offset.is_nan() { 0.0 } else { offset.trunc() };
    if offset < 0.0 {
        return Err(vm_range(vm, "décalage négatif"));
    }
    let (_, len) = validate(vm, this)?;
    let source = arg(args, 0);
    let values: Vec<Value> = match source.as_object().filter(|s| vm.typed_array(*s).is_some()) {
        Some(source_id) => {
            validate(vm, &source)?;
            elements(vm, source_id)
        }
        None => {
            let object = vm.to_object(&source)?;
            super::array_like_to_vec(vm, &object)?
        }
    };
    #[expect(clippy::cast_precision_loss, reason = "longueurs petites devant 2^53")]
    if offset + values.len() as f64 > len as f64 {
        return Err(vm_range(vm, "la source dépasse la fin du tableau typé"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "décalage vérifié dans [0, len]"
    )]
    let offset = offset as usize;
    for (index, value) in values.iter().enumerate() {
        vm.typed_store(id, Some(offset + index), value)?;
    }
    Ok(Value::Undefined)
}

/// Trie des éléments : ordre numérique par défaut (`-0` avant `+0`, `NaN` à
/// la fin), sinon selon `comparator`.
fn sort_elements(vm: &mut Vm, mut values: Vec<Value>, comparator: &Value) -> JsResult<Vec<Value>> {
    if *comparator == Value::Undefined {
        values.sort_by(|a, b| match (a, b) {
            (Value::Number(x), Value::Number(y)) => x.total_cmp(y),
            (Value::BigInt(x), Value::BigInt(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        });
        // `total_cmp` place les NaN négatifs en tête : tous les NaN vont à la fin.
        let (numbers, nans): (Vec<Value>, Vec<Value>) = values
            .into_iter()
            .partition(|v| !matches!(v, Value::Number(n) if n.is_nan()));
        return Ok(numbers.into_iter().chain(nans).collect());
    }
    // Tri par insertion binaire stable : le comparateur du script peut être
    // incohérent, ce que ce tri supporte sans paniquer.
    let mut sorted: Vec<Value> = Vec::with_capacity(values.len());
    for value in values {
        let (mut low, mut high) = (0, sorted.len());
        while low < high {
            vm.tick(1)?;
            let middle = low + (high - low) / 2;
            let pivot = sorted.get(middle).cloned().unwrap_or(Value::Undefined);
            let order = vm.call(comparator, &Value::Undefined, &[pivot, value.clone()])?;
            if vm.to_number(&order)? > 0.0 {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        sorted.insert(low, value);
    }
    Ok(sorted)
}

fn comparator_arg(vm: &mut Vm, args: &[Value]) -> JsResult<Value> {
    let comparator = arg(args, 0);
    if comparator != Value::Undefined && !is_callable(vm, &comparator) {
        return Err(vm.type_error("le comparateur de tri doit être une fonction"));
    }
    Ok(comparator)
}

fn typed_sort(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let comparator = comparator_arg(vm, args)?;
    let (id, _) = validate_write(vm, this)?;
    let sorted = sort_elements(vm, elements(vm, id), &comparator)?;
    for (index, value) in sorted.into_iter().enumerate() {
        vm.typed_set(id, index, &value);
    }
    Ok(this.clone())
}

fn typed_to_sorted(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let comparator = comparator_arg(vm, args)?;
    let (id, len) = validate(vm, this)?;
    let sorted = sort_elements(vm, elements(vm, id), &comparator)?;
    let out = vm.new_typed(kind_of(vm, id), len)?;
    for (index, value) in sorted.into_iter().enumerate() {
        vm.typed_set(out, index, &value);
    }
    Ok(Value::Object(out))
}

fn typed_to_reversed(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let out = vm.new_typed(kind_of(vm, id), len)?;
    for (index, value) in elements(vm, id).into_iter().rev().enumerate() {
        vm.typed_set(out, index, &value);
    }
    Ok(Value::Object(out))
}

fn typed_with(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let (id, len) = validate(vm, this)?;
    let relative = vm.to_number(&arg(args, 0))?;
    let relative = if relative.is_nan() {
        0.0
    } else {
        relative.trunc()
    };
    let value = convert(vm, kind_of(vm, id), &arg(args, 1))?;
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    let index = if relative < 0.0 {
        len as f64 + relative
    } else {
        relative
    };
    // La conversion de la valeur a pu rétrécir le tableau.
    let current = vm.typed_length(id).unwrap_or(0);
    #[expect(clippy::cast_precision_loss, reason = "longueur petite devant 2^53")]
    if index < 0.0 || index >= current as f64 {
        return Err(vm_range(vm, "index hors du tableau typé"));
    }
    let out = vm.new_typed(kind_of(vm, id), len)?;
    for (i, element) in elements(vm, id).into_iter().enumerate() {
        vm.typed_set(out, i, &element);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "index vérifié dans [0, len)"
    )]
    vm.typed_set(out, index as usize, &value);
    Ok(Value::Object(out))
}

// ---- DataView --------------------------------------------------------------------

fn data_view_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        return Err(vm.type_error("DataView doit être appelé avec « new »"));
    }
    let Some(buffer) = arg(args, 0)
        .as_object()
        .filter(|id| vm.buffer(*id).is_some())
    else {
        return Err(vm.type_error("DataView : un ArrayBuffer est attendu"));
    };
    let offset = to_index(vm, &arg(args, 1))?;
    live_buffer(vm, buffer)?;
    let (buffer_len, resizable) = vm
        .buffer(buffer)
        .map_or((0, false), |b| (b.len(), b.max_len.is_some()));
    if offset > buffer_len {
        return Err(vm_range(vm, "décalage au-delà de la fin du tampon"));
    }
    let length = match arg(args, 2) {
        Value::Undefined if resizable => None,
        Value::Undefined => Some(buffer_len - offset),
        other => {
            let length = to_index(vm, &other)?;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > buffer_len)
            {
                return Err(vm_range(vm, "la vue dépasse la fin du tampon"));
            }
            Some(length)
        }
    };
    // Les conversions ont pu détacher ou rétrécir le tampon.
    live_buffer(vm, buffer)?;
    let current = vm.buffer(buffer).map_or(0, BufferData::len);
    if offset > current || length.is_some_and(|l| offset + l > current) {
        return Err(vm_range(vm, "la vue dépasse la fin du tampon"));
    }
    Ok(Value::Object(vm.heap.alloc(Object::new(
        Some(vm.intrinsics.data_view_proto),
        Kind::DataView {
            buffer,
            offset,
            length,
        },
    ))))
}

/// Tampon, décalage et longueur courante de la vue `this` ; `None` pour la
/// longueur si la vue est hors des bornes de son tampon.
fn data_view(vm: &mut Vm, this: &Value) -> JsResult<(ObjId, usize, Option<usize>)> {
    let found = this.as_object().and_then(|id| match &vm.heap.get(id).kind {
        Kind::DataView {
            buffer,
            offset,
            length,
        } => Some((*buffer, *offset, *length)),
        _ => None,
    });
    let Some((buffer, offset, length)) = found else {
        return Err(vm.type_error("le receveur n'est pas un DataView"));
    };
    let current = vm
        .buffer(buffer)
        .filter(|b| !b.detached)
        .and_then(|b| b.len().checked_sub(offset))
        .and_then(|available| match length {
            Some(length) => (length <= available).then_some(length),
            None => Some(available),
        });
    Ok((buffer, offset, current))
}

fn data_view_buffer(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    data_view(vm, this).map(|(buffer, _, _)| Value::Object(buffer))
}

fn data_view_byte_length(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    match data_view(vm, this)? {
        (_, _, Some(length)) => Ok(number(length)),
        _ => Err(vm.type_error("le DataView est détaché ou hors des bornes de son tampon")),
    }
}

fn data_view_byte_offset(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    match data_view(vm, this)? {
        (_, offset, Some(_)) => Ok(number(offset)),
        _ => Err(vm.type_error("le DataView est détaché ou hors des bornes de son tampon")),
    }
}

fn view_get(vm: &mut Vm, this: &Value, args: &[Value], kind: ElementKind) -> JsResult {
    data_view(vm, this)?;
    let index = to_index(vm, &arg(args, 0))?;
    let little_endian = arg(args, 1).truthy();
    let (buffer, offset, Some(length)) = data_view(vm, this)? else {
        return Err(vm.type_error("le DataView est détaché ou hors des bornes de son tampon"));
    };
    let size = kind.size();
    if index.checked_add(size).is_none_or(|end| end > length) {
        return Err(vm_range(vm, "lecture au-delà de la fin de la vue"));
    }
    let start = offset + index;
    let mut bytes: Vec<u8> = vm
        .buffer(buffer)
        .map(|b| b.copy_range(start, size))
        .unwrap_or_default();
    if !little_endian {
        bytes.reverse();
    }
    Ok(decode_value(kind, &bytes))
}

fn view_set(vm: &mut Vm, this: &Value, args: &[Value], kind: ElementKind) -> JsResult {
    let (viewed, ..) = data_view(vm, this)?;
    mutable_buffer(vm, viewed)?;
    let index = to_index(vm, &arg(args, 0))?;
    let value = convert(vm, kind, &arg(args, 1))?;
    let little_endian = arg(args, 2).truthy();
    let (buffer, offset, Some(length)) = data_view(vm, this)? else {
        return Err(vm.type_error("le DataView est détaché ou hors des bornes de son tampon"));
    };
    let (mut bytes, size) = encode_value(kind, &value);
    if index.checked_add(size).is_none_or(|end| end > length) {
        return Err(vm_range(vm, "écriture au-delà de la fin de la vue"));
    }
    if !little_endian {
        bytes[..size].reverse();
    }
    let start = offset + index;
    if let Some(target) = vm.buffer_mut(buffer) {
        target.write(start, &bytes[..size]);
    }
    Ok(Value::Undefined)
}

macro_rules! view_accessors {
    ($($get:ident, $set:ident => $kind:ident),* $(,)?) => {
        $(
            fn $get(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
                view_get(vm, this, args, ElementKind::$kind)
            }
            fn $set(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
                view_set(vm, this, args, ElementKind::$kind)
            }
        )*
    };
}
view_accessors! {
    get_int8, set_int8 => Int8, get_uint8, set_uint8 => Uint8,
    get_int16, set_int16 => Int16, get_uint16, set_uint16 => Uint16,
    get_int32, set_int32 => Int32, get_uint32, set_uint32 => Uint32,
    get_float16, set_float16 => Float16, get_float32, set_float32 => Float32,
    get_float64, set_float64 => Float64,
    get_bigint64, set_bigint64 => BigInt64, get_biguint64, set_biguint64 => BigUint64,
}

// ---- Installation ----------------------------------------------------------------

/// Accesseur natif, nommé `name`, dont la clé est un symbole.
fn symbol_getter(
    vm: &mut Vm,
    target: ObjId,
    symbol: &crate::value::Symbol,
    name: &str,
    func: NativeFn,
) {
    let get = vm.new_function(name, 0, func);
    vm.heap.get_mut(target).insert_prop(
        PropertyKey::Symbol(symbol.clone()),
        crate::heap::Property {
            slot: crate::heap::Slot::Accessor {
                get: Some(get),
                set: None,
            },
            writable: true,
            enumerable: false,
            configurable: true,
        },
    );
}

pub(super) fn install(vm: &mut Vm) {
    let known = vm.well_known.clone();

    // ArrayBuffer.
    let buffer_proto = vm.intrinsics.array_buffer_proto;
    let buffer_ctor = constructor(vm, "ArrayBuffer", 1, array_buffer_ctor, buffer_proto);
    vm.define_method(buffer_ctor, "isView", 1, array_buffer_is_view);
    let buffer_getters: &[(&str, NativeFn)] = &[
        ("byteLength", buffer_byte_length),
        ("maxByteLength", buffer_max_byte_length),
        ("resizable", buffer_resizable),
        ("detached", buffer_detached),
        ("immutable", buffer_immutable),
    ];
    for (name, func) in buffer_getters {
        getter(vm, buffer_proto, name, *func);
    }
    let buffer_methods: &[(&str, u32, NativeFn)] = &[
        ("slice", 2, buffer_slice),
        ("resize", 1, buffer_resize),
        ("transfer", 0, buffer_transfer),
        ("transferToFixedLength", 0, buffer_transfer_fixed),
        ("sliceToImmutable", 2, buffer_slice_immutable),
        ("transferToImmutable", 0, buffer_transfer_immutable),
    ];
    for (name, arity, func) in buffer_methods {
        vm.define_method(buffer_proto, name, *arity, *func);
    }
    vm.define_hidden_symbol(
        buffer_proto,
        &known.to_string_tag,
        Value::string("ArrayBuffer"),
    );

    // %TypedArray% : constructeur abstrait commun à tous les types.
    let typed_proto = vm.new_object();
    let typed_ctor = vm.new_native("TypedArray", 0, typed_array_abstract, true);
    vm.define_hidden(typed_ctor, "prototype", Value::Object(typed_proto));
    vm.define_hidden(typed_proto, "constructor", Value::Object(typed_ctor));
    vm.define_method(typed_ctor, "from", 1, typed_from);
    vm.define_method(typed_ctor, "of", 0, typed_of);
    let typed_getters: &[(&str, NativeFn)] = &[
        ("buffer", typed_buffer),
        ("byteLength", typed_byte_length),
        ("byteOffset", typed_byte_offset),
        ("length", typed_length_getter),
    ];
    for (name, func) in typed_getters {
        getter(vm, typed_proto, name, *func);
    }
    let typed_methods: &[(&str, u32, NativeFn)] = &[
        ("at", 1, typed_at),
        ("copyWithin", 2, typed_copy_within),
        ("entries", 0, typed_entries),
        ("every", 1, typed_every),
        ("fill", 1, typed_fill),
        ("filter", 1, typed_filter),
        ("find", 1, typed_find),
        ("findIndex", 1, typed_find_index),
        ("findLast", 1, typed_find_last),
        ("findLastIndex", 1, typed_find_last_index),
        ("forEach", 1, typed_for_each),
        ("includes", 1, typed_includes),
        ("indexOf", 1, typed_index_of),
        ("join", 1, typed_join),
        ("keys", 0, typed_keys),
        ("lastIndexOf", 1, typed_last_index_of),
        ("map", 1, typed_map),
        ("reduce", 1, typed_reduce),
        ("reduceRight", 1, typed_reduce_right),
        ("reverse", 0, typed_reverse),
        ("set", 1, typed_set_method),
        ("slice", 2, typed_slice),
        ("some", 1, typed_some),
        ("sort", 1, typed_sort),
        ("subarray", 2, typed_subarray),
        ("toLocaleString", 0, typed_to_locale_string),
        ("toReversed", 0, typed_to_reversed),
        ("toSorted", 1, typed_to_sorted),
        ("values", 0, typed_values),
        ("with", 2, typed_with),
    ];
    for (name, arity, func) in typed_methods {
        vm.define_method(typed_proto, name, *arity, *func);
    }
    // `toString` est la fonction même de `Array.prototype`.
    let array_proto = Value::Object(vm.intrinsics.array_proto);
    if let Ok(to_string) = vm.get(&array_proto, "toString") {
        vm.define_hidden(typed_proto, "toString", to_string);
    }
    if let Ok(values) = vm.get(&Value::Object(typed_proto), "values") {
        vm.define_hidden_symbol(typed_proto, &known.iterator, values);
    }
    symbol_getter(
        vm,
        typed_proto,
        &known.to_string_tag,
        "get [Symbol.toStringTag]",
        typed_tag,
    );
    symbol_getter(
        vm,
        typed_ctor,
        &known.species,
        "get [Symbol.species]",
        typed_species,
    );

    // Un constructeur par type d'éléments.
    let mut protos = Vec::with_capacity(2 * KINDS.len() + 1);
    let mut ctors = Vec::with_capacity(KINDS.len());
    for ((kind, name), func) in KINDS.iter().zip(CONSTRUCTORS) {
        let proto = vm.new_object();
        vm.heap.get_mut(proto).proto = Some(typed_proto);
        let ctor = constructor(vm, name, 3, func, proto);
        vm.heap.get_mut(ctor).proto = Some(typed_ctor);
        let size = number(kind.size());
        for target in [ctor, proto] {
            vm.heap.get_mut(target).props.insert(
                Arc::from("BYTES_PER_ELEMENT"),
                crate::heap::Property {
                    slot: crate::heap::Slot::Data(size.clone()),
                    writable: false,
                    enumerable: false,
                    configurable: false,
                },
            );
        }
        if *kind == ElementKind::Uint8 {
            super::base64::install(vm, ctor, proto);
        }
        protos.push(proto);
        ctors.push(ctor);
    }
    // SharedArrayBuffer : son prototype suit ceux des tableaux typés.
    let shared_proto = vm.new_object();
    let shared_ctor = constructor(vm, "SharedArrayBuffer", 1, shared_buffer_ctor, shared_proto);
    let shared_getters: &[(&str, NativeFn)] = &[
        ("byteLength", shared_byte_length),
        ("growable", shared_growable),
        ("maxByteLength", shared_max_byte_length),
    ];
    for (name, func) in shared_getters {
        getter(vm, shared_proto, name, *func);
    }
    vm.define_method(shared_proto, "grow", 1, shared_grow);
    vm.define_method(shared_proto, "slice", 2, shared_slice);
    vm.define_hidden_symbol(
        shared_proto,
        &known.to_string_tag,
        Value::string("SharedArrayBuffer"),
    );
    let _ = shared_ctor;
    protos.push(shared_proto);
    // Puis les constructeurs d'origine, pour l'espèce par défaut.
    protos.extend(ctors);
    vm.typed_protos = protos;

    // DataView.
    let view_proto = vm.intrinsics.data_view_proto;
    constructor(vm, "DataView", 1, data_view_ctor, view_proto);
    let view_getters: &[(&str, NativeFn)] = &[
        ("buffer", data_view_buffer),
        ("byteLength", data_view_byte_length),
        ("byteOffset", data_view_byte_offset),
    ];
    for (name, func) in view_getters {
        getter(vm, view_proto, name, *func);
    }
    let view_methods: &[(&str, u32, NativeFn)] = &[
        ("getInt8", 1, get_int8),
        ("setInt8", 2, set_int8),
        ("getUint8", 1, get_uint8),
        ("setUint8", 2, set_uint8),
        ("getInt16", 1, get_int16),
        ("setInt16", 2, set_int16),
        ("getUint16", 1, get_uint16),
        ("setUint16", 2, set_uint16),
        ("getInt32", 1, get_int32),
        ("setInt32", 2, set_int32),
        ("getUint32", 1, get_uint32),
        ("setUint32", 2, set_uint32),
        ("getFloat16", 1, get_float16),
        ("setFloat16", 2, set_float16),
        ("getFloat32", 1, get_float32),
        ("setFloat32", 2, set_float32),
        ("getFloat64", 1, get_float64),
        ("setFloat64", 2, set_float64),
        ("getBigInt64", 1, get_bigint64),
        ("setBigInt64", 2, set_bigint64),
        ("getBigUint64", 1, get_biguint64),
        ("setBigUint64", 2, set_biguint64),
    ];
    for (name, arity, func) in view_methods {
        vm.define_method(view_proto, name, *arity, *func);
    }
    vm.define_hidden_symbol(view_proto, &known.to_string_tag, Value::string("DataView"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_round_trip() {
        for (value, bits) in [
            (0.0, 0x0000),
            (-0.0, 0x8000),
            (1.0, 0x3c00),
            (-2.0, 0xc000),
            (65504.0, 0x7bff),
            (65519.99, 0x7bff),
            (65520.0, 0x7c00),
            (f64::INFINITY, 0x7c00),
            (f64::NEG_INFINITY, 0xfc00),
            // Plus petit dénormalisé et ses demis.
            (5.960_464_477_539_063e-8, 0x0001),
            (6.097_555_160_522_461e-5, 0x03ff),
            (6.103_515_625e-5, 0x0400),
            // Demi exact entre 1 et 1 + 2^-10 : vers le pair.
            (1.000_488_281_25, 0x3c00),
            (1.001_464_843_75, 0x3c02),
            (0.333_251_953_125, 0x3555),
        ] {
            assert_eq!(f64_to_f16_bits(value), bits, "{value}");
        }
        // Moitié exacte du plus petit dénormalisé : vers le pair (zéro) ;
        // juste au-dessus : le plus petit dénormalisé.
        let half = 2.0_f64.powi(-25);
        assert_eq!(f64_to_f16_bits(half), 0x0000);
        assert_eq!(f64_to_f16_bits(f64::from_bits(half.to_bits() + 1)), 0x0001);
        assert_eq!(f64_to_f16_bits(-half * 3.0), 0x8002);
        assert_eq!(f64_to_f16_bits(f64::NAN), 0x7e00);
        for bits in [
            0x0000_u16, 0x0001, 0x03ff, 0x0400, 0x3c00, 0x7bff, 0xfbff, 0x8001,
        ] {
            assert_eq!(f64_to_f16_bits(f16_bits_to_f64(bits)), bits, "{bits:#06x}");
        }
        assert!(f16_bits_to_f64(0x7e00).is_nan());
        assert_eq!(f16_bits_to_f64(0x7c00), f64::INFINITY);
    }

    #[test]
    fn element_conversions() {
        assert_eq!(clamp_u8(-5.0), 0);
        assert_eq!(clamp_u8(300.0), 255);
        assert_eq!(clamp_u8(0.5), 0);
        assert_eq!(clamp_u8(1.5), 2);
        assert_eq!(clamp_u8(2.5), 2);
        assert_eq!(clamp_u8(f64::NAN), 0);
        let round = |kind, value| {
            let (bytes, size) = encode(kind, value);
            decode(kind, &bytes[..size])
        };
        assert_eq!(round(ElementKind::Int8, 200.0), -56.0);
        assert_eq!(round(ElementKind::Uint8, -1.0), 255.0);
        assert_eq!(round(ElementKind::Int16, 65535.0), -1.0);
        assert_eq!(round(ElementKind::Uint32, -1.0), 4_294_967_295.0);
        assert_eq!(round(ElementKind::Int32, 2_147_483_648.0), -2_147_483_648.0);
        assert_eq!(round(ElementKind::Float32, 0.1), f64::from(0.1_f32));
        assert_eq!(round(ElementKind::Float64, 0.1), 0.1);
        assert!(round(ElementKind::Float32, f64::NAN).is_nan());
        assert_eq!(round(ElementKind::Uint8, f64::INFINITY), 0.0);
    }
}
