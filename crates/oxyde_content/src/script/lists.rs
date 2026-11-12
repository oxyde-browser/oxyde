//! Collections vivantes (DOM Standard §4.2.10) : `childNodes`, `children`,
//! `getElementsByTagName`, `getElementsByTagNameNS`, `getElementsByClassName`.
//!
//! Une collection vivante ne garde pas ses nœuds : elle garde la question
//! (« les enfants de tel nœud », « les éléments `p` sous tel nœud ») et y
//! répond à chaque lecture. La réponse est mise en cache tant que le
//! document n'a pas changé.
//!
//! LIMITES CONNUES : une collection n'est jamais libérée avant la fin de la
//! page ; les propriétés nommées d'une `HTMLCollection` ne s'énumèrent pas.

use oxyde_dom::{Document, Namespace, NodeId, QuirksMode};
use oxyde_js::{JsResult, Value, Vm};

use super::documents::{document_of, is_html_document};
use super::{CLASS_LIVE, arg, host, this_node, wrap};

/// Nombre maximal de collections vivantes par page.
const MAX_LISTS: usize = 200_000;

/// La question que pose une collection.
pub(super) enum Query {
    /// Tous les enfants.
    ChildNodes,
    /// Les enfants qui sont des éléments.
    Children,
    /// Les éléments descendants de nom qualifié donné (`*` : tous).
    TagName {
        name: String,
        lowered: String,
        html_document: bool,
    },
    /// Les éléments descendants d'un espace de noms (`None` : tous) et d'un
    /// nom local (`*` : tous).
    TagNameNs {
        ns: Option<Namespace>,
        local: String,
    },
    /// Les éléments descendants qui portent toutes ces classes.
    ClassName(Vec<String>),
}

/// Une collection vivante.
pub(super) struct LiveList {
    root: NodeId,
    query: Query,
    /// Vrai pour une `HTMLCollection` (propriétés nommées).
    named: bool,
    /// Réponse et version du document pour laquelle elle vaut.
    cache: Option<(u64, Vec<NodeId>)>,
}

fn evaluate(doc: &Document, root: NodeId, query: &Query) -> Vec<NodeId> {
    match query {
        Query::ChildNodes => doc.children(root).collect(),
        Query::Children => doc
            .children(root)
            .filter(|c| doc.element(*c).is_some())
            .collect(),
        Query::TagName {
            name,
            lowered,
            html_document,
        } => doc
            .descendants(root)
            .filter(|n| {
                doc.element(*n).is_some_and(|e| {
                    name == "*"
                        || if *html_document && e.ns == Namespace::Html {
                            e.qualified() == *lowered
                        } else {
                            e.qualified() == *name
                        }
                })
            })
            .collect(),
        Query::TagNameNs { ns, local } => doc
            .descendants(root)
            .filter(|n| {
                doc.element(*n).is_some_and(|e| {
                    ns.is_none_or(|ns| e.ns == ns) && (local == "*" || e.name == *local)
                })
            })
            .collect(),
        Query::ClassName(wanted) => {
            // En mode de compatibilité, les classes se comparent sans casse.
            let quirks = doc.quirks_mode == QuirksMode::Quirks;
            let same = |a: &str, b: &str| {
                if quirks {
                    a.eq_ignore_ascii_case(b)
                } else {
                    a == b
                }
            };
            doc.descendants(root)
                .filter(|n| {
                    doc.element(*n).is_some_and(|e| {
                        !wanted.is_empty() && wanted.iter().all(|w| e.classes().any(|c| same(c, w)))
                    })
                })
                .collect()
        }
    }
}

/// Crée une collection vivante et rend son objet.
fn new_list(vm: &mut Vm, root: NodeId, query: Query, named: bool) -> JsResult {
    let interface = if named { "HTMLCollection" } else { "NodeList" };
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    if h.lists.len() >= MAX_LISTS {
        return Err(vm.error("RangeError", "trop de collections créées par cette page"));
    }
    let Some(proto) = h.protos.get(interface).copied() else {
        return Ok(Value::Null);
    };
    h.lists.push(LiveList {
        root,
        query,
        named,
        cache: None,
    });
    let index = (h.lists.len() - 1) as u64;
    Ok(Value::Object(vm.new_host_object(proto, CLASS_LIVE, index)))
}

/// Nœuds de la collection `data`, à jour.
fn nodes(vm: &mut Vm, data: u64) -> Option<Vec<NodeId>> {
    let h = host(vm)?;
    let version = h.doc.version();
    let list = h.lists.get_mut(usize::try_from(data).ok()?)?;
    if let Some((cached, nodes)) = &list.cache
        && *cached == version
    {
        return Some(nodes.clone());
    }
    let fresh = evaluate(&h.doc, list.root, &list.query);
    list.cache = Some((version, fresh.clone()));
    Some(fresh)
}

