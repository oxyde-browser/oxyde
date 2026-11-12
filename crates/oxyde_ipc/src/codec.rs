//! Codage binaire des messages.
//!
//! Format volontairement simple : entiers en petit-boutiste, chaînes et
//! listes précédées de leur longueur sur 32 bits, variantes d'énumération
//! désignées par un octet. Le décodage ne fait confiance à rien : chaque
//! longueur est comparée aux octets réellement présents avant toute
//! allocation, et une entrée malformée donne une erreur, jamais une panique.

use std::sync::Arc;

/// Erreur de décodage d'un message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Le message s'arrête avant la fin de la valeur.
    Truncated,
    /// Variante d'énumération inconnue.
    UnknownVariant(u8),
    /// Chaîne qui n'est pas de l'UTF-8.
    InvalidText,
    /// Valeur hors du domaine attendu (booléen autre que 0 ou 1, entier
    /// trop grand pour sa destination…).
    InvalidValue,
    /// Des octets restent après la valeur.
    TrailingBytes,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("message tronqué"),
            Self::UnknownVariant(tag) => write!(f, "variante inconnue ({tag})"),
            Self::InvalidText => f.write_str("texte qui n'est pas de l'UTF-8"),
            Self::InvalidValue => f.write_str("valeur hors du domaine attendu"),
            Self::TrailingBytes => f.write_str("octets en trop après le message"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Tampon d'écriture d'un message.
#[derive(Debug, Default)]
pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    /// Tampon vide.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Octets écrits.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Ajoute des octets bruts.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Ajoute une longueur. Une longueur qui ne tient pas sur 32 bits est
    /// écrite saturée : le décodage la refusera, faute d'octets.
    pub fn length(&mut self, len: usize) {
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        self.raw(&len.to_le_bytes());
    }
}

/// Curseur de lecture sur un message reçu.
#[derive(Debug)]
pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Lecteur placé au début de `bytes`.
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Nombre d'octets restant à lire.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.bytes.len()
    }

    /// Lit `len` octets bruts.
    ///
    /// # Errors
    ///
    /// [`DecodeError::Truncated`] s'il en reste moins.
    pub fn raw(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let (head, tail) = self
            .bytes
            .split_at_checked(len)
            .ok_or(DecodeError::Truncated)?;
        self.bytes = tail;
        Ok(head)
    }

    /// Lit un tableau de `N` octets.
    fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        self.raw(N)?.try_into().map_err(|_| DecodeError::Truncated)
    }

    /// Lit une longueur et vérifie qu'il reste au moins un octet par
    /// élément annoncé : une longueur mensongère ne provoque aucune
    /// allocation démesurée.
    ///
    /// # Errors
    ///
    /// [`DecodeError::Truncated`] si le message est trop court pour elle.
    pub fn length(&mut self) -> Result<usize, DecodeError> {
        let len = u32::from_le_bytes(self.array()?);
        let len = usize::try_from(len).map_err(|_| DecodeError::InvalidValue)?;
        if len > self.remaining() {
            return Err(DecodeError::Truncated);
        }
        Ok(len)
    }
}

/// Valeur qui s'écrit dans un message.
pub trait Encode {
    /// Écrit la valeur dans `writer`.
    fn encode(&self, writer: &mut Writer);
}

/// Valeur qui se lit dans un message.
pub trait Decode: Sized {
    /// Lit la valeur dans `reader`.
    ///
    /// # Errors
    ///
    /// Une [`DecodeError`] si les octets ne forment pas une valeur valide.
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError>;
}

/// Code `value` en un message complet.
#[must_use]
pub fn to_bytes<T: Encode + ?Sized>(value: &T) -> Vec<u8> {
    let mut writer = Writer::new();
    value.encode(&mut writer);
    writer.into_bytes()
}

