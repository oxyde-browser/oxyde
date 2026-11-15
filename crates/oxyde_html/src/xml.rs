//! Analyseur XML 1.0 (cinquième édition) avec espaces de noms, qui produit un
//! [`oxyde_dom::Document`].
//!
//! Il vérifie que le document est bien formé : une seule racine, balises
//! appariées, noms valides, attributs uniques, préfixes déclarés, caractères
//! permis. La première faute arrête l'analyse et rend une [`XmlError`], comme
//! le veut XML (pas de rattrapage, à l'inverse du HTML).
//!
//! Le sous-ensemble interne du DOCTYPE est lu pour ses entités générales
//! internes (`<!ENTITY nom "valeur">`), que le contenu et les attributs
//! peuvent ensuite citer. Les documents XHTML (reconnus à l'identifiant
//! public de leur DOCTYPE) disposent en plus des entités nommées du HTML
//! (`&nbsp;`…), comme dans les autres navigateurs.
//!
//! LIMITES CONNUES : aucune ressource externe n'est chargée (DTD externe,
//! entité externe : une référence à une entité externe ne produit rien) ; les
//! valeurs par défaut d'attributs d'un `<!ATTLIST>` sont ignorées ; les
//! entités paramètres ne sont pas développées ; aucune validation contre la
//! DTD ; la déclaration `<?xml … ?>` est vérifiée dans sa forme, mais son
//! `encoding` n'est pas suivi (le texte est déjà décodé) ni son `standalone`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use oxyde_dom::{Attribute, Document, Namespace, NodeId};

use crate::entities;

/// Espace de noms réservé au préfixe `xml`.
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// Espace de noms réservé aux déclarations `xmlns`.
const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

/// Taille totale que peuvent atteindre les entités développées : au-delà,
/// le document est refusé (entités qui se citent en cascade pour grossir
/// sans fin).
const MAX_EXPANSION: usize = 1 << 24;
/// Profondeur maximale d'entités citées les unes dans les autres.
const MAX_ENTITY_DEPTH: usize = 32;

/// Identifiants publics des DOCTYPE qui donnent droit aux entités du HTML.
const XHTML_PUBLIC_IDS: &[&str] = &[
    "-//W3C//DTD XHTML 1.0 Transitional//EN",
    "-//W3C//DTD XHTML 1.1//EN",
    "-//W3C//DTD XHTML 1.0 Strict//EN",
    "-//W3C//DTD XHTML 1.0 Frameset//EN",
    "-//W3C//DTD XHTML Basic 1.0//EN",
    "-//W3C//DTD XHTML 1.1 plus MathML 2.0//EN",
    "-//W3C//DTD XHTML 1.1 plus MathML 2.0 plus SVG 1.1//EN",
    "-//W3C//DTD MathML 2.0//EN",
    "-//WAPFORUM//DTD XHTML Mobile 1.0//EN",
    "-//WAPFORUM//DTD XHTML Mobile 1.1//EN",
    "-//WAPFORUM//DTD XHTML Mobile 1.2//EN",
];

/// Faute qui rend un document XML mal formé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlError {
    /// Ce qui ne va pas.
    pub message: String,
    /// Ligne de la faute dans le source, à partir de 1.
    pub line: usize,
    /// Colonne de la faute, en caractères, à partir de 1.
    pub column: usize,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (ligne {}, colonne {})",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for XmlError {}

/// Faute en cours d'analyse : le message seul, situé ensuite dans le source.
type Fault = String;

/// Vrai si `c` peut commencer un nom XML.
fn is_name_start(c: char) -> bool {
    matches!(c,
        ':' | '_' | 'A'..='Z' | 'a'..='z'
        | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}' | '\u{f8}'..='\u{2ff}'
        | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}'
        | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}'
        | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}')
}

/// Vrai si `c` peut figurer dans un nom XML.
fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
}

/// Vrai si `c` est un caractère permis dans un document XML.
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}

/// Vrai si `c` est un blanc XML.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// Longueur, en octets, du nom XML par lequel commence `text` ; 0 s'il n'y en
/// a pas.
fn name_len(text: &str) -> usize {
    let mut chars = text.char_indices();
    match chars.next() {
        Some((_, first)) if is_name_start(first) => {}
        _ => return 0,
    }
    chars
        .find(|(_, c)| !is_name_char(*c))
        .map_or(text.len(), |(at, _)| at)
}

/// Sépare un nom qualifié en préfixe et nom local ; faute si le nom a plus
/// d'un deux-points ou une moitié vide.
fn split_qualified(name: &str) -> Result<(Option<&str>, &str), Fault> {
    match name.split_once(':') {
        None => Ok((None, name)),
        Some((prefix, local))
            if !prefix.is_empty()
                && !local.is_empty()
                && !local.contains(':')
                && local.chars().next().is_some_and(is_name_start) =>
        {
            Ok((Some(prefix), local))
        }
        Some(_) => Err(format!("nom qualifié invalide : « {name} »")),
    }
}

