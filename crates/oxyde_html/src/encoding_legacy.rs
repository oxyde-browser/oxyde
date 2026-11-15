//! Décodeurs des encodages à plusieurs octets de l'Encoding Standard :
//! `Shift_JIS`, `EUC-JP`, `ISO-2022-JP`, `EUC-KR`, `Big5`, `GBK` et `gb18030`
//! (ces deux derniers se décodent de la même façon).
//!
//! Chaque décodeur suit l'algorithme de la norme. Un octet « remis dans la
//! file » y est toujours l'un des derniers octets lus : remettre revient à
//! reculer la position de lecture.

use crate::encoding_multibyte::{BIG5, EUC_KR, GB18030, GB18030_RANGES, JIS0208, JIS0212};

/// Encodage à plusieurs octets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MultiByte {
    ShiftJis,
    EucJp,
    Iso2022Jp,
    EucKr,
    Big5,
    /// `GBK` et `gb18030`.
    Gb18030,
}

/// Résultat d'un décodage.
pub(crate) struct Decoded {
    /// Texte décodé.
    pub text: String,
    /// Une suite d'octets invalide a été remplacée par U+FFFD.
    pub replaced: bool,
    /// Nombre d'octets de fin laissés de côté parce qu'ils commencent un
    /// caractère inachevé (toujours 0 pour un décodage complet).
    pub pending: usize,
}

/// Texte en cours de décodage.
struct Output {
    text: String,
    replaced: bool,
}

impl Output {
    /// Ajoute le point de code `code`.
    fn push(&mut self, code: u32) {
        match char::from_u32(code) {
            Some(c) => self.text.push(c),
            None => self.error(),
        }
    }

    /// Ajoute `code`, ou signale une erreur s'il n'y en a pas.
    fn push_or_error(&mut self, code: Option<u32>) {
        match code {
            Some(code) => self.push(code),
            None => self.error(),
        }
    }

    /// Remplace une suite invalide par U+FFFD.
    fn error(&mut self) {
        self.text.push('\u{fffd}');
        self.replaced = true;
    }
}

/// Point de code de rang `pointer` dans un index à valeurs de 16 bits.
fn lookup(index: &[u16], pointer: usize) -> Option<u32> {
    index
        .get(pointer)
        .copied()
        .filter(|code| *code != 0)
        .map(u32::from)
}

/// Décode `bytes`. Si `last` est faux, d'autres octets suivront : un
/// caractère inachevé en fin de `bytes` n'est pas une erreur, et ses octets
/// sont comptés dans [`Decoded::pending`].
pub(crate) fn decode(kind: MultiByte, bytes: &[u8], last: bool) -> Decoded {
    let mut out = Output {
        // Au plus un caractère par octet, souvent un pour deux.
        text: String::with_capacity(bytes.len()),
        replaced: false,
    };
    let pending = match kind {
        MultiByte::ShiftJis => shift_jis(bytes, &mut out),
        MultiByte::EucJp => euc_jp(bytes, &mut out),
        MultiByte::Iso2022Jp => iso_2022_jp(bytes, last, &mut out),
        MultiByte::EucKr => two_bytes(bytes, &mut out, euc_kr_pair),
        MultiByte::Big5 => two_bytes(bytes, &mut out, big5_pair),
        MultiByte::Gb18030 => gb18030(bytes, &mut out),
    };
    // Fin de la file au milieu d'un caractère : une erreur, sauf si la suite
    // arrive plus tard.
    let pending = if last {
        if pending > 0 && kind != MultiByte::Iso2022Jp {
            out.error();
        }
        0
    } else {
        pending
    };
    Decoded {
        text: out.text,
        replaced: out.replaced,
        pending,
    }
}

/// `Shift_JIS`. Renvoie le nombre d'octets en attente à la fin.
fn shift_jis(bytes: &[u8], out: &mut Output) -> usize {
    let mut lead = 0u8;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        if lead != 0 {
            let first = usize::from(lead);
            lead = 0;
            let offset = if byte < 0x7f { 0x40 } else { 0x41 };
            let lead_offset = if first < 0xa0 { 0x81 } else { 0xc1 };
            let code = if matches!(byte, 0x40..=0x7e | 0x80..=0xfc) {
                let pointer = (first - lead_offset) * 188 + usize::from(byte) - offset;
                if (8836..=10715).contains(&pointer) {
                    // Caractères définis par l'utilisateur.
                    u32::try_from(0xe000 - 8836 + pointer).ok()
                } else {
                    lookup(&JIS0208, pointer)
                }
            } else {
                None
            };
            if code.is_none() && byte.is_ascii() {
                i -= 1;
            }
            out.push_or_error(code);
        } else {
            match byte {
                0x00..=0x80 => out.push(u32::from(byte)),
                0xa1..=0xdf => out.push(0xff61 - 0xa1 + u32::from(byte)),
                0x81..=0x9f | 0xe0..=0xfc => lead = byte,
                _ => out.error(),
            }
        }
    }
    usize::from(lead != 0)
}

