//! Géométrie vue des scripts (CSSOM View) : `getBoundingClientRect`,
//! `offsetWidth`, `clientWidth`, `getComputedStyle`…
//!
//! Une lecture de géométrie exige une mise en page à jour : si le document a
//! changé depuis la dernière, le style et la mise en page sont recalculés sur
//! le champ, comme dans un navigateur (« mise en page forcée »). Le résultat
//! est gardé tant que le document et la fenêtre ne changent pas.
//!
//! Le défilement (`scrollTop`, `scrollTo`, `scrollIntoView`, `window.scrollY`…)
//! est tenu par l'arbre de défilement que la page partage avec son moteur
//! de script ; il déplace les rectangles sans redemander de mise en page.
//!
//! LIMITES CONNUES : seul le document de la page est mis en page (pas ceux
//! des cadres) ; le défilement est toujours immédiat (`behavior: "smooth"`
//! est ignoré) ; `getComputedStyle` ne connaît que
//! les propriétés du style calculé d'Oxyde et ignore les pseudo-éléments ;
//! `getClientRects` rend un seul rectangle par élément.

use std::collections::HashMap;
use std::sync::Arc;

use oxyde_css::{BorderStyle, Color, ComputedStyle, Display, LineHeight, Size, StyleMap};
use oxyde_dom::{Document, NodeId};
use oxyde_js::{JsResult, Value, Vm};
use oxyde_layout::{Fragment, Rect};
use oxyde_text::TextContext;
use oxyde_text::parley::PositionedLayoutItem;

use super::{CLASS_COMPUTED, arg, css_name, host, node_arg, string, this_node, wrap_opt};

/// Géométrie relevée après une mise en page.
pub(super) struct Geometry {
    /// Version du document et fenêtre pour lesquelles elle vaut.
    version: u64,
    viewport: (f32, f32),
    /// Instant du calcul, si une animation ou une transition y contribuait :
    /// la géométrie ne vaut alors que pour cet instant.
    time: Option<f64>,
    /// Boîte de bordure de chaque nœud mis en page, en coordonnées de page.
    boxes: HashMap<u32, Rect>,
    /// Rectangle englobant, tel qu'il apparaît dans la fenêtre, de chaque
    /// boîte déplacée par un `transform` ou par un défilement (les siens ou
    /// ceux d'un ancêtre).
    transformed: HashMap<u32, Rect>,
    /// Zones qui découpent leur contenu : largeur et hauteur du contenu,
    /// puis de la partie visible.
    extents: HashMap<u32, (f32, f32, f32, f32)>,
    /// Version du défilement pour laquelle les rectangles ont été relevés.
    measured: Option<u64>,
    /// Fragment racine de la mise en page, gardé pour relever de nouveau les
    /// rectangles quand le défilement change.
    root: Option<Fragment>,
    /// Largeur et hauteur du contenu de la page.
    content: (f32, f32),
    /// Styles calculés.
    styles: StyleMap,
    /// Pistes de chaque conteneur en grille.
    grids: HashMap<NodeId, oxyde_layout::GridTracks>,
}

fn index(node: NodeId) -> u32 {
    u32::try_from(node.index()).unwrap_or(u32::MAX)
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect {
        x,
        y,
        w: (a.x + a.w).max(b.x + b.w) - x,
        h: (a.y + a.h).max(b.y + b.h) - y,
    }
}

/// Rectangles relevés dans l'arbre de fragments.
#[derive(Default)]
struct Collected {
    /// Boîtes de bloc, telles que mises en page.
    boxes: HashMap<u32, Rect>,
    /// Rectangles du texte des éléments en ligne.
    inline: HashMap<u32, Rect>,
    /// Les mêmes, tels qu'ils apparaissent dans la fenêtre.
    inline_visual: HashMap<u32, Rect>,
    /// Rectangles englobants des boîtes transformées ou défilées.
    transformed: HashMap<u32, Rect>,
    /// Étendue du contenu et de la partie visible des zones qui découpent.
    extents: HashMap<u32, (f32, f32, f32, f32)>,
}

/// Compose `inner` après la transformation héritée `outer`.
fn compose(
    outer: Option<oxyde_css::Affine>,
    inner: oxyde_css::Affine,
) -> Option<oxyde_css::Affine> {
    Some(match outer {
        Some(outer) => outer.then_inner(inner),
        None => inner,
    })
}