/// Caractère que désigne la référence numérique `body` (`#38`, `#x26`).
fn char_reference(body: &str) -> Result<char, Fault> {
    let code = match body.strip_prefix("#x") {
        Some(hex) if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
            u32::from_str_radix(hex, 16).ok()
        }
        Some(_) => None,
        None => body
            .strip_prefix('#')
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|digits| digits.parse::<u32>().ok()),
    };
    code.and_then(char::from_u32)
        .filter(|c| is_xml_char(*c))
        .ok_or_else(|| format!("référence de caractère invalide : « &{body}; »"))
}

/// Entité prédéfinie de XML.
fn predefined(name: &str) -> Option<&'static str> {
    match name {
        "lt" => Some("<"),
        "gt" => Some(">"),
        "amp" => Some("&"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        _ => None,
    }
}

/// Texte en cours de lecture : le source, ou le texte d'une entité citée.
struct Input {
    text: Rc<str>,
    pos: usize,
    /// Nombre d'éléments ouverts à l'entrée : une entité doit refermer ceux
    /// qu'elle ouvre, et aucun autre.
    depth: usize,
    /// Nom de l'entité lue, pour refuser qu'elle se cite elle-même.
    entity: Option<Rc<str>>,
}

/// Élément ouvert.
struct Open {
    node: NodeId,
    qualified: String,
    /// Nombre de liaisons d'espaces de noms avant cet élément.
    bindings: usize,
}

/// Attribut lu dans une balise ouvrante, avant résolution de son préfixe.
struct RawAttr {
    name: String,
    value: String,
}

struct Parser {
    doc: Document,
    inputs: Vec<Input>,
    open: Vec<Open>,
    /// Liaisons préfixe -> adresse en vigueur, la plus récente en dernier ;
    /// le préfixe vide est l'espace de noms par défaut.
    bindings: Vec<(String, String)>,
    /// Entités générales déclarées ; `None` pour une entité externe.
    entities: HashMap<String, Option<Rc<str>>>,
    /// Le DOCTYPE donne droit aux entités nommées du HTML.
    html_entities: bool,
    /// Octets d'entités développés jusqu'ici.
    expanded: usize,
    /// Texte en attente d'un nœud.
    text: String,
}

/// Analyse le document XML `source`.
///
/// # Errors
///
/// Rend la première faute qui rend le document mal formé.
pub fn parse_xml(source: &str) -> Result<Document, XmlError> {
    // Fins de ligne ramenées à `\n`, marque d'ordre d'octets retirée.
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let text: Rc<str> = Rc::from(normalized.strip_prefix('\u{feff}').unwrap_or(&normalized));
    let mut parser = Parser {
        doc: Document::new(),
        inputs: vec![Input {
            text: Rc::clone(&text),
            pos: 0,
            depth: 0,
            entity: None,
        }],
        open: Vec::new(),
        bindings: Vec::new(),
        entities: HashMap::new(),
        html_entities: false,
        expanded: 0,
        text: String::new(),
    };
    match parser.document() {
        Ok(()) => Ok(parser.doc),
        Err(message) => {
            // La faute est située dans le source, même lue dans une entité.
            let pos = parser.inputs.first().map_or(0, |input| input.pos);
            let before = text.get(..pos).unwrap_or(&text);
            let line = before.matches('\n').count() + 1;
            let column = before
                .rsplit('\n')
                .next()
                .map_or(0, |last| last.chars().count())
                + 1;
            Err(XmlError {
                message,
                line,
                column,
            })
        }
    }
}

impl Parser {
    // ---- Lecture ---------------------------------------------------------------------

    /// Texte en cours et position dans ce texte.
    fn cursor(&self) -> (Rc<str>, usize) {
        self.inputs.last().map_or_else(
            || (Rc::from(""), 0),
            |input| (Rc::clone(&input.text), input.pos),
        )
    }

    /// Avance de `len` octets.
    fn advance(&mut self, len: usize) {
        if let Some(input) = self.inputs.last_mut() {
            input.pos = input.pos.saturating_add(len).min(input.text.len());
        }
    }

    /// Vrai si la suite du texte en cours commence par `prefix`.
    fn at(&self, prefix: &str) -> bool {
        let (text, pos) = self.cursor();
        text.get(pos..).is_some_and(|rest| rest.starts_with(prefix))
    }

    /// Consomme `prefix` s'il est là.
    fn eat(&mut self, prefix: &str) -> bool {
        let found = self.at(prefix);
        if found {
            self.advance(prefix.len());
        }
        found
    }

    /// Exige `prefix`.
    fn expect(&mut self, prefix: &str) -> Result<(), Fault> {
        if self.eat(prefix) {
            Ok(())
        } else {
            Err(format!("« {prefix} » attendu"))
        }
    }

    /// Saute les blancs ; vrai s'il y en avait.
    fn skip_space(&mut self) -> bool {
        let (text, pos) = self.cursor();
        let rest = text.get(pos..).unwrap_or_default();
        let len = rest.find(|c| !is_space(c)).unwrap_or(rest.len());
        self.advance(len);
        len > 0
    }