/// Décode un message complet.
///
/// # Errors
///
/// Une [`DecodeError`] si les octets ne forment pas exactement une valeur.
pub fn from_bytes<T: Decode>(bytes: &[u8]) -> Result<T, DecodeError> {
    let mut reader = Reader::new(bytes);
    let value = T::decode(&mut reader)?;
    if reader.remaining() != 0 {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(value)
}

macro_rules! numbers {
    ($($type:ty),*) => {$(
        impl Encode for $type {
            fn encode(&self, writer: &mut Writer) {
                writer.raw(&self.to_le_bytes());
            }
        }
        impl Decode for $type {
            fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
                Ok(<$type>::from_le_bytes(reader.array()?))
            }
        }
    )*};
}
numbers!(u8, u16, u32, u64, i32, i64, f32, f64);

impl Encode for usize {
    fn encode(&self, writer: &mut Writer) {
        // Toujours sur 64 bits : le format ne dépend pas de la plateforme.
        (*self as u64).encode(writer);
    }
}

impl Decode for usize {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        usize::try_from(u64::decode(reader)?).map_err(|_| DecodeError::InvalidValue)
    }
}

impl Encode for bool {
    fn encode(&self, writer: &mut Writer) {
        u8::from(*self).encode(writer);
    }
}

impl Decode for bool {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::decode(reader)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::InvalidValue),
        }
    }
}

impl Encode for str {
    fn encode(&self, writer: &mut Writer) {
        writer.length(self.len());
        writer.raw(self.as_bytes());
    }
}

impl Encode for String {
    fn encode(&self, writer: &mut Writer) {
        self.as_str().encode(writer);
    }
}

impl Decode for String {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let len = reader.length()?;
        let bytes = reader.raw(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| DecodeError::InvalidText)
    }
}

impl<T: Encode> Encode for [T] {
    fn encode(&self, writer: &mut Writer) {
        writer.length(self.len());
        for item in self {
            item.encode(writer);
        }
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, writer: &mut Writer) {
        self.as_slice().encode(writer);
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        // `length` a vérifié qu'il reste au moins `len` octets : la réserve
        // est bornée par la taille du message.
        let len = reader.length()?;
        let mut items = Vec::with_capacity(len);
        for _ in 0..len {
            items.push(T::decode(reader)?);
        }
        Ok(items)
    }
}

/// Octets bruts, écrits d'un bloc (une `Vec<u8>` ordinaire le serait octet
/// par octet).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bytes(pub Vec<u8>);

impl Encode for Bytes {
    fn encode(&self, writer: &mut Writer) {
        writer.length(self.0.len());
        writer.raw(&self.0);
    }
}

impl Decode for Bytes {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let len = reader.length()?;
        Ok(Self(reader.raw(len)?.to_vec()))
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, writer: &mut Writer) {
        match self {
            None => 0u8.encode(writer),
            Some(value) => {
                1u8.encode(writer);
                value.encode(writer);
            }
        }
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::decode(reader)? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(reader)?)),
            tag => Err(DecodeError::UnknownVariant(tag)),
        }
    }
}

impl<T: Encode, E: Encode> Encode for Result<T, E> {
    fn encode(&self, writer: &mut Writer) {
        match self {
            Ok(value) => {
                0u8.encode(writer);
                value.encode(writer);
            }
            Err(error) => {
                1u8.encode(writer);
                error.encode(writer);
            }
        }
    }
}

impl<T: Decode, E: Decode> Decode for Result<T, E> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match u8::decode(reader)? {
            0 => Ok(Ok(T::decode(reader)?)),
            1 => Ok(Err(E::decode(reader)?)),
            tag => Err(DecodeError::UnknownVariant(tag)),
        }
    }
}

impl<T: Encode + ?Sized> Encode for &T {
    fn encode(&self, writer: &mut Writer) {
        (**self).encode(writer);
    }
}

impl<T: Encode + ?Sized> Encode for Box<T> {
    fn encode(&self, writer: &mut Writer) {
        (**self).encode(writer);
    }
}

impl<T: Decode> Decode for Box<T> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        T::decode(reader).map(Box::new)
    }
}

impl<T: Encode + ?Sized> Encode for Arc<T> {
    fn encode(&self, writer: &mut Writer) {
        (**self).encode(writer);
    }
}