/// `EUC-JP`.
fn euc_jp(bytes: &[u8], out: &mut Output) -> usize {
    let mut lead = 0u8;
    let mut jis0212 = false;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        if lead == 0x8e && matches!(byte, 0xa1..=0xdf) {
            lead = 0;
            out.push(0xff61 - 0xa1 + u32::from(byte));
        } else if lead == 0x8f && matches!(byte, 0xa1..=0xfe) {
            jis0212 = true;
            lead = byte;
        } else if lead != 0 {
            let first = lead;
            lead = 0;
            let code = if matches!(first, 0xa1..=0xfe) && matches!(byte, 0xa1..=0xfe) {
                let pointer = usize::from(first - 0xa1) * 94 + usize::from(byte - 0xa1);
                if jis0212 {
                    lookup(&JIS0212, pointer)
                } else {
                    lookup(&JIS0208, pointer)
                }
            } else {
                None
            };
            jis0212 = false;
            if code.is_none() && byte.is_ascii() {
                i -= 1;
            }
            out.push_or_error(code);
        } else {
            match byte {
                0x00..=0x7f => out.push(u32::from(byte)),
                0x8e | 0x8f | 0xa1..=0xfe => lead = byte,
                _ => out.error(),
            }
        }
    }
    match (lead != 0, jis0212) {
        (false, _) => 0,
        (true, false) => 1,
        (true, true) => 2,
    }
}

/// État du décodeur `ISO-2022-JP`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Jp {
    Ascii,
    Roman,
    Katakana,
    Lead,
    Trail,
    EscapeStart,
    Escape,
}

/// `ISO-2022-JP`. Les erreurs de fin de file sont signalées ici (quand
/// `last`), pas par l'appelant : la norme y reprend la lecture.
fn iso_2022_jp(bytes: &[u8], last: bool, out: &mut Output) -> usize {
    let mut state = Jp::Ascii;
    let mut output_state = Jp::Ascii;
    let mut lead = 0u8;
    let mut output = false;
    let mut i = 0;
    loop {
        let byte = bytes.get(i).copied();
        if byte.is_some() {
            i += 1;
        }
        match state {
            Jp::Ascii | Jp::Roman | Jp::Katakana | Jp::Lead => {
                let Some(byte) = byte else {
                    return 0;
                };
                if byte == 0x1b {
                    state = Jp::EscapeStart;
                    continue;
                }
                output = false;
                match (state, byte) {
                    (Jp::Roman, 0x5c) => out.push(0xa5),
                    (Jp::Roman, 0x7e) => out.push(0x203e),
                    (Jp::Ascii | Jp::Roman, 0x00..=0x7f) if !matches!(byte, 0x0e | 0x0f) => {
                        out.push(u32::from(byte));
                    }
                    (Jp::Katakana, 0x21..=0x5f) => out.push(0xff61 - 0x21 + u32::from(byte)),
                    (Jp::Lead, 0x21..=0x7e) => {
                        lead = byte;
                        state = Jp::Trail;
                    }
                    _ => out.error(),
                }
            }
            Jp::Trail => match byte {
                Some(0x1b) => {
                    state = Jp::EscapeStart;
                    out.error();
                }
                Some(byte @ 0x21..=0x7e) => {
                    state = Jp::Lead;
                    let pointer =
                        usize::from(lead.saturating_sub(0x21)) * 94 + usize::from(byte - 0x21);
                    out.push_or_error(lookup(&JIS0208, pointer));
                }
                Some(_) => {
                    state = Jp::Lead;
                    out.error();
                }
                None => {
                    if !last {
                        return 1;
                    }
                    state = Jp::Lead;
                    out.error();
                }
            },
            Jp::EscapeStart => match byte {
                Some(byte @ (0x24 | 0x28)) => {
                    lead = byte;
                    state = Jp::Escape;
                }
                other => {
                    if other.is_some() {
                        i -= 1;
                    } else if !last {
                        return 1;
                    }
                    output = false;
                    state = output_state;
                    out.error();
                }
            },
            Jp::Escape => {
                let next = match (lead, byte) {
                    (0x28, Some(0x42)) => Some(Jp::Ascii),
                    (0x28, Some(0x4a)) => Some(Jp::Roman),
                    (0x28, Some(0x49)) => Some(Jp::Katakana),
                    (0x24, Some(0x40 | 0x42)) => Some(Jp::Lead),
                    _ => None,
                };
                if let Some(next) = next {
                    state = next;
                    output_state = next;
                    // Deux séquences d'échappement de suite, sans caractère
                    // entre elles : une erreur.
                    if output {
                        out.error();
                    }
                    output = true;
                } else {
                    if byte.is_some() {
                        i -= 2;
                    } else if last {
                        i -= 1;
                    } else {
                        return 2;
                    }
                    output = false;
                    state = output_state;
                    out.error();
                }
            }
        }
    }
}