/// Collection `childNodes` ou `children` de `node` : toujours le même objet.
pub(super) fn same_object(vm: &mut Vm, node: NodeId, elements: bool) -> JsResult {
    let slot = if elements {
        "nœud.children"
    } else {
        "nœud.childNodes"
    };
    let wrapper = wrap(vm, node);
    let Some(wrapper_id) = wrapper.as_object() else {
        return Ok(Value::Null);
    };
    if let Some(existing) = vm.slot(wrapper_id, slot) {
        return Ok(existing);
    }
    let query = if elements {
        Query::Children
    } else {
        Query::ChildNodes
    };
    let list = new_list(vm, node, query, elements)?;
    vm.set_slot(wrapper_id, slot, list.clone());
    Ok(list)
}

/// Propriété `key` de la collection `data` : `length`, un rang, ou — pour
/// une `HTMLCollection` — l'identifiant ou le nom d'un élément.
pub(super) fn property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    let found = nodes(vm, data)?;
    if key == "length" {
        return Some(Ok(super::number(found.len())));
    }
    if let Ok(index) = key.parse::<usize>() {
        // Un rang hors de la collection n'est pas une propriété.
        return found.get(index).map(|node| Ok(wrap(vm, *node)));
    }
    let h = host(vm)?;
    let list = h.lists.get(usize::try_from(data).ok()?)?;
    if !list.named || key.is_empty() {
        return None;
    }
    let named = found.iter().copied().find(|node| {
        h.doc.element(*node).is_some_and(|e| {
            e.attr("id") == Some(key) || (e.ns == Namespace::Html && e.attr("name") == Some(key))
        })
    })?;
    // Une méthode du prototype n'est jamais masquée par un élément nommé.
    let proto = h.protos.get("HTMLCollection").copied()?;
    if !matches!(vm.get(&Value::Object(proto), key), Ok(Value::Undefined)) {
        return None;
    }
    Some(Ok(wrap(vm, named)))
}

/// Clés propres de la collection `data` : ses rangs, puis — pour une
/// `HTMLCollection` — les identifiants et les noms de ses éléments.
pub(super) fn keys(vm: &mut Vm, data: u64) -> Vec<String> {
    let found = nodes(vm, data).unwrap_or_default();
    let mut keys: Vec<String> = (0..found.len()).map(|index| index.to_string()).collect();
    let Some(h) = host(vm) else { return keys };
    let named = usize::try_from(data)
        .ok()
        .and_then(|index| h.lists.get(index))
        .is_some_and(|list| list.named);
    if named {
        for node in found {
            let Some(element) = h.doc.element(node) else {
                continue;
            };
            let name = (element.ns == Namespace::Html)
                .then(|| element.attr("name"))
                .flatten();
            for key in [element.attr("id"), name].into_iter().flatten() {
                if !key.is_empty() && !keys.iter().any(|k| k == key) {
                    keys.push(key.to_owned());
                }
            }
        }
    }
    keys
}

pub(super) fn get_elements_by_tag_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let root = this_node(vm, this)?;
    if args.is_empty() {
        return Err(vm.error("TypeError", "getElementsByTagName attend un argument"));
    }
    let name = vm.to_string(&arg(args, 0))?.to_string();
    let document = document_of(vm, root);
    let query = Query::TagName {
        lowered: name.to_ascii_lowercase(),
        name,
        html_document: is_html_document(vm, document),
    };
    new_list(vm, root, query, true)
}

pub(super) fn get_elements_by_tag_name_ns(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let root = this_node(vm, this)?;
    if args.len() < 2 {
        return Err(vm.error("TypeError", "getElementsByTagNameNS attend deux arguments"));
    }
    let namespace = match arg(args, 0) {
        Value::Null | Value::Undefined => String::new(),
        other => vm.to_string(&other)?.to_string(),
    };
    let local = vm.to_string(&arg(args, 1))?.to_string();
    let ns = if namespace == "*" {
        None
    } else {
        Some(host(vm).map_or(Namespace::None, |h| h.doc.namespace(&namespace)))
    };
    new_list(vm, root, Query::TagNameNs { ns, local }, true)
}

pub(super) fn get_elements_by_class_name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let root = this_node(vm, this)?;
    if args.is_empty() {
        return Err(vm.error("TypeError", "getElementsByClassName attend un argument"));
    }
    let names = vm.to_string(&arg(args, 0))?;
    let wanted = names.split_ascii_whitespace().map(str::to_owned).collect();
    new_list(vm, root, Query::ClassName(wanted), true)
}
