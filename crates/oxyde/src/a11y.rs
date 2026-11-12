//! Accessibilité : expose l'interface aux technologies d'assistance via
//! AccessKit (UI Automation sous Windows).
//!
//! LIMITE : seul le pourtour du navigateur est exposé (boutons, onglets,
//! barre d'adresse, pages internes). Le contenu des pages web apparaît comme
//! un document nommé d'après son titre, sans arbre détaillé.

use accesskit::{Action, Node, NodeId, Rect, Role, TreeId, TreeInfo, TreeUpdate};
use oxyde_ui::{Chrome, Hit, Role as UiRole};

/// Identifiant du nœud racine (la fenêtre).
const ROOT: NodeId = NodeId(0);

/// Identifiant du nœud correspondant au widget d'index `index`.
fn widget_id(index: usize) -> NodeId {
    // `usize` tient toujours dans `u64` sur les plateformes prises en charge.
    NodeId(index as u64 + 1)
}

/// Action de l'interface désignée par un nœud de l'arbre, si c'en est une.
pub(crate) fn hit_for(chrome: &Chrome, node: NodeId) -> Option<Hit> {
    let index = usize::try_from(node.0.checked_sub(1)?).ok()?;
    chrome.widgets.get(index)?.hit
}

/// Arbre d'accessibilité complet de la fenêtre. `scale` convertit les pixels
/// logiques de l'interface en pixels physiques ; `editing` indique si la
/// barre d'adresse a le focus.
pub(crate) fn tree_update(chrome: &Chrome, scale: f64, title: &str, editing: bool) -> TreeUpdate {
    let mut nodes = Vec::with_capacity(chrome.widgets.len() + 1);
    let mut root = Node::new(Role::Window);
    root.set_label(title);
    root.set_children((0..chrome.widgets.len()).map(widget_id).collect::<Vec<_>>());
    nodes.push((ROOT, root));

    let mut focus = ROOT;
    for (index, widget) in chrome.widgets.iter().enumerate() {
        let mut node = Node::new(match widget.role {
            UiRole::Button => Role::Button,
            UiRole::TextInput => Role::TextInput,
            UiRole::Tab => Role::Tab,
            UiRole::Document => Role::Document,
            UiRole::Heading => Role::Heading,
            UiRole::Text => Role::Label,
            UiRole::Switch => Role::Switch,
        });
        node.set_label(widget.label.as_str());
        let r = widget.rect;
        node.set_bounds(Rect {
            x0: f64::from(r.x) * scale,
            y0: f64::from(r.y) * scale,
            x1: f64::from(r.x + r.w) * scale,
            y1: f64::from(r.y + r.h) * scale,
        });
        if widget.hit.is_some() {
            node.add_action(Action::Click);
        }
        if widget.role == UiRole::TextInput {
            node.add_action(Action::Focus);
            // Le premier champ portant une valeur est celui en cours de saisie.
            if editing && focus == ROOT && widget.value.is_some() {
                focus = widget_id(index);
            }
        }
        if widget.selected {
            node.set_selected(true);
        }
        if widget.disabled {
            node.set_disabled();
        }
        if let Some(value) = &widget.value {
            node.set_value(value.as_str());
        }
        nodes.push((widget_id(index), node));
    }
    TreeUpdate {
        nodes,
        tree: Some(TreeInfo::new(ROOT)),
        tree_id: TreeId::ROOT,
        focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_paint::Rect as UiRect;
    use oxyde_ui::Widget;

    fn widget(role: UiRole, label: &str, hit: Option<Hit>) -> Widget {
        Widget {
            hit,
            role,
            label: label.to_owned(),
            rect: UiRect {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
            },
            selected: false,
            disabled: false,
            value: None,
        }
    }

    #[test]
    fn tree_mirrors_widgets() {
        let mut chrome = Chrome::default();
        chrome
            .widgets
            .push(widget(UiRole::Button, "Actualiser", Some(Hit::Reload)));
        let mut address = widget(UiRole::TextInput, "Adresse", Some(Hit::Address));
        address.value = Some("a.test".to_owned());
        chrome.widgets.push(address);
        chrome.widgets.push(widget(UiRole::Heading, "Titre", None));

        let update = tree_update(&chrome, 2.0, "Oxyde", true);
        assert_eq!(update.nodes.len(), 4);
        let (root_id, root) = &update.nodes[0];
        assert_eq!((*root_id, root.role()), (ROOT, Role::Window));
        assert_eq!(root.children(), [NodeId(1), NodeId(2), NodeId(3)]);
        let (_, button) = &update.nodes[1];
        assert_eq!(button.label(), Some("Actualiser"));
        assert_eq!(
            button.bounds(),
            Some(Rect {
                x0: 20.0,
                y0: 40.0,
                x1: 80.0,
                y1: 120.0
            })
        );
        assert_eq!(update.nodes[2].1.value(), Some("a.test"));
        assert_eq!(
            update.focus,
            NodeId(2),
            "le champ en cours de saisie a le focus"
        );
        assert_eq!(tree_update(&chrome, 1.0, "Oxyde", false).focus, ROOT);

        assert_eq!(hit_for(&chrome, NodeId(1)), Some(Hit::Reload));
        assert_eq!(
            hit_for(&chrome, NodeId(3)),
            None,
            "un titre n'est pas actionnable"
        );
        assert_eq!(hit_for(&chrome, ROOT), None);
        assert_eq!(hit_for(&chrome, NodeId(99)), None);
    }
}
