//! Construction de l'arbre (WHATWG HTML, §13.2.6), équivalent de
//! `HTMLTreeBuilder` / `HTMLConstructionSite` dans Blink.
//!
//! LIMITES CONNUES (suivies dans `conformance/`) :
//! - `<template>` est traité comme un élément ordinaire (pas de contenu de modèle) ;
//! - `<frameset>` est ignoré ;
//! - les scripts ne sont pas exécutés (drapeau « scripting » à faux) ;
//! - la détection du mode quirks ne connaît qu'une partie des identifiants publics.

use oxyde_dom::{Attribute, Document, Namespace, NodeId, QuirksMode};

use crate::tokenizer::{ContentState, Tag, Token, Tokenizer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    AfterBody,
    AfterAfterBody,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Default,
    ListItem,
    Button,
    Table,
    Select,
}

#[derive(Clone, Debug)]
enum FormatEntry {
    Marker,
    Element { node: NodeId, tag: Tag },
}

const WS: [char; 4] = ['\t', '\n', '\x0c', ' '];

const SPECIAL: &[&str] = &[
    "address", "applet", "area", "article", "aside", "base", "basefont", "bgsound", "blockquote",
    "body", "br", "button", "caption", "center", "col", "colgroup", "dd", "details", "dir", "div",
    "dl", "dt", "embed", "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset",
    "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "hgroup", "hr", "html", "iframe", "img",
    "input", "keygen", "li", "link", "listing", "main", "marquee", "menu", "meta", "nav",
    "noembed", "noframes", "noscript", "object", "ol", "p", "param", "plaintext", "pre", "script",
    "search", "section", "select", "source", "style", "summary", "table", "tbody", "td",
    "template", "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul", "wbr", "xmp",
];

const IMPLIED_END: &[&str] =
    &["dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc"];

const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

const FORMATTING: &[&str] =
    &["a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u"];

const BLOCK_START: &[&str] = &[
    "address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div", "dl",
    "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "main", "menu", "nav", "ol",
    "p", "search", "section", "summary", "ul",
];

const BLOCK_END: &[&str] = &[
    "address", "article", "aside", "blockquote", "button", "center", "details", "dialog", "dir",
    "div", "dl", "fieldset", "figcaption", "figure", "footer", "header", "hgroup", "listing",
    "main", "menu", "nav", "ol", "pre", "search", "section", "summary", "ul",
];

const FOREIGN_BREAKOUT: &[&str] = &[
    "b", "big", "blockquote", "body", "br", "center", "code", "dd", "div", "dl", "dt", "em",
    "embed", "h1", "h2", "h3", "h4", "h5", "h6", "head", "hr", "i", "img", "li", "listing", "menu",
    "meta", "nobr", "ol", "p", "pre", "ruby", "s", "small", "span", "strong", "strike", "sub",
    "sup", "table", "tt", "u", "ul", "var",
];

const SVG_TAG_CASE: &[(&str, &str)] = &[
    ("altglyph", "altGlyph"),
    ("altglyphdef", "altGlyphDef"),
    ("altglyphitem", "altGlyphItem"),
    ("animatecolor", "animateColor"),
    ("animatemotion", "animateMotion"),
    ("animatetransform", "animateTransform"),
    ("clippath", "clipPath"),
    ("feblend", "feBlend"),
    ("fecolormatrix", "feColorMatrix"),
    ("fecomponenttransfer", "feComponentTransfer"),
    ("fecomposite", "feComposite"),
    ("feconvolvematrix", "feConvolveMatrix"),
    ("fediffuselighting", "feDiffuseLighting"),
    ("fedisplacementmap", "feDisplacementMap"),
    ("fedistantlight", "feDistantLight"),
    ("fedropshadow", "feDropShadow"),
    ("feflood", "feFlood"),
    ("fefunca", "feFuncA"),
    ("fefuncb", "feFuncB"),
    ("fefuncg", "feFuncG"),
    ("fefuncr", "feFuncR"),
    ("fegaussianblur", "feGaussianBlur"),
    ("feimage", "feImage"),
    ("femerge", "feMerge"),
    ("femergenode", "feMergeNode"),
    ("femorphology", "feMorphology"),
    ("feoffset", "feOffset"),
    ("fepointlight", "fePointLight"),
    ("fespecularlighting", "feSpecularLighting"),
    ("fespotlight", "feSpotLight"),
    ("fetile", "feTile"),
    ("feturbulence", "feTurbulence"),
    ("foreignobject", "foreignObject"),
    ("glyphref", "glyphRef"),
    ("lineargradient", "linearGradient"),
    ("radialgradient", "radialGradient"),
    ("textpath", "textPath"),
];

/// Analyse un document HTML complet.
#[must_use]
pub fn parse_document(source: &str) -> Document {
    let mut builder = TreeBuilder {
        doc: Document::new(),
        tok: Tokenizer::new(source),
        mode: Mode::Initial,
        original_mode: Mode::InBody,
        open: Vec::new(),
        formatting: Vec::new(),
        head: None,
        form: None,
        foster_parenting: false,
        skip_lf: false,
        done: false,
    };
    while !builder.done {
        let token = builder.tok.next_token();
        builder.dispatch(token);
        builder.tok.allow_cdata =
            builder.open.last().is_some_and(|n| builder.ns(*n) != Namespace::Html);
    }
    builder.doc
}

struct TreeBuilder {
    doc: Document,
    tok: Tokenizer,
    mode: Mode,
    original_mode: Mode,
    /// Pile des éléments ouverts (le dernier est le nœud courant).
    open: Vec<NodeId>,
    /// Liste des éléments de mise en forme actifs.
    formatting: Vec<FormatEntry>,
    head: Option<NodeId>,
    form: Option<NodeId>,
    foster_parenting: bool,
    skip_lf: bool,
    done: bool,
}

fn end_tag(name: &str) -> Token {
    Token::EndTag(Tag { name: name.to_owned(), ..Tag::default() })
}

fn start_tag(name: &str) -> Tag {
    Tag { name: name.to_owned(), ..Tag::default() }
}

impl TreeBuilder {
    // ---- Accès à la pile -------------------------------------------------

    fn current(&self) -> NodeId {
        self.open.last().copied().unwrap_or_else(|| self.doc.root())
    }

    fn ns(&self, id: NodeId) -> Namespace {
        self.doc.element(id).map_or(Namespace::Html, |e| e.ns)
    }

    fn name(&self, id: NodeId) -> &str {
        self.doc.element(id).map_or("", |e| e.name.as_str())
    }

    fn is(&self, id: NodeId, name: &str) -> bool {
        self.doc.element(id).is_some_and(|e| e.is_html(name))
    }

    fn is_any(&self, id: NodeId, names: &[&str]) -> bool {
        self.doc.element(id).is_some_and(|e| e.ns == Namespace::Html && names.contains(&e.name.as_str()))
    }

    fn current_is(&self, name: &str) -> bool {
        self.is(self.current(), name)
    }

    fn is_scope_boundary(&self, id: NodeId, scope: Scope) -> bool {
        let Some(e) = self.doc.element(id) else { return true };
        let n = e.name.as_str();
        match scope {
            Scope::Select => !(e.ns == Namespace::Html && matches!(n, "optgroup" | "option")),
            Scope::Table => e.ns == Namespace::Html && matches!(n, "html" | "table" | "template"),
            _ => match e.ns {
                Namespace::Html => {
                    matches!(
                        n,
                        "applet" | "caption" | "html" | "table" | "td" | "th" | "marquee"
                            | "object" | "template"
                    ) || (scope == Scope::ListItem && matches!(n, "ol" | "ul"))
                        || (scope == Scope::Button && n == "button")
                }
                Namespace::MathMl => matches!(n, "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml"),
                Namespace::Svg => matches!(n, "foreignObject" | "desc" | "title"),
            },
        }
    }

