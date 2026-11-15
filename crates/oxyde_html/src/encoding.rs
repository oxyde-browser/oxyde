//! Encodages de texte (Encoding Standard) : reconnaître l'encodage d'une
//! ressource et la décoder en chaîne.
//!
//! Pris en charge, en lecture : tous les encodages de la norme. UTF-8,
//! UTF-16 (les deux boutismes), les vingt-huit encodages à un octet
//! (`windows-1252`, `ISO-8859-*`, `KOI8-R`…), `x-user-defined`, les
//! encodages à plusieurs octets (`Shift_JIS`, `EUC-JP`, `ISO-2022-JP`,
//! `EUC-KR`, `Big5`, `GBK`, `gb18030`) et `replacement`.
//!
//! LIMITES CONNUES : aucun encodeur autre qu'UTF-8 (un formulaire ou une
//! URL d'une page en `Shift_JIS` partent en UTF-8) ; la recherche de
//! `<meta charset>` est une lecture simplifiée des 1024 premiers octets, pas
//! l'algorithme de pré-analyse de la norme.

use crate::encoding_legacy::{self, MultiByte};
use crate::encoding_tables::{LABELS, SINGLE_BYTE};

/// Caractère de remplacement, pour une suite d'octets invalide.
const REPLACEMENT: char = '\u{fffd}';

/// Nombre d'octets où chercher `<meta charset>`.
const PRESCAN: usize = 1024;

/// Façon de lire les octets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Utf8,
    Utf16 {
        big_endian: bool,
    },
    /// Octets 0x80 à 0xFF traduits par un index.
    SingleByte(&'static [u16; 128]),
    /// `x-user-defined` : 0x80 à 0xFF donnent U+F780 à U+F7FF.
    UserDefined,
    /// Encodage à plusieurs octets.
    MultiByte(MultiByte),
    /// `replacement` : tout contenu non vide se lit comme un seul U+FFFD
    /// (encodages anciens dont la lecture serait dangereuse).
    Replacement,
}

/// Encodage de texte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    name: &'static str,
    kind: Kind,
}

/// UTF-8, l'encodage par défaut du web.
pub const UTF_8: Encoding = Encoding {
    name: "UTF-8",
    kind: Kind::Utf8,
};

