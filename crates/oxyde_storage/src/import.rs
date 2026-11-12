//! Import des favoris d'un autre navigateur.
//!
//! Chrome et Edge rangent leurs favoris dans un fichier JSON (`Bookmarks`) :
//! un arbre de dossiers dont les feuilles de type `url` portent un nom et une
//! adresse. Ce module lit ce fichier, qui vient d'un autre programme et
//! n'est donc pas fiable : la lecture est bornée en taille et en profondeur
//! et ne panique jamais.
//!
//! LIMITES CONNUES : les dossiers sont aplatis ; ni l'historique ni les mots
//! de passe ne sont importés (bases SQLite, et mots de passe chiffrés).

/// Profondeur maximale de l'arbre JSON accepté.
const MAX_DEPTH: usize = 64;
/// Nombre maximal de favoris importés.
const MAX_IMPORTED: usize = 10_000;

/// Valeur JSON, réduite à ce dont l'import a besoin.
#[derive(Debug, PartialEq)]
enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    String(String),
    /// Nombre, booléen ou `null` : leur valeur ne sert pas.
    Other,
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Option<()> {
        self.skip_space();
        (self.peek() == Some(byte)).then(|| self.at += 1)
    }

    fn value(&mut self, depth: usize) -> Option<Json> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.skip_space();
        match self.peek()? {
            b'{' => {
                self.at += 1;
                let mut members = Vec::new();
                if self.eat(b'}').is_some() {
                    return Some(Json::Object(members));
                }
                loop {
                    self.skip_space();
                    let key = self.string()?;
                    self.eat(b':')?;
                    members.push((key, self.value(depth + 1)?));
                    if self.eat(b',').is_none() {
                        self.eat(b'}')?;
                        return Some(Json::Object(members));
                    }
                }
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                if self.eat(b']').is_some() {
                    return Some(Json::Array(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    if self.eat(b',').is_none() {
                        self.eat(b']')?;
                        return Some(Json::Array(items));
                    }
                }
            }
            b'"' => self.string().map(Json::String),
            _ => {
                // Nombre, `true`, `false`, `null` : jusqu'au prochain délimiteur.
                let start = self.at;
                while self.peek().is_some_and(|b| {
                    !matches!(b, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
                }) {
                    self.at += 1;
                }
                (self.at > start).then_some(Json::Other)
            }
        }
    }

    /// Quatre chiffres hexadécimaux d'un échappement `\u`.
    fn hex4(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.at..self.at + 4)?;
        self.at += 4;
        u32::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()
    }

    fn string(&mut self) -> Option<String> {
        if self.peek()? != b'"' {
            return None;
        }
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let byte = self.peek()?;
            self.at += 1;
            match byte {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let escape = self.peek()?;
                    self.at += 1;
                    let c = match escape {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let high = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&high)
                                && self.bytes.get(self.at..self.at + 2) == Some(b"\\u")
                            {
                                // Paire de substituts : un seul caractère.
                                self.at += 2;
                                let low = self.hex4()?;
                                0x10000 + ((high - 0xD800) << 10) + low.checked_sub(0xDC00)?
                            } else {
                                high
                            };
                            char::from_u32(code).unwrap_or('\u{FFFD}')
                        }
                        other => char::from(other),
                    };
                    out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                }
                _ => out.push(byte),
            }
        }
    }
}

fn member<'a>(members: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    members
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text<'a>(members: &'a [(String, Json)], name: &str) -> Option<&'a str> {
    match member(members, name)? {
        Json::String(text) => Some(text),
        _ => None,
    }
}

/// Favoris (adresse, titre) du fichier `Bookmarks` de Chrome ou d'Edge, dans
/// l'ordre de l'arbre, dossiers aplatis. Seules les adresses `http` et
/// `https` sont gardées. Un fichier illisible ne donne rien.
#[must_use]
pub fn chromium_bookmarks(json: &str) -> Vec<(String, String)> {
    let mut parser = Parser {
        bytes: json.as_bytes(),
        at: 0,
    };
    let Some(root) = parser.value(0) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    // Parcours itératif, dans l'ordre du fichier.
    let mut stack = vec![&root];
    while let Some(value) = stack.pop() {
        if found.len() >= MAX_IMPORTED {
            break;
        }
        match value {
            Json::Object(members) => {
                if text(members, "type") == Some("url")
                    && let Some(url) = text(members, "url")
                    && (url.starts_with("http://") || url.starts_with("https://"))
                {
                    let title = text(members, "name").unwrap_or("");
                    found.push((url.to_owned(), title.to_owned()));
                }
                stack.extend(members.iter().rev().map(|(_, value)| value));
            }
            Json::Array(items) => stack.extend(items.iter().rev()),
            Json::String(_) | Json::Other => {}
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "checksum": "abc", "version": 1,
        "roots": {
            "bookmark_bar": { "type": "folder", "name": "Barre", "children": [
                { "type": "url", "name": "Rust \u2014 \"langage\"", "url": "https://www.rust-lang.org/" },
                { "type": "folder", "name": "Docs", "children": [
                    { "type": "url", "name": "wgpu \ud83d\ude80", "url": "https://docs.rs/wgpu" },
                    { "type": "url", "name": "Local", "url": "file:///C:/secret.txt" },
                    { "type": "url", "name": "Script", "url": "javascript:alert(1)" }
                ] }
            ] },
            "other": { "type": "folder", "children": [
                { "type": "url", "name": "", "url": "http://exemple.test/a?b=1" }
            ] }
        }
    }"#;

    #[test]
    fn web_bookmarks_come_out_in_file_order() {
        assert_eq!(
            chromium_bookmarks(SAMPLE),
            [
                (
                    "https://www.rust-lang.org/".to_owned(),
                    "Rust — \"langage\"".to_owned()
                ),
                ("https://docs.rs/wgpu".to_owned(), "wgpu 🚀".to_owned()),
                ("http://exemple.test/a?b=1".to_owned(), String::new()),
            ]
        );
    }

    #[test]
    fn damaged_or_hostile_files_give_nothing() {
        for bad in [
            "",
            "{",
            "[1,2",
            "{\"a\":}",
            "\"sans fin",
            "{\"a\" 1}",
            "nul",
        ] {
            assert!(chromium_bookmarks(bad).is_empty(), "{bad}");
        }
        // Une imbrication démesurée est refusée sans épuiser la pile.
        let deep = "[".repeat(100_000);
        assert!(chromium_bookmarks(&deep).is_empty());
        // Échappement tronqué, substitut isolé.
        assert!(
            chromium_bookmarks(r#"{"type":"url","url":"https://a.test/","name":"\u12"#).is_empty()
        );
        assert_eq!(
            chromium_bookmarks(r#"{"type":"url","url":"https://a.test/","name":"\ud83d"}"#),
            [("https://a.test/".to_owned(), "\u{FFFD}".to_owned())]
        );
    }
}