    /// Lit un nom XML.
    fn name(&mut self) -> Result<String, Fault> {
        let (text, pos) = self.cursor();
        let rest = text.get(pos..).unwrap_or_default();
        let len = name_len(rest);
        if len == 0 {
            return Err("nom attendu".to_owned());
        }
        self.advance(len);
        Ok(rest.get(..len).unwrap_or_default().to_owned())
    }

    /// Lit jusqu'à `end`, qu'il consomme ; rend ce qui précède.
    fn until(&mut self, end: &str, what: &str) -> Result<String, Fault> {
        let (text, pos) = self.cursor();
        let rest = text.get(pos..).unwrap_or_default();
        let Some(stop) = rest.find(end) else {
            return Err(format!("{what} non terminé"));
        };
        let inner = rest.get(..stop).unwrap_or_default();
        if let Some(bad) = inner.chars().find(|c| !is_xml_char(*c)) {
            return Err(format!(
                "caractère interdit en XML : U+{:04X}",
                u32::from(bad)
            ));
        }
        self.advance(stop + end.len());
        Ok(inner.to_owned())
    }

    /// Lit une valeur entre guillemets, sans l'interpréter.
    fn quoted(&mut self, what: &str) -> Result<String, Fault> {
        let quote = if self.eat("\"") {
            "\""
        } else if self.eat("'") {
            "'"
        } else {
            return Err(format!("{what} entre guillemets attendu"));
        };
        self.until(quote, what)
    }

    // ---- Construction ----------------------------------------------------------------

    /// Nœud qui reçoit les nœuds lus : l'élément ouvert, sinon le document.
    fn parent(&self) -> NodeId {
        self.open.last().map_or(self.doc.root(), |open| open.node)
    }

    /// Verse le texte en attente dans un nœud de texte.
    fn flush_text(&mut self) {
        if !self.text.is_empty() {
            let node = self.doc.create_text(&self.text);
            self.doc.append_child(self.parent(), node);
            self.text.clear();
        }
    }

    /// Ajoute `node` au nœud courant, après le texte en attente.
    fn append(&mut self, node: NodeId) {
        self.flush_text();
        self.doc.append_child(self.parent(), node);
    }

    /// Adresse liée au préfixe `prefix` (vide : espace de noms par défaut).
    fn lookup(&self, prefix: &str) -> Option<&str> {
        if prefix == "xml" {
            return Some(XML_NS);
        }
        self.bindings
            .iter()
            .rev()
            .find(|(known, _)| known == prefix)
            .map(|(_, uri)| uri.as_str())
    }

    // ---- Document --------------------------------------------------------------------

    fn document(&mut self) -> Result<(), Fault> {
        self.xml_declaration()?;
        let mut doctype_seen = false;
        // Prologue : commentaires, instructions, DOCTYPE, puis la racine.
        loop {
            self.skip_space();
            if self.at("<!--") {
                self.comment()?;
            } else if self.at("<?") {
                self.processing_instruction()?;
            } else if self.at("<!DOCTYPE") {
                if doctype_seen {
                    return Err("un seul DOCTYPE est permis".to_owned());
                }
                doctype_seen = true;
                self.doctype()?;
            } else if self.at("<") {
                break;
            } else if self.cursor().0.len() == self.cursor().1 {
                return Err("aucun élément racine".to_owned());
            } else {
                return Err("texte hors de l'élément racine".to_owned());
            }
        }
        self.start_tag()?;
        self.content()?;
        // Épilogue : commentaires et instructions seulement.
        loop {
            self.skip_space();
            if self.at("<!--") {
                self.comment()?;
            } else if self.at("<?") {
                self.processing_instruction()?;
            } else if self.cursor().0.len() == self.cursor().1 {
                return Ok(());
            } else if self.at("<") {
                return Err("un seul élément racine est permis".to_owned());
            } else {
                return Err("texte hors de l'élément racine".to_owned());
            }
        }
    }

    /// Déclaration `<?xml version="1.0" encoding="…" standalone="…"?>`, en
    /// tout début de document.
    fn xml_declaration(&mut self) -> Result<(), Fault> {
        let (text, pos) = self.cursor();
        let rest = text.get(pos..).unwrap_or_default();
        let declared = rest
            .strip_prefix("<?xml")
            .is_some_and(|after| after.starts_with(is_space));
        if !declared {
            return Ok(());
        }
        self.advance(5);
        let mut seen: Vec<String> = Vec::new();
        loop {
            let spaced = self.skip_space();
            if self.eat("?>") {
                break;
            }
            if !spaced {
                return Err("déclaration XML mal formée".to_owned());
            }
            let key = self.name()?;
            self.skip_space();
            self.expect("=")?;
            self.skip_space();
            let value = self.quoted("valeur de la déclaration XML")?;
            let valid = match (key.as_str(), seen.len()) {
                ("version", 0) => value.strip_prefix("1.").is_some_and(|minor| {
                    !minor.is_empty() && minor.bytes().all(|b| b.is_ascii_digit())
                }),
                ("encoding", 1) => {
                    value
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
                        && value
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                }
                ("standalone", 1 | 2) => {
                    !seen.iter().any(|k| k == "standalone")
                        && matches!(value.as_str(), "yes" | "no")
                }
                _ => false,
            };
            if !valid {
                return Err(format!("déclaration XML : « {key} » inattendu ou invalide"));
            }
            seen.push(key);
        }
        if seen.is_empty() {
            return Err("déclaration XML sans version".to_owned());
        }
        Ok(())
    }