impl Encoding {
    /// Encodage que désigne `label` (`"latin1"`, `" UTF-8 "`…), à la casse
    /// et aux blancs près ; `None` si l'étiquette est inconnue.
    #[must_use]
    pub fn for_label(label: &str) -> Option<Self> {
        let label = label
            .trim_matches(|c: char| matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' '))
            .to_ascii_lowercase();
        let at = LABELS
            .binary_search_by(|(known, _)| (*known).cmp(label.as_str()))
            .ok()?;
        let name = LABELS.get(at)?.1;
        let kind = match name {
            "UTF-8" => Kind::Utf8,
            "UTF-16LE" => Kind::Utf16 { big_endian: false },
            "UTF-16BE" => Kind::Utf16 { big_endian: true },
            "x-user-defined" => Kind::UserDefined,
            "replacement" => Kind::Replacement,
            "Shift_JIS" => Kind::MultiByte(MultiByte::ShiftJis),
            "EUC-JP" => Kind::MultiByte(MultiByte::EucJp),
            "ISO-2022-JP" => Kind::MultiByte(MultiByte::Iso2022Jp),
            "EUC-KR" => Kind::MultiByte(MultiByte::EucKr),
            "Big5" => Kind::MultiByte(MultiByte::Big5),
            "GBK" | "gb18030" => Kind::MultiByte(MultiByte::Gb18030),
            // `ISO-8859-8-I` se lit comme `ISO-8859-8` (seul l'ordre
            // d'affichage diffère).
            "ISO-8859-8-I" => Kind::SingleByte(index_of("ISO-8859-8")?),
            other => Kind::SingleByte(index_of(other)?),
        };
        Some(Self { name, kind })
    }

    /// Nom de l'encodage (`"UTF-8"`, `"windows-1252"`…).
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Marque d'ordre d'octets propre à cet encodage, s'il en a une.
    #[must_use]
    pub fn bom(&self) -> &'static [u8] {
        match self.kind {
            Kind::Utf8 => b"\xEF\xBB\xBF",
            Kind::Utf16 { big_endian: false } => b"\xFF\xFE",
            Kind::Utf16 { big_endian: true } => b"\xFE\xFF",
            Kind::SingleByte(_) | Kind::UserDefined | Kind::MultiByte(_) | Kind::Replacement => b"",
        }
    }

    /// Décode `bytes` tels quels (une marque d'ordre d'octets devient
    /// U+FEFF). Renvoie le texte et `true` si une suite d'octets invalide a
    /// été remplacée par U+FFFD.
    #[must_use]
    pub fn decode_raw(&self, bytes: &[u8]) -> (String, bool) {
        match self.kind {
            Kind::Utf8 => {
                let text = String::from_utf8_lossy(bytes);
                let replaced = matches!(text, std::borrow::Cow::Owned(_));
                (text.into_owned(), replaced)
            }
            Kind::Utf16 { big_endian } => {
                // Une unité haute en fin de texte et un octet en trop ne
                // font ensemble qu'une erreur.
                let unfinished = utf16_pending(bytes, big_endian);
                let whole = bytes.len().saturating_sub(unfinished);
                let (pairs, _) = bytes.get(..whole).unwrap_or(bytes).as_chunks::<2>();
                let dangling = unfinished > 0;
                let units = pairs.iter().map(|pair| {
                    if big_endian {
                        u16::from_be_bytes(*pair)
                    } else {
                        u16::from_le_bytes(*pair)
                    }
                });
                let mut replaced = dangling;
                let mut text: String = char::decode_utf16(units)
                    .map(|unit| {
                        unit.unwrap_or_else(|_| {
                            replaced = true;
                            REPLACEMENT
                        })
                    })
                    .collect();
                if dangling {
                    text.push(REPLACEMENT);
                }
                (text, replaced)
            }
            Kind::SingleByte(index) => {
                let mut replaced = false;
                let text = bytes
                    .iter()
                    .map(|byte| match byte.checked_sub(0x80) {
                        None => char::from(*byte),
                        Some(high) => {
                            let code = index.get(usize::from(high)).copied().unwrap_or(0xfffd);
                            if code == 0xfffd {
                                replaced = true;
                            }
                            char::from_u32(u32::from(code)).unwrap_or(REPLACEMENT)
                        }
                    })
                    .collect();
                (text, replaced)
            }
            Kind::UserDefined => {
                let text = bytes
                    .iter()
                    .map(|byte| match byte.checked_sub(0x80) {
                        None => char::from(*byte),
                        Some(high) => {
                            char::from_u32(0xf780 + u32::from(high)).unwrap_or(REPLACEMENT)
                        }
                    })
                    .collect();
                (text, false)
            }
            Kind::MultiByte(kind) => {
                let decoded = encoding_legacy::decode(kind, bytes, true);
                (decoded.text, decoded.replaced)
            }
            Kind::Replacement => {
                if bytes.is_empty() {
                    (String::new(), false)
                } else {
                    (REPLACEMENT.to_string(), true)
                }
            }
        }
    }

    /// Décode le début d'un flux dont la suite arrivera : comme
    /// [`Encoding::decode_raw`], mais un caractère inachevé en fin de
    /// `bytes` n'est pas une erreur. Renvoie en plus le nombre d'octets de
    /// fin ainsi laissés de côté, à représenter avec la suite.
    #[must_use]
    pub fn decode_partial(&self, bytes: &[u8]) -> (String, bool, usize) {
        let pending = match self.kind {
            Kind::MultiByte(kind) => {
                let decoded = encoding_legacy::decode(kind, bytes, false);
                return (decoded.text, decoded.replaced, decoded.pending);
            }
            Kind::Utf8 => utf8_pending(bytes),
            Kind::Utf16 { big_endian } => utf16_pending(bytes, big_endian),
            Kind::SingleByte(_) | Kind::UserDefined | Kind::Replacement => 0,
        };
        let whole = bytes.len().saturating_sub(pending);
        let (text, replaced) = self.decode_raw(bytes.get(..whole).unwrap_or(bytes));
        (text, replaced, pending)
    }

    /// Vrai pour un encodage dont le décodeur garde un état d'un caractère
    /// au suivant (`ISO-2022-JP`) : un flux s'y décode depuis son début.
    #[must_use]
    pub fn is_stateful(&self) -> bool {
        self.kind == Kind::MultiByte(MultiByte::Iso2022Jp)
    }

    /// Vrai pour `replacement`, qui ne se choisit pas par script.
    #[must_use]
    pub fn is_replacement(&self) -> bool {
        self.kind == Kind::Replacement
    }

    /// Décode `bytes`, sa propre marque d'ordre d'octets retirée.
    #[must_use]
    pub fn decode(&self, bytes: &[u8]) -> String {
        let bom = self.bom();
        let bytes = if bom.is_empty() {
            bytes
        } else {
            bytes.strip_prefix(bom).unwrap_or(bytes)
        };
        self.decode_raw(bytes).0
    }
}