    fn in_scope_any(&self, names: &[&str], scope: Scope) -> bool {
        for &id in self.open.iter().rev() {
            if self.is_any(id, names) {
                return true;
            }
            if self.is_scope_boundary(id, scope) {
                return false;
            }
        }
        false
    }

    fn in_scope(&self, name: &str, scope: Scope) -> bool {
        self.in_scope_any(&[name], scope)
    }

    fn pop(&mut self) {
        self.open.pop();
    }

    /// Dépile jusqu'à avoir dépilé un élément HTML dont le nom est dans `names`.
    fn pop_until_any(&mut self, names: &[&str]) {
        while let Some(id) = self.open.pop() {
            if self.is_any(id, names) {
                break;
            }
        }
    }

    fn pop_until(&mut self, name: &str) {
        self.pop_until_any(&[name]);
    }

    /// Dépile tant que le nœud courant n'est pas dans `names` (ni `html`).
    fn clear_stack_to(&mut self, names: &[&str]) {
        while let Some(&id) = self.open.last() {
            if self.is_any(id, names) || self.is(id, "html") {
                break;
            }
            self.open.pop();
        }
    }

    fn generate_implied_end_tags(&mut self, except: Option<&str>) {
        while let Some(&id) = self.open.last() {
            if self.is_any(id, IMPLIED_END) && except.is_none_or(|x| !self.is(id, x)) {
                self.open.pop();
            } else {
                break;
            }
        }
    }

    fn close_p(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        self.pop_until("p");
    }

    fn close_p_if_in_button_scope(&mut self) {
        if self.in_scope("p", Scope::Button) {
            self.close_p();
        }
    }

    // ---- Insertion -------------------------------------------------------

    /// « Appropriate place for inserting a node » : (parent, nœud suivant).
    fn insertion_place(&self, override_target: Option<NodeId>) -> (NodeId, Option<NodeId>) {
        let target = override_target.unwrap_or_else(|| self.current());
        if self.foster_parenting
            && self.is_any(target, &["table", "tbody", "tfoot", "thead", "tr"])
            && let Some(pos) = self.open.iter().rposition(|n| self.is(*n, "table"))
        {
            let table = self.open[pos];
            return match self.doc.node(table).parent {
                Some(parent) => (parent, Some(table)),
                None => (pos.checked_sub(1).map_or(self.doc.root(), |i| self.open[i]), None),
            };
        }
        (target, None)
    }

    fn create_element(&mut self, tag: &Tag, ns: Namespace) -> NodeId {
        self.doc.create_element(&tag.name, ns, tag.attrs.clone())
    }

    fn insert_element(&mut self, tag: &Tag) -> NodeId {
        self.insert_element_ns(tag, Namespace::Html)
    }

    fn insert_element_ns(&mut self, tag: &Tag, ns: Namespace) -> NodeId {
        let node = self.create_element(tag, ns);
        let (parent, before) = self.insertion_place(None);
        self.doc.insert_before(parent, node, before);
        self.open.push(node);
        node
    }

    fn insert_text(&mut self, text: &str) {
        let (parent, before) = self.insertion_place(None);
        if parent == self.doc.root() {
            return; // Un document ne peut pas avoir d'enfant texte.
        }
        self.doc.insert_text(parent, before, text);
    }

    fn insert_comment(&mut self, text: &str) {
        let node = self.doc.create_comment(text);
        let (parent, before) = self.insertion_place(None);
        self.doc.insert_before(parent, node, before);
    }

    fn insert_comment_in(&mut self, parent: NodeId, text: &str) {
        let node = self.doc.create_comment(text);
        self.doc.append_child(parent, node);
    }

    fn merge_attrs(&mut self, target: NodeId, tag: &Tag) {
        if let Some(e) = self.doc.element_mut(target) {
            for a in &tag.attrs {
                if !e.has_attr(&a.name) {
                    e.attrs.push(a.clone());
                }
            }
        }
    }

    fn parse_raw(&mut self, tag: &Tag, state: ContentState) {
        self.insert_element(tag);
        self.tok.set_content_state(state);
        self.original_mode = self.mode;
        self.mode = Mode::Text;
    }

    // ---- Éléments de mise en forme actifs ---------------------------------

    fn push_formatting(&mut self, node: NodeId, tag: Tag) {
        // Clause de l'arche de Noé : au plus trois entrées identiques après le dernier marqueur.
        let mut same = Vec::new();
        for (i, entry) in self.formatting.iter().enumerate().rev() {
            match entry {
                FormatEntry::Marker => break,
                FormatEntry::Element { tag: t, .. } => {
                    if t.name == tag.name && t.attrs.len() == tag.attrs.len() && {
                        t.attrs.iter().all(|a| tag.attr(&a.name) == Some(a.value.as_str()))
                    } {
                        same.push(i);
                    }
                }
            }
        }
        if same.len() >= 3
            && let Some(&earliest) = same.last()
        {
            self.formatting.remove(earliest);
        }
        self.formatting.push(FormatEntry::Element { node, tag });
    }

    fn formatting_index_of(&self, node: NodeId) -> Option<usize> {
        self.formatting
            .iter()
            .position(|e| matches!(e, FormatEntry::Element { node: n, .. } if *n == node))
    }

    fn remove_from_formatting(&mut self, node: NodeId) {
        if let Some(i) = self.formatting_index_of(node) {
            self.formatting.remove(i);
        }
    }

    fn remove_from_stack(&mut self, node: NodeId) {
        if let Some(i) = self.open.iter().rposition(|n| *n == node) {
            self.open.remove(i);
        }
    }

    fn clear_formatting_to_marker(&mut self) {
        while let Some(entry) = self.formatting.pop() {
            if matches!(entry, FormatEntry::Marker) {
                break;
            }
        }
    }

    fn reconstruct_formatting(&mut self) {
        let is_open = |this: &Self, e: &FormatEntry| match e {
            FormatEntry::Marker => true,
            FormatEntry::Element { node, .. } => this.open.contains(node),
        };
        let Some(last) = self.formatting.last() else { return };
        if is_open(self, last) {
            return;
        }
        let mut i = self.formatting.len() - 1;
        while i > 0 && !is_open(self, &self.formatting[i - 1]) {
            i -= 1;
        }
        for idx in i..self.formatting.len() {
            let FormatEntry::Element { tag, .. } = &self.formatting[idx] else { continue };
            let tag = tag.clone();
            let node = self.insert_element(&tag);
            self.formatting[idx] = FormatEntry::Element { node, tag };
        }
    }

    /// Dernier élément de mise en forme nommé `name` après le dernier marqueur.
    fn formatting_element(&self, name: &str) -> Option<(usize, NodeId)> {
        for (i, entry) in self.formatting.iter().enumerate().rev() {
            match entry {
                FormatEntry::Marker => return None,
                FormatEntry::Element { node, tag } if tag.name == name => return Some((i, *node)),
                FormatEntry::Element { .. } => {}
            }
        }
        None
    }