/// Relève les boîtes de `fragment` ; `matrix` est le déplacement hérité des
/// ancêtres (transformations et défilements), s'il y en a un, et `scroll`
/// donne le défilement de chaque zone.
fn collect(
    fragment: &Fragment,
    matrix: Option<oxyde_css::Affine>,
    scroll: &dyn Fn(u32) -> (f32, f32),
    out: &mut Collected,
) {
    use oxyde_css::Affine;
    match fragment {
        Fragment::Box(b) => {
            let mut matrix = matrix;
            // Une boîte fixe ne suit pas le défilement de la fenêtre.
            if b.style.position == oxyde_css::Position::Fixed {
                let (x, y) = scroll(oxyde_compositor::ROOT_SCROLLER);
                if x != 0.0 || y != 0.0 {
                    matrix = compose(matrix, Affine::translate(x, y));
                }
            }
            if let Some(own) = oxyde_layout::box_transform(b) {
                matrix = compose(matrix, own);
            }
            let mut inner = matrix;
            if let Some(node) = b.node {
                out.boxes
                    .entry(index(node))
                    .and_modify(|rect| *rect = union(*rect, b.border_box))
                    .or_insert(b.border_box);
                if let Some(matrix) = matrix {
                    let r = b.border_box;
                    let (x, y, w, h) = matrix.bounds(r.x, r.y, r.w, r.h);
                    let bounds = Rect { x, y, w, h };
                    out.transformed
                        .entry(index(node))
                        .and_modify(|rect| *rect = union(*rect, bounds))
                        .or_insert(bounds);
                }
                if oxyde_layout::clips_content(&b.style) {
                    let port = oxyde_layout::padding_box(b);
                    let (width, height) = oxyde_layout::scrollable_extent(b);
                    out.extents
                        .insert(index(node), (width, height, port.w, port.h));
                    let (x, y) = scroll(crate::scroll_id(node));
                    if x != 0.0 || y != 0.0 {
                        inner = compose(matrix, Affine::translate(-x, -y));
                    }
                }
            }
            for child in &b.children {
                collect(child, inner, scroll, out);
            }
        }
        Fragment::Text(t) => {
            for child in &t.boxes {
                collect(child, matrix, scroll, out);
            }
            for line in t.layout.lines() {
                let metrics = line.metrics();
                for item in line.items() {
                    if let PositionedLayoutItem::GlyphRun(run) = item {
                        let rect = Rect {
                            x: t.x + run.offset(),
                            y: t.y + metrics.block_min_coord,
                            w: run.advance(),
                            h: metrics.block_max_coord - metrics.block_min_coord,
                        };
                        let node = run.style().brush.node;
                        out.inline
                            .entry(node)
                            .and_modify(|known| *known = union(*known, rect))
                            .or_insert(rect);
                        if let Some(matrix) = matrix {
                            let (x, y, w, h) = matrix.bounds(rect.x, rect.y, rect.w, rect.h);
                            let bounds = Rect { x, y, w, h };
                            out.inline_visual
                                .entry(node)
                                .and_modify(|known| *known = union(*known, bounds))
                                .or_insert(bounds);
                        }
                    }
                }
            }
        }
    }
}

/// Relève les rectangles de la mise en page `root` pour le défilement
/// `scroll` et les range dans `geometry`.
fn measure(doc: &Document, geometry: &mut Geometry, scroll: &dyn Fn(u32) -> (f32, f32)) {
    let mut collected = Collected::default();
    if let Some(root) = &geometry.root {
        // Le défilement de la fenêtre déplace toute la page.
        let (x, y) = scroll(oxyde_compositor::ROOT_SCROLLER);
        let matrix = (x != 0.0 || y != 0.0).then(|| oxyde_css::Affine::translate(-x, -y));
        collect(root, matrix, scroll, &mut collected);
    }
    let Collected {
        mut boxes,
        inline,
        inline_visual,
        mut transformed,
        extents,
    } = collected;
    // Un élément en ligne n'a pas de boîte : son rectangle est celui de
    // son texte, et celui d'un ancêtre en ligne englobe ses descendants.
    let styles = &geometry.styles;
    let spread =
        |from: HashMap<u32, Rect>, into: &mut HashMap<u32, Rect>, blocks: &HashMap<u32, Rect>| {
            for (node, rect) in from {
                let mut current = doc.id_from_index(node as usize);
                while let Some(id) = current {
                    let key = index(id);
                    if blocks.contains_key(&key) && !inline_only(styles, id) {
                        break;
                    }
                    into.entry(key)
                        .and_modify(|known| *known = union(*known, rect))
                        .or_insert(rect);
                    current = doc.node(id).parent;
                }
            }
        };
    let blocks = boxes.clone();
    spread(inline, &mut boxes, &blocks);
    spread(inline_visual, &mut transformed, &blocks);
    geometry.boxes = boxes;
    geometry.transformed = transformed;
    geometry.extents = extents;
}

/// Géométrie à jour du document de la page, recalculée si besoin.
fn ensure(vm: &mut Vm) -> Option<&Geometry> {
    let h = host(vm)?;
    let version = h.doc.version();
    let viewport = h.viewport;
    let now = h.layout_env.as_ref().map(|env| env.animations.now());
    let fresh = h.geometry.as_ref().is_some_and(|g| {
        g.version == version && g.viewport == viewport && (g.time.is_none() || g.time == now)
    });
    if !fresh {
        let env = h.layout_env.as_ref()?;
        let text = h.text.get_or_insert_with(TextContext::new);
        let (styles, tree, time) = crate::layout_snapshot(&h.doc, env, viewport, text);
        h.geometry = Some(Geometry {
            version,
            viewport,
            time,
            boxes: HashMap::new(),
            transformed: HashMap::new(),
            extents: HashMap::new(),
            measured: None,
            content: (tree.width, tree.height),
            root: tree.root,
            styles,
            grids: tree.grids,
        });
    }
    // Les rectangles dépendent du défilement : ils sont relevés de nouveau
    // quand il change, sans refaire la mise en page.
    let scrolling = std::sync::Arc::clone(&h.layout_env.as_ref()?.scrolling);
    let tree = scrolling.lock();
    let scroll_version = tree.version();
    let geometry = h.geometry.as_mut()?;
    if geometry.measured != Some(scroll_version) {
        measure(&h.doc, geometry, &|id| tree.offset(id));
        geometry.measured = Some(scroll_version);
    }
    drop(tree);
    host(vm)?.geometry.as_ref()
}