/// Nombre d'octets de fin de `bytes` qui commencent un caractère UTF-8
/// valable jusque-là mais inachevé.
fn utf8_pending(bytes: &[u8]) -> usize {
    let len = bytes.len();
    // Un caractère tient en quatre octets : seuls les trois derniers
    // peuvent en commencer un sans le finir.
    for start in len.saturating_sub(3)..len {
        match std::str::from_utf8(bytes.get(start..).unwrap_or_default()) {
            Ok(_) => return 0,
            Err(error) if error.error_len().is_none() => {
                return len.saturating_sub(start + error.valid_up_to());
            }
            // `start` tombe au milieu d'un caractère, ou sur un octet
            // invalide : la suite est examinée seule.
            Err(_) => {}
        }
    }
    0
}

/// Nombre d'octets de fin de `bytes` qui commencent un caractère UTF-16
/// inachevé : un octet seul, une unité haute sans sa moitié basse.
fn utf16_pending(bytes: &[u8], big_endian: bool) -> usize {
    let odd = bytes.len() % 2;
    let even = bytes.len() - odd;
    let last = even
        .checked_sub(2)
        .and_then(|at| bytes.get(at..even))
        .and_then(|pair| <[u8; 2]>::try_from(pair).ok())
        .map(|pair| {
            if big_endian {
                u16::from_be_bytes(pair)
            } else {
                u16::from_le_bytes(pair)
            }
        });
    match last {
        Some(0xd800..=0xdbff) => odd + 2,
        _ => odd,
    }
}

/// Index de l'encodage à un octet `name`.
fn index_of(name: &str) -> Option<&'static [u16; 128]> {
    SINGLE_BYTE
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, index)| index)
}

/// Encodage annoncé par une marque d'ordre d'octets en tête de `bytes`.
fn from_bom(bytes: &[u8]) -> Option<Encoding> {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        Some(UTF_8)
    } else if bytes.starts_with(b"\xFE\xFF") {
        Encoding::for_label("utf-16be")
    } else if bytes.starts_with(b"\xFF\xFE") {
        Encoding::for_label("utf-16le")
    } else {
        None
    }
}

/// `windows-1252`, l'encodage supposé d'une ressource ancienne.
fn windows_1252() -> Encoding {
    Encoding::for_label("windows-1252").unwrap_or(UTF_8)
}

/// Étiquette d'encodage d'une valeur `charset=…` : la suite de `text`
/// après `charset`, un `=` et d'éventuels guillemets.
fn charset_value(text: &[u8]) -> Option<String> {
    let is_space = |b: &u8| matches!(b, b'\t' | b'\n' | 0x0c | b'\r' | b' ');
    let mut rest = text;
    while let [first, tail @ ..] = rest {
        if !is_space(first) {
            break;
        }
        rest = tail;
    }
    let [b'=', tail @ ..] = rest else {
        return None;
    };
    rest = tail;
    while let [first, tail @ ..] = rest {
        if !is_space(first) {
            break;
        }
        rest = tail;
    }
    let (quote, rest) = match rest {
        [quote @ (b'"' | b'\''), tail @ ..] => (Some(*quote), tail),
        _ => (None, rest),
    };
    let end = rest
        .iter()
        .position(|b| match quote {
            Some(quote) => *b == quote,
            None => is_space(b) || matches!(b, b';' | b'>' | b'/' | b'"' | b'\''),
        })
        .unwrap_or(rest.len());
    let label = rest.get(..end)?;
    (!label.is_empty()).then(|| String::from_utf8_lossy(label).into_owned())
}