    /// Algorithme d'adoption (« adoption agency »), §13.2.6.4.7.
    fn adoption_agency(&mut self, subject: &str) {
        let current = self.current();
        if self.is(current, subject) && self.formatting_index_of(current).is_none() {
            self.pop();
            return;
        }
        for _ in 0..8 {
            let Some((fe_list_idx, fe)) = self.formatting_element(subject) else {
                self.any_other_end_tag(subject);
                return;
            };
            let Some(fe_stack_idx) = self.open.iter().rposition(|n| *n == fe) else {
                self.formatting.remove(fe_list_idx);
                return;
            };
            // L'élément doit être dans la portée par défaut.
            let in_scope = !self.open[fe_stack_idx + 1..]
                .iter()
                .any(|n| self.is_scope_boundary(*n, Scope::Default));
            if !in_scope {
                return;
            }
            let furthest = self.open[fe_stack_idx + 1..]
                .iter()
                .position(|n| self.is_any(*n, SPECIAL) || self.ns(*n) != Namespace::Html)
                .map(|off| fe_stack_idx + 1 + off);
            let Some(fb_stack_idx) = furthest else {
                self.open.truncate(fe_stack_idx);
                self.formatting.remove(fe_list_idx);
                return;
            };
            let furthest_block = self.open[fb_stack_idx];
            let common_ancestor =
                fe_stack_idx.checked_sub(1).map_or(self.doc.root(), |i| self.open[i]);
            let mut bookmark = fe_list_idx;
            let mut node_idx = fb_stack_idx;
            let mut last_node = furthest_block;
            let mut inner = 0u32;
            loop {
                inner += 1;
                node_idx -= 1; // INVARIANT : node_idx > fe_stack_idx avant décrément.
                let node = self.open[node_idx];
                if node == fe {
                    break;
                }
                let mut list_idx = self.formatting_index_of(node);
                if inner > 3
                    && let Some(i) = list_idx
                {
                    self.formatting.remove(i);
                    if i < bookmark {
                        bookmark -= 1;
                    }
                    list_idx = None;
                }
                let Some(list_idx) = list_idx else {
                    self.open.remove(node_idx);
                    continue;
                };
                let FormatEntry::Element { tag, .. } = &self.formatting[list_idx] else { continue };
                let tag = tag.clone();
                let new_node = self.create_element(&tag, Namespace::Html);
                self.formatting[list_idx] = FormatEntry::Element { node: new_node, tag };
                self.open[node_idx] = new_node;
                if last_node == furthest_block {
                    bookmark = list_idx + 1;
                }
                self.doc.append_child(new_node, last_node);
                last_node = new_node;
            }
            let (parent, before) = self.insertion_place(Some(common_ancestor));
            self.doc.insert_before(parent, last_node, before);

            let Some(fe_list_idx) = self.formatting_index_of(fe) else { return };
            let FormatEntry::Element { tag, .. } = &self.formatting[fe_list_idx] else { return };
            let tag = tag.clone();
            let new_fe = self.create_element(&tag, Namespace::Html);
            self.doc.reparent_children(furthest_block, new_fe);
            self.doc.append_child(furthest_block, new_fe);

            self.formatting.remove(fe_list_idx);
            if fe_list_idx < bookmark {
                bookmark -= 1;
            }
            let bookmark = bookmark.min(self.formatting.len());
            self.formatting.insert(bookmark, FormatEntry::Element { node: new_fe, tag });

            self.remove_from_stack(fe);
            if let Some(fb) = self.open.iter().rposition(|n| *n == furthest_block) {
                self.open.insert(fb + 1, new_fe);
            }
        }
    }

    fn any_other_end_tag(&mut self, name: &str) {
        for i in (0..self.open.len()).rev() {
            let node = self.open[i];
            if self.is(node, name) {
                self.generate_implied_end_tags(Some(name));
                self.open.truncate(i);
                return;
            }
            if self.is_any(node, SPECIAL) {
                return;
            }
        }
    }

    fn reset_insertion_mode(&mut self) {
        for (i, &node) in self.open.iter().enumerate().rev() {
            let last = i == 0;
            let Some(e) = self.doc.element(node) else { continue };
            if e.ns != Namespace::Html {
                continue;
            }
            self.mode = match e.name.as_str() {
                "select" => {
                    let in_table = self.open[..i].iter().any(|n| self.is(*n, "table"));
                    if in_table { Mode::InSelectInTable } else { Mode::InSelect }
                }
                "td" | "th" if !last => Mode::InCell,
                "tr" => Mode::InRow,
                "tbody" | "thead" | "tfoot" => Mode::InTableBody,
                "caption" => Mode::InCaption,
                "colgroup" => Mode::InColumnGroup,
                "table" => Mode::InTable,
                "head" if !last => Mode::InHead,
                "body" => Mode::InBody,
                "html" => {
                    if self.head.is_none() { Mode::BeforeHead } else { Mode::AfterHead }
                }
                _ if last => Mode::InBody,
                _ => continue,
            };
            return;
        }
        self.mode = Mode::InBody;
    }

    // ---- Aiguillage ------------------------------------------------------

    fn is_html_integration_point(&self, id: NodeId) -> bool {
        let Some(e) = self.doc.element(id) else { return false };
        match e.ns {
            Namespace::Svg => matches!(e.name.as_str(), "foreignObject" | "desc" | "title"),
            Namespace::MathMl => {
                e.name == "annotation-xml"
                    && e.attr("encoding").is_some_and(|v| {
                        v.eq_ignore_ascii_case("text/html")
                            || v.eq_ignore_ascii_case("application/xhtml+xml")
                    })
            }
            Namespace::Html => false,
        }
    }

    fn is_mathml_text_integration_point(&self, id: NodeId) -> bool {
        self.doc.element(id).is_some_and(|e| {
            e.ns == Namespace::MathMl && matches!(e.name.as_str(), "mi" | "mo" | "mn" | "ms" | "mtext")
        })
    }

    fn dispatch(&mut self, mut token: Token) {
        if std::mem::take(&mut self.skip_lf)
            && let Token::Characters(s) = &mut token
        {
            if s.starts_with('\n') {
                s.remove(0);
            }
            if s.is_empty() {
                return;
            }
        }
        let html_rules = match self.open.last() {
            None => true,
            Some(&cur) => {
                self.ns(cur) == Namespace::Html
                    || matches!(token, Token::Eof)
                    || (self.is_mathml_text_integration_point(cur)
                        && match &token {
                            Token::StartTag(t) => !matches!(t.name.as_str(), "mglyph" | "malignmark"),
                            Token::Characters(_) => true,
                            _ => false,
                        })
                    || (self.ns(cur) == Namespace::MathMl
                        && self.name(cur) == "annotation-xml"
                        && matches!(&token, Token::StartTag(t) if t.name == "svg"))
                    || (self.is_html_integration_point(cur)
                        && matches!(token, Token::StartTag(_) | Token::Characters(_)))
            }
        };
        if html_rules {
            self.process(self.mode, token);
        } else {
            self.process_foreign(token);
        }
    }