impl<T: Decode> Decode for Arc<T> {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        T::decode(reader).map(Arc::new)
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, writer: &mut Writer) {
        for item in self {
            item.encode(writer);
        }
    }
}

impl<T: Decode + Copy + Default, const N: usize> Decode for [T; N] {
    fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let mut items = [T::default(); N];
        for slot in &mut items {
            *slot = T::decode(reader)?;
        }
        Ok(items)
    }
}

macro_rules! tuples {
    ($(($($name:ident),+)),*) => {$(
        impl<$($name: Encode),+> Encode for ($($name,)+) {
            fn encode(&self, writer: &mut Writer) {
                #[allow(non_snake_case, reason = "variables nommées comme les paramètres de type")]
                let ($($name,)+) = self;
                $($name.encode(writer);)+
            }
        }
        impl<$($name: Decode),+> Decode for ($($name,)+) {
            fn decode(reader: &mut Reader<'_>) -> Result<Self, DecodeError> {
                Ok(($($name::decode(reader)?,)+))
            }
        }
    )*};
}
tuples!((A, B), (A, B, C), (A, B, C, D));

/// Implémente [`Encode`] et [`Decode`] pour une structure à champs nommés,
/// champ par champ dans l'ordre donné.
///
/// ```
/// use oxyde_ipc::{from_bytes, ipc_struct, to_bytes};
///
/// #[derive(Debug, PartialEq)]
/// struct Point { x: f32, y: f32 }
/// ipc_struct!(Point { x, y });
///
/// let point = Point { x: 1.0, y: -2.0 };
/// assert_eq!(from_bytes::<Point>(&to_bytes(&point)), Ok(point));
/// ```
#[macro_export]
macro_rules! ipc_struct {
    ($name:ident { $($field:ident),* $(,)? }) => {
        impl $crate::Encode for $name {
            fn encode(&self, writer: &mut $crate::Writer) {
                $($crate::Encode::encode(&self.$field, writer);)*
                let _ = writer;
            }
        }
        impl $crate::Decode for $name {
            fn decode(
                reader: &mut $crate::Reader<'_>,
            ) -> ::core::result::Result<Self, $crate::DecodeError> {
                let _ = &reader;
                ::core::result::Result::Ok(Self {
                    $($field: $crate::Decode::decode(reader)?,)*
                })
            }
        }
    };
}

