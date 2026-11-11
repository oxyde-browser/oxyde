//! Arbre DOM d'Oxyde.
//!
//! Le document possède tous ses nœuds dans une arène (`Vec<Node>`) ; les liens
//! parent / enfants / frères sont des [`NodeId`]. Ce choix évite les cycles de
//! `Rc<RefCell<_>>`, garde les nœuds contigus en mémoire et rend l'arbre
//! trivialement `Send`.
//!
//! INVARIANT : tout `NodeId` stocké dans un nœud d'un [`Document`] est un index
//! valide de l'arène de ce même document. Les nœuds ne sont jamais retirés de
//! l'arène (seulement détachés de l'arbre), donc un `NodeId` émis par un
//! document reste valide pendant toute la vie de celui-ci.

use std::fmt;

/// Identifiant d'un nœud dans l'arène de son [`Document`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

impl NodeId {
    /// Index du nœud dans l'arène.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Espace de noms d'un élément.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// `http://www.w3.org/1999/xhtml`
    Html,
    /// `http://www.w3.org/2000/svg`
    Svg,
    /// `http://www.w3.org/1998/Math/MathML`
    MathMl,
}

/// Un attribut d'élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// Nom (en minuscules pour les éléments HTML).
    pub name: String,
    /// Valeur.
    pub value: String,
}

/// Données propres à un élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementData {
    /// Nom local (`div`, `p`, …).
    pub name: String,
    /// Espace de noms.
    pub ns: Namespace,
    /// Attributs, dans l'ordre du source, sans doublon de nom.
    pub attrs: Vec<Attribute>,
}

impl ElementData {
    /// Valeur de l'attribut `name`, s'il existe.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Vrai si l'attribut `name` est présent.
    #[must_use]
    pub fn has_attr(&self, name: &str) -> bool {
        self.attrs.iter().any(|a| a.name == name)
    }

    /// Valeur de l'attribut `id`.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }

    /// Itère sur les classes de l'attribut `class`.
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.attr("class").unwrap_or("").split_ascii_whitespace()
    }

    /// Vrai si l'élément est l'élément HTML `name`.
    #[must_use]
    pub fn is_html(&self, name: &str) -> bool {
        self.ns == Namespace::Html && self.name == name
    }
}

/// Contenu d'un nœud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeData {
    /// Racine du document.
    Document,
    /// `<!DOCTYPE …>`
    Doctype {
        /// Nom du type de document.
        name: String,
        /// Identifiant public.
        public_id: String,
        /// Identifiant système.
        system_id: String,
    },
    /// Élément.
    Element(ElementData),
    /// Nœud texte.
    Text(String),
    /// Commentaire.
    Comment(String),
}

/// Un nœud de l'arbre et ses liens.
#[derive(Clone, Debug)]
pub struct Node {
    /// Parent, `None` pour la racine ou un nœud détaché.
    pub parent: Option<NodeId>,
    /// Premier enfant.
    pub first_child: Option<NodeId>,
    /// Dernier enfant.
    pub last_child: Option<NodeId>,
    /// Frère précédent.
    pub prev_sibling: Option<NodeId>,
    /// Frère suivant.
    pub next_sibling: Option<NodeId>,
    /// Contenu.
    pub data: NodeData,
}

/// Mode de compatibilité du document (déterminé par le DOCTYPE).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QuirksMode {
    /// Mode standard.
    #[default]
    NoQuirks,
    /// Mode quasi standard.
    LimitedQuirks,
    /// Mode de compatibilité.
    Quirks,
}