/// Décodeur commun de `EUC-KR` et `Big5` : un octet ASCII, ou un octet de
/// tête 0x81 à 0xFE suivi d'un second octet que `pair` traduit.
fn two_bytes(bytes: &[u8], out: &mut Output, pair: fn(u8, u8, &mut Output) -> bool) -> usize {
    let mut lead = 0u8;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        if lead != 0 {
            let first = lead;
            lead = 0;
            if !pair(first, byte, out) {
                if byte.is_ascii() {
                    i -= 1;
                }
                out.error();
            }
        } else {
            match byte {
                0x00..=0x7f => out.push(u32::from(byte)),
                0x81..=0xfe => lead = byte,
                _ => out.error(),
            }
        }
    }
    usize::from(lead != 0)
}

/// Caractère `EUC-KR` de tête `lead` et de second octet `byte` ; faux s'il
/// n'y en a pas.
fn euc_kr_pair(lead: u8, byte: u8, out: &mut Output) -> bool {
    if !matches!(byte, 0x41..=0xfe) {
        return false;
    }
    let pointer = usize::from(lead - 0x81) * 190 + usize::from(byte - 0x41);
    match lookup(&EUC_KR, pointer) {
        Some(code) => {
            out.push(code);
            true
        }
        None => false,
    }
}

/// Caractère `Big5` de tête `lead` et de second octet `byte` ; faux s'il n'y
/// en a pas.
fn big5_pair(lead: u8, byte: u8, out: &mut Output) -> bool {
    let offset = match byte {
        0x40..=0x7e => 0x40,
        0xa1..=0xfe => 0x62,
        _ => return false,
    };
    let pointer = usize::from(lead - 0x81) * 157 + usize::from(byte - offset);
    // Quatre pointeurs valent deux points de code : une lettre et son accent.
    let (base, mark) = match pointer {
        1133 => (0xca, 0x304),
        1135 => (0xca, 0x30c),
        1164 => (0xea, 0x304),
        1166 => (0xea, 0x30c),
        _ => {
            return match BIG5.get(pointer).copied().filter(|code| *code != 0) {
                Some(code) => {
                    out.push(code);
                    true
                }
                None => false,
            };
        }
    };
    out.push(base);
    out.push(mark);
    true
}

/// Point de code du pointeur `pointer` des plages de `gb18030` à quatre
/// octets.
fn gb18030_range(pointer: u32) -> Option<u32> {
    if (pointer > 39419 && pointer < 189_000) || pointer > 1_237_575 {
        return None;
    }
    if pointer == 7457 {
        return Some(0xe7c7);
    }
    let at = GB18030_RANGES.partition_point(|(start, _)| *start <= pointer);
    let (start, code) = GB18030_RANGES.get(at.checked_sub(1)?).copied()?;
    Some(code + (pointer - start))
}