    fn process(&mut self, mode: Mode, token: Token) {
        match mode {
            Mode::Initial => self.mode_initial(token),
            Mode::BeforeHtml => self.mode_before_html(token),
            Mode::BeforeHead => self.mode_before_head(token),
            Mode::InHead => self.mode_in_head(token),
            Mode::InHeadNoscript => self.mode_in_head_noscript(token),
            Mode::AfterHead => self.mode_after_head(token),
            Mode::InBody => self.mode_in_body(token),
            Mode::Text => self.mode_text(token),
            Mode::InTable => self.mode_in_table(token),
            Mode::InCaption => self.mode_in_caption(token),
            Mode::InColumnGroup => self.mode_in_column_group(token),
            Mode::InTableBody => self.mode_in_table_body(token),
            Mode::InRow => self.mode_in_row(token),
            Mode::InCell => self.mode_in_cell(token),
            Mode::InSelect | Mode::InSelectInTable => self.mode_in_select(token),
            Mode::AfterBody => self.mode_after_body(token),
            Mode::AfterAfterBody => self.mode_after_after_body(token),
        }
    }

    /// Sépare les blancs de tête ; renvoie (blancs, reste éventuel).
    fn split_ws(s: &str) -> (&str, &str) {
        let rest = s.trim_start_matches(WS);
        s.split_at(s.len() - rest.len())
    }

    // ---- Modes d'insertion -----------------------------------------------