/// Vrai si `node` est un élément en ligne (sans boîte de bloc à lui).
fn inline_only(styles: &StyleMap, node: NodeId) -> bool {
    styles
        .get(node)
        .is_some_and(|style| style.display == Display::Inline)
}

/// Boîte de bordure de `node`, s'il est mis en page.
fn border_box(vm: &mut Vm, node: NodeId) -> Option<Rect> {
    ensure(vm)?.boxes.get(&index(node)).copied()
}

fn style_of(vm: &mut Vm, node: NodeId) -> Option<Arc<ComputedStyle>> {
    ensure(vm)?.styles.get(node).cloned()
}

fn number(value: f32) -> Value {
    Value::Number(f64::from(value))
}

/// Objet `DOMRect` de rectangle `rect`.
fn dom_rect(vm: &mut Vm, rect: Rect) -> JsResult {
    let object = Value::Object(vm.new_object());
    for (key, value) in [
        ("x", rect.x),
        ("y", rect.y),
        ("width", rect.w),
        ("height", rect.h),
        ("top", rect.y),
        ("left", rect.x),
        ("right", rect.x + rect.w),
        ("bottom", rect.y + rect.h),
    ] {
        vm.set(&object, key, number(value))?;
    }
    Ok(object)
}

/// Rectangle de `node` tel qu'il apparaît à l'écran, transformations
/// comprises.
fn visual_box(vm: &mut Vm, node: NodeId) -> Option<Rect> {
    let geometry = ensure(vm)?;
    let key = index(node);
    geometry
        .transformed
        .get(&key)
        .or_else(|| geometry.boxes.get(&key))
        .copied()
}

pub(super) fn get_bounding_client_rect(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let rect = visual_box(vm, node).unwrap_or_default();
    dom_rect(vm, rect)
}

pub(super) fn get_client_rects(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let rects = match visual_box(vm, node) {
        Some(rect) => vec![dom_rect(vm, rect)?],
        None => Vec::new(),
    };
    Ok(vm.new_array(rects))
}

/// Identifiant de défilement de `node` : celui de la fenêtre pour l'élément
/// racine.
fn scroller_of(vm: &mut Vm, node: NodeId) -> Option<u32> {
    let h = host(vm)?;
    h.doc.element(node)?;
    let is_root = h.doc.node(node).parent == Some(h.doc.root());
    Some(if is_root {
        oxyde_compositor::ROOT_SCROLLER
    } else {
        crate::scroll_id(node)
    })
}

/// Défilement de `node`.
fn scroll_offset(vm: &mut Vm, node: NodeId) -> (f32, f32) {
    let Some(id) = scroller_of(vm, node) else {
        return (0.0, 0.0);
    };
    host(vm)
        .and_then(|h| h.layout_env.as_ref())
        .map_or((0.0, 0.0), |env| env.scrolling.lock().offset(id))
}

/// Plus grand défilement de `node` d'après la mise en page courante ;
/// `None` s'il ne défile pas.
fn scroll_limit(vm: &mut Vm, node: NodeId) -> Option<(f32, f32)> {
    let id = scroller_of(vm, node)?;
    let geometry = ensure(vm)?;
    if id == oxyde_compositor::ROOT_SCROLLER {
        return Some((
            (geometry.content.0 - geometry.viewport.0).max(0.0),
            (geometry.content.1 - geometry.viewport.1).max(0.0),
        ));
    }
    let (width, height, port_w, port_h) = *geometry.extents.get(&index(node))?;
    // `overflow: clip` découpe sans défiler.
    let style = geometry.styles.get(node)?;
    let scrollable = |overflow: oxyde_css::Overflow| {
        !matches!(
            overflow,
            oxyde_css::Overflow::Visible | oxyde_css::Overflow::Clip
        )
    };
    (scrollable(style.overflow_x) || scrollable(style.overflow_y))
        .then_some(((width - port_w).max(0.0), (height - port_h).max(0.0)))
}

/// Fait défiler `node` jusqu'à (`x`, `y`) ; une coordonnée absente garde sa
/// valeur, et `relative` les ajoute au défilement courant.
fn scroll_node(vm: &mut Vm, node: NodeId, x: Option<f64>, y: Option<f64>, relative: bool) {
    let (Some(id), Some(max)) = (scroller_of(vm, node), scroll_limit(vm, node)) else {
        return;
    };
    let (old_x, old_y) = scroll_offset(vm, node);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "décalage en pixels : f32 suffit"
    )]
    let target = |wanted: Option<f64>, old: f32| match wanted {
        Some(value) if value.is_finite() && relative => old + value as f32,
        Some(value) if value.is_finite() => value as f32,
        Some(_) => 0.0,
        None => old,
    };
    let (x, y) = (target(x, old_x), target(y, old_y));
    if let Some(env) = host(vm).and_then(|h| h.layout_env.as_ref()) {
        env.scrolling.lock().scroll_to(id, x, y, max);
    }
}

