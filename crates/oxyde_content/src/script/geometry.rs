//! Géométrie vue des scripts (CSSOM View) : `getBoundingClientRect`,
//! `offsetWidth`, `clientWidth`, `getComputedStyle`…
//!
//! Une lecture de géométrie exige une mise en page à jour : si le document a
//! changé depuis la dernière, le style et la mise en page sont recalculés sur
//! le champ, comme dans un navigateur (« mise en page forcée »). Le résultat
//! est gardé tant que le document et la fenêtre ne changent pas.
//!
//! LIMITES CONNUES : seul le document de la page est mis en page (pas ceux
//! des cadres) ; rien ne défile, donc `scrollTop` et `scrollLeft` valent 0 et
//! les rectangles sont relatifs à la page ; `getComputedStyle` ne connaît que
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
    /// Boîte de bordure de chaque nœud mis en page, en coordonnées de page.
    boxes: HashMap<u32, Rect>,
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

/// Relève les boîtes de `fragment` : `boxes` pour les boîtes de bloc,
/// `inline` pour les rectangles du texte d'un élément en ligne.
fn collect(fragment: &Fragment, boxes: &mut HashMap<u32, Rect>, inline: &mut HashMap<u32, Rect>) {
    match fragment {
        Fragment::Box(b) => {
            if let Some(node) = b.node {
                boxes
                    .entry(index(node))
                    .and_modify(|rect| *rect = union(*rect, b.border_box))
                    .or_insert(b.border_box);
            }
            for child in &b.children {
                collect(child, boxes, inline);
            }
        }
        Fragment::Text(t) => {
            for child in &t.boxes {
                collect(child, boxes, inline);
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
                        inline
                            .entry(run.style().brush.node)
                            .and_modify(|known| *known = union(*known, rect))
                            .or_insert(rect);
                    }
                }
            }
        }
    }
}

/// Géométrie à jour du document de la page, recalculée si besoin.
fn ensure(vm: &mut Vm) -> Option<&Geometry> {
    let h = host(vm)?;
    let version = h.doc.version();
    let viewport = h.viewport;
    let fresh = h
        .geometry
        .as_ref()
        .is_some_and(|g| g.version == version && g.viewport == viewport);
    if !fresh {
        let env = h.layout_env.as_ref()?;
        let text = h.text.get_or_insert_with(TextContext::new);
        let (styles, tree) = crate::layout_snapshot(&h.doc, env, viewport, text);
        let (mut boxes, mut inline) = (HashMap::new(), HashMap::new());
        if let Some(root) = &tree.root {
            collect(root, &mut boxes, &mut inline);
        }
        // Un élément en ligne n'a pas de boîte : son rectangle est celui de
        // son texte, et celui d'un ancêtre en ligne englobe ses descendants.
        let doc = &h.doc;
        for (node, rect) in inline {
            let mut current = doc.id_from_index(node as usize);
            while let Some(id) = current {
                let key = index(id);
                if boxes.contains_key(&key) && !inline_only(&styles, id) {
                    break;
                }
                boxes
                    .entry(key)
                    .and_modify(|known| *known = union(*known, rect))
                    .or_insert(rect);
                current = doc.node(id).parent;
            }
        }
        h.geometry = Some(Geometry {
            version,
            viewport,
            boxes,
            styles,
            grids: tree.grids,
        });
    }
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

pub(super) fn get_bounding_client_rect(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let rect = border_box(vm, node).unwrap_or_default();
    dom_rect(vm, rect)
}

pub(super) fn get_client_rects(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let node = this_node(vm, this)?;
    let rects = match border_box(vm, node) {
        Some(rect) => vec![dom_rect(vm, rect)?],
        None => Vec::new(),
    };
    Ok(vm.new_array(rects))
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
        return Some(Ok(Value::Number(0.0)));
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
