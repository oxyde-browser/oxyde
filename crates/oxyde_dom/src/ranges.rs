//! Plages vivantes et données textuelles (DOM Standard §4.10 et §5).
//!
//! Le document garde les points de frontière de chaque `Range` créé par un
//! script et les ajuste lui-même à chaque insertion, retrait ou modification
//! de texte : c'est ce qui rend une plage « vivante ». Les positions dans le
//! texte comptent des unités UTF-16, comme le veut le DOM.

use crate::{Document, Mutation, NodeData, NodeId};

/// Point de frontière : un nœud et une position dans ce nœud (rang d'enfant,
/// ou rang d'unité UTF-16 pour un nœud de texte).
pub type Boundary = (NodeId, u32);

/// Plage vivante : de `start` à `end`, dans l'ordre de l'arbre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveRange {
    /// Début de la plage.
    pub start: Boundary,
    /// Fin de la plage.
    pub end: Boundary,
}

/// État d'un `NodeIterator` : sa racine, son nœud de référence et le côté
/// du pointeur (avant ou après ce nœud).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IteratorState {
    /// Racine du parcours.
    pub root: NodeId,
    /// Nœud de référence.
    pub reference: NodeId,
    /// Vrai si le pointeur est avant le nœud de référence.
    pub before: bool,
}