    fn mode_initial(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let (_, rest) = Self::split_ws(&s);
                if !rest.is_empty() {
                    self.doc.quirks_mode = QuirksMode::Quirks;
                    self.mode = Mode::BeforeHtml;
                    self.process(self.mode, Token::Characters(rest.to_owned()));
                }
            }
            Token::Comment(c) => self.insert_comment_in(self.doc.root(), &c),
            Token::Doctype { name, public_id, system_id, force_quirks } => {
                let n = name.clone().unwrap_or_default();
                let node = self.doc.create_doctype(
                    &n,
                    public_id.as_deref().unwrap_or(""),
                    system_id.as_deref().unwrap_or(""),
                );
                self.doc.append_child(self.doc.root(), node);
                self.doc.quirks_mode =
                    quirks_mode(&n, public_id.as_deref(), system_id.as_deref(), force_quirks);
                self.mode = Mode::BeforeHtml;
            }
            other => {
                self.doc.quirks_mode = QuirksMode::Quirks;
                self.mode = Mode::BeforeHtml;
                self.process(self.mode, other);
            }
        }
    }

    fn mode_before_html(&mut self, token: Token) {
        match token {
            Token::Doctype { .. } => {}
            Token::Comment(c) => self.insert_comment_in(self.doc.root(), &c),
            Token::Characters(s) => {
                let (_, rest) = Self::split_ws(&s);
                if !rest.is_empty() {
                    self.before_html_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::StartTag(tag) if tag.name == "html" => {
                let node = self.create_element(&tag, Namespace::Html);
                self.doc.append_child(self.doc.root(), node);
                self.open.push(node);
                self.mode = Mode::BeforeHead;
            }
            Token::EndTag(tag) if !matches!(tag.name.as_str(), "head" | "body" | "html" | "br") => {}
            other => self.before_html_else(other),
        }
    }

    fn before_html_else(&mut self, token: Token) {
        let node = self.create_element(&start_tag("html"), Namespace::Html);
        self.doc.append_child(self.doc.root(), node);
        self.open.push(node);
        self.mode = Mode::BeforeHead;
        self.process(self.mode, token);
    }

    fn mode_before_head(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let (_, rest) = Self::split_ws(&s);
                if !rest.is_empty() {
                    self.before_head_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::StartTag(tag) if tag.name == "html" => {
                self.mode_in_body(Token::StartTag(tag));
            }
            Token::StartTag(tag) if tag.name == "head" => {
                let node = self.insert_element(&tag);
                self.head = Some(node);
                self.mode = Mode::InHead;
            }
            Token::EndTag(tag) if !matches!(tag.name.as_str(), "head" | "body" | "html" | "br") => {}
            other => self.before_head_else(other),
        }
    }

    fn before_head_else(&mut self, token: Token) {
        let node = self.insert_element(&start_tag("head"));
        self.head = Some(node);
        self.mode = Mode::InHead;
        self.process(self.mode, token);
    }

    fn mode_in_head(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let (ws, rest) = Self::split_ws(&s);
                self.insert_text(ws);
                if !rest.is_empty() {
                    self.in_head_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::StartTag(tag) => match tag.name.as_str() {
                "html" => self.mode_in_body(Token::StartTag(tag)),
                "base" | "basefont" | "bgsound" | "link" | "meta" => {
                    self.insert_element(&tag);
                    self.pop();
                }
                "title" => self.parse_raw(&tag, ContentState::Rcdata),
                "noscript" => {
                    self.insert_element(&tag);
                    self.mode = Mode::InHeadNoscript;
                }
                "noframes" | "style" => self.parse_raw(&tag, ContentState::Rawtext),
                "script" => self.parse_raw(&tag, ContentState::ScriptData),
                "head" => {}
                _ => self.in_head_else(Token::StartTag(tag)),
            },
            Token::EndTag(tag) => match tag.name.as_str() {
                "head" => {
                    self.pop();
                    self.mode = Mode::AfterHead;
                }
                "body" | "html" | "br" => self.in_head_else(Token::EndTag(tag)),
                _ => {}
            },
            Token::Eof => self.in_head_else(Token::Eof),
        }
    }

    fn in_head_else(&mut self, token: Token) {
        self.pop();
        self.mode = Mode::AfterHead;
        self.process(self.mode, token);
    }

    fn mode_in_head_noscript(&mut self, token: Token) {
        match token {
            Token::Doctype { .. } => {}
            Token::Comment(_) => self.mode_in_head(token),
            Token::Characters(s) => {
                let (ws, rest) = Self::split_ws(&s);
                self.insert_text(ws);
                if !rest.is_empty() {
                    self.in_head_noscript_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::StartTag(tag) => match tag.name.as_str() {
                "html" => self.mode_in_body(Token::StartTag(tag)),
                "basefont" | "bgsound" | "link" | "meta" | "noframes" | "style" => {
                    self.mode_in_head(Token::StartTag(tag));
                }
                "head" | "noscript" => {}
                _ => self.in_head_noscript_else(Token::StartTag(tag)),
            },
            Token::EndTag(tag) => match tag.name.as_str() {
                "noscript" => {
                    self.pop();
                    self.mode = Mode::InHead;
                }
                "br" => self.in_head_noscript_else(Token::EndTag(tag)),
                _ => {}
            },
            Token::Eof => self.in_head_noscript_else(Token::Eof),
        }
    }

    fn in_head_noscript_else(&mut self, token: Token) {
        self.pop();
        self.mode = Mode::InHead;
        self.process(self.mode, token);
    }

    fn mode_after_head(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let (ws, rest) = Self::split_ws(&s);
                self.insert_text(ws);
                if !rest.is_empty() {
                    self.after_head_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::StartTag(tag) => match tag.name.as_str() {
                "html" => self.mode_in_body(Token::StartTag(tag)),
                "body" => {
                    self.insert_element(&tag);
                    self.mode = Mode::InBody;
                }
                "frameset" | "head" => {}
                "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script"
                | "style" | "title" => {
                    if let Some(head) = self.head {
                        self.open.push(head);
                        self.mode_in_head(Token::StartTag(tag));
                        self.remove_from_stack(head);
                    }
                }
                _ => self.after_head_else(Token::StartTag(tag)),
            },
            Token::EndTag(tag) => match tag.name.as_str() {
                "body" | "html" | "br" => self.after_head_else(Token::EndTag(tag)),
                _ => {}
            },
            Token::Eof => self.after_head_else(Token::Eof),
        }
    }

    fn after_head_else(&mut self, token: Token) {
        self.insert_element(&start_tag("body"));
        self.mode = Mode::InBody;
        self.process(self.mode, token);
    }

    fn mode_text(&mut self, token: Token) {
        match token {
            Token::Characters(s) => self.insert_text(&s),
            Token::Eof => {
                self.pop();
                self.mode = self.original_mode;
                self.process(self.mode, Token::Eof);
            }
            Token::EndTag(_) => {
                self.pop();
                self.mode = self.original_mode;
            }
            _ => {}
        }
    }

    fn mode_in_body(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let text: String = s.chars().filter(|c| *c != '\0').collect();
                if text.is_empty() {
                    return;
                }
                self.reconstruct_formatting();
                self.insert_text(&text);
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::Eof => self.done = true,
            Token::StartTag(tag) => self.in_body_start(tag),
            Token::EndTag(tag) => self.in_body_end(&tag),
        }
    }

    fn in_body_start(&mut self, tag: Tag) {
        let name = tag.name.clone();
        let name = name.as_str();
        match name {
            "html" => {
                if let Some(&html) = self.open.first() {
                    self.merge_attrs(html, &tag);
                }
            }
            "base" | "basefont" | "bgsound" | "link" | "meta" | "noframes" | "script" | "style"
            | "title" => self.mode_in_head(Token::StartTag(tag)),
            "body" => {
                if let Some(&body) = self.open.get(1)
                    && self.is(body, "body")
                {
                    self.merge_attrs(body, &tag);
                }
            }
            "frameset" => {}
            _ if BLOCK_START.contains(&name) => {
                self.close_p_if_in_button_scope();
                self.insert_element(&tag);
            }
            _ if HEADINGS.contains(&name) => {
                self.close_p_if_in_button_scope();
                if self.is_any(self.current(), HEADINGS) {
                    self.pop();
                }
                self.insert_element(&tag);
            }
            "pre" | "listing" => {
                self.close_p_if_in_button_scope();
                self.insert_element(&tag);
                self.skip_lf = true;
            }
            "form" => {
                if self.form.is_none() {
                    self.close_p_if_in_button_scope();
                    self.form = Some(self.insert_element(&tag));
                }
            }
            "li" | "dd" | "dt" => {
                let closers: &[&str] = if name == "li" { &["li"] } else { &["dd", "dt"] };
                for i in (0..self.open.len()).rev() {
                    let node = self.open[i];
                    if self.is_any(node, closers) {
                        let closed = self.name(node).to_owned();
                        self.generate_implied_end_tags(Some(&closed));
                        self.pop_until(&closed);
                        break;
                    }
                    if self.is_any(node, SPECIAL) && !self.is_any(node, &["address", "div", "p"]) {
                        break;
                    }
                }
                self.close_p_if_in_button_scope();
                self.insert_element(&tag);
            }
            "plaintext" => {
                self.close_p_if_in_button_scope();
                self.insert_element(&tag);
                self.tok.set_content_state(ContentState::Plaintext);
            }
            "button" => {
                if self.in_scope("button", Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until("button");
                }
                self.reconstruct_formatting();
                self.insert_element(&tag);
            }
            "a" => {
                if let Some((_, node)) = self.formatting_element("a") {
                    self.adoption_agency("a");
                    self.remove_from_formatting(node);
                    self.remove_from_stack(node);
                }
                self.reconstruct_formatting();
                let node = self.insert_element(&tag);
                self.push_formatting(node, tag);
            }
            "b" | "big" | "code" | "em" | "font" | "i" | "s" | "small" | "strike" | "strong"
            | "tt" | "u" => {
                self.reconstruct_formatting();
                let node = self.insert_element(&tag);
                self.push_formatting(node, tag);
            }
            "nobr" => {
                self.reconstruct_formatting();
                if self.in_scope("nobr", Scope::Default) {
                    self.adoption_agency("nobr");
                    self.reconstruct_formatting();
                }
                let node = self.insert_element(&tag);
                self.push_formatting(node, tag);
            }
            "applet" | "marquee" | "object" => {
                self.reconstruct_formatting();
                self.insert_element(&tag);
                self.formatting.push(FormatEntry::Marker);
            }
            "table" => {
                if self.doc.quirks_mode != QuirksMode::Quirks {
                    self.close_p_if_in_button_scope();
                }
                self.insert_element(&tag);
                self.mode = Mode::InTable;
            }
            "area" | "br" | "embed" | "img" | "keygen" | "wbr" | "input" => {
                self.reconstruct_formatting();
                self.insert_element(&tag);
                self.pop();
            }
            "param" | "source" | "track" => {
                self.insert_element(&tag);
                self.pop();
            }
            "hr" => {
                self.close_p_if_in_button_scope();
                self.insert_element(&tag);
                self.pop();
            }
            "image" => {
                let mut tag = tag;
                tag.name = "img".to_owned();
                self.in_body_start(tag);
            }
            "textarea" => {
                self.parse_raw(&tag, ContentState::Rcdata);
                self.skip_lf = true;
            }
            "xmp" => {
                self.close_p_if_in_button_scope();
                self.reconstruct_formatting();
                self.parse_raw(&tag, ContentState::Rawtext);
            }
            "iframe" | "noembed" => self.parse_raw(&tag, ContentState::Rawtext),
            "select" => {
                self.reconstruct_formatting();
                self.insert_element(&tag);
                self.mode = match self.mode {
                    Mode::InTable | Mode::InCaption | Mode::InTableBody | Mode::InRow
                    | Mode::InCell => Mode::InSelectInTable,
                    _ => Mode::InSelect,
                };
            }
            "optgroup" | "option" => {
                if self.current_is("option") {
                    self.pop();
                }
                self.reconstruct_formatting();
                self.insert_element(&tag);
            }
            "rb" | "rtc" => {
                if self.in_scope("ruby", Scope::Default) {
                    self.generate_implied_end_tags(None);
                }
                self.insert_element(&tag);
            }
            "rp" | "rt" => {
                if self.in_scope("ruby", Scope::Default) {
                    self.generate_implied_end_tags(Some("rtc"));
                }
                self.insert_element(&tag);
            }
            "math" | "svg" => {
                self.reconstruct_formatting();
                let ns = if name == "svg" { Namespace::Svg } else { Namespace::MathMl };
                self.insert_element_ns(&tag, ns);
                if tag.self_closing {
                    self.pop();
                }
            }
            "caption" | "col" | "colgroup" | "frame" | "head" | "tbody" | "td" | "tfoot" | "th"
            | "thead" | "tr" => {}
            _ => {
                self.reconstruct_formatting();
                self.insert_element(&tag);
            }
        }
    }

    fn in_body_end(&mut self, tag: &Tag) {
        let name = tag.name.as_str();
        match name {
            "body" => {
                if self.in_scope("body", Scope::Default) {
                    self.mode = Mode::AfterBody;
                }
            }
            "html" => {
                if self.in_scope("body", Scope::Default) {
                    self.mode = Mode::AfterBody;
                    self.process(self.mode, end_tag("html"));
                }
            }
            _ if BLOCK_END.contains(&name) => {
                if self.in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(name);
                }
            }
            "form" => {
                if let Some(node) = self.form.take()
                    && self.open.contains(&node)
                    && self.in_scope("form", Scope::Default)
                {
                    self.generate_implied_end_tags(None);
                    self.remove_from_stack(node);
                }
            }
            "p" => {
                if !self.in_scope("p", Scope::Button) {
                    self.insert_element(&start_tag("p"));
                }
                self.close_p();
            }
            "li" => {
                if self.in_scope("li", Scope::ListItem) {
                    self.generate_implied_end_tags(Some("li"));
                    self.pop_until("li");
                }
            }
            "dd" | "dt" => {
                if self.in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(Some(name));
                    self.pop_until(name);
                }
            }
            _ if HEADINGS.contains(&name) => {
                if self.in_scope_any(HEADINGS, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until_any(HEADINGS);
                }
            }
            _ if FORMATTING.contains(&name) => self.adoption_agency(name),
            "applet" | "marquee" | "object" => {
                if self.in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(name);
                    self.clear_formatting_to_marker();
                }
            }
            "br" => self.in_body_start(start_tag("br")),
            _ => self.any_other_end_tag(name),
        }
    }

    fn mode_in_table(&mut self, token: Token) {
        match token {
            Token::Characters(s)
                if self.is_any(self.current(), &["table", "tbody", "tfoot", "thead", "tr"]) =>
            {
                if s.chars().all(|c| WS.contains(&c)) {
                    self.insert_text(&s);
                } else {
                    self.in_table_else(Token::Characters(s));
                }
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::StartTag(tag) => match tag.name.as_str() {
                "caption" => {
                    self.clear_stack_to(&["table"]);
                    self.formatting.push(FormatEntry::Marker);
                    self.insert_element(&tag);
                    self.mode = Mode::InCaption;
                }
                "colgroup" => {
                    self.clear_stack_to(&["table"]);
                    self.insert_element(&tag);
                    self.mode = Mode::InColumnGroup;
                }
                "col" => {
                    self.clear_stack_to(&["table"]);
                    self.insert_element(&start_tag("colgroup"));
                    self.mode = Mode::InColumnGroup;
                    self.process(self.mode, Token::StartTag(tag));
                }
                "tbody" | "tfoot" | "thead" => {
                    self.clear_stack_to(&["table"]);
                    self.insert_element(&tag);
                    self.mode = Mode::InTableBody;
                }
                "td" | "th" | "tr" => {
                    self.clear_stack_to(&["table"]);
                    self.insert_element(&start_tag("tbody"));
                    self.mode = Mode::InTableBody;
                    self.process(self.mode, Token::StartTag(tag));
                }
                "table" => {
                    if self.in_scope("table", Scope::Table) {
                        self.pop_until("table");
                        self.reset_insertion_mode();
                        self.process(self.mode, Token::StartTag(tag));
                    }
                }
                "style" | "script" => self.mode_in_head(Token::StartTag(tag)),
                "input" if tag.attr("type").is_some_and(|t| t.eq_ignore_ascii_case("hidden")) => {
                    self.insert_element(&tag);
                    self.pop();
                }
                "form" => {
                    if self.form.is_none() {
                        self.form = Some(self.insert_element(&tag));
                        self.pop();
                    }
                }
                _ => self.in_table_else(Token::StartTag(tag)),
            },
            Token::EndTag(tag) => match tag.name.as_str() {
                "table" => {
                    if self.in_scope("table", Scope::Table) {
                        self.pop_until("table");
                        self.reset_insertion_mode();
                    }
                }
                "body" | "caption" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot"
                | "th" | "thead" | "tr" => {}
                _ => self.in_table_else(Token::EndTag(tag)),
            },
            Token::Eof => self.mode_in_body(Token::Eof),
            other @ Token::Characters(_) => self.mode_in_body(other),
        }
    }

    /// « Anything else » du mode tableau : règles du corps avec adoption nourricière.
    fn in_table_else(&mut self, token: Token) {
        self.foster_parenting = true;
        self.mode_in_body(token);
        self.foster_parenting = false;
    }

    fn close_caption(&mut self) -> bool {
        if !self.in_scope("caption", Scope::Table) {
            return false;
        }
        self.generate_implied_end_tags(None);
        self.pop_until("caption");
        self.clear_formatting_to_marker();
        self.mode = Mode::InTable;
        true
    }

    fn mode_in_caption(&mut self, token: Token) {
        match &token {
            Token::EndTag(tag) if tag.name == "caption" => {
                self.close_caption();
            }
            Token::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead"
                        | "tr"
                ) =>
            {
                if self.close_caption() {
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag) if tag.name == "table" => {
                if self.close_caption() {
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "col" | "colgroup" | "html" | "tbody" | "td" | "tfoot" | "th"
                        | "thead" | "tr"
                ) => {}
            _ => self.mode_in_body(token),
        }
    }

    fn mode_in_column_group(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let (ws, rest) = Self::split_ws(&s);
                self.insert_text(ws);
                if !rest.is_empty() {
                    self.in_column_group_else(Token::Characters(rest.to_owned()));
                }
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::StartTag(tag) if tag.name == "html" => self.mode_in_body(Token::StartTag(tag)),
            Token::StartTag(tag) if tag.name == "col" => {
                self.insert_element(&tag);
                self.pop();
            }
            Token::EndTag(tag) if tag.name == "colgroup" => {
                if self.current_is("colgroup") {
                    self.pop();
                    self.mode = Mode::InTable;
                }
            }
            Token::EndTag(tag) if tag.name == "col" => {}
            Token::Eof => self.mode_in_body(Token::Eof),
            other => self.in_column_group_else(other),
        }
    }

    fn in_column_group_else(&mut self, token: Token) {
        if self.current_is("colgroup") {
            self.pop();
            self.mode = Mode::InTable;
            self.process(self.mode, token);
        }
    }

    fn mode_in_table_body(&mut self, token: Token) {
        const BODY_CTX: &[&str] = &["tbody", "tfoot", "thead"];
        match &token {
            Token::StartTag(tag) if tag.name == "tr" => {
                self.clear_stack_to(BODY_CTX);
                self.insert_element(tag);
                self.mode = Mode::InRow;
            }
            Token::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_stack_to(BODY_CTX);
                self.insert_element(&start_tag("tr"));
                self.mode = Mode::InRow;
                self.process(self.mode, token);
            }
            Token::EndTag(tag) if BODY_CTX.contains(&tag.name.as_str()) => {
                if self.in_scope(&tag.name, Scope::Table) {
                    self.clear_stack_to(BODY_CTX);
                    self.pop();
                    self.mode = Mode::InTable;
                }
            }
            Token::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead"
                ) =>
            {
                self.close_table_body(token);
            }
            Token::EndTag(tag) if tag.name == "table" => self.close_table_body(token),
            Token::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th" | "tr"
                ) => {}
            _ => self.mode_in_table(token),
        }
    }

    fn close_table_body(&mut self, token: Token) {
        const BODY_CTX: &[&str] = &["tbody", "tfoot", "thead"];
        if self.in_scope_any(BODY_CTX, Scope::Table) {
            self.clear_stack_to(BODY_CTX);
            self.pop();
            self.mode = Mode::InTable;
            self.process(self.mode, token);
        }
    }

    fn close_row(&mut self) -> bool {
        if !self.in_scope("tr", Scope::Table) {
            return false;
        }
        self.clear_stack_to(&["tr"]);
        self.pop();
        self.mode = Mode::InTableBody;
        true
    }

    fn mode_in_row(&mut self, token: Token) {
        match &token {
            Token::StartTag(tag) if matches!(tag.name.as_str(), "th" | "td") => {
                self.clear_stack_to(&["tr"]);
                self.insert_element(tag);
                self.mode = Mode::InCell;
                self.formatting.push(FormatEntry::Marker);
            }
            Token::EndTag(tag) if tag.name == "tr" => {
                self.close_row();
            }
            Token::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "tfoot" | "thead" | "tr"
                ) =>
            {
                if self.close_row() {
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag) if tag.name == "table" => {
                if self.close_row() {
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag) if matches!(tag.name.as_str(), "tbody" | "tfoot" | "thead") => {
                if self.in_scope(&tag.name, Scope::Table) && self.close_row() {
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "body" | "caption" | "col" | "colgroup" | "html" | "td" | "th"
                ) => {}
            _ => self.mode_in_table(token),
        }
    }

    fn close_cell(&mut self) {
        self.generate_implied_end_tags(None);
        self.pop_until_any(&["td", "th"]);
        self.clear_formatting_to_marker();
        self.mode = Mode::InRow;
    }

    fn mode_in_cell(&mut self, token: Token) {
        match &token {
            Token::EndTag(tag) if matches!(tag.name.as_str(), "td" | "th") => {
                if self.in_scope(&tag.name, Scope::Table) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(&tag.name);
                    self.clear_formatting_to_marker();
                    self.mode = Mode::InRow;
                }
            }
            Token::StartTag(tag)
                if matches!(
                    tag.name.as_str(),
                    "caption" | "col" | "colgroup" | "tbody" | "td" | "tfoot" | "th" | "thead"
                        | "tr"
                ) =>
            {
                if self.in_scope_any(&["td", "th"], Scope::Table) {
                    self.close_cell();
                    self.process(self.mode, token);
                }
            }
            Token::EndTag(tag)
                if matches!(tag.name.as_str(), "body" | "caption" | "col" | "colgroup" | "html") => {}
            Token::EndTag(tag)
                if matches!(tag.name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") =>
            {
                if self.in_scope(&tag.name, Scope::Table) {
                    self.close_cell();
                    self.process(self.mode, token);
                }
            }
            _ => self.mode_in_body(token),
        }
    }

    fn close_select(&mut self) -> bool {
        if !self.in_scope("select", Scope::Select) {
            return false;
        }
        self.pop_until("select");
        self.reset_insertion_mode();
        true
    }

    fn mode_in_select(&mut self, token: Token) {
        const TABLE_PARTS: &[&str] = &["caption", "table", "tbody", "tfoot", "thead", "tr", "td", "th"];
        let in_table = self.mode == Mode::InSelectInTable;
        match token {
            Token::Characters(s) => {
                let text: String = s.chars().filter(|c| *c != '\0').collect();
                self.insert_text(&text);
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::Eof => self.mode_in_body(Token::Eof),
            Token::StartTag(tag) => match tag.name.as_str() {
                n if in_table && TABLE_PARTS.contains(&n) => {
                    self.pop_until("select");
                    self.reset_insertion_mode();
                    self.process(self.mode, Token::StartTag(tag));
                }
                "html" => self.mode_in_body(Token::StartTag(tag)),
                "option" => {
                    if self.current_is("option") {
                        self.pop();
                    }
                    self.insert_element(&tag);
                }
                "optgroup" | "hr" => {
                    if self.current_is("option") {
                        self.pop();
                    }
                    if self.current_is("optgroup") {
                        self.pop();
                    }
                    self.insert_element(&tag);
                    if tag.name == "hr" {
                        self.pop();
                    }
                }
                "select" => {
                    self.close_select();
                }
                "input" | "keygen" | "textarea" => {
                    if self.close_select() {
                        self.process(self.mode, Token::StartTag(tag));
                    }
                }
                "script" => self.mode_in_head(Token::StartTag(tag)),
                _ => {}
            },
            Token::EndTag(tag) => match tag.name.as_str() {
                n if in_table && TABLE_PARTS.contains(&n) => {
                    if self.in_scope(n, Scope::Table) {
                        self.pop_until("select");
                        self.reset_insertion_mode();
                        self.process(self.mode, Token::EndTag(tag));
                    }
                }
                "optgroup" => {
                    if self.current_is("option")
                        && self.open.len() >= 2
                        && self.is(self.open[self.open.len() - 2], "optgroup")
                    {
                        self.pop();
                    }
                    if self.current_is("optgroup") {
                        self.pop();
                    }
                }
                "option" => {
                    if self.current_is("option") {
                        self.pop();
                    }
                }
                "select" => {
                    self.close_select();
                }
                _ => {}
            },
        }
    }

    fn mode_after_body(&mut self, token: Token) {
        match token {
            Token::Characters(s) if s.chars().all(|c| WS.contains(&c)) => {
                self.mode_in_body(Token::Characters(s));
            }
            Token::Comment(c) => {
                let html = self.open.first().copied().unwrap_or_else(|| self.doc.root());
                self.insert_comment_in(html, &c);
            }
            Token::Doctype { .. } => {}
            Token::StartTag(tag) if tag.name == "html" => self.mode_in_body(Token::StartTag(tag)),
            Token::EndTag(tag) if tag.name == "html" => self.mode = Mode::AfterAfterBody,
            Token::Eof => self.done = true,
            other => {
                self.mode = Mode::InBody;
                self.process(self.mode, other);
            }
        }
    }

    fn mode_after_after_body(&mut self, token: Token) {
        match token {
            Token::Comment(c) => self.insert_comment_in(self.doc.root(), &c),
            Token::Doctype { .. } => {}
            Token::Characters(s) if s.chars().all(|c| WS.contains(&c)) => {
                self.mode_in_body(Token::Characters(s));
            }
            Token::StartTag(tag) if tag.name == "html" => self.mode_in_body(Token::StartTag(tag)),
            Token::Eof => self.done = true,
            other => {
                self.mode = Mode::InBody;
                self.process(self.mode, other);
            }
        }
    }

    // ---- Contenu étranger (SVG, MathML) ------------------------------------

    fn process_foreign(&mut self, token: Token) {
        match token {
            Token::Characters(s) => {
                let text: String =
                    s.chars().map(|c| if c == '\0' { '\u{fffd}' } else { c }).collect();
                self.insert_text(&text);
            }
            Token::Comment(c) => self.insert_comment(&c),
            Token::Doctype { .. } => {}
            Token::Eof => self.process(self.mode, Token::Eof),
            Token::StartTag(tag) => {
                let breakout = FOREIGN_BREAKOUT.contains(&tag.name.as_str())
                    || (tag.name == "font"
                        && tag.attrs.iter().any(|a| matches!(a.name.as_str(), "color" | "face" | "size")));
                if breakout {
                    self.pop_foreign();
                    self.process(self.mode, Token::StartTag(tag));
                    return;
                }
                let ns = self.ns(self.current());
                let mut tag = tag;
                if ns == Namespace::Svg
                    && let Some((_, fixed)) = SVG_TAG_CASE.iter().find(|(l, _)| *l == tag.name)
                {
                    tag.name = (*fixed).to_owned();
                }
                self.insert_element_ns(&tag, ns);
                if tag.self_closing {
                    self.pop();
                }
            }
            Token::EndTag(tag) => {
                if matches!(tag.name.as_str(), "br" | "p") {
                    self.pop_foreign();
                    self.process(self.mode, Token::EndTag(tag));
                    return;
                }
                for i in (0..self.open.len()).rev() {
                    let node = self.open[i];
                    if self.ns(node) == Namespace::Html {
                        self.process(self.mode, Token::EndTag(tag));
                        return;
                    }
                    if self.name(node).eq_ignore_ascii_case(&tag.name) {
                        self.open.truncate(i);
                        return;
                    }
                }
            }
        }
    }

    fn pop_foreign(&mut self) {
        while let Some(&cur) = self.open.last() {
            if self.ns(cur) == Namespace::Html
                || self.is_html_integration_point(cur)
                || self.is_mathml_text_integration_point(cur)
            {
                break;
            }
            self.open.pop();
        }
    }
}

fn quirks_mode(
    name: &str,
    public_id: Option<&str>,
    system_id: Option<&str>,
    force_quirks: bool,
) -> QuirksMode {
    const QUIRKY_PREFIXES: &[&str] = &[
        "-//ietf//dtd html",
        "-//w3c//dtd html 3",
        "-//w3c//dtd html 4.0 transitional//",
        "-//w3c//dtd html 4.0 frameset//",
        "-//w3c//dtd w3 html",
        "-//netscape comm. corp.//dtd",
        "-//microsoft//dtd internet explorer",
        "-//w3o//dtd w3 html",
        "+//silmaril//dtd html pro v0r11 19970101//",
    ];
    const LIMITED_PREFIXES: &[&str] =
        &["-//w3c//dtd xhtml 1.0 frameset//", "-//w3c//dtd xhtml 1.0 transitional//"];
    const HTML401_LOOSE: &[&str] =
        &["-//w3c//dtd html 4.01 frameset//", "-//w3c//dtd html 4.01 transitional//"];

    if force_quirks || name != "html" {
        return QuirksMode::Quirks;
    }
    let public = public_id.unwrap_or("").to_ascii_lowercase();
    if public == "html"
        || public == "-//w3o//dtd w3 html strict 3.0//en//"
        || public == "-/w3c/dtd html 4.0 transitional/en"
        || system_id.is_some_and(|s| {
            s.eq_ignore_ascii_case("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd")
        })
        || QUIRKY_PREFIXES.iter().any(|p| public.starts_with(p))
    {
        return QuirksMode::Quirks;
    }
    let loose = HTML401_LOOSE.iter().any(|p| public.starts_with(p));
    if loose && system_id.is_none() {
        return QuirksMode::Quirks;
    }
    if loose || LIMITED_PREFIXES.iter().any(|p| public.starts_with(p)) {
        return QuirksMode::LimitedQuirks;
    }
    QuirksMode::NoQuirks
}

/// Rend un attribut « nom=valeur » ; utile aux tests.
#[cfg(test)]
fn attr(name: &str, value: &str) -> Attribute {
    Attribute { name: name.into(), value: value.into() }
}

#[cfg(not(test))]
#[allow(dead_code, reason = "garde l'import `Attribute` vivant hors tests")]
fn _attr_type_check(a: Attribute) -> Attribute {
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dump(src: &str) -> String {
        parse_document(src).dump()
    }

    #[test]
    fn minimal_document() {
        assert_eq!(
            dump("<!DOCTYPE html><title>T</title><p>Bonjour"),
            "| <!DOCTYPE html>\n| <html>\n|   <head>\n|     <title>\n|       \"T\"\n|   <body>\n|     <p>\n|       \"Bonjour\"\n"
        );
    }

    #[test]
    fn empty_input_builds_skeleton() {
        assert_eq!(dump(""), "| <html>\n|   <head>\n|   <body>\n");
        assert_eq!(parse_document("").quirks_mode, QuirksMode::Quirks);
        assert_eq!(parse_document("<!doctype html>").quirks_mode, QuirksMode::NoQuirks);
    }

    #[test]
    fn implied_p_and_li_close() {
        assert_eq!(
            dump("<p>a<p>b<ul><li>1<li>2</ul>"),
            "| <html>\n|   <head>\n|   <body>\n|     <p>\n|       \"a\"\n|     <p>\n|       \"b\"\n|     <ul>\n|       <li>\n|         \"1\"\n|       <li>\n|         \"2\"\n"
        );
    }

    #[test]
    fn adoption_agency_misnested() {
        // Exemple du standard : <b>1<p>2</b>3</p>
        assert_eq!(
            dump("<b>1<p>2</b>3</p>"),
            "| <html>\n|   <head>\n|   <body>\n|     <b>\n|       \"1\"\n|     <p>\n|       <b>\n|         \"2\"\n|       \"3\"\n"
        );
        // <a> imbriqués : le premier est fermé avant d'ouvrir le second.
        assert_eq!(
            dump("<a>1<a>2"),
            "| <html>\n|   <head>\n|   <body>\n|     <a>\n|       \"1\"\n|     <a>\n|       \"2\"\n"
        );
    }

    #[test]
    fn formatting_reconstruction() {
        assert_eq!(
            dump("<p><b>a<p>b"),
            "| <html>\n|   <head>\n|   <body>\n|     <p>\n|       <b>\n|         \"a\"\n|     <p>\n|       <b>\n|         \"b\"\n"
        );
    }

    #[test]
    fn table_structure_and_foster_parenting() {
        assert_eq!(
            dump("<table><tr><td>1<td>2</table>"),
            "| <html>\n|   <head>\n|   <body>\n|     <table>\n|       <tbody>\n|         <tr>\n|           <td>\n|             \"1\"\n|           <td>\n|             \"2\"\n"
        );
        assert_eq!(
            dump("<table>x<tr><td>1</table>"),
            "| <html>\n|   <head>\n|   <body>\n|     \"x\"\n|     <table>\n|       <tbody>\n|         <tr>\n|           <td>\n|             \"1\"\n"
        );
    }

    #[test]
    fn raw_text_elements() {
        assert_eq!(
            dump("<style>p > a { }</style><script>if (a<b) x()</script><textarea>\n<b></textarea>"),
            "| <html>\n|   <head>\n|     <style>\n|       \"p > a { }\"\n|     <script>\n|       \"if (a<b) x()\"\n|   <body>\n|     <textarea>\n|       \"<b>\"\n"
        );
    }

    #[test]
    fn foreign_content() {
        assert_eq!(
            dump("<svg><clippath/><foreignObject><p>x</p></foreignObject></svg><p>y"),
            "| <html>\n|   <head>\n|   <body>\n|     <svg svg>\n|       <svg clipPath>\n|       <svg foreignObject>\n|         <p>\n|           \"x\"\n|     <p>\n|       \"y\"\n"
        );
    }

    #[test]
    fn attributes_and_merge() {
        let doc = parse_document("<html lang=fr><body class=a><html dir=ltr lang=en>");
        let html = doc.find_html_element(doc.root(), "html").expect("élément html");
        let e = doc.element(html).expect("élément");
        assert_eq!(e.attrs, vec![attr("lang", "fr"), attr("dir", "ltr")]);
    }

    #[test]
    fn select_and_after_body() {
        assert_eq!(
            dump("<select><option>a<option>b</select></body></html><!-- fin -->"),
            "| <html>\n|   <head>\n|   <body>\n|     <select>\n|       <option>\n|         \"a\"\n|       <option>\n|         \"b\"\n| <!--  fin  -->\n"
        );
    }

    #[test]
    fn never_panics_on_truncated_or_hostile_input() {
        let src = "<!DOCTYPE html><table><caption><b>x<tr><td><select><option><table></b><svg><p><math><mi><a><a></table></p></caption><template><frameset>&amp;</html>z";
        let chars: Vec<char> = src.chars().collect();
        for n in 0..=chars.len() {
            let prefix: String = chars[..n].iter().collect();
            let _ = parse_document(&prefix);
        }
        let _ = parse_document(&"</b>".repeat(200));
        let _ = parse_document(&"<b><p>".repeat(200));
    }
}