/// Position de `needle`, sans égard à la casse ASCII, dans `haystack`.
fn find_ascii(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

/// Encodage que déclare un `<meta charset=…>` ou un
/// `<meta http-equiv=Content-Type content="…; charset=…">` dans le début de
/// `bytes`.
fn from_meta(bytes: &[u8]) -> Option<Encoding> {
    let head = bytes.get(..PRESCAN.min(bytes.len()))?;
    let mut rest = head;
    while let Some(at) = find_ascii(rest, b"<meta") {
        let tag = rest.get(at + 5..)?;
        let end = tag.iter().position(|b| *b == b'>').unwrap_or(tag.len());
        let inside = tag.get(..end)?;
        if let Some(found) = find_ascii(inside, b"charset")
            && let Some(label) = inside.get(found + 7..).and_then(charset_value)
            && let Some(encoding) = Encoding::for_label(&label)
        {
            // Une page lue octet par octet jusqu'ici n'est pas en UTF-16 :
            // la norme lit alors de l'UTF-8.
            return Some(match encoding.kind {
                Kind::Utf16 { .. } => UTF_8,
                Kind::UserDefined => windows_1252(),
                _ => encoding,
            });
        }
        rest = tag.get(end..)?;
    }
    None
}

/// Encodage de la valeur d'un en-tête `Content-Type` (`text/html;
/// charset=iso-8859-1`), s'il en déclare un que ce module sait lire.
#[must_use]
pub fn from_content_type(content_type: &str) -> Option<Encoding> {
    let bytes = content_type.as_bytes();
    let at = find_ascii(bytes, b"charset")?;
    let label = charset_value(bytes.get(at + 7..)?)?;
    Encoding::for_label(&label)
}

/// Encodage d'un document HTML reçu sans encodage annoncé par le
/// transport : sa marque d'ordre d'octets, sinon son `<meta charset>`,
/// sinon UTF-8 s'il en est, sinon `windows-1252`.
#[must_use]
pub fn sniff_document(bytes: &[u8]) -> Encoding {
    from_bom(bytes)
        .or_else(|| from_meta(bytes))
        .unwrap_or_else(|| sniff_text(bytes))
}

/// Encodage d'une ressource texte sans déclaration (feuille de style,
/// script) : sa marque d'ordre d'octets, sinon UTF-8 si elle en est, sinon
/// `windows-1252`.
#[must_use]
pub fn sniff_text(bytes: &[u8]) -> Encoding {
    from_bom(bytes).unwrap_or_else(|| {
        if std::str::from_utf8(bytes).is_ok() {
            UTF_8
        } else {
            windows_1252()
        }
    })
}

/// Décode un document HTML : voir [`sniff_document`].
#[must_use]
pub fn decode_document(bytes: &[u8]) -> String {
    sniff_document(bytes).decode(bytes)
}

/// Décode une ressource texte : voir [`sniff_text`].
#[must_use]
pub fn decode_text(bytes: &[u8]) -> String {
    sniff_text(bytes).decode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_resolve_to_encodings() {
        let name = |label: &str| Encoding::for_label(label).map(|e| e.name());
        assert_eq!(name(" Latin1\n"), Some("windows-1252"));
        assert_eq!(name("ISO-8859-1"), Some("windows-1252"));
        assert_eq!(name("utf8"), Some("UTF-8"));
        assert_eq!(name("unicode"), Some("UTF-16LE"));
        assert_eq!(name("koi8-r"), Some("KOI8-R"));
        assert_eq!(name("shift_jis"), Some("Shift_JIS"));
        assert_eq!(name("gb2312"), Some("GBK"));
        assert_eq!(name("hz-gb-2312"), Some("replacement"));
        assert_eq!(name("inconnu"), None);
        // La table est triée : la recherche dichotomique y trouve tout.
        assert!(LABELS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(
            LABELS
                .iter()
                .all(|(label, _)| Encoding::for_label(label).is_some())
        );
    }

    #[test]
    fn single_byte_and_utf16_decode() {
        let latin = Encoding::for_label("windows-1252").expect("windows-1252");
        assert_eq!(latin.decode(b"caf\xE9 \x80"), "café €");
        let cyrillic = Encoding::for_label("windows-1251").expect("windows-1251");
        assert_eq!(cyrillic.decode(b"\xCF\xF0\xE8"), "При");
        let le = Encoding::for_label("utf-16le").expect("utf-16le");
        assert_eq!(le.decode(b"\xFF\xFEh\x00\xE9\x00"), "hé");
        // Unité haute en fin de texte et octet en trop : une seule erreur.
        assert_eq!(le.decode_raw(b"\x00\xD8a"), ("\u{fffd}".to_owned(), true));
        assert_eq!(
            le.decode_raw(b"\x00\xD8a\x00"),
            ("\u{fffd}a".to_owned(), true)
        );
        assert_eq!(le.decode_raw(b"a\x00b"), ("a\u{fffd}".to_owned(), true));
        // Dans un flux, ces octets attendent la suite.
        assert_eq!(
            le.decode_partial(b"a\x00\x3D\xD8\x00"),
            ("a".to_owned(), false, 3)
        );
        assert_eq!(
            UTF_8.decode_partial(b"a\xF0\x9F\x98"),
            ("a".to_owned(), false, 3)
        );
        assert_eq!(
            UTF_8.decode_partial(b"\xC3\xA9\xE2"),
            ("é".to_owned(), false, 1)
        );
        assert_eq!(
            UTF_8.decode_partial(b"\xC3\xA9"),
            ("é".to_owned(), false, 0)
        );
        // Suite invalide, pas inachevée : remplacée tout de suite.
        assert_eq!(
            UTF_8.decode_partial(b"a\xFF\x80"),
            ("a\u{fffd}\u{fffd}".to_owned(), true, 0)
        );
        assert_eq!(
            UTF_8.decode_partial(b"\xE0\x80"),
            ("\u{fffd}\u{fffd}".to_owned(), true, 0)
        );
        let greek = Encoding::for_label("windows-1253").expect("windows-1253");
        assert!(greek.decode_raw(b"\xAA").1, "octet sans caractère");
        let user = Encoding::for_label("x-user-defined").expect("x-user-defined");
        assert_eq!(user.decode(b"a\x80"), "a\u{f780}");
    }

    #[test]
    fn documents_are_sniffed() {
        let name = |bytes: &[u8]| sniff_document(bytes).name();
        assert_eq!(name(b"\xEF\xBB\xBF<p>"), "UTF-8");
        assert_eq!(name(b"\xFF\xFE<\x00"), "UTF-16LE");
        assert_eq!(name(b"<meta charset=iso-8859-15><p>\xA4"), "ISO-8859-15");
        assert_eq!(
            name(b"<META http-equiv='Content-Type' content=\"text/html; charset=KOI8-R\">"),
            "KOI8-R"
        );
        assert_eq!(name(b"<meta charset='utf-16'>"), "UTF-8");
        assert_eq!(name("<p>é".as_bytes()), "UTF-8");
        assert_eq!(name(b"<p>\xE9"), "windows-1252");
        assert_eq!(
            decode_document(b"<meta charset=latin1>\xE9t\xE9"),
            "<meta charset=latin1>été"
        );
        assert_eq!(decode_text(b"caf\xE9"), "café");
        assert_eq!(
            from_content_type("text/html; Charset=\"windows-1250\"").map(|e| e.name()),
            Some("windows-1250")
        );
        assert_eq!(from_content_type("text/html"), None);
        // Encodages à plusieurs octets et `replacement`.
        assert_eq!(
            decode_document(b"<meta charset=shift_jis>\x93\xFA\x96\x7B"),
            "<meta charset=shift_jis>日本"
        );
        assert_eq!(
            decode_document(b"<meta charset=gbk>\xD6\xD0"),
            "<meta charset=gbk>中"
        );
        assert_eq!(
            decode_document(b"<meta charset=iso-2022-kr>abc"),
            "\u{fffd}"
        );
        let kr = Encoding::for_label("euc-kr").expect("euc-kr");
        assert_eq!(kr.decode_partial(b"a\xC7"), ("a".to_owned(), false, 1));
        assert_eq!(kr.decode_raw(b"a\xC7"), ("a\u{fffd}".to_owned(), true));
    }
}
