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

mod attrs;
mod ranges;

pub use attrs::{AttrData, HTML_NS, MATHML_NS, SVG_NS, XML_NS, XMLNS_NS};
pub use ranges::{Boundary, IteratorState, LiveRange};

/// État d'élément : boîte de dialogue ouverte par `showModal()` (`:modal`).
pub const STATE_MODAL: u8 = 1;
/// État d'élément : popover affiché (`:popover-open`).
pub const STATE_POPOVER_OPEN: u8 = 2;

/// Espace de noms d'un élément.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// `http://www.w3.org/1999/xhtml`
    Html,
    /// `http://www.w3.org/2000/svg`
    Svg,
    /// `http://www.w3.org/1998/Math/MathML`
    MathMl,
    /// Aucun espace de noms.
    None,
    /// `http://www.w3.org/XML/1998/namespace`
    Xml,
    /// `http://www.w3.org/2000/xmlns/`
    Xmlns,
    /// Autre espace de noms : rang dans la table du document (voir
    /// [`Document::namespace_uri`]).
    Other(u32),
}

/// Un attribut d'élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// Nom qualifié (en minuscules pour les éléments HTML).
    pub name: String,
    /// Valeur.
    pub value: String,
    /// Espace de noms ; [`Namespace::None`] pour un attribut ordinaire.
    pub ns: Namespace,
    /// Préfixe d'espace de noms, début du nom qualifié.
    pub prefix: Option<String>,
    /// Nœud `Attr` qui désigne cet attribut, si un script en a demandé un.
    pub node: Option<NodeId>,
}

impl Attribute {
    /// Attribut ordinaire, sans espace de noms.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            ns: Namespace::None,
            prefix: None,
            node: None,
        }
    }

    /// Nom local : le nom qualifié sans son préfixe.
    #[must_use]
    pub fn local(&self) -> &str {
        match &self.prefix {
            Some(prefix) => self.name.get(prefix.len() + 1..).unwrap_or(&self.name),
            None => &self.name,
        }
    }
}

/// Données propres à un élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementData {
    /// Nom local (`div`, `p`, …).
    pub name: String,
    /// Espace de noms.
    pub ns: Namespace,
    /// Préfixe d'espace de noms (`svg` dans `svg:rect`).
    pub prefix: Option<String>,
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

    /// Nom qualifié : `préfixe:nom`, ou le nom local seul.
    #[must_use]
    pub fn qualified(&self) -> std::borrow::Cow<'_, str> {
        match &self.prefix {
            Some(prefix) => std::borrow::Cow::Owned(format!("{prefix}:{}", self.name)),
            None => std::borrow::Cow::Borrowed(&self.name),
        }
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
    /// Fragment de document : conteneur détaché dont les enfants prennent la
    /// place quand on l'insère dans un autre nœud.
    Fragment,
    /// Nœud texte.
    Text(String),
    /// Commentaire.
    Comment(String),
    /// Section CDATA (documents XML seulement).
    CData(String),
    /// Instruction de traitement (`<?cible données?>`).
    ProcessingInstruction {
        /// Cible de l'instruction.
        target: String,
        /// Données.
        data: String,
    },
    /// Nœud d'attribut, créé à la demande d'un script.
    Attr(AttrData),
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
    /// Document auquel le nœud appartient (« node document ») ; un nœud
    /// document est son propre propriétaire.
    pub owner: NodeId,
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
    /// Journal des modifications, tenu seulement quand des observateurs de
    /// mutation l'ont demandé (voir [`Document::record_mutations`]). Chaque
    /// modification est accompagnée du nœud touché et de ses ancêtres à cet
    /// instant : un nœud détaché ensuite reste attribué à son ancien arbre.
    mutations: Option<Vec<(Mutation, Vec<NodeId>)>>,
    /// Groupe de chaque entrée du journal (0 : hors groupe), en parallèle.
    groups: Vec<u32>,
    /// Groupe en cours (0 : aucun) et dernier numéro attribué.
    group: u32,
    last_group: u32,
    /// Plages vivantes (`Range`) créées par les scripts.
    ranges: Vec<LiveRange>,
    /// Itérateurs (`NodeIterator`) créés par les scripts.
    iterators: Vec<IteratorState>,
    /// Adresses des espaces de noms [`Namespace::Other`].
    namespaces: Vec<String>,
    /// Compteur de modifications (voir [`Document::version`]).
    version: u64,
    /// Élément qui a le focus, s'il y en a un.
    focus: Option<NodeId>,
    /// Position du curseur dans la valeur du champ de texte qui a le focus,
    /// en octets.
    caret: usize,
    /// Autre bout de la sélection dans cette valeur, en octets ; égal à
    /// `caret` quand rien n'est sélectionné.
    anchor: usize,
    /// Racine de l'arbre fantôme de chaque élément hôte.
    shadow_roots: std::collections::HashMap<NodeId, NodeId>,
    /// Hôte de chaque racine fantôme, et si elle est ouverte (`mode: "open"`).
    /// INVARIANT : l'inverse exact de `shadow_roots`.
    shadow_hosts: std::collections::HashMap<NodeId, (NodeId, bool)>,
    /// États d'élément que le style voit (`:modal`, `:popover-open`) :
    /// voir [`STATE_MODAL`] et [`STATE_POPOVER_OPEN`].
    states: std::collections::HashMap<NodeId, u8>,
}