/// Arguments de `scrollTo` et `scrollBy` : `(x, y)` ou `{ left, top }`.
fn scroll_arguments(vm: &mut Vm, args: &[Value]) -> JsResult<(Option<f64>, Option<f64>)> {
    match args {
        [] => Ok((None, None)),
        [options @ Value::Object(_)] => {
            let mut read = |name: &str| -> JsResult<Option<f64>> {
                let value = vm.get(options, name)?;
                if matches!(value, Value::Undefined) {
                    Ok(None)
                } else {
                    vm.to_number(&value).map(Some)
                }
            };
            Ok((read("left")?, read("top")?))
        }
        [x] => Ok((Some(vm.to_number(x)?), None)),
        [x, y, ..] => Ok((Some(vm.to_number(x)?), Some(vm.to_number(y)?))),
    }
}

/// Élément racine du document de la page : il porte le défilement de la
/// fenêtre.
fn root_element(vm: &mut Vm) -> Option<NodeId> {
    let h = host(vm)?;
    h.doc
        .children(h.doc.root())
        .find(|n| h.doc.element(*n).is_some())
}

pub(super) fn set_scroll_property(
    vm: &mut Vm,
    node: NodeId,
    key: &str,
    value: &Value,
) -> JsResult<()> {
    let value = vm.to_number(value)?;
    if key == "scrollTop" {
        scroll_node(vm, node, None, Some(value), false);
    } else {
        scroll_node(vm, node, Some(value), None, false);
    }
    Ok(())
}

pub(super) fn element_scroll_to(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let (x, y) = scroll_arguments(vm, args)?;
    scroll_node(vm, node, x, y, false);
    Ok(Value::Undefined)
}

pub(super) fn element_scroll_by(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let (x, y) = scroll_arguments(vm, args)?;
    scroll_node(vm, node, x, y, true);
    Ok(Value::Undefined)
}

pub(super) fn window_scroll_to(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (x, y) = scroll_arguments(vm, args)?;
    if let Some(root) = root_element(vm) {
        scroll_node(vm, root, x, y, false);
    }
    Ok(Value::Undefined)
}

pub(super) fn window_scroll_by(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let (x, y) = scroll_arguments(vm, args)?;
    if let Some(root) = root_element(vm) {
        scroll_node(vm, root, x, y, true);
    }
    Ok(Value::Undefined)
}

pub(super) fn window_scroll_x(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let x = root_element(vm).map_or(0.0, |root| scroll_offset(vm, root).0);
    Ok(Value::Number(f64::from(x)))
}

pub(super) fn window_scroll_y(vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let y = root_element(vm).map_or(0.0, |root| scroll_offset(vm, root).1);
    Ok(Value::Number(f64::from(y)))
}

/// `scrollIntoView` : chaque zone défilante qui contient l'élément, puis la
/// fenêtre, défile pour l'amener en vue — en haut par défaut, en bas avec
/// `false` ou `{ block: "end" }`, au milieu avec `{ block: "center" }`, et
/// le moins possible avec `{ block: "nearest" }`.
///
/// LIMITE : l'axe horizontal ne défile que si l'élément est hors de vue.
pub(super) fn scroll_into_view(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let block = match args.first() {
        Some(Value::Bool(false)) => "end".to_owned(),
        Some(options @ Value::Object(_)) => {
            let value = vm.get(options, "block")?;
            if matches!(value, Value::Undefined) {
                "start".to_owned()
            } else {
                vm.to_string(&value)?.to_string()
            }
        }
        _ => "start".to_owned(),
    };
    let Some(mut rect) = border_box(vm, node) else {
        return Ok(Value::Undefined);
    };
    // Position à donner au défilement pour qu'un segment [start, start + size[
    // du contenu soit en vue dans une fenêtre de longueur `port` déjà
    // défilée de `offset`.
    let along = |start: f32, size: f32, port: f32, offset: f32, mode: &str| match mode {
        "end" => start + size - port,
        "center" => start + size / 2.0 - port / 2.0,
        "nearest" => {
            if start < offset {
                start
            } else if start + size > offset + port {
                start + size - port
            } else {
                offset
            }
        }
        _ => start,
    };
    let ancestors: Vec<NodeId> = match host(vm) {
        Some(h) => h.doc.ancestors(node).collect(),
        None => return Ok(Value::Undefined),
    };
    for ancestor in ancestors {
        let is_root = scroller_of(vm, ancestor) == Some(oxyde_compositor::ROOT_SCROLLER);
        let Some(max) = scroll_limit(vm, ancestor) else {
            continue;
        };
        // Origine et taille de la partie visible de la zone.
        let (origin, port) = if is_root {
            let viewport = host(vm).map_or((0.0, 0.0), |h| h.viewport);
            ((0.0, 0.0), viewport)
        } else {
            let (Some(outer), Some(style)) = (border_box(vm, ancestor), style_of(vm, ancestor))
            else {
                continue;
            };
            let borders = style.border_widths();
            let Some((_, _, port_w, port_h)) =
                ensure(vm).and_then(|geometry| geometry.extents.get(&index(ancestor)).copied())
            else {
                continue;
            };
            (
                (outer.x + borders.left, outer.y + borders.top),
                (port_w, port_h),
            )
        };
        let (old_x, old_y) = scroll_offset(vm, ancestor);
        let x = along(rect.x - origin.0, rect.w, port.0, old_x, "nearest");
        let y = along(rect.y - origin.1, rect.h, port.1, old_y, &block);
        let Some(id) = scroller_of(vm, ancestor) else {
            continue;
        };
        if let Some(env) = host(vm).and_then(|h| h.layout_env.as_ref()) {
            env.scrolling.lock().scroll_to(id, x, y, max);
        }
        // Pour les zones plus extérieures, l'élément est là où cette zone
        // vient de l'amener.
        let (new_x, new_y) = scroll_offset(vm, ancestor);
        rect.x -= new_x;
        rect.y -= new_y;
    }
    Ok(Value::Undefined)
}