    fn comment(&mut self) -> Result<(), Fault> {
        self.expect("<!--")?;
        let inner = self.until("-->", "commentaire")?;
        if inner.contains("--") || inner.ends_with('-') {
            return Err("« -- » est interdit dans un commentaire".to_owned());
        }
        let node = self.doc.create_comment(&inner);
        self.append(node);
        Ok(())
    }

    fn processing_instruction(&mut self) -> Result<(), Fault> {
        self.expect("<?")?;
        let target = self.name()?;
        if target.eq_ignore_ascii_case("xml") {
            return Err("la déclaration XML n'est permise qu'en tout début de document".to_owned());
        }
        if target.contains(':') {
            return Err(format!("cible d'instruction invalide : « {target} »"));
        }
        let data = if self.eat("?>") {
            String::new()
        } else {
            if !self.skip_space() {
                return Err("blanc attendu après la cible de l'instruction".to_owned());
            }
            self.until("?>", "instruction de traitement")?
        };
        let node = self.doc.create_processing_instruction(&target, &data);
        self.append(node);
        Ok(())
    }

    // ---- DOCTYPE ---------------------------------------------------------------------

    fn doctype(&mut self) -> Result<(), Fault> {
        self.expect("<!DOCTYPE")?;
        if !self.skip_space() {
            return Err("blanc attendu après « <!DOCTYPE »".to_owned());
        }
        let name = self.name()?;
        let spaced = self.skip_space();
        let (public_id, system_id) = if self.at("SYSTEM") || self.at("PUBLIC") {
            if !spaced {
                return Err("blanc attendu avant l'identifiant du DOCTYPE".to_owned());
            }
            self.external_id()?
        } else {
            (String::new(), String::new())
        };
        self.skip_space();
        if self.eat("[") {
            self.internal_subset()?;
            self.skip_space();
        }
        self.expect(">")?;
        self.html_entities = XHTML_PUBLIC_IDS.contains(&public_id.as_str());
        let node = self.doc.create_doctype(&name, &public_id, &system_id);
        self.append(node);
        Ok(())
    }

    /// `SYSTEM "…"` ou `PUBLIC "…" "…"` : rend (identifiant public,
    /// identifiant système).
    fn external_id(&mut self) -> Result<(String, String), Fault> {
        if self.eat("SYSTEM") {
            if !self.skip_space() {
                return Err("blanc attendu après « SYSTEM »".to_owned());
            }
            return Ok((String::new(), self.quoted("identifiant système")?));
        }
        self.expect("PUBLIC")?;
        if !self.skip_space() {
            return Err("blanc attendu après « PUBLIC »".to_owned());
        }
        let public_id = self.quoted("identifiant public")?;
        let allowed = |c: char| c.is_ascii_alphanumeric() || " \n-'()+,./:=?;!*#@$_%".contains(c);
        if !public_id.chars().all(allowed) {
            return Err("identifiant public invalide".to_owned());
        }
        if !self.skip_space() {
            return Err("blanc attendu après l'identifiant public".to_owned());
        }
        Ok((public_id, self.quoted("identifiant système")?))
    }

    /// Sous-ensemble interne du DOCTYPE, jusqu'à son `]` : seules les entités
    /// générales internes sont retenues.
    fn internal_subset(&mut self) -> Result<(), Fault> {
        loop {
            self.skip_space();
            if self.eat("]") {
                return Ok(());
            }
            if self.at("<!--") {
                self.expect("<!--")?;
                let inner = self.until("-->", "commentaire")?;
                if inner.contains("--") || inner.ends_with('-') {
                    return Err("« -- » est interdit dans un commentaire".to_owned());
                }
            } else if self.eat("<?") {
                self.name()?;
                self.until("?>", "instruction de traitement")?;
            } else if self.eat("%") {
                // Référence à une entité paramètre : non développée.
                self.name()?;
                self.expect(";")?;
            } else if self.eat("<!ENTITY") {
                self.entity_declaration()?;
            } else if self.eat("<!ELEMENT") || self.eat("<!ATTLIST") || self.eat("<!NOTATION") {
                self.skip_declaration()?;
            } else if self.cursor().0.len() == self.cursor().1 {
                return Err("DOCTYPE non terminé".to_owned());
            } else {
                return Err("déclaration inattendue dans le DOCTYPE".to_owned());
            }
        }
    }

