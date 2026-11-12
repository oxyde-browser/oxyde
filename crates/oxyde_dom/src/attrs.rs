//! Espaces de noms et nœuds d'attribut (DOM Standard §4.9).
//!
//! Les attributs d'un élément restent rangés dans son `ElementData` ; un
//! nœud `Attr` n'est créé que lorsqu'un script en demande un. Tant que
//! l'attribut est porté par un élément, sa valeur vit dans la liste de
//! l'élément et le nœud n'en est que la poignée ; détaché, le nœud garde sa
//! propre valeur.

use crate::{Attribute, Document, Mutation, Namespace, NodeData, NodeId};

/// Espace de noms XHTML.
pub const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
/// Espace de noms SVG.
pub const SVG_NS: &str = "http://www.w3.org/2000/svg";
/// Espace de noms MathML.
pub const MATHML_NS: &str = "http://www.w3.org/1998/Math/MathML";
/// Espace de noms XML.
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// Espace de noms XMLNS.
pub const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

/// Données d'un nœud d'attribut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrData {
    /// Nom local.
    pub local: String,
    /// Préfixe d'espace de noms.
    pub prefix: Option<String>,
    /// Espace de noms.
    pub ns: Namespace,
    /// Valeur, quand l'attribut n'est porté par aucun élément.
    pub value: String,
    /// Élément qui porte l'attribut.
    pub element: Option<NodeId>,
}

impl AttrData {
    /// Nom qualifié : `préfixe:local`, ou le nom local seul.
    #[must_use]
    pub fn qualified(&self) -> String {
        match &self.prefix {
            Some(prefix) => format!("{prefix}:{}", self.local),
            None => self.local.clone(),
        }
    }
}

impl Document {
    /// Espace de noms d'adresse `uri` ; la chaîne vide vaut « aucun ».
    pub fn namespace(&mut self, uri: &str) -> Namespace {
        match uri {
            "" => Namespace::None,
            HTML_NS => Namespace::Html,
            SVG_NS => Namespace::Svg,
            MATHML_NS => Namespace::MathMl,
            XML_NS => Namespace::Xml,
            XMLNS_NS => Namespace::Xmlns,
            other => {
                let index = match self.namespaces.iter().position(|n| n == other) {
                    Some(index) => index,
                    None => {
                        self.namespaces.push(other.to_owned());
                        self.namespaces.len() - 1
                    }
                };
                Namespace::Other(u32::try_from(index).unwrap_or(u32::MAX))
            }
        }
    }

    /// Adresse de l'espace de noms `ns`, `None` pour « aucun ».
    #[must_use]
    pub fn namespace_uri(&self, ns: Namespace) -> Option<&str> {
        match ns {
            Namespace::None => None,
            Namespace::Html => Some(HTML_NS),
            Namespace::Svg => Some(SVG_NS),
            Namespace::MathMl => Some(MATHML_NS),
            Namespace::Xml => Some(XML_NS),
            Namespace::Xmlns => Some(XMLNS_NS),
            Namespace::Other(index) => self.namespaces.get(index as usize).map(String::as_str),
        }
    }

    /// Crée un élément détaché de nom local `local`, avec préfixe.
    pub fn create_element_ns(
        &mut self,
        local: &str,
        ns: Namespace,
        prefix: Option<&str>,
    ) -> NodeId {
        let node = self.create_element(local, ns, Vec::new());
        if let Some(element) = self.element_mut(node) {
            element.prefix = prefix.map(str::to_owned);
        }
        node
    }

    /// Crée un nœud d'attribut détaché, sans valeur.
    pub fn create_attr(&mut self, local: &str, ns: Namespace, prefix: Option<&str>) -> NodeId {
        self.push(NodeData::Attr(AttrData {
            local: local.to_owned(),
            prefix: prefix.map(str::to_owned),
            ns,
            value: String::new(),
            element: None,
        }))
    }

    /// Données du nœud d'attribut `id`.
    #[must_use]
    pub fn attr_data(&self, id: NodeId) -> Option<&AttrData> {
        match &self.node(id).data {
            NodeData::Attr(data) => Some(data),
            _ => None,
        }
    }

    fn attr_data_mut(&mut self, id: NodeId) -> Option<&mut AttrData> {
        match &mut self.node_mut(id).data {
            NodeData::Attr(data) => Some(data),
            _ => None,
        }
    }

    /// Rang, dans `element`, de l'attribut que désigne le nœud `attr`.
    fn attr_slot(&self, attr: NodeId) -> Option<(NodeId, usize)> {
        let element = self.attr_data(attr)?.element?;
        let index = self
            .element(element)?
            .attrs
            .iter()
            .position(|a| a.node == Some(attr))?;
        Some((element, index))
    }

    /// Valeur du nœud d'attribut `id`.
    #[must_use]
    pub fn attr_value(&self, id: NodeId) -> Option<&str> {
        match self.attr_slot(id) {
            Some((element, index)) => self
                .element(element)
                .and_then(|e| e.attrs.get(index))
                .map(|a| a.value.as_str()),
            None => self.attr_data(id).map(|data| data.value.as_str()),
        }
    }