/// Ancêtre par rapport auquel `offsetLeft` et `offsetTop` se mesurent : le
/// plus proche ancêtre positionné, une cellule ou un tableau, sinon `<body>`.
fn offset_parent(doc: &Document, styles: &StyleMap, node: NodeId) -> Option<NodeId> {
    let element = doc.element(node)?;
    if element.is_html("body") || element.is_html("html") {
        return None;
    }
    doc.ancestors(node).find(|ancestor| {
        doc.element(*ancestor).is_some_and(|e| {
            e.is_html("body")
                || styles.get(*ancestor).is_some_and(|style| {
                    style.is_positioned()
                        || matches!(style.display, Display::Table | Display::TableCell)
                })
        })
    })
}

/// Propriété de géométrie `key` de l'élément `node`.
pub(super) fn property(vm: &mut Vm, node: NodeId, key: &str) -> Option<JsResult> {
    if matches!(key, "scrollTop" | "scrollLeft") {
        let (x, y) = scroll_offset(vm, node);
        let value = if key == "scrollTop" { y } else { x };
        return Some(Ok(Value::Number(f64::from(value))));
    }
    let rect = border_box(vm, node);
    let style = style_of(vm, node);
    let h = host(vm)?;
    let is_root = h.doc.node(node).parent == Some(h.doc.root());
    let viewport = h.viewport;
    if key == "offsetParent" {
        let parent = match (&rect, &h.geometry) {
            (Some(_), Some(geometry)) => offset_parent(&h.doc, &geometry.styles, node),
            _ => None,
        };
        return Some(Ok(wrap_opt(vm, parent)));
    }
    let (Some(rect), Some(style)) = (rect, style) else {
        return Some(Ok(Value::Number(0.0)));
    };
    let borders = style.border_widths();
    // Boîte de marge intérieure : la boîte de bordure moins les bordures.
    let (client_w, client_h) = if is_root {
        viewport
    } else if style.display == Display::Inline {
        (0.0, 0.0)
    } else {
        (
            rect.w - borders.left - borders.right,
            rect.h - borders.top - borders.bottom,
        )
    };
    let value = match key {
        "offsetWidth" => rect.w.round(),
        "offsetHeight" => rect.h.round(),
        "clientWidth" => client_w.round(),
        "clientHeight" => client_h.round(),
        "scrollWidth" | "scrollHeight" => {
            // Étendue du contenu : la boîte de marge intérieure, agrandie
            // jusqu'au bord des descendants qui en débordent vers la droite
            // ou vers le bas.
            let geometry = h.geometry.as_ref()?;
            // Zone qui découpe : l'étendue de son contenu défilable.
            // (`overflow` ne s'applique pas à un élément en ligne.)
            if let Some((width, height, _, _)) = geometry
                .extents
                .get(&index(node))
                .filter(|_| !is_root && style.display != Display::Inline)
            {
                let value = if key == "scrollWidth" {
                    *width
                } else {
                    *height
                };
                return Some(Ok(Value::Number(f64::from(value.round()))));
            }
            let (left, top) = (rect.x + borders.left, rect.y + borders.top);
            let (mut right, mut bottom) = (left + client_w, top + client_h);
            for descendant in h.doc.descendants(node) {
                if let Some(inner) = geometry.boxes.get(&index(descendant)) {
                    right = right.max(inner.x + inner.w);
                    bottom = bottom.max(inner.y + inner.h);
                }
            }
            if key == "scrollWidth" {
                (right - left).round()
            } else {
                (bottom - top).round()
            }
        }
        "clientLeft" => borders.left.round(),
        "clientTop" => borders.top.round(),
        "offsetLeft" | "offsetTop" => {
            let geometry = h.geometry.as_ref()?;
            // Origine : le bord intérieur de la bordure du parent de décalage.
            // Un `<body>` non positionné compte pour l'origine de la page,
            // comme dans tous les navigateurs.
            let origin = offset_parent(&h.doc, &geometry.styles, node)
                .and_then(|parent| {
                    let parent_style = geometry.styles.get(parent)?;
                    let is_body = h.doc.element(parent).is_some_and(|e| e.is_html("body"));
                    if is_body && !parent_style.is_positioned() {
                        return None;
                    }
                    let outer = geometry.boxes.get(&index(parent))?;
                    let widths = parent_style.border_widths();
                    Some((outer.x + widths.left, outer.y + widths.top))
                })
                .unwrap_or((0.0, 0.0));
            if key == "offsetLeft" {
                (rect.x - origin.0).round()
            } else {
                (rect.y - origin.1).round()
            }
        }
        _ => return None,
    };
    Some(Ok(number(value)))
}