    /// Saute une déclaration jusqu'à son `>`, guillemets respectés.
    fn skip_declaration(&mut self) -> Result<(), Fault> {
        loop {
            let (text, pos) = self.cursor();
            let rest = text.get(pos..).unwrap_or_default();
            let Some(stop) = rest.find(['>', '"', '\'']) else {
                return Err("déclaration non terminée".to_owned());
            };
            self.advance(stop);
            if self.eat(">") {
                return Ok(());
            }
            self.quoted("valeur")?;
        }
    }

    /// `<!ENTITY nom "valeur">`, après `<!ENTITY`.
    fn entity_declaration(&mut self) -> Result<(), Fault> {
        if !self.skip_space() {
            return Err("blanc attendu après « <!ENTITY »".to_owned());
        }
        // Entité paramètre : lue pour sa forme, puis oubliée.
        let parameter = self.eat("%");
        if parameter && !self.skip_space() {
            return Err("blanc attendu après « % »".to_owned());
        }
        let name = self.name()?;
        if name.contains(':') {
            return Err(format!("nom d'entité invalide : « {name} »"));
        }
        if !self.skip_space() {
            return Err("blanc attendu après le nom de l'entité".to_owned());
        }
        let value = if self.at("SYSTEM") || self.at("PUBLIC") {
            self.external_id()?;
            if self.skip_space() && self.eat("NDATA") {
                if !self.skip_space() {
                    return Err("blanc attendu après « NDATA »".to_owned());
                }
                self.name()?;
            }
            None
        } else {
            let literal = self.quoted("valeur d'entité")?;
            Some(Rc::from(Self::entity_value(&literal)?))
        };
        self.skip_space();
        self.expect(">")?;
        // La première déclaration d'un nom l'emporte.
        if !parameter {
            self.entities.entry(name).or_insert(value);
        }
        Ok(())
    }

    /// Texte de remplacement d'une entité : les références de caractères y
    /// sont développées dès la déclaration, les références d'entités
    /// laissées telles quelles.
    fn entity_value(literal: &str) -> Result<String, Fault> {
        let mut out = String::with_capacity(literal.len());
        let mut rest = literal;
        while let Some(at) = rest.find(['&', '%']) {
            out.push_str(rest.get(..at).unwrap_or_default());
            let tail = rest.get(at..).unwrap_or_default();
            let Some(end) = tail.find(';') else {
                return Err("référence non terminée dans une valeur d'entité".to_owned());
            };
            let body = tail.get(1..end).unwrap_or_default();
            if tail.starts_with('%') {
                return Err("entité paramètre citée dans une valeur d'entité".to_owned());
            }
            if body.starts_with('#') {
                out.push(char_reference(body)?);
            } else if name_len(body) == body.len() && !body.is_empty() {
                out.push_str(tail.get(..=end).unwrap_or_default());
            } else {
                return Err("référence mal formée dans une valeur d'entité".to_owned());
            }
            rest = tail.get(end + 1..).unwrap_or_default();
        }
        out.push_str(rest);
        Ok(out)
    }

    // ---- Contenu ---------------------------------------------------------------------

    /// Contenu de l'élément racine, jusqu'à sa balise fermante.
    fn content(&mut self) -> Result<(), Fault> {
        while !self.open.is_empty() {
            let (text, pos) = self.cursor();
            let rest = text.get(pos..).unwrap_or_default();
            if rest.is_empty() {
                // Fin d'une entité : elle doit avoir refermé ce qu'elle a
                // ouvert. Fin du source : il reste un élément ouvert.
                let balanced = self
                    .inputs
                    .last()
                    .is_some_and(|input| input.depth == self.open.len());
                if self.inputs.len() <= 1 {
                    let name = self.open.last().map_or("", |open| open.qualified.as_str());
                    return Err(format!("élément non fermé : « {name} »"));
                }
                if !balanced {
                    return Err("entité dont les balises ne sont pas appariées".to_owned());
                }
                self.inputs.pop();
            } else if rest.starts_with("<!--") {
                self.comment()?;
            } else if rest.starts_with("<![CDATA[") {
                self.advance(9);
                let data = self.until("]]>", "section CDATA")?;
                let node = self.doc.create_cdata(&data);
                self.append(node);
            } else if rest.starts_with("<?") {
                self.processing_instruction()?;
            } else if rest.starts_with("</") {
                self.end_tag()?;
            } else if rest.starts_with('<') {
                self.start_tag()?;
            } else if rest.starts_with('&') {
                self.reference()?;
            } else {
                let len = rest.find(['<', '&']).unwrap_or(rest.len());
                let chunk = rest.get(..len).unwrap_or_default();
                if chunk.contains("]]>") {
                    return Err("« ]]> » est interdit dans le texte".to_owned());
                }
                if let Some(bad) = chunk.chars().find(|c| !is_xml_char(*c)) {
                    return Err(format!(
                        "caractère interdit en XML : U+{:04X}",
                        u32::from(bad)
                    ));
                }
                self.text.push_str(chunk);
                self.advance(len);
            }
        }
        Ok(())
    }