/// Un document et l'arène de ses nœuds.
#[derive(Clone, Debug)]
pub struct Document {
    nodes: Vec<Node>,
    /// Mode de compatibilité.
    pub quirks_mode: QuirksMode,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// Crée un document vide (seulement le nœud racine).
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: vec![Node {
                parent: None,
                first_child: None,
                last_child: None,
                prev_sibling: None,
                next_sibling: None,
                data: NodeData::Document,
            }],
            quirks_mode: QuirksMode::NoQuirks,
        }
    }

    /// Nœud racine du document.
    #[must_use]
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Identifiant du nœud d'index `index` dans l'arène, s'il existe.
    #[must_use]
    pub fn id_from_index(&self, index: usize) -> Option<NodeId> {
        (index < self.nodes.len())
            .then(|| u32::try_from(index).ok().map(NodeId))
            .flatten()
    }

    /// Nombre de nœuds dans l'arène (attachés ou non).
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Toujours faux : un document contient au moins sa racine.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Accès à un nœud.
    ///
    /// # Panics
    /// Si `id` provient d'un autre document plus grand (violation de l'invariant du module).
    #[must_use]
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.index()]
    }

    /// Données d'élément du nœud, si c'en est un.
    #[must_use]
    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match &self.node(id).data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    /// Données d'élément modifiables.
    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.node_mut(id).data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    /// Texte du nœud, si c'est un nœud texte.
    #[must_use]
    pub fn text(&self, id: NodeId) -> Option<&str> {
        match &self.node(id).data {
            NodeData::Text(t) => Some(t),
            _ => None,
        }
    }

    fn push(&mut self, data: NodeData) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).expect("moins de 2^32 nœuds par document"));
        self.nodes.push(Node {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            data,
        });
        id
    }

    /// Crée un élément détaché.
    pub fn create_element(&mut self, name: &str, ns: Namespace, attrs: Vec<Attribute>) -> NodeId {
        self.push(NodeData::Element(ElementData {
            name: name.to_owned(),
            ns,
            attrs,
        }))
    }

    /// Crée un nœud texte détaché.
    pub fn create_text(&mut self, text: &str) -> NodeId {
        self.push(NodeData::Text(text.to_owned()))
    }

    /// Crée un commentaire détaché.
    pub fn create_comment(&mut self, text: &str) -> NodeId {
        self.push(NodeData::Comment(text.to_owned()))
    }

    /// Crée un nœud DOCTYPE détaché.
    pub fn create_doctype(&mut self, name: &str, public_id: &str, system_id: &str) -> NodeId {
        self.push(NodeData::Doctype {
            name: name.to_owned(),
            public_id: public_id.to_owned(),
            system_id: system_id.to_owned(),
        })
    }

    /// Détache `child` de son parent (sans effet s'il est déjà détaché).
    pub fn detach(&mut self, child: NodeId) {
        let (parent, prev, next) = {
            let n = self.node(child);
            (n.parent, n.prev_sibling, n.next_sibling)
        };
        let Some(parent) = parent else { return };
        match prev {
            Some(p) => self.node_mut(p).next_sibling = next,
            None => self.node_mut(parent).first_child = next,
        }
        match next {
            Some(n) => self.node_mut(n).prev_sibling = prev,
            None => self.node_mut(parent).last_child = prev,
        }
        let n = self.node_mut(child);
        n.parent = None;
        n.prev_sibling = None;
        n.next_sibling = None;
    }

    /// Ajoute `child` comme dernier enfant de `parent` (en le détachant d'abord).
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.insert_before(parent, child, None);
    }

    /// Insère `child` dans `parent` avant `reference` (à la fin si `None`).
    ///
    /// `reference`, s'il est fourni, doit être un enfant de `parent` ; sinon
    /// l'insertion se fait à la fin.
    pub fn insert_before(&mut self, parent: NodeId, child: NodeId, reference: Option<NodeId>) {
        if child == parent || reference == Some(child) {
            return;
        }
        self.detach(child);
        let reference = reference.filter(|r| self.node(*r).parent == Some(parent));
        match reference {
            None => {
                let last = self.node(parent).last_child;
                self.node_mut(child).prev_sibling = last;
                match last {
                    Some(l) => self.node_mut(l).next_sibling = Some(child),
                    None => self.node_mut(parent).first_child = Some(child),
                }
                self.node_mut(parent).last_child = Some(child);
            }
            Some(r) => {
                let prev = self.node(r).prev_sibling;
                {
                    let c = self.node_mut(child);
                    c.prev_sibling = prev;
                    c.next_sibling = Some(r);
                }
                self.node_mut(r).prev_sibling = Some(child);
                match prev {
                    Some(p) => self.node_mut(p).next_sibling = Some(child),
                    None => self.node_mut(parent).first_child = Some(child),
                }
            }
        }
        self.node_mut(child).parent = Some(parent);
    }

    /// Ajoute du texte dans `parent` avant `reference`, en fusionnant avec le
    /// nœud texte adjacent s'il existe (comportement de l'analyseur HTML).
    pub fn insert_text(&mut self, parent: NodeId, reference: Option<NodeId>, text: &str) {
        if text.is_empty() {
            return;
        }
        let adjacent = match reference.filter(|r| self.node(*r).parent == Some(parent)) {
            Some(r) => self.node(r).prev_sibling,
            None => self.node(parent).last_child,
        };
        if let Some(adj) = adjacent
            && let NodeData::Text(existing) = &mut self.node_mut(adj).data
        {
            existing.push_str(text);
            return;
        }
        let node = self.create_text(text);
        self.insert_before(parent, node, reference);
    }

    /// Déplace tous les enfants de `from` à la fin de `to`.
    pub fn reparent_children(&mut self, from: NodeId, to: NodeId) {
        while let Some(child) = self.node(from).first_child {
            self.append_child(to, child);
        }
    }

    /// Itère sur les enfants directs de `id`.
    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(self.node(id).first_child, |c| self.node(*c).next_sibling)
    }

    /// Itère sur les ancêtres de `id`, du parent vers la racine.
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(self.node(id).parent, |p| self.node(*p).parent)
    }

    /// Itère sur les descendants de `id` en ordre du document (préfixe), `id` exclu.
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut next = self.node(id).first_child;
        std::iter::from_fn(move || {
            let current = next?;
            next = self.node(current).first_child.or_else(|| {
                let mut n = current;
                loop {
                    if n == id {
                        return None;
                    }
                    if let Some(s) = self.node(n).next_sibling {
                        return Some(s);
                    }
                    n = self.node(n).parent?;
                }
            });
            Some(current)
        })
    }

    /// Premier élément descendant de `id` portant le nom HTML `name`.
    #[must_use]
    pub fn find_html_element(&self, id: NodeId, name: &str) -> Option<NodeId> {
        self.descendants(id)
            .find(|n| self.element(*n).is_some_and(|e| e.is_html(name)))
    }

    /// Concatène le texte de tous les descendants de `id`.
    #[must_use]
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        if let Some(t) = self.text(id) {
            out.push_str(t);
        }
        for n in self.descendants(id) {
            if let Some(t) = self.text(n) {
                out.push_str(t);
            }
        }
        out
    }

    /// Sérialise le sous-arbre au format des tests html5lib (`| <html>` …).
    #[must_use]
    pub fn dump(&self) -> String {
        let mut out = String::new();
        for child in self.children(self.root()) {
            self.dump_node(child, 0, &mut out);
        }
        out
    }

    fn dump_node(&self, id: NodeId, depth: usize, out: &mut String) {
        use fmt::Write as _;
        let indent = "  ".repeat(depth);
        // `write!` vers une `String` ne peut pas échouer.
        let _ = match &self.node(id).data {
            NodeData::Document => Ok(()),
            NodeData::Doctype {
                name,
                public_id,
                system_id,
            } => {
                if public_id.is_empty() && system_id.is_empty() {
                    writeln!(out, "| {indent}<!DOCTYPE {name}>")
                } else {
                    writeln!(
                        out,
                        "| {indent}<!DOCTYPE {name} \"{public_id}\" \"{system_id}\">"
                    )
                }
            }
            NodeData::Element(e) => {
                let prefix = match e.ns {
                    Namespace::Html => "",
                    Namespace::Svg => "svg ",
                    Namespace::MathMl => "math ",
                };
                let mut r = writeln!(out, "| {indent}<{prefix}{}>", e.name);
                let mut attrs: Vec<&Attribute> = e.attrs.iter().collect();
                attrs.sort_by(|a, b| a.name.cmp(&b.name));
                for a in attrs {
                    r = r.and_then(|()| writeln!(out, "| {indent}  {}=\"{}\"", a.name, a.value));
                }
                r
            }
            NodeData::Text(t) => writeln!(out, "| {indent}\"{t}\""),
            NodeData::Comment(c) => writeln!(out, "| {indent}<!-- {c} -->"),
        };
        for child in self.children(id) {
            self.dump_node(child, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(doc: &mut Document, name: &str) -> NodeId {
        doc.create_element(name, Namespace::Html, Vec::new())
    }

    #[test]
    fn append_insert_detach() {
        let mut doc = Document::new();
        let root = doc.root();
        let a = el(&mut doc, "a");
        let b = el(&mut doc, "b");
        let c = el(&mut doc, "c");
        doc.append_child(root, a);
        doc.append_child(root, c);
        doc.insert_before(root, b, Some(c));
        assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![a, b, c]);
        doc.detach(b);
        assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![a, c]);
        assert_eq!(doc.node(b).parent, None);
        doc.detach(a);
        doc.detach(c);
        assert_eq!(doc.node(root).first_child, None);
        assert_eq!(doc.node(root).last_child, None);
    }

    #[test]
    fn text_merging_and_content() {
        let mut doc = Document::new();
        let root = doc.root();
        let p = el(&mut doc, "p");
        doc.append_child(root, p);
        doc.insert_text(p, None, "bon");
        doc.insert_text(p, None, "jour");
        assert_eq!(doc.children(p).count(), 1);
        assert_eq!(doc.text_content(root), "bonjour");
    }

    #[test]
    fn descendants_preorder() {
        let mut doc = Document::new();
        let root = doc.root();
        let a = el(&mut doc, "a");
        let b = el(&mut doc, "b");
        let c = el(&mut doc, "c");
        let d = el(&mut doc, "d");
        doc.append_child(root, a);
        doc.append_child(a, b);
        doc.append_child(b, c);
        doc.append_child(root, d);
        assert_eq!(doc.descendants(root).collect::<Vec<_>>(), vec![a, b, c, d]);
        assert_eq!(doc.descendants(a).collect::<Vec<_>>(), vec![b, c]);
        assert_eq!(doc.ancestors(c).collect::<Vec<_>>(), vec![b, a, root]);
    }
}