// ---- getComputedStyle ---------------------------------------------------------------------

fn px(value: f32) -> String {
    // Jusqu'à trois décimales, sans zéros inutiles.
    let rounded = (f64::from(value) * 1000.0).round() / 1000.0;
    format!("{rounded}px")
}

fn color(value: Color) -> String {
    if value.a == 255 {
        format!("rgb({}, {}, {})", value.r, value.g, value.b)
    } else {
        let alpha = (f64::from(value.a) / 255.0 * 1000.0).round() / 1000.0;
        format!("rgba({}, {}, {}, {alpha})", value.r, value.g, value.b)
    }
}

fn size(value: Size, base: f32, auto: &str) -> String {
    match value.resolve(base) {
        Some(resolved) => px(resolved),
        None => auto.to_owned(),
    }
}

fn display_name(display: Display) -> &'static str {
    match display {
        Display::None => "none",
        Display::Block => "block",
        Display::Inline => "inline",
        Display::InlineBlock => "inline-block",
        Display::ListItem => "list-item",
        Display::Flex => "flex",
        Display::Grid => "grid",
        Display::Table => "table",
        Display::TableRowGroup => "table-row-group",
        Display::TableRow => "table-row",
        Display::TableCell => "table-cell",
        Display::TableCaption => "table-caption",
        Display::Contents => "contents",
    }
}

fn border_style_name(style: BorderStyle) -> String {
    format!("{style:?}").to_ascii_lowercase()
}

/// Valeur résolue de la propriété CSS `name` (écrite avec des tirets) pour
/// un élément de style `style`, de boîte de bordure `rect` et dont le bloc
/// conteneur a pour largeur `base`.
/// Liste de noms, ou `default` si elle est vide ou absente.
fn names(list: Option<&[String]>, default: &str) -> String {
    match list {
        Some(list) if !list.is_empty() => list.join(", "),
        _ => default.to_owned(),
    }
}

/// Liste de durées, en secondes ; `0s` si elle est vide ou absente.
fn times(list: Option<&[f32]>) -> String {
    match list {
        Some(list) if !list.is_empty() => list
            .iter()
            .map(|ms| format!("{}s", ms / 1000.0))
            .collect::<Vec<_>>()
            .join(", "),
        _ => "0s".to_owned(),
    }
}

/// Liste de fonctions de temporisation ; `ease` si elle est vide ou absente.
fn easings(list: Option<&[oxyde_css::Easing]>) -> String {
    use oxyde_css::Easing;
    let one = |easing: &Easing| match *easing {
        Easing::Linear => "linear".to_owned(),
        Easing::CubicBezier(0.25, 0.1, 0.25, 1.0) => "ease".to_owned(),
        Easing::CubicBezier(0.42, 0.0, 1.0, 1.0) => "ease-in".to_owned(),
        Easing::CubicBezier(0.0, 0.0, 0.58, 1.0) => "ease-out".to_owned(),
        Easing::CubicBezier(0.42, 0.0, 0.58, 1.0) => "ease-in-out".to_owned(),
        Easing::CubicBezier(x1, y1, x2, y2) => format!("cubic-bezier({x1}, {y1}, {x2}, {y2})"),
        Easing::Steps(count, true) => format!("steps({count}, start)"),
        Easing::Steps(count, false) => format!("steps({count})"),
    };
    match list {
        Some(list) if !list.is_empty() => list.iter().map(one).collect::<Vec<_>>().join(", "),
        _ => "ease".to_owned(),
    }
}