    /// Référence dans le contenu : `&#38;`, `&amp;`, `&nom;`.
    fn reference(&mut self) -> Result<(), Fault> {
        let (text, pos) = self.cursor();
        let rest = text.get(pos..).unwrap_or_default();
        let Some(end) = rest.find(';') else {
            return Err("référence non terminée".to_owned());
        };
        let body = rest.get(1..end).unwrap_or_default();
        if body.starts_with('#') {
            self.text.push(char_reference(body)?);
            self.advance(end + 1);
            return Ok(());
        }
        if body.is_empty() || name_len(body) != body.len() {
            return Err("référence mal formée".to_owned());
        }
        self.advance(end + 1);
        if let Some(value) = predefined(body) {
            self.text.push_str(value);
            return Ok(());
        }
        match self.entities.get(body).cloned() {
            // Entité externe : jamais chargée.
            Some(None) => Ok(()),
            Some(Some(value)) => {
                self.expanded = self.expanded.saturating_add(value.len());
                if self.expanded > MAX_EXPANSION {
                    return Err("entités trop volumineuses une fois développées".to_owned());
                }
                if !value.contains(['<', '&']) {
                    self.text.push_str(&value);
                    return Ok(());
                }
                let recursive = self
                    .inputs
                    .iter()
                    .any(|input| input.entity.as_deref() == Some(body));
                if recursive || self.inputs.len() > MAX_ENTITY_DEPTH {
                    return Err(format!("l'entité « {body} » se cite elle-même"));
                }
                self.inputs.push(Input {
                    text: value,
                    pos: 0,
                    depth: self.open.len(),
                    entity: Some(Rc::from(body)),
                });
                Ok(())
            }
            None => match entities::lookup(body).filter(|_| self.html_entities) {
                Some(value) => {
                    self.text.push_str(value);
                    Ok(())
                }
                None => Err(format!("entité non déclarée : « &{body}; »")),
            },
        }
    }

    /// Valeur d'attribut normalisée : références développées, tabulations et
    /// fins de ligne ramenées à des espaces.
    fn attribute_value(&mut self, raw: &str, depth: usize) -> Result<String, Fault> {
        let mut out = String::with_capacity(raw.len());
        let mut rest = raw;
        loop {
            let len = rest.find(['&', '<', '\t', '\n']).unwrap_or(rest.len());
            out.push_str(rest.get(..len).unwrap_or_default());
            let tail = rest.get(len..).unwrap_or_default();
            let Some(first) = tail.chars().next() else {
                return Ok(out);
            };
            match first {
                '<' => return Err("« < » est interdit dans une valeur d'attribut".to_owned()),
                '&' => {
                    let Some(end) = tail.find(';') else {
                        return Err("référence non terminée".to_owned());
                    };
                    let body = tail.get(1..end).unwrap_or_default();
                    if body.starts_with('#') {
                        out.push(char_reference(body)?);
                    } else if body.is_empty() || name_len(body) != body.len() {
                        return Err("référence mal formée".to_owned());
                    } else if let Some(value) = predefined(body) {
                        out.push_str(value);
                    } else {
                        match self.entities.get(body).cloned() {
                            Some(Some(value)) => {
                                self.expanded = self.expanded.saturating_add(value.len());
                                if self.expanded > MAX_EXPANSION || depth >= MAX_ENTITY_DEPTH {
                                    return Err(format!(
                                        "l'entité « {body} » se développe sans fin"
                                    ));
                                }
                                out.push_str(&self.attribute_value(&value, depth + 1)?);
                            }
                            Some(None) => {
                                return Err(format!(
                                    "entité externe citée dans un attribut : « &{body}; »"
                                ));
                            }
                            None => match entities::lookup(body).filter(|_| self.html_entities) {
                                Some(value) => out.push_str(value),
                                None => {
                                    return Err(format!("entité non déclarée : « &{body}; »"));
                                }
                            },
                        }
                    }
                    rest = tail.get(end + 1..).unwrap_or_default();
                }
                _ => {
                    out.push(' ');
                    rest = tail.get(1..).unwrap_or_default();
                }
            }
        }
    }

    // ---- Balises ---------------------------------------------------------------------

