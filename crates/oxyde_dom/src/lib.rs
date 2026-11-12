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
}

/// Modification du document, consignée pour les observateurs de mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// L'attribut `name` de `target` a changé ; `old` est son ancienne valeur.
    Attribute {
        /// Élément modifié.
        target: NodeId,
        /// Nom de l'attribut.
        name: String,
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
        self.node_mut(id).owner = owner;
        let descendants: Vec<NodeId> = self.descendants(id).collect();
        for d in descendants {
            self.node_mut(d).owner = owner;
        }
    }

    /// Copie le nœud `id` — avec son sous-arbre si `deep` — dans le document
    /// `owner` ; renvoie la copie, détachée. Les copies ne sont pas
    /// consignées au journal des mutations.
    pub fn clone_subtree(&mut self, id: NodeId, deep: bool, owner: NodeId) -> NodeId {
        let journal = self.mutations.take();
        let root = self.push(self.node(id).data.clone());
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
                let copy = self.push(self.node(src).data.clone());
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

    fn log(&mut self, mutation: impl FnOnce() -> Mutation) {
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
                    element.attrs.push(Attribute {
                        name: name.to_owned(),
                        value: value.to_owned(),
                    });
                    None
                }
            };
            self.log(|| Mutation::Attribute {
                target: id,
                name: name.to_owned(),
                old,
            });
        }
    }

    /// Retire l'attribut `name` de l'élément `id`.
    pub fn remove_attr(&mut self, id: NodeId, name: &str) {
        if let Some(element) = self.element_mut(id)
            && let Some(position) = element.attrs.iter().position(|a| a.name == name)
        {
            let old = element.attrs.remove(position).value;
            self.log(|| Mutation::Attribute {
                target: id,
                name: name.to_owned(),
                old: Some(old),
            });
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
        let root = self.push(source.node(node).data.clone());
        let mut pending: Vec<(NodeId, NodeId)> = source.children(node).map(|c| (c, root)).collect();
        pending.reverse();
        while let Some((src, parent)) = pending.pop() {
            let copy = self.push(source.node(src).data.clone());
            self.append_child(parent, copy);
            let before = pending.len();
            pending.extend(source.children(src).map(|c| (c, copy)));
            pending[before..].reverse();
        }
        root
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