fn resolved(style: &ComputedStyle, rect: Option<Rect>, base: f32, name: &str) -> Option<String> {
    let borders = style.border_widths();
    let pad = |edge: Size| edge.resolve(base).unwrap_or(0.0);
    let laid_out = rect.filter(|_| !matches!(style.display, Display::Inline | Display::None));
    let side = |name: &str| match name.rsplit('-').next() {
        Some("top") => 0,
        Some("right") => 1,
        Some("bottom") => 2,
        _ => 3,
    };
    let edge_of = |edges: [f32; 4], name: &str| edges[side(name)];
    Some(match name {
        "display" => display_name(style.display).to_owned(),
        "color" => color(style.color),
        "background-color" => color(style.background_color),
        "font-size" => px(style.font_size),
        "font-weight" => format!("{}", style.font_weight),
        "font-style" => if style.italic { "italic" } else { "normal" }.to_owned(),
        "font-family" => style.font_family.join(", "),
        "line-height" => match style.line_height {
            LineHeight::Normal => "normal".to_owned(),
            LineHeight::Number(n) => px(n * style.font_size),
            LineHeight::Px(v) => px(v),
        },
        "text-align" => format!("{:?}", style.text_align).to_ascii_lowercase(),
        "white-space" => match format!("{:?}", style.white_space).as_str() {
            "PreWrap" => "pre-wrap".to_owned(),
            "PreLine" => "pre-line".to_owned(),
            other => other.to_ascii_lowercase(),
        },
        "box-sizing" => if style.border_box {
            "border-box"
        } else {
            "content-box"
        }
        .to_owned(),
        "width" => match laid_out {
            Some(rect) => px((rect.w
                - borders.left
                - borders.right
                - pad(style.padding.left)
                - pad(style.padding.right))
            .max(0.0)),
            None => size(style.width, base, "auto"),
        },
        "height" => match laid_out {
            Some(rect) => px((rect.h
                - borders.top
                - borders.bottom
                - pad(style.padding.top)
                - pad(style.padding.bottom))
            .max(0.0)),
            None => size(style.height, base, "auto"),
        },
        "min-width" => size(style.min_width, base, "auto"),
        "min-height" => size(style.min_height, base, "auto"),
        "max-width" => size(style.max_width, base, "none"),
        "max-height" => size(style.max_height, base, "none"),
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            let m = style.margin;
            px(edge_of(
                [pad(m.top), pad(m.right), pad(m.bottom), pad(m.left)],
                name,
            ))
        }
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            let p = style.padding;
            px(edge_of(
                [pad(p.top), pad(p.right), pad(p.bottom), pad(p.left)],
                name,
            ))
        }
        "border-top-width" | "border-right-width" | "border-bottom-width" | "border-left-width" => {
            let trimmed = name.trim_end_matches("-width");
            px(edge_of(
                [borders.top, borders.right, borders.bottom, borders.left],
                trimmed,
            ))
        }
        "border-top-style" | "border-right-style" | "border-bottom-style" | "border-left-style" => {
            let s = style.border_style;
            let all = [s.top, s.right, s.bottom, s.left];
            border_style_name(all[side(name.trim_end_matches("-style"))])
        }
        "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
            let c = style.border_color;
            let all = [c.top, c.right, c.bottom, c.left];
            color(all[side(name.trim_end_matches("-color"))].unwrap_or(style.color))
        }
        "position" => style.position_name().to_owned(),
        "top" | "right" | "bottom" | "left" => {
            let inset = style.inset;
            let all = [inset.top, inset.right, inset.bottom, inset.left];
            size(all[side(name)], base, "auto")
        }
        "float" => format!("{:?}", style.float).to_ascii_lowercase(),
        "clear" => format!("{:?}", style.clear).to_ascii_lowercase(),
        "z-index" => style
            .z_index
            .map_or_else(|| "auto".to_owned(), |z| z.to_string()),
        "overflow-x" => format!("{:?}", style.overflow_x).to_ascii_lowercase(),
        "overflow-y" => format!("{:?}", style.overflow_y).to_ascii_lowercase(),
        "visibility" => format!("{:?}", style.visibility).to_ascii_lowercase(),
        "opacity" => format!("{}", style.opacity),
        "animation-name" => names(style.animation.as_ref().map(|a| a.names.as_slice()), "none"),
        "animation-duration" => times(style.animation.as_ref().map(|a| a.durations.as_slice())),
        "animation-delay" => times(style.animation.as_ref().map(|a| a.delays.as_slice())),
        "animation-timing-function" => {
            easings(style.animation.as_ref().map(|a| a.easings.as_slice()))
        }
        "animation-iteration-count" => match style.animation.as_ref().map(|a| &a.iterations) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|n| {
                    if n.is_finite() {
                        format!("{n}")
                    } else {
                        "infinite".to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => "1".to_owned(),
        },
        "animation-direction" => match style.animation.as_ref().map(|a| &a.directions) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|d| match d {
                    oxyde_css::animation::Direction::Normal => "normal",
                    oxyde_css::animation::Direction::Reverse => "reverse",
                    oxyde_css::animation::Direction::Alternate => "alternate",
                    oxyde_css::animation::Direction::AlternateReverse => "alternate-reverse",
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => "normal".to_owned(),
        },
        "animation-fill-mode" => match style.animation.as_ref().map(|a| &a.fill_modes) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|f| match f {
                    oxyde_css::animation::FillMode::None => "none",
                    oxyde_css::animation::FillMode::Forwards => "forwards",
                    oxyde_css::animation::FillMode::Backwards => "backwards",
                    oxyde_css::animation::FillMode::Both => "both",
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => "none".to_owned(),
        },
        "animation-play-state" => match style.animation.as_ref().map(|a| &a.paused) {
            Some(list) if !list.is_empty() => list
                .iter()
                .map(|paused| if *paused { "paused" } else { "running" })
                .collect::<Vec<_>>()
                .join(", "),
            _ => "running".to_owned(),
        },
        "transition-property" => names(
            style.transition.as_ref().map(|t| t.properties.as_slice()),
            "all",
        ),
        "transition-duration" => times(style.transition.as_ref().map(|t| t.durations.as_slice())),
        "transition-delay" => times(style.transition.as_ref().map(|t| t.delays.as_slice())),
        "transition-timing-function" => {
            easings(style.transition.as_ref().map(|t| t.easings.as_slice()))
        }
        "transform" => {
            let (w, h) = rect.map_or((0.0, 0.0), |r| (r.w, r.h));
            match style.transform_functions_matrix(w, h) {
                Some(matrix) => {
                    let [a, b, c, d, e, f] = matrix.0;
                    format!("matrix({a}, {b}, {c}, {d}, {e}, {f})")
                }
                None => "none".to_owned(),
            }
        }
        "transform-origin" => {
            let (w, h) = rect.map_or((0.0, 0.0), |r| (r.w, r.h));
            format!(
                "{} {}",
                px(style.transform_origin.0.resolve(w).unwrap_or(0.0)),
                px(style.transform_origin.1.resolve(h).unwrap_or(0.0))
            )
        }
        "aspect-ratio" => match style.aspect_ratio {
            Some(ratio) => format!("{ratio} / 1"),
            None => "auto".to_owned(),
        },
        "flex-direction" => match style.flex_direction {
            oxyde_css::FlexDirection::Row => "row",
            oxyde_css::FlexDirection::RowReverse => "row-reverse",
            oxyde_css::FlexDirection::Column => "column",
            oxyde_css::FlexDirection::ColumnReverse => "column-reverse",
        }
        .to_owned(),
        "flex-wrap" => match style.flex_wrap {
            oxyde_css::FlexWrap::NoWrap => "nowrap",
            oxyde_css::FlexWrap::Wrap => "wrap",
            oxyde_css::FlexWrap::WrapReverse => "wrap-reverse",
        }
        .to_owned(),
        "flex-grow" => format!("{}", style.flex_grow),
        "flex-shrink" => format!("{}", style.flex_shrink),
        "flex-basis" => match style.flex_basis {
            None => "content".to_owned(),
            Some(Size::Auto) => "auto".to_owned(),
            Some(Size::Percent(p)) => format!("{p}%"),
            Some(other) => size(other, base, "auto"),
        },
        "order" => style.order.to_string(),
        "row-gap" => size(style.row_gap, base, "normal"),
        "column-gap" => size(style.column_gap, base, "normal"),
        _ => return None,
    })
}