/// Modification du document, consignée pour les observateurs de mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// L'attribut `name` de `target` a changé ; `old` est son ancienne valeur.
    Attribute {
        /// Élément modifié.
        target: NodeId,
        /// Nom local de l'attribut.
        name: String,
        /// Espace de noms de l'attribut.
        ns: Namespace,
        /// Valeur précédente, `None` si l'attribut n'existait pas.
        old: Option<String>,
    },
    /// Le texte de `target` (nœud texte ou commentaire) a changé.
    Text {
        /// Nœud modifié.
        target: NodeId,
        /// Texte précédent.
        old: String,
    },
    /// Les enfants de `parent` ont changé : `added` ont été insérés et
    /// `removed` retirés, entre `previous` et `next`. Les changements d'une
    /// même opération (voir [`Document::begin_group`]) sont réunis.
    Children {
        /// Parent dont la liste d'enfants a changé.
        parent: NodeId,
        /// Nœuds insérés, dans l'ordre.
        added: Vec<NodeId>,
        /// Nœuds retirés, dans l'ordre.
        removed: Vec<NodeId>,
        /// Frère qui précède la zone modifiée.
        previous: Option<NodeId>,
        /// Frère qui suit la zone modifiée.
        next: Option<NodeId>,
    },
}

/// Nombre maximal de modifications gardées dans le journal entre deux relevés.
const MAX_MUTATIONS: usize = 100_000;

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
                owner: NodeId(0),
                data: NodeData::Document,
            }],
            quirks_mode: QuirksMode::NoQuirks,
            mutations: None,
            groups: Vec::new(),
            group: 0,
            last_group: 0,
            ranges: Vec::new(),
            iterators: Vec::new(),
            namespaces: Vec::new(),
            version: 0,
            focus: None,
            caret: 0,
            anchor: 0,
            shadow_roots: std::collections::HashMap::new(),
            shadow_hosts: std::collections::HashMap::new(),
            states: std::collections::HashMap::new(),
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
            owner: NodeId(0),
            data,
        });
        id
    }

    /// Crée un document supplémentaire, vide et détaché, dans la même arène
    /// (`document.implementation.createHTMLDocument()`, `new Document()`).
    pub fn create_document(&mut self) -> NodeId {
        let id = self.push(NodeData::Document);
        self.node_mut(id).owner = id;
        id
    }

    /// Document auquel `id` appartient ; un document se rend lui-même.
    #[must_use]
    pub fn owner(&self, id: NodeId) -> NodeId {
        self.node(id).owner
    }

    /// Vrai si `id` est un nœud document.
    #[must_use]
    pub fn is_document(&self, id: NodeId) -> bool {
        matches!(self.node(id).data, NodeData::Document)
    }

    /// Fait de `owner` le document de `id` et de tout son sous-arbre
    /// (« adopt »). Sans effet sur un nœud document.
    pub fn set_owner(&mut self, id: NodeId, owner: NodeId) {
        if self.is_document(id) || self.node(id).owner == owner {
            return;
        }
        let mut moved: Vec<NodeId> = std::iter::once(id).chain(self.descendants(id)).collect();
        let handles: Vec<NodeId> = moved
            .iter()
            .filter_map(|n| self.element(*n))
            .flat_map(|e| e.attrs.iter().filter_map(|a| a.node))
            .collect();
        moved.extend(handles);
        for node in moved {
            self.node_mut(node).owner = owner;
        }
    }

    /// Copie le nœud `id` — avec son sous-arbre si `deep` — dans le document
    /// `owner` ; renvoie la copie, détachée. Les copies ne sont pas
    /// consignées au journal des mutations.
    pub fn clone_subtree(&mut self, id: NodeId, deep: bool, owner: NodeId) -> NodeId {
        let journal = self.mutations.take();
        let root = self.push(self.copied_data(id));
        if self.is_document(root) {
            self.node_mut(root).owner = root;
        } else {
            self.node_mut(root).owner = owner;
        }
        let inner = if self.is_document(root) { root } else { owner };
        if deep {
            // Parcours itératif : (nœud source, parent de la copie).
            let mut pending: Vec<(NodeId, NodeId)> = self.children(id).map(|c| (c, root)).collect();
            pending.reverse();
            while let Some((src, parent)) = pending.pop() {
                let copy = self.push(self.copied_data(src));
                self.node_mut(copy).owner = inner;
                self.append_child(parent, copy);
                let before = pending.len();
                pending.extend(self.children(src).map(|c| (c, copy)));
                pending[before..].reverse();
            }
        }
        self.mutations = journal;
        root
    }

    /// Contenu d'une copie de `id` : les attributs perdent leur nœud `Attr`,
    /// et un nœud d'attribut copié est détaché, avec sa valeur du moment.
    fn copied_data(&self, id: NodeId) -> NodeData {
        let mut data = self.node(id).data.clone();
        match &mut data {
            NodeData::Element(element) => {
                for attribute in &mut element.attrs {
                    attribute.node = None;
                }
            }
            NodeData::Attr(attr) => {
                attr.value = self.attr_value(id).unwrap_or("").to_owned();
                attr.element = None;
            }
            _ => {}
        }
        data
    }

    /// Vrai si le journal tient des modifications pas encore relevées.
    #[must_use]
    pub fn has_mutations(&self) -> bool {
        self.mutations.as_ref().is_some_and(|log| !log.is_empty())
    }

    /// Tient (ou cesse de tenir) le journal des modifications.
    pub fn record_mutations(&mut self, enabled: bool) {
        if enabled {
            self.mutations.get_or_insert_with(Vec::new);
        } else {
            self.mutations = None;
        }
    }

    /// Rend les modifications consignées depuis le dernier relevé, chacune
    /// avec le nœud touché suivi de ses ancêtres au moment où elle a eu lieu.
    pub fn take_mutations(&mut self) -> Vec<(Mutation, Vec<NodeId>)> {
        self.groups.clear();
        self.mutations
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// Ouvre un groupe : jusqu'à [`Document::end_group`], les insertions et
    /// retraits d'enfants d'un même parent forment une seule entrée du
    /// journal (un seul `MutationRecord`). Sans effet si un groupe est déjà
    /// ouvert ; renvoie `true` si c'est cet appel qui l'a ouvert.
    pub fn begin_group(&mut self) -> bool {
        if self.group != 0 {
            return false;
        }
        self.last_group = self.last_group.wrapping_add(1).max(1);
        self.group = self.last_group;
        true
    }

    /// Ferme le groupe ouvert par [`Document::begin_group`].
    pub fn end_group(&mut self) {
        self.group = 0;
    }

    /// Élément qui a le focus (`document.activeElement`, quand ce n'est pas
    /// `<body>`).
    #[must_use]
    pub fn focus(&self) -> Option<NodeId> {
        self.focus
    }

    /// Donne le focus à `node`, ou le retire ; le curseur va à `caret`. Le
    /// style dépend du focus (`:focus`) : la version du document avance.
    pub fn set_focus(&mut self, node: Option<NodeId>, caret: usize) {
        self.set_field_selection(node, caret, caret);
    }

    /// Donne le focus à `node` et y sélectionne le texte compris entre
    /// `anchor` et `caret`, où se tient le curseur.
    pub fn set_field_selection(&mut self, node: Option<NodeId>, anchor: usize, caret: usize) {
        if self.focus != node || self.caret != caret || self.anchor != anchor {
            self.focus = node;
            self.caret = caret;
            self.anchor = anchor;
            self.version = self.version.wrapping_add(1);
        }
    }

    /// Autre bout de la sélection dans la valeur du champ qui a le focus,
    /// en octets ; égal à [`Document::caret`] quand rien n'est sélectionné.
    #[must_use]
    pub fn selection_anchor(&self) -> usize {
        self.anchor
    }

    /// Position du curseur dans la valeur du champ qui a le focus, en
    /// octets ; à ramener sur une frontière de caractère avant usage.
    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret
    }

    // ---- Arbres fantômes (Shadow DOM) ------------------------------------------------------

    /// Attache un arbre fantôme à l'élément `host` et renvoie sa racine, un
    /// fragment qui n'est l'enfant de personne : `ancestors` et `descendants`
    /// ne passent pas d'un arbre à l'autre. `None` si `host` n'est pas un
    /// élément ou porte déjà un arbre fantôme.
    pub fn attach_shadow(&mut self, host: NodeId, open: bool) -> Option<NodeId> {
        if self.element(host).is_none() || self.shadow_roots.contains_key(&host) {
            return None;
        }
        let root = self.create_fragment();
        self.shadow_roots.insert(host, root);
        self.shadow_hosts.insert(root, (host, open));
        self.version = self.version.wrapping_add(1);
        Some(root)
    }

    /// Arbres fantômes déclaratifs (HTML Standard §4.12.3) : chaque
    /// `<template shadowrootmode="open|closed">`, enfant d'un élément sans
    /// arbre fantôme, lui en donne un, y verse son contenu et disparaît.
    /// À appeler une fois, sur un document tout juste analysé. Renvoie le
    /// nombre d'arbres attachés.
    pub fn adopt_declarative_shadow_roots(&mut self) -> usize {
        let mut attached = 0;
        let mut pending = vec![self.root()];
        while let Some(scope) = pending.pop() {
            let templates: Vec<(NodeId, bool)> = self
                .descendants(scope)
                .filter_map(|node| {
                    let element = self.element(node).filter(|e| e.is_html("template"))?;
                    match element.attr("shadowrootmode")? {
                        "open" => Some((node, true)),
                        "closed" => Some((node, false)),
                        _ => None,
                    }
                })
                .collect();
            for (template, open) in templates {
                let Some(parent) = self.node(template).parent else {
                    continue;
                };
                // Un seul arbre par hôte : un second gabarit reste un gabarit.
                let Some(shadow) = self.attach_shadow(parent, open) else {
                    continue;
                };
                let children: Vec<NodeId> = self.children(template).collect();
                for child in children {
                    self.append_child(shadow, child);
                }
                self.detach(template);
                attached += 1;
                pending.push(shadow);
            }
        }
        attached
    }

    /// Racine de l'arbre fantôme de l'élément `host`, s'il en porte un.
    #[must_use]
    pub fn shadow_root(&self, host: NodeId) -> Option<NodeId> {
        self.shadow_roots.get(&host).copied()
    }

    /// Hôte de la racine fantôme `root` ; `None` si `root` n'en est pas une.
    #[must_use]
    pub fn shadow_host(&self, root: NodeId) -> Option<NodeId> {
        self.shadow_hosts.get(&root).map(|(host, _)| *host)
    }

    /// Vrai si la racine fantôme `root` est ouverte aux scripts.
    #[must_use]
    pub fn shadow_is_open(&self, root: NodeId) -> bool {
        self.shadow_hosts.get(&root).is_some_and(|(_, open)| *open)
    }

    /// Pose (`on`) ou retire l'état `flag` de l'élément `node`. Le style
    /// en dépend : la version du document avance.
    pub fn set_state(&mut self, node: NodeId, flag: u8, on: bool) {
        let current = self.states.get(&node).copied().unwrap_or(0);
        let next = if on { current | flag } else { current & !flag };
        if next == current {
            return;
        }
        if next == 0 {
            self.states.remove(&node);
        } else {
            self.states.insert(node, next);
        }
        self.version = self.version.wrapping_add(1);
    }

    /// Vrai si l'élément `node` porte l'état `flag`.
    #[must_use]
    pub fn has_state(&self, node: NodeId, flag: u8) -> bool {
        self.states
            .get(&node)
            .is_some_and(|state| state & flag != 0)
    }

    /// Vrai si le document porte au moins un arbre fantôme.
    #[must_use]
    pub fn has_shadow_roots(&self) -> bool {
        !self.shadow_roots.is_empty()
    }

    /// Racines fantômes du document, dans l'ordre de leur création.
    #[must_use]
    pub fn shadow_root_list(&self) -> Vec<NodeId> {
        let mut roots: Vec<NodeId> = self.shadow_hosts.keys().copied().collect();
        roots.sort_by_key(|root| root.index());
        roots
    }

    /// Racine de l'arbre qui contient `node` : son plus lointain ancêtre
    /// (le document, une racine fantôme, ou le sommet d'un arbre détaché).
    #[must_use]
    pub fn tree_root(&self, node: NodeId) -> NodeId {
        self.ancestors(node).last().unwrap_or(node)
    }

    /// Racine fantôme de l'arbre qui contient `node`, s'il est dans un
    /// arbre fantôme.
    #[must_use]
    pub fn containing_shadow_root(&self, node: NodeId) -> Option<NodeId> {
        let root = self.tree_root(node);
        self.shadow_hosts.contains_key(&root).then_some(root)
    }

    /// Racine de `node` à travers les arbres fantômes : celle de l'arbre
    /// de l'hôte, tant que l'arbre courant est un arbre fantôme.
    #[must_use]
    pub fn composed_root(&self, node: NodeId) -> NodeId {
        let mut root = self.tree_root(node);
        // Borne : un hôte glissé dans son propre arbre fantôme ne fait pas
        // tourner sans fin.
        for _ in 0..=self.shadow_hosts.len() {
            match self.shadow_host(root) {
                Some(host) => root = self.tree_root(host),
                None => break,
            }
        }
        root
    }

    /// Vrai si `node` est relié au document, à travers les arbres fantômes.
    #[must_use]
    pub fn is_connected(&self, node: NodeId) -> bool {
        self.composed_root(node) == self.root()
    }

    /// Emplacement (`<slot>`) de l'arbre fantôme de son parent où `node`
    /// est distribué : le premier, dans l'ordre de l'arbre, dont le nom est
    /// celui que demande `node` (attribut `slot` ; vide pour un texte).
    ///
    /// LIMITE : la recherche parcourt l'arbre fantôme à chaque appel.
    #[must_use]
    pub fn assigned_slot(&self, node: NodeId) -> Option<NodeId> {
        let host = self.node(node).parent?;
        let root = self.shadow_root(host)?;
        if self.element(node).is_none() && self.text(node).is_none() {
            return None;
        }
        let wanted = self
            .element(node)
            .and_then(|e| e.attr("slot"))
            .unwrap_or("");
        self.descendants(root).find(|n| {
            self.element(*n)
                .is_some_and(|e| e.is_html("slot") && e.attr("name").unwrap_or("") == wanted)
        })
    }

    /// Nœuds distribués dans l'emplacement `slot`, dans l'ordre des enfants
    /// de l'hôte ; vide si `slot` n'est pas un `<slot>` d'un arbre fantôme.
    #[must_use]
    pub fn assigned_nodes(&self, slot: NodeId) -> Vec<NodeId> {
        let is_slot = self.element(slot).is_some_and(|e| e.is_html("slot"));
        let Some(host) = self
            .containing_shadow_root(slot)
            .filter(|_| is_slot)
            .and_then(|root| self.shadow_host(root))
        else {
            return Vec::new();
        };
        self.children(host)
            .filter(|n| self.assigned_slot(*n) == Some(slot))
            .collect()
    }

    /// Enfants de `node` dans l'arbre aplati, celui que l'on affiche : ceux
    /// de sa racine fantôme pour un hôte, les nœuds distribués pour un
    /// emplacement qui en reçoit (sinon son contenu de repli), ses enfants
    /// sinon.
    #[must_use]
    pub fn flat_children(&self, node: NodeId) -> Vec<NodeId> {
        if let Some(root) = self.shadow_root(node) {
            return self.children(root).collect();
        }
        if !self.shadow_roots.is_empty() {
            let assigned = self.assigned_nodes(node);
            if !assigned.is_empty() {
                return assigned;
            }
        }
        self.children(node).collect()
    }

    /// Parent de `node` dans l'arbre aplati : l'hôte pour un enfant de
    /// racine fantôme, l'emplacement pour un enfant d'hôte qui y est
    /// distribué (`None` s'il ne l'est nulle part : il n'est pas affiché),
    /// son parent sinon.
    #[must_use]
    pub fn flat_parent(&self, node: NodeId) -> Option<NodeId> {
        let parent = self.node(node).parent?;
        if let Some(host) = self.shadow_host(parent) {
            return Some(host);
        }
        if self.shadow_roots.contains_key(&parent) {
            return self.assigned_slot(node);
        }
        Some(parent)
    }

    /// Descendants de `root` dans l'arbre aplati, en ordre préfixe, `root`
    /// exclu.
    #[must_use]
    pub fn flat_descendants(&self, root: NodeId) -> Vec<NodeId> {
        let mut found = Vec::new();
        // Parcours itératif : la profondeur d'un document n'est pas bornée.
        let mut stack: Vec<NodeId> = self.flat_children(root);
        stack.reverse();
        while let Some(node) = stack.pop() {
            found.push(node);
            // Borne : un arbre mal formé ne fait pas tourner sans fin.
            if found.len() > self.len() {
                break;
            }
            let mut children = self.flat_children(node);
            children.reverse();
            stack.extend(children);
        }
        found
    }

    /// Compteur qui avance à chaque modification de l'arbre, d'un attribut
    /// ou d'un texte : deux lectures à la même version voient le même
    /// document.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    fn log(&mut self, mutation: impl FnOnce() -> Mutation) {
        self.version = self.version.wrapping_add(1);
        if !self
            .mutations
            .as_ref()
            .is_some_and(|log| log.len() < MAX_MUTATIONS)
        {
            return;
        }
        let mutation = mutation();
        let target = match &mutation {
            Mutation::Attribute { target, .. } | Mutation::Text { target, .. } => *target,
            Mutation::Children { parent, .. } => *parent,
        };
        // Dans un groupe, les changements d'enfants d'un même parent fusionnent.
        if self.group != 0
            && let Mutation::Children {
                parent,
                added,
                removed,
                next,
                ..
            } = &mutation
            && let Some(log) = &mut self.mutations
        {
            for (index, (entry, _)) in log.iter_mut().enumerate().rev() {
                if self.groups.get(index).copied() != Some(self.group) {
                    break;
                }
                if let Mutation::Children {
                    parent: p,
                    added: a,
                    removed: r,
                    next: n,
                    ..
                } = entry
                    && p == parent
                {
                    a.extend(added.iter().copied());
                    r.extend(removed.iter().copied());
                    *n = *next;
                    return;
                }
            }
        }
        let path: Vec<NodeId> = std::iter::once(target)
            .chain(self.ancestors(target))
            .collect();
        if let Some(log) = &mut self.mutations {
            log.push((mutation, path));
            self.groups.push(self.group);
        }
    }

    /// Remplace le texte d'un nœud texte ou commentaire ; sans effet sur les autres nœuds.
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if let NodeData::Text(t)
        | NodeData::Comment(t)
        | NodeData::CData(t)
        | NodeData::ProcessingInstruction { data: t, .. } = &mut self.node_mut(id).data
        {
            let old = std::mem::replace(t, text.to_owned());
            self.log(|| Mutation::Text { target: id, old });
        }
    }

    /// Fixe l'attribut `name` de l'élément `id` (créé s'il n'existe pas).
    pub fn set_attr(&mut self, id: NodeId, name: &str, value: &str) {
        if let Some(element) = self.element_mut(id) {
            let old = match element.attrs.iter_mut().find(|a| a.name == name) {
                Some(attr) => Some(std::mem::replace(&mut attr.value, value.to_owned())),
                None => {
                    element.attrs.push(Attribute::new(name, value));
                    None
                }
            };
            self.log(|| Mutation::Attribute {
                target: id,
                name: name.to_owned(),
                ns: Namespace::None,
                old,
            });
        }
    }

    /// Retire l'attribut `name` de l'élément `id`.
    pub fn remove_attr(&mut self, id: NodeId, name: &str) {
        if let Some(index) = self.attr_index(id, name) {
            self.remove_attr_at(id, index);
        }
    }

    /// Détache tous les enfants de `id`.
    pub fn remove_children(&mut self, id: NodeId) {
        while let Some(child) = self.node(id).first_child {
            self.detach(child);
        }
    }

    /// Vrai si `ancestor` est `node` ou l'un de ses ancêtres.
    #[must_use]
    pub fn contains(&self, ancestor: NodeId, node: NodeId) -> bool {
        ancestor == node || self.ancestors(node).any(|a| a == ancestor)
    }

    /// Copie dans ce document le sous-arbre `node` du document `source` ;
    /// renvoie la racine de la copie, détachée.
    pub fn import(&mut self, source: &Document, node: NodeId) -> NodeId {
        // Parcours itératif : (nœud source, parent de la copie).
        let data = self.imported_data(source, node);
        let root = self.push(data);
        let mut pending: Vec<(NodeId, NodeId)> = source.children(node).map(|c| (c, root)).collect();
        pending.reverse();
        while let Some((src, parent)) = pending.pop() {
            let data = self.imported_data(source, src);
            let copy = self.push(data);
            self.append_child(parent, copy);
            let before = pending.len();
            pending.extend(source.children(src).map(|c| (c, copy)));
            pending[before..].reverse();
        }
        root
    }

    /// Contenu du nœud `node` de `source`, prêt à entrer dans ce document :
    /// les espaces de noms propres à `source` sont traduits dans la table
    /// de ce document, et les nœuds `Attr` de `source` oubliés.
    fn imported_data(&mut self, source: &Document, node: NodeId) -> NodeData {
        let mut data = source.node(node).data.clone();
        if let NodeData::Element(element) = &mut data {
            element.ns = self.imported_namespace(source, element.ns);
            for attr in &mut element.attrs {
                attr.ns = self.imported_namespace(source, attr.ns);
                attr.node = None;
            }
        }
        data
    }

    /// Espace de noms `ns` de `source`, dans la table de ce document.
    fn imported_namespace(&mut self, source: &Document, ns: Namespace) -> Namespace {
        match ns {
            Namespace::Other(_) => match source.namespace_uri(ns) {
                Some(uri) => self.namespace(uri),
                None => Namespace::None,
            },
            known => known,
        }
    }

    /// Crée un élément détaché.
    pub fn create_element(&mut self, name: &str, ns: Namespace, attrs: Vec<Attribute>) -> NodeId {
        self.push(NodeData::Element(ElementData {
            name: name.to_owned(),
            ns,
            prefix: None,
            attrs,
        }))
    }

    /// Crée un nœud texte détaché.
    pub fn create_text(&mut self, text: &str) -> NodeId {
        self.push(NodeData::Text(text.to_owned()))
    }

    /// Crée un fragment de document vide.
    pub fn create_fragment(&mut self) -> NodeId {
        self.push(NodeData::Fragment)
    }

    /// Crée un commentaire détaché.
    pub fn create_comment(&mut self, text: &str) -> NodeId {
        self.push(NodeData::Comment(text.to_owned()))
    }

    /// Crée une section CDATA détachée.
    pub fn create_cdata(&mut self, text: &str) -> NodeId {
        self.push(NodeData::CData(text.to_owned()))
    }

    /// Crée une instruction de traitement détachée.
    pub fn create_processing_instruction(&mut self, target: &str, data: &str) -> NodeId {
        self.push(NodeData::ProcessingInstruction {
            target: target.to_owned(),
            data: data.to_owned(),
        })
    }

    /// Données du nœud s'il en porte (texte, commentaire, section CDATA,
    /// instruction de traitement).
    #[must_use]
    pub fn character_data(&self, id: NodeId) -> Option<&str> {
        match &self.node(id).data {
            NodeData::Text(t)
            | NodeData::Comment(t)
            | NodeData::CData(t)
            | NodeData::ProcessingInstruction { data: t, .. } => Some(t),
            _ => None,
        }
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
        self.ranges_before_removal(child, parent);
        self.iterators_before_removal(child);
        self.log(|| Mutation::Children {
            parent,
            added: Vec::new(),
            removed: vec![child],
            previous: prev,
            next,
        });
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
        // Un fragment ne s'insère pas : ses enfants prennent sa place.
        if matches!(self.node(child).data, NodeData::Fragment) {
            let children: Vec<NodeId> = self.children(child).collect();
            for moved in children {
                self.insert_before(parent, moved, reference);
            }
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
        let owner = if self.is_document(parent) {
            parent
        } else {
            self.node(parent).owner
        };
        self.set_owner(child, owner);
        self.ranges_after_insertion(child, parent);
        let (previous, next) = {
            let inserted = self.node(child);
            (inserted.prev_sibling, inserted.next_sibling)
        };
        self.log(|| Mutation::Children {
            parent,
            added: vec![child],
            removed: Vec::new(),
            previous,
            next,
        });
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
            let old = existing.clone();
            existing.push_str(text);
            self.log(|| Mutation::Text { target: adj, old });
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
        let text = |n: NodeId| match &self.node(n).data {
            NodeData::Text(t) | NodeData::CData(t) => Some(t.as_str()),
            _ => None,
        };
        let mut out = String::new();
        if let Some(t) = text(id) {
            out.push_str(t);
        }
        for n in self.descendants(id) {
            if let Some(t) = text(n) {
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
            NodeData::CData(t) => writeln!(out, "| {indent}<![CDATA[{t}]]>"),
            NodeData::ProcessingInstruction { target, data } => {
                writeln!(out, "| {indent}<?{target} {data}>")
            }
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
                    Namespace::None | Namespace::Xml | Namespace::Xmlns | Namespace::Other(_) => "",
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
            NodeData::Fragment => writeln!(out, "| {indent}#document-fragment"),
            NodeData::Attr(_) => Ok(()),
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

    #[test]
    fn declarative_shadow_roots_are_attached() {
        let mut doc = Document::new();
        let root = doc.root();
        let host = doc.create_element("div", Namespace::Html, Vec::new());
        doc.append_child(root, host);
        let template = doc.create_element(
            "template",
            Namespace::Html,
            vec![Attribute::new("shadowrootmode", "closed")],
        );
        doc.append_child(host, template);
        let inner = doc.create_element("span", Namespace::Html, Vec::new());
        doc.append_child(template, inner);
        // Un second gabarit sur le même hôte reste en place.
        let second = doc.create_element(
            "template",
            Namespace::Html,
            vec![Attribute::new("shadowrootmode", "open")],
        );
        doc.append_child(host, second);
        assert_eq!(doc.adopt_declarative_shadow_roots(), 1);
        let shadow = doc.shadow_root(host).expect("arbre fantôme");
        assert!(!doc.shadow_is_open(shadow));
        assert_eq!(doc.children(shadow).collect::<Vec<_>>(), vec![inner]);
        assert_eq!(doc.children(host).collect::<Vec<_>>(), vec![second]);
    }
}