/// Nombre maximal de plages vivantes par page : chaque mutation les
/// parcourt toutes.
const MAX_RANGES: usize = 100_000;

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Document {
    /// Enregistre une plage vivante et rend son identifiant, ou `None` si la
    /// page en a déjà trop.
    pub fn add_range(&mut self, start: Boundary, end: Boundary) -> Option<usize> {
        if self.ranges.len() >= MAX_RANGES {
            return None;
        }
        self.ranges.push(LiveRange { start, end });
        Some(self.ranges.len() - 1)
    }

    /// Plage vivante d'identifiant `id`.
    #[must_use]
    pub fn range(&self, id: usize) -> Option<LiveRange> {
        self.ranges.get(id).copied()
    }

    /// Remplace les frontières de la plage `id`.
    pub fn set_range(&mut self, id: usize, range: LiveRange) {
        if let Some(slot) = self.ranges.get_mut(id) {
            *slot = range;
        }
    }

    /// Enregistre un itérateur placé avant sa racine `root` ; `None` si la
    /// page en a déjà trop.
    pub fn add_iterator(&mut self, root: NodeId) -> Option<usize> {
        if self.iterators.len() >= MAX_RANGES {
            return None;
        }
        self.iterators.push(IteratorState {
            root,
            reference: root,
            before: true,
        });
        Some(self.iterators.len() - 1)
    }

    /// État de l'itérateur `id`.
    #[must_use]
    pub fn iterator(&self, id: usize) -> Option<IteratorState> {
        self.iterators.get(id).copied()
    }

    /// Déplace l'itérateur `id`.
    pub fn set_iterator(&mut self, id: usize, reference: NodeId, before: bool) {
        if let Some(state) = self.iterators.get_mut(id) {
            state.reference = reference;
            state.before = before;
        }
    }

    /// « Pre-removing steps » des itérateurs, avant le retrait de `child` :
    /// une référence située dans le sous-arbre retiré se replie sur un nœud
    /// qui reste.
    pub(crate) fn iterators_before_removal(&mut self, child: NodeId) {
        if self.iterators.is_empty() {
            return;
        }
        let mut iterators = std::mem::take(&mut self.iterators);
        for state in &mut iterators {
            if !self.contains(child, state.reference) || self.contains(child, state.root) {
                continue;
            }
            if state.before {
                // Premier nœud après le sous-arbre retiré, sans sortir de la racine.
                let mut node = child;
                let next = loop {
                    if node == state.root {
                        break None;
                    }
                    if let Some(sibling) = self.node(node).next_sibling {
                        break Some(sibling);
                    }
                    match self.node(node).parent {
                        Some(parent) => node = parent,
                        None => break None,
                    }
                };
                if let Some(next) = next {
                    state.reference = next;
                    continue;
                }
                state.before = false;
            }
            state.reference = match self.node(child).prev_sibling {
                Some(mut previous) => {
                    while let Some(last) = self.node(previous).last_child {
                        previous = last;
                    }
                    previous
                }
                None => self.node(child).parent.unwrap_or(state.root),
            };
        }
        self.iterators = iterators;
    }

    /// Rang de `node` parmi les enfants de son parent (0 s'il n'en a pas).
    #[must_use]
    pub fn index_of(&self, node: NodeId) -> u32 {
        to_u32(
            std::iter::successors(self.node(node).prev_sibling, |n| self.node(*n).prev_sibling)
                .count(),
        )
    }

    /// Longueur des données de `node` en unités UTF-16 (0 s'il n'en porte pas).
    #[must_use]
    pub fn data_len(&self, node: NodeId) -> u32 {
        to_u32(
            self.character_data(node)
                .map_or(0, |t| t.encode_utf16().count()),
        )
    }

    /// « Longueur » d'un nœud : 0 pour un DOCTYPE, la longueur des données
    /// pour un nœud de texte, le nombre d'enfants sinon.
    #[must_use]
    pub fn node_length(&self, node: NodeId) -> u32 {
        match &self.node(node).data {
            NodeData::Doctype { .. } => 0,
            NodeData::Text(_)
            | NodeData::Comment(_)
            | NodeData::CData(_)
            | NodeData::ProcessingInstruction { .. } => self.data_len(node),
            _ => to_u32(self.children(node).count()),
        }
    }

    /// Enfant de `node` au rang `index`, s'il existe.
    #[must_use]
    pub fn child_at(&self, node: NodeId, index: u32) -> Option<NodeId> {
        self.children(node).nth(index as usize)
    }

    /// « Substring data » : `count` unités à partir de `offset`. `None` si
    /// `offset` dépasse la longueur.
    #[must_use]
    pub fn substring_data(&self, node: NodeId, offset: u32, count: u32) -> Option<String> {
        let units: Vec<u16> = self.character_data(node)?.encode_utf16().collect();
        let start = offset as usize;
        if start > units.len() {
            return None;
        }
        let end = start.saturating_add(count as usize).min(units.len());
        Some(String::from_utf16_lossy(units.get(start..end)?))
    }

    /// « Replace data » : remplace `count` unités à partir de `offset` par
    /// `data`, puis ajuste les plages vivantes. Renvoie `false`, sans rien
    /// changer, si `offset` dépasse la longueur ou si le nœud ne porte pas
    /// de données.
    pub fn replace_data(&mut self, node: NodeId, offset: u32, count: u32, data: &str) -> bool {
        let Some(current) = self.character_data(node) else {
            return false;
        };
        let mut units: Vec<u16> = current.encode_utf16().collect();
        let start = offset as usize;
        if start > units.len() {
            return false;
        }
        let end = start.saturating_add(count as usize).min(units.len());
        let count = to_u32(end - start);
        let added = to_u32(data.encode_utf16().count());
        units.splice(start..end, data.encode_utf16());
        let text = String::from_utf16_lossy(&units);
        if let NodeData::Text(t)
        | NodeData::Comment(t)
        | NodeData::CData(t)
        | NodeData::ProcessingInstruction { data: t, .. } = &mut self.node_mut(node).data
        {
            let old = std::mem::replace(t, text);
            self.log(|| Mutation::Text { target: node, old });
        }
        let last = offset.saturating_add(count);
        for range in &mut self.ranges {
            for (container, position) in [&mut range.start, &mut range.end] {
                if *container != node || *position <= offset {
                    continue;
                }
                if *position <= last {
                    *position = offset;
                } else {
                    *position = position.saturating_add(added).saturating_sub(count);
                }
            }
        }
        true
    }

    /// « Split » d'un nœud texte à la position `offset` : la fin devient un
    /// nouveau nœud texte, inséré juste après. `None` si `offset` dépasse la
    /// longueur ou si `node` n'est pas un texte.
    pub fn split_text(&mut self, node: NodeId, offset: u32) -> Option<NodeId> {
        if !matches!(self.node(node).data, NodeData::Text(_) | NodeData::CData(_)) {
            return None;
        }
        let length = self.data_len(node);
        if offset > length {
            return None;
        }
        let count = length - offset;
        let tail = self.substring_data(node, offset, count)?;
        let new = self.create_text(&tail);
        let owner = self.node(node).owner;
        self.set_owner(new, owner);
        let (parent, next) = {
            let links = self.node(node);
            (links.parent, links.next_sibling)
        };
        if let Some(parent) = parent {
            self.insert_before(parent, new, next);
            let after = self.index_of(node).saturating_add(1);
            for range in &mut self.ranges {
                for (container, position) in [&mut range.start, &mut range.end] {
                    if *container == node && *position > offset {
                        *container = new;
                        *position -= offset;
                    } else if *container == parent && *position == after {
                        *position += 1;
                    }
                }
            }
        }
        self.replace_data(node, offset, count, "");
        Some(new)
    }

    /// Ajuste les plages avant le retrait de `child`, enfant de `parent`.
    pub(crate) fn ranges_before_removal(&mut self, child: NodeId, parent: NodeId) {
        if self.ranges.is_empty() {
            return;
        }
        let index = self.index_of(child);
        let mut ranges = std::mem::take(&mut self.ranges);
        for range in &mut ranges {
            for (container, position) in [&mut range.start, &mut range.end] {
                if self.contains(child, *container) {
                    *container = parent;
                    *position = index;
                } else if *container == parent && *position > index {
                    *position -= 1;
                }
            }
        }
        self.ranges = ranges;
    }

    /// Ajuste les plages après l'insertion de `child` dans `parent`.
    pub(crate) fn ranges_after_insertion(&mut self, child: NodeId, parent: NodeId) {
        if self.ranges.is_empty() {
            return;
        }
        let index = self.index_of(child);
        for range in &mut self.ranges {
            for (container, position) in [&mut range.start, &mut range.end] {
                if *container == parent && *position > index {
                    *position = position.saturating_add(1);
                }
            }
        }
    }
}