/// `getComputedStyle(element)` : un objet vivant, dont chaque lecture rend la
/// valeur résolue du moment.
pub(super) fn get_computed_style(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let node = node_arg(vm, &arg(args, 0))?;
    let Some(proto) = host(vm).and_then(|h| h.computed_proto) else {
        return Ok(Value::Null);
    };
    Ok(Value::Object(vm.new_host_object(
        proto,
        CLASS_COMPUTED,
        u64::from(index(node)),
    )))
}

/// Valeur de la propriété `name` (avec des tirets) pour le nœud `data`.
fn computed_value(vm: &mut Vm, data: u64, name: &str) -> Option<String> {
    let node = host(vm)?.doc.id_from_index(usize::try_from(data).ok()?)?;
    let rect = border_box(vm, node);
    let style = style_of(vm, node)?;
    // Largeur du bloc conteneur : celle du contenu du parent, à défaut la fenêtre.
    let h = host(vm)?;
    let geometry = h.geometry.as_ref()?;
    // Pistes d'une grille : leurs tailles utilisées, `none` hors d'une grille.
    if matches!(name, "grid-template-columns" | "grid-template-rows") {
        let sizes = geometry.grids.get(&node).map(|tracks| {
            if name == "grid-template-columns" {
                &tracks.columns
            } else {
                &tracks.rows
            }
        });
        return Some(match sizes {
            Some(sizes) if !sizes.is_empty() => sizes
                .iter()
                .map(|size| px(*size))
                .collect::<Vec<_>>()
                .join(" "),
            _ => "none".to_owned(),
        });
    }
    let base = h
        .doc
        .node(node)
        .parent
        .and_then(|parent| {
            let outer = geometry.boxes.get(&index(parent))?;
            let parent_style = geometry.styles.get(parent)?;
            let borders = parent_style.border_widths();
            let pad = |edge: Size| edge.resolve(outer.w).unwrap_or(0.0);
            Some(
                outer.w
                    - borders.left
                    - borders.right
                    - pad(parent_style.padding.left)
                    - pad(parent_style.padding.right),
            )
        })
        .unwrap_or(h.viewport.0);
    resolved(&style, rect, base, name)
}

/// Propriété `key` d'un objet rendu par `getComputedStyle`.
pub(super) fn computed_property(vm: &mut Vm, data: u64, key: &str) -> Option<JsResult> {
    if matches!(key, "getPropertyValue" | "constructor" | "toString") {
        return None;
    }
    let name = css_name(key);
    computed_value(vm, data, &name).map(|value| string(&value))
}

/// `getPropertyValue(nom)` d'un objet rendu par `getComputedStyle`.
pub(super) fn computed_get_property_value(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let name = vm.to_string(&arg(args, 0))?.to_ascii_lowercase();
    let data = vm
        .host_data(this)
        .filter(|(class, _)| *class == CLASS_COMPUTED)
        .map(|(_, data)| data);
    let value = data.and_then(|data| computed_value(vm, data, &name));
    string(&value.unwrap_or_default())
}