    fn start_tag(&mut self) -> Result<(), Fault> {
        self.expect("<")?;
        let qualified = self.name()?;
        let mut raw: Vec<RawAttr> = Vec::new();
        let empty = loop {
            let spaced = self.skip_space();
            if self.eat("/>") {
                break true;
            }
            if self.eat(">") {
                break false;
            }
            if !spaced {
                return Err(format!("balise « {qualified} » mal formée"));
            }
            let name = self.name()?;
            self.skip_space();
            self.expect("=")?;
            self.skip_space();
            let literal = self.quoted("valeur d'attribut")?;
            let value = self.attribute_value(&literal, 0)?;
            if raw.iter().any(|other| other.name == name) {
                return Err(format!("attribut répété : « {name} »"));
            }
            raw.push(RawAttr { name, value });
        };

        // Déclarations d'espaces de noms de cette balise, d'abord.
        let scope = self.bindings.len();
        for attr in &raw {
            let prefix = match attr.name.as_str() {
                "xmlns" => "",
                other => match other.strip_prefix("xmlns:") {
                    Some(prefix) => prefix,
                    None => continue,
                },
            };
            let reserved = attr.value == XML_NS || attr.value == XMLNS_NS;
            let valid = match prefix {
                "xmlns" => false,
                "xml" => attr.value == XML_NS,
                "" => !reserved,
                _ => !reserved && !attr.value.is_empty() && !prefix.contains(':'),
            };
            if !valid {
                return Err(format!(
                    "déclaration d'espace de noms invalide : « {} »",
                    attr.name
                ));
            }
            self.bindings.push((prefix.to_owned(), attr.value.clone()));
        }

        let (prefix, local) = split_qualified(&qualified)?;
        let uri = match prefix {
            Some(prefix) => self
                .lookup(prefix)
                .ok_or_else(|| format!("préfixe non déclaré : « {prefix} »"))?
                .to_owned(),
            None => self.lookup("").unwrap_or_default().to_owned(),
        };
        let ns = self.doc.namespace(&uri);

        let mut attrs: Vec<Attribute> = Vec::with_capacity(raw.len());
        for attr in raw {
            let (attr_prefix, attr_local) = split_qualified(&attr.name)?;
            let attr_ns = match (attr_prefix, attr.name.as_str()) {
                (_, "xmlns") | (Some("xmlns"), _) => Namespace::Xmlns,
                (Some(attr_prefix), _) => {
                    let uri = self
                        .lookup(attr_prefix)
                        .ok_or_else(|| format!("préfixe non déclaré : « {attr_prefix} »"))?
                        .to_owned();
                    self.doc.namespace(&uri)
                }
                (None, _) => Namespace::None,
            };
            // Deux préfixes liés au même espace de noms : même attribut.
            let repeated = attr_ns != Namespace::None
                && attrs
                    .iter()
                    .any(|other| other.ns == attr_ns && other.local() == attr_local);
            if repeated {
                return Err(format!("attribut répété : « {} »", attr.name));
            }
            attrs.push(Attribute {
                prefix: attr_prefix.map(str::to_owned),
                ns: attr_ns,
                node: None,
                name: attr.name,
                value: attr.value,
            });
        }

        let node = self.doc.create_element(local, ns, attrs);
        if let (Some(prefix), Some(element)) = (prefix, self.doc.element_mut(node)) {
            element.prefix = Some(prefix.to_owned());
        }
        self.append(node);
        if empty {
            self.bindings.truncate(scope);
        } else {
            self.open.push(Open {
                node,
                qualified,
                bindings: scope,
            });
        }
        Ok(())
    }