    /// Change la valeur du nœud d'attribut `id`.
    pub fn set_attr_value(&mut self, id: NodeId, value: &str) {
        match self.attr_slot(id) {
            Some((element, index)) => self.set_attr_at(element, index, value),
            None => {
                if let Some(data) = self.attr_data_mut(id) {
                    value.clone_into(&mut data.value);
                }
            }
        }
    }

    /// Rang du premier attribut de `element` de nom qualifié `name`.
    #[must_use]
    pub fn attr_index(&self, element: NodeId, name: &str) -> Option<usize> {
        self.element(element)?
            .attrs
            .iter()
            .position(|a| a.name == name)
    }

    /// Rang de l'attribut de `element` d'espace de noms `ns` et de nom local
    /// `local`.
    #[must_use]
    pub fn attr_index_ns(&self, element: NodeId, ns: Namespace, local: &str) -> Option<usize> {
        self.element(element)?
            .attrs
            .iter()
            .position(|a| a.ns == ns && a.local() == local)
    }

    /// Change la valeur de l'attribut de rang `index`.
    pub fn set_attr_at(&mut self, element: NodeId, index: usize, value: &str) {
        let Some(attr) = self
            .element_mut(element)
            .and_then(|e| e.attrs.get_mut(index))
        else {
            return;
        };
        let old = std::mem::replace(&mut attr.value, value.to_owned());
        let name = attr.local().to_owned();
        self.log(|| Mutation::Attribute {
            target: element,
            name,
            old: Some(old),
        });
    }

    /// Ajoute à `element` l'attribut `attribute`.
    pub fn push_attr(&mut self, element: NodeId, attribute: Attribute) {
        let name = attribute.local().to_owned();
        let Some(data) = self.element_mut(element) else {
            return;
        };
        data.attrs.push(attribute);
        self.log(|| Mutation::Attribute {
            target: element,
            name,
            old: None,
        });
    }

    /// Retire de `element` l'attribut de rang `index` et le rend. Le nœud
    /// `Attr` qui le désignait, s'il existe, garde la valeur et devient
    /// détaché.
    pub fn remove_attr_at(&mut self, element: NodeId, index: usize) -> Option<Attribute> {
        let data = self.element_mut(element)?;
        if index >= data.attrs.len() {
            return None;
        }
        let removed = data.attrs.remove(index);
        if let Some(handle) = removed.node
            && let Some(attr) = self.attr_data_mut(handle)
        {
            attr.value.clone_from(&removed.value);
            attr.element = None;
        }
        let name = removed.local().to_owned();
        let old = removed.value.clone();
        self.log(|| Mutation::Attribute {
            target: element,
            name,
            old: Some(old),
        });
        Some(removed)
    }

    /// Nœud `Attr` de l'attribut de rang `index`, créé à la demande.
    pub fn attr_node(&mut self, element: NodeId, index: usize) -> Option<NodeId> {
        let attribute = self.element(element)?.attrs.get(index)?;
        if let Some(existing) = attribute.node {
            return Some(existing);
        }
        let data = AttrData {
            local: attribute.local().to_owned(),
            prefix: attribute.prefix.clone(),
            ns: attribute.ns,
            value: String::new(),
            element: Some(element),
        };
        let owner = self.node(element).owner;
        let node = self.push(NodeData::Attr(data));
        self.node_mut(node).owner = owner;
        if let Some(slot) = self
            .element_mut(element)
            .and_then(|e| e.attrs.get_mut(index))
        {
            slot.node = Some(node);
        }
        Some(node)
    }

    /// Fait porter par `element` le nœud d'attribut détaché `attr` : il
    /// remplace l'attribut de rang `replaced` s'il est donné, sinon il
    /// s'ajoute à la fin.
    pub fn attach_attr(&mut self, element: NodeId, attr: NodeId, replaced: Option<usize>) {
        let Some(data) = self.attr_data(attr).cloned() else {
            return;
        };
        let attribute = Attribute {
            name: data.qualified(),
            value: data.value.clone(),
            ns: data.ns,
            prefix: data.prefix.clone(),
            node: Some(attr),
        };
        let owner = self.node(element).owner;
        self.node_mut(attr).owner = owner;
        if let Some(handle) = self.attr_data_mut(attr) {
            handle.element = Some(element);
        }
        match replaced {
            Some(index) => {
                // L'ancien nœud, s'il existe, garde sa valeur et se détache.
                let old = self
                    .element(element)
                    .and_then(|e| e.attrs.get(index))
                    .cloned();
                if let Some(old) = &old
                    && let Some(handle) = old.node
                    && let Some(previous) = self.attr_data_mut(handle)
                {
                    previous.value.clone_from(&old.value);
                    previous.element = None;
                }
                let name = attribute.local().to_owned();
                if let Some(slot) = self
                    .element_mut(element)
                    .and_then(|e| e.attrs.get_mut(index))
                {
                    *slot = attribute;
                }
                self.log(|| Mutation::Attribute {
                    target: element,
                    name,
                    old: old.map(|o| o.value),
                });
            }
            None => self.push_attr(element, attribute),
        }
    }
}