/// `gb18030` et `GBK`.
fn gb18030(bytes: &[u8], out: &mut Output) -> usize {
    let (mut first, mut second, mut third) = (0u8, 0u8, 0u8);
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        if third != 0 {
            let code = if byte.is_ascii_digit() {
                gb18030_range(
                    u32::from(first - 0x81) * 12600
                        + u32::from(second - 0x30) * 1260
                        + u32::from(third - 0x81) * 10
                        + u32::from(byte - 0x30),
                )
            } else {
                // Le deuxième et le troisième octets sont relus, avec
                // celui-ci.
                i -= 3;
                None
            };
            (first, second, third) = (0, 0, 0);
            out.push_or_error(code);
        } else if second != 0 {
            if matches!(byte, 0x81..=0xfe) {
                third = byte;
            } else {
                i -= 2;
                (first, second) = (0, 0);
                out.error();
            }
        } else if first != 0 {
            if byte.is_ascii_digit() {
                second = byte;
                continue;
            }
            let lead = first;
            first = 0;
            let offset = if byte < 0x7f { 0x40 } else { 0x41 };
            let code = if matches!(byte, 0x40..=0x7e | 0x80..=0xfe) {
                lookup(
                    &GB18030,
                    usize::from(lead - 0x81) * 190 + usize::from(byte - offset),
                )
            } else {
                None
            };
            if code.is_none() && byte.is_ascii() {
                i -= 1;
            }
            out.push_or_error(code);
        } else {
            match byte {
                0x00..=0x7f => out.push(u32::from(byte)),
                0x80 => out.push(0x20ac),
                0x81..=0xfe => first = byte,
                _ => out.error(),
            }
        }
    }
    usize::from(first != 0) + usize::from(second != 0) + usize::from(third != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(kind: MultiByte, bytes: &[u8]) -> String {
        decode(kind, bytes, true).text
    }

    #[test]
    fn japanese_encodings_decode() {
        // « 日本語 » puis un katakana demi-chasse.
        assert_eq!(
            text(MultiByte::ShiftJis, b"\x93\xFA\x96\x7B\x8C\xEA\xB1"),
            "日本語ｱ"
        );
        assert_eq!(
            text(MultiByte::EucJp, b"\xC6\xFC\xCB\xDC\xB8\xEC\x8E\xB1"),
            "日本語ｱ"
        );
        assert_eq!(
            text(
                MultiByte::Iso2022Jp,
                b"a\x1B$B\x46\x7C\x4B\x5C\x38\x6C\x1B(Bz"
            ),
            "a日本語z"
        );
        // JIS X 0212 par 0x8F.
        assert_eq!(text(MultiByte::EucJp, b"\x8F\xB0\xA1"), "丂");
        // Tête sans second octet valable : l'octet ASCII est gardé.
        assert_eq!(text(MultiByte::ShiftJis, b"\x81\x20"), "\u{fffd} ");
        assert_eq!(text(MultiByte::ShiftJis, b"\x81"), "\u{fffd}");
        // Deux échappements de suite : une erreur ; yen et macron en Roman.
        assert_eq!(
            text(MultiByte::Iso2022Jp, b"\x1B(J\x1B(J\x5C\x7E"),
            "\u{fffd}\u{a5}\u{203e}"
        );
        // Échappement inconnu : l'octet 0x1B est une erreur, la suite relue.
        assert_eq!(text(MultiByte::Iso2022Jp, b"\x1B(Xa"), "\u{fffd}(Xa");
        assert_eq!(text(MultiByte::Iso2022Jp, b"\x1B$"), "\u{fffd}$");
    }

    #[test]
    fn korean_and_chinese_encodings_decode() {
        assert_eq!(text(MultiByte::EucKr, b"\xC7\xD1\xB1\xDB"), "한글");
        assert_eq!(text(MultiByte::Big5, b"\xA4\xA4\xA4\xE5"), "中文");
        assert_eq!(text(MultiByte::Big5, b"\x88\x62"), "\u{ca}\u{304}");
        assert_eq!(text(MultiByte::Gb18030, b"\xD6\xD0\xCE\xC4\x80"), "中文€");
        // Quatre octets : plan de base, plans supplémentaires, cas à part.
        assert_eq!(text(MultiByte::Gb18030, b"\x81\x30\x81\x30"), "\u{80}");
        assert_eq!(text(MultiByte::Gb18030, b"\x90\x30\x81\x30"), "\u{10000}");
        assert_eq!(text(MultiByte::Gb18030, b"\xE3\x32\x9A\x35"), "\u{10ffff}");
        assert_eq!(text(MultiByte::Gb18030, b"\x81\x35\xF4\x37"), "\u{e7c7}");
        assert_eq!(text(MultiByte::Gb18030, b"\xA8\xBC"), "\u{1e3f}");
        // Suite invalide : les octets déjà lus sont relus.
        assert_eq!(
            text(MultiByte::Gb18030, b"\x81\x30\xFE\x29"),
            "\u{fffd}0\u{fffd})"
        );
        assert_eq!(
            text(MultiByte::Gb18030, b"\x81\x30\xFF"),
            "\u{fffd}0\u{fffd}"
        );
        assert_eq!(text(MultiByte::Gb18030, b"\x84\x31\xA5\x30"), "\u{fffd}");
    }

    #[test]
    fn unfinished_characters_wait_for_more_bytes() {
        let part = decode(MultiByte::Gb18030, b"a\x81\x30\x81", false);
        assert_eq!((part.text.as_str(), part.pending), ("a", 3));
        assert!(!part.replaced);
        let part = decode(MultiByte::EucJp, b"\x8F\xB0", false);
        assert_eq!((part.text.as_str(), part.pending), ("", 2));
        let part = decode(MultiByte::Iso2022Jp, b"a\x1B$", false);
        assert_eq!((part.text.as_str(), part.pending), ("a", 2));
        let whole = decode(MultiByte::Big5, b"\xA4", true);
        assert_eq!((whole.text.as_str(), whole.pending), ("\u{fffd}", 0));
        assert!(whole.replaced);
    }
}