    fn end_tag(&mut self) -> Result<(), Fault> {
        self.expect("</")?;
        let name = self.name()?;
        self.skip_space();
        self.expect(">")?;
        // Une entité ne referme pas un élément ouvert hors d'elle.
        let floor = self.inputs.last().map_or(0, |input| input.depth);
        match self.open.last() {
            Some(open) if open.qualified == name && self.open.len() > floor => {}
            Some(open) => {
                return Err(format!(
                    "balise fermante « {name} » inattendue : « {} » est ouvert",
                    open.qualified
                ));
            }
            None => return Err(format!("balise fermante « {name} » inattendue")),
        }
        self.flush_text();
        if let Some(open) = self.open.pop() {
            self.bindings.truncate(open.bindings);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(source: &str) -> Document {
        match parse_xml(source) {
            Ok(doc) => doc,
            Err(error) => panic!("document refusé : {error}"),
        }
    }

    fn fails(source: &str) -> XmlError {
        match parse_xml(source) {
            Ok(_) => panic!("document accepté : {source}"),
            Err(error) => error,
        }
    }

    fn root(doc: &Document) -> NodeId {
        doc.children(doc.root())
            .find(|child| doc.element(*child).is_some())
            .expect("élément racine")
    }

    #[test]
    fn elements_text_and_namespaces() {
        let doc = ok("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!-- avant -->\n\
             <a:liste xmlns:a=\"urn:x\" xmlns=\"urn:d\" xml:lang=\"fr\" a:n='1' n=\"2\">\
             <item>un &amp; deux&#x20;&#33;</item><a:vide/><![CDATA[<brut>]]><?cible  données ?>\
             </a:liste>\n<!-- après -->");
        let list = root(&doc);
        let element = doc.element(list).expect("racine");
        assert_eq!(
            (element.name.as_str(), element.prefix.as_deref()),
            ("liste", Some("a"))
        );
        assert_eq!(doc.namespace_uri(element.ns), Some("urn:x"));
        let attr = |name: &str| {
            let attr = element
                .attrs
                .iter()
                .find(|a| a.name == name)
                .expect("attribut");
            (doc.namespace_uri(attr.ns), attr.value.as_str())
        };
        assert_eq!(attr("xmlns:a"), (Some(XMLNS_NS), "urn:x"));
        assert_eq!(attr("xmlns"), (Some(XMLNS_NS), "urn:d"));
        assert_eq!(attr("xml:lang"), (Some(XML_NS), "fr"));
        assert_eq!(attr("a:n"), (Some("urn:x"), "1"));
        assert_eq!(attr("n"), (None, "2"));
        let children: Vec<NodeId> = doc.children(list).collect();
        assert_eq!(children.len(), 4);
        let item = doc.element(children[0]).expect("item");
        assert_eq!(doc.namespace_uri(item.ns), Some("urn:d"));
        assert_eq!(doc.text_content(children[0]), "un & deux !");
        assert_eq!(doc.text_content(list), "un & deux !<brut>");
        // Un commentaire de part et d'autre de la racine.
        assert_eq!(doc.children(doc.root()).count(), 3);
    }

    #[test]
    fn doctype_and_entities() {
        let doc = ok("<!DOCTYPE note SYSTEM \"note.dtd\" [\n\
             <!ELEMENT note (#PCDATA)>\n\
             <!ATTLIST note id CDATA \"a>b\">\n\
             <!ENTITY nom \"Oxyde\">\n\
             <!ENTITY gras \"<b>&nom;</b> &#38;#38;\">\n\
             <!ENTITY % param \"ignoré\">\n\
             <!ENTITY ext SYSTEM \"ailleurs.xml\">\n\
             ]>\n<note titre=\"&nom;\tici\">&gras;&ext;</note>");
        let note = root(&doc);
        assert_eq!(doc.text_content(note), "Oxyde &");
        assert_eq!(
            doc.element(note).and_then(|e| e.attr("titre")),
            Some("Oxyde ici")
        );
        let bold = doc.children(note).next().expect("b");
        assert_eq!(doc.element(bold).map(|e| e.name.as_str()), Some("b"));
        // Entités du HTML pour un DOCTYPE XHTML, et pour lui seul.
        let xhtml = ok(
            "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \
             \"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\
             <html xmlns=\"http://www.w3.org/1999/xhtml\"><body>a&nbsp;b</body></html>",
        );
        let html = root(&xhtml);
        assert_eq!(xhtml.element(html).map(|e| e.ns), Some(Namespace::Html));
        assert_eq!(xhtml.text_content(html), "a\u{a0}b");
        assert!(fails("<p>a&nbsp;b</p>").message.contains("non déclarée"));
    }

    #[test]
    fn malformed_documents_are_refused() {
        for source in [
            "",
            "texte",
            "<a>",
            "<a></b>",
            "<a/><b/>",
            "<a/>texte",
            "<a b=1/>",
            "<a b='1' b='2'/>",
            "<a b='<'/>",
            "<a b='1'c='2'/>",
            "<x:a/>",
            "<a x:b='1'/>",
            "<a xmlns:p='urn:x' xmlns:q='urn:x' p:b='1' q:b='2'/>",
            "<a xmlns:xmlns='urn:x'/>",
            "<a xmlns:p=''/>",
            "<a:b:c xmlns:a='urn:x'/>",
            "<a><!-- a -- b --></a>",
            "<a>]]></a>",
            "<a>&inconnue;</a>",
            "<a>&#0;</a>",
            "<a>&#xD800;</a>",
            "<a>\u{1}</a>",
            "<a><?xml version='1.0'?></a>",
            " <?xml version='1.0'?><a/>",
            "<?xml encoding='UTF-8'?><a/>",
            "<?xml version='2.0'?><a/>",
            "<a/><!DOCTYPE a>",
            "<!DOCTYPE a><!DOCTYPE a><a/>",
            "<!DOCTYPE a [<!ENTITY e '&e;'>]><a>&e;</a>",
            "<!DOCTYPE a [<!ENTITY e '<b>'>]><a>&e;</b></a>",
            "<!DOCTYPE a [<!ENTITY e '</a>'>]><a>&e;",
            "<1a/>",
        ] {
            fails(source);
        }
        let error = fails("<a>\n  <b></c>\n</a>");
        assert_eq!((error.line, error.column), (2, 10));
        assert!(error.to_string().contains("ligne 2"));
    }

    #[test]
    fn entity_expansion_is_bounded() {
        // Chaque entité cite dix fois la précédente.
        let mut source = String::from("<!DOCTYPE a [<!ENTITY e0 'xxxxxxxxxx'>");
        for level in 1..12 {
            let previous = format!("&e{};", level - 1).repeat(10);
            source.push_str(&format!("<!ENTITY e{level} '{previous}'>"));
        }
        source.push_str("]><a>&e11;</a>");
        assert!(fails(&source).message.contains("volumineuses"));
    }

    #[test]
    fn line_ends_and_deep_nesting() {
        let doc = ok("\u{feff}<a>un\r\ndeux\rtrois</a>\r\n");
        assert_eq!(doc.text_content(root(&doc)), "un\ndeux\ntrois");
        // Imbrication profonde : pas de récursion.
        let depth = 100_000;
        let source = format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth));
        ok(&source);
    }
}