/// Implémente [`Encode`] et [`Decode`] pour une énumération : chaque
/// variante reçoit un octet d'étiquette, suivi de ses champs. Les variantes
/// sont sans champ, à champs nommés ou à champs positionnels (nommés ici
/// pour la liaison).
///
/// ```
/// use oxyde_ipc::{from_bytes, ipc_enum, to_bytes};
///
/// #[derive(Debug, PartialEq)]
/// enum Shape { Empty, Circle { radius: f32 }, Label(String) }
/// ipc_enum!(Shape {
///     0 => Empty,
///     1 => Circle { radius },
///     2 => Label(text),
/// });
///
/// let shape = Shape::Label("rond".to_owned());
/// assert_eq!(from_bytes::<Shape>(&to_bytes(&shape)), Ok(shape));
/// assert!(from_bytes::<Shape>(&[9]).is_err());
/// ```
#[macro_export]
macro_rules! ipc_enum {
    ($name:ident { $($tag:literal => $variant:ident $({ $($field:ident),* $(,)? })? $(( $($item:ident),* $(,)? ))?),* $(,)? }) => {
        impl $crate::Encode for $name {
            fn encode(&self, writer: &mut $crate::Writer) {
                match self {
                    $(
                        Self::$variant $({ $($field),* })? $(( $($item),* ))? => {
                            $crate::Encode::encode(&($tag as u8), writer);
                            $($($crate::Encode::encode($field, writer);)*)?
                            $($($crate::Encode::encode($item, writer);)*)?
                        }
                    )*
                }
            }
        }
        impl $crate::Decode for $name {
            fn decode(
                reader: &mut $crate::Reader<'_>,
            ) -> ::core::result::Result<Self, $crate::DecodeError> {
                let tag: u8 = $crate::Decode::decode(reader)?;
                match tag {
                    $(
                        $tag => ::core::result::Result::Ok(Self::$variant
                            $({ $($field: $crate::Decode::decode(reader)?),* })?
                            $(( $({
                                let $item = $crate::Decode::decode(reader)?;
                                $item
                            }),* ))?
                        ),
                    )*
                    other => ::core::result::Result::Err($crate::DecodeError::UnknownVariant(other)),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Inner {
        id: u32,
        name: String,
    }
    ipc_struct!(Inner { id, name });

    #[derive(Debug, PartialEq)]
    enum Message {
        Ping,
        Text(String),
        Move { x: f32, y: f32 },
        Batch(Vec<Inner>, Option<u64>),
    }
    ipc_enum!(Message {
        0 => Ping,
        1 => Text(text),
        2 => Move { x, y },
        3 => Batch(items, count),
    });

    fn round_trip<T: Encode + Decode + PartialEq + std::fmt::Debug>(value: T) {
        let bytes = to_bytes(&value);
        assert_eq!(from_bytes::<T>(&bytes), Ok(value));
    }

    #[test]
    fn values_round_trip() {
        round_trip(0x1234_5678_9abc_def0_u64);
        round_trip(-7_i32);
        round_trip(f64::MIN_POSITIVE);
        round_trip(true);
        round_trip("héhé — ünïcödé".to_owned());
        round_trip(vec![Some(1u8), None, Some(3)]);
        round_trip((1u8, "a".to_owned(), [1.5f32, 2.5, 3.5]));
        round_trip(Bytes(vec![0, 255, 7]));
        round_trip(Ok::<u8, String>(3));
        round_trip(Err::<u8, String>("non".to_owned()));
        round_trip(Message::Ping);
        round_trip(Message::Text("bonjour".to_owned()));
        round_trip(Message::Move { x: 1.0, y: -2.5 });
        round_trip(Message::Batch(
            vec![Inner {
                id: 7,
                name: "sept".to_owned(),
            }],
            Some(9),
        ));
    }

    #[test]
    fn malformed_input_is_rejected_without_panicking() {
        // Tronqué à chaque longueur possible.
        let bytes = to_bytes(&Message::Batch(
            vec![Inner {
                id: 1,
                name: "un".to_owned(),
            }],
            None,
        ));
        for len in 0..bytes.len() {
            assert!(
                from_bytes::<Message>(&bytes[..len]).is_err(),
                "longueur {len}"
            );
        }
        // Longueur mensongère : quatre milliards d'éléments annoncés.
        let lie = [0xff, 0xff, 0xff, 0xff, 1, 2, 3];
        assert_eq!(from_bytes::<Vec<u64>>(&lie), Err(DecodeError::Truncated));
        assert_eq!(from_bytes::<String>(&lie), Err(DecodeError::Truncated));
        assert_eq!(from_bytes::<Bytes>(&lie), Err(DecodeError::Truncated));
        // Valeurs hors domaine.
        assert_eq!(from_bytes::<bool>(&[2]), Err(DecodeError::InvalidValue));
        assert_eq!(
            from_bytes::<Message>(&[42]),
            Err(DecodeError::UnknownVariant(42))
        );
        assert_eq!(
            from_bytes::<String>(&[2, 0, 0, 0, 0xff, 0xfe]),
            Err(DecodeError::InvalidText)
        );
        assert_eq!(from_bytes::<u8>(&[1, 2]), Err(DecodeError::TrailingBytes));
        // Du bruit ne fait jamais paniquer.
        let mut seed = 0x9e37_79b9_u32;
        for _ in 0..2000 {
            let noise: Vec<u8> = (0..(seed % 40))
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 24) as u8
                })
                .collect();
            let _ = from_bytes::<Message>(&noise);
            let _ = from_bytes::<Vec<Inner>>(&noise);
        }
    }
}
