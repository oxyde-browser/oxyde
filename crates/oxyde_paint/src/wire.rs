//! Liste d'affichage sur le fil : sa forme entre le processus de rendu, qui
//! la peint, et le processus navigateur, qui la compose.
//!
//! Les polices et les images sont volumineuses et reviennent d'une image à
//! l'autre : elles ne voyagent qu'une fois. L'expéditeur ([`ResourceSender`])
//! les joint à la première liste qui s'en sert, puis ne les désigne plus que
//! par leur identifiant ; le destinataire ([`ResourceStore`]) les garde.
//!
//! Le destinataire ne fait pas confiance à l'expéditeur : une ressource
//! inconnue, une image de taille incohérente ou un dépassement du plafond de
//! mémoire donnent une erreur de décodage.
//!
//! LIMITE CONNUE : les ressources reçues sont gardées tant que vit le
//! [`ResourceStore`] ; rien ne les évince.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kurbo::{BezPath, PathEl, Point};
use oxyde_css::Color;
use oxyde_ipc::{Bytes, DecodeError, ipc_enum, ipc_struct};
use oxyde_text::parley::FontData;

use crate::{DisplayItem, DisplayList, GlyphPos, GlyphRun, Image, Rect, Scroller};

/// Plafond de mémoire des ressources gardées par un [`ResourceStore`].
const MAX_RESOURCE_BYTES: usize = 1 << 30;

/// Police ou image jointe à une liste d'affichage.
#[derive(Clone, Debug, PartialEq)]
pub enum Resource {
    /// Fichier de police.
    Font {
        /// Identifiant choisi par l'expéditeur.
        id: u64,
        /// Contenu du fichier.
        bytes: Bytes,
    },
    /// Image décodée, en RGBA.
    Image {
        /// Identifiant choisi par l'expéditeur.
        id: u64,
        /// Largeur en pixels.
        width: u32,
        /// Hauteur en pixels.
        height: u32,
        /// `width × height × 4` octets.
        rgba: Bytes,
    },
}
ipc_enum!(Resource {
    0 => Font { id, bytes },
    1 => Image { id, width, height, rgba },
});

/// Élément de tracé : (commande, coordonnées).
type WirePathEl = (u8, [f64; 6]);

/// Zone défilante sur le fil.
#[derive(Clone, Debug, PartialEq)]
struct WireScroller {
    id: u32,
    parent: Option<u32>,
    port: [f32; 4],
    content: [f32; 2],
    wheel_x: bool,
    wheel_y: bool,
}
ipc_struct!(WireScroller {
    id,
    parent,
    port,
    content,
    wheel_x,
    wheel_y
});

/// Primitive de la liste d'affichage sur le fil : comme [`DisplayItem`], les
/// polices et les images étant désignées par leur identifiant.
#[derive(Clone, Debug, PartialEq)]
enum WireItem {
    Rect {
        rect: [f32; 4],
        color: [u8; 4],
        radius: f32,
    },
    StrokeRect {
        rect: [f32; 4],
        color: [u8; 4],
        width: f32,
        radius: f32,
    },
    Glyphs {
        font: u64,
        index: u32,
        size: f32,
        color: [u8; 4],
        glyphs: Vec<(u32, f32, f32)>,
        skew: Option<f32>,
        coords: Vec<i32>,
    },
    Path {
        path: Vec<WirePathEl>,
        x: f32,
        y: f32,
        color: [u8; 4],
        stroke: Option<f32>,
    },
    PushClip {
        rect: [f32; 4],
        radius: f32,
    },
    PushOpacity {
        rect: [f32; 4],
        alpha: f32,
    },
    PopClip,
    PushTransform {
        matrix: [f32; 6],
    },
    PopTransform,
    PushScroll {
        id: u32,
    },
    PushFixed,
    Scrollbar {
        id: u32,
        track: [f32; 4],
        vertical: bool,
        visible: f32,
        total: f32,
    },
    Image {
        rect: [f32; 4],
        image: u64,
    },
}
ipc_enum!(WireItem {
    0 => Rect { rect, color, radius },
    1 => StrokeRect { rect, color, width, radius },
    2 => Glyphs { font, index, size, color, glyphs, skew, coords },
    3 => Path { path, x, y, color, stroke },
    4 => PushClip { rect, radius },
    5 => PushOpacity { rect, alpha },
    6 => PopClip,
    7 => PushTransform { matrix },
    8 => PopTransform,
    9 => PushScroll { id },
    10 => PushFixed,
    11 => Scrollbar { id, track, vertical, visible, total },
    12 => Image { rect, image },
});

/// Liste d'affichage sur le fil, avec les ressources que le destinataire ne
/// connaît pas encore.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WireList {
    /// Polices et images nouvelles pour le destinataire.
    pub resources: Vec<Resource>,
    items: Vec<WireItem>,
    scrollers: Vec<WireScroller>,
}
ipc_struct!(WireList {
    resources,
    items,
    scrollers
});

fn rect_out(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.w, rect.h]
}

fn rect_in(rect: [f32; 4]) -> Rect {
    Rect {
        x: rect[0],
        y: rect[1],
        w: rect[2],
        h: rect[3],
    }
}

fn color_out(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a]
}

fn color_in(color: [u8; 4]) -> Color {
    Color {
        r: color[0],
        g: color[1],
        b: color[2],
        a: color[3],
    }
}

fn path_out(path: &BezPath) -> Vec<WirePathEl> {
    path.elements()
        .iter()
        .map(|element| match *element {
            PathEl::MoveTo(p) => (0, [p.x, p.y, 0.0, 0.0, 0.0, 0.0]),
            PathEl::LineTo(p) => (1, [p.x, p.y, 0.0, 0.0, 0.0, 0.0]),
            PathEl::QuadTo(a, p) => (2, [a.x, a.y, p.x, p.y, 0.0, 0.0]),
            PathEl::CurveTo(a, b, p) => (3, [a.x, a.y, b.x, b.y, p.x, p.y]),
            PathEl::ClosePath => (4, [0.0; 6]),
        })
        .collect()
}

fn path_in(elements: &[WirePathEl]) -> Result<BezPath, DecodeError> {
    let mut path = BezPath::new();
    for (command, c) in elements {
        let point = |i: usize| Point::new(c[i], c[i + 1]);
        path.push(match command {
            0 => PathEl::MoveTo(point(0)),
            1 => PathEl::LineTo(point(0)),
            2 => PathEl::QuadTo(point(0), point(2)),
            3 => PathEl::CurveTo(point(0), point(2), point(4)),
            4 => PathEl::ClosePath,
            other => return Err(DecodeError::UnknownVariant(*other)),
        });
    }
    Ok(path)
}

/// Côté expéditeur : se souvient des ressources déjà envoyées.
#[derive(Debug, Default)]
pub struct ResourceSender {
    fonts: HashSet<u64>,
    images: HashSet<u64>,
}

impl ResourceSender {
    /// Expéditeur qui n'a encore rien envoyé.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Met `list` sous sa forme de fil, en y joignant les polices et les
    /// images que le destinataire n'a pas encore reçues.
    pub fn encode(&mut self, list: &DisplayList) -> WireList {
        let mut resources = Vec::new();
        let items = list
            .items
            .iter()
            .map(|item| match item {
                DisplayItem::Rect {
                    rect,
                    color,
                    radius,
                } => WireItem::Rect {
                    rect: rect_out(*rect),
                    color: color_out(*color),
                    radius: *radius,
                },
                DisplayItem::StrokeRect {
                    rect,
                    color,
                    width,
                    radius,
                } => WireItem::StrokeRect {
                    rect: rect_out(*rect),
                    color: color_out(*color),
                    width: *width,
                    radius: *radius,
                },
                DisplayItem::Glyphs(run) => {
                    let id = run.font.data.id();
                    if self.fonts.insert(id) {
                        resources.push(Resource::Font {
                            id,
                            bytes: Bytes(run.font.data.data().to_vec()),
                        });
                    }
                    WireItem::Glyphs {
                        font: id,
                        index: run.font.index,
                        size: run.size,
                        color: color_out(run.color),
                        glyphs: run.glyphs.iter().map(|g| (g.id, g.x, g.y)).collect(),
                        skew: run.skew,
                        coords: run.coords.iter().map(|c| i32::from(*c)).collect(),
                    }
                }
                DisplayItem::Path {
                    path,
                    x,
                    y,
                    color,
                    stroke,
                } => WireItem::Path {
                    path: path_out(path),
                    x: *x,
                    y: *y,
                    color: color_out(*color),
                    stroke: *stroke,
                },
                DisplayItem::PushClip { rect, radius } => WireItem::PushClip {
                    rect: rect_out(*rect),
                    radius: *radius,
                },
                DisplayItem::PushOpacity { rect, alpha } => WireItem::PushOpacity {
                    rect: rect_out(*rect),
                    alpha: *alpha,
                },
                DisplayItem::PopClip => WireItem::PopClip,
                DisplayItem::PushTransform { matrix } => {
                    WireItem::PushTransform { matrix: *matrix }
                }
                DisplayItem::PopTransform => WireItem::PopTransform,
                DisplayItem::PushScroll { id } => WireItem::PushScroll { id: *id },
                DisplayItem::PushFixed => WireItem::PushFixed,
                DisplayItem::Scrollbar {
                    id,
                    track,
                    vertical,
                    visible,
                    total,
                } => WireItem::Scrollbar {
                    id: *id,
                    track: rect_out(*track),
                    vertical: *vertical,
                    visible: *visible,
                    total: *total,
                },
                DisplayItem::Image { rect, image } => {
                    let data = image.data();
                    let id = data.data.id();
                    if self.images.insert(id) {
                        resources.push(Resource::Image {
                            id,
                            width: data.width,
                            height: data.height,
                            rgba: Bytes(data.data.data().to_vec()),
                        });
                    }
                    WireItem::Image {
                        rect: rect_out(*rect),
                        image: id,
                    }
                }
            })
            .collect();
        let scrollers = list
            .scrollers
            .iter()
            .map(|scroller| WireScroller {
                id: scroller.id,
                parent: scroller.parent.and_then(|p| u32::try_from(p).ok()),
                port: rect_out(scroller.port),
                content: [scroller.content.0, scroller.content.1],
                wheel_x: scroller.wheel.0,
                wheel_y: scroller.wheel.1,
            })
            .collect();
        WireList {
            resources,
            items,
            scrollers,
        }
    }
}

/// Côté destinataire : garde les ressources reçues.
#[derive(Debug, Default)]
pub struct ResourceStore {
    fonts: HashMap<u64, FontData>,
    images: HashMap<u64, Image>,
    /// Octets de ressources gardés.
    bytes: usize,
}

impl ResourceStore {
    /// Magasin vide.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstruit la liste d'affichage reçue, après avoir rangé les
    /// ressources qui l'accompagnent.
    ///
    /// # Errors
    ///
    /// Une [`DecodeError`] si la liste désigne une ressource inconnue, si
    /// une image n'a pas la taille annoncée, si une zone défilante désigne
    /// un parent qui ne la précède pas, ou si le plafond de mémoire des
    /// ressources est dépassé.
    pub fn decode(&mut self, wire: WireList) -> Result<DisplayList, DecodeError> {
        for resource in wire.resources {
            let size = match &resource {
                Resource::Font { bytes, .. } => bytes.0.len(),
                Resource::Image { rgba, .. } => rgba.0.len(),
            };
            self.bytes = self
                .bytes
                .checked_add(size)
                .filter(|total| *total <= MAX_RESOURCE_BYTES)
                .ok_or(DecodeError::InvalidValue)?;
            match resource {
                Resource::Font { id, bytes } => {
                    let blob = peniko::Blob::new(Arc::new(bytes.0));
                    self.fonts.insert(id, FontData::new(blob, 0));
                }
                Resource::Image {
                    id,
                    width,
                    height,
                    rgba,
                } => {
                    let image =
                        Image::from_rgba(width, height, rgba.0).ok_or(DecodeError::InvalidValue)?;
                    self.images.insert(id, image);
                }
            }
        }
        let mut list = DisplayList::default();
        for item in wire.items {
            list.items.push(match item {
                WireItem::Rect {
                    rect,
                    color,
                    radius,
                } => DisplayItem::Rect {
                    rect: rect_in(rect),
                    color: color_in(color),
                    radius,
                },
                WireItem::StrokeRect {
                    rect,
                    color,
                    width,
                    radius,
                } => DisplayItem::StrokeRect {
                    rect: rect_in(rect),
                    color: color_in(color),
                    width,
                    radius,
                },
                WireItem::Glyphs {
                    font,
                    index,
                    size,
                    color,
                    glyphs,
                    skew,
                    coords,
                } => {
                    let data = self.fonts.get(&font).ok_or(DecodeError::InvalidValue)?;
                    let coords: Result<Vec<i16>, _> =
                        coords.into_iter().map(i16::try_from).collect();
                    DisplayItem::Glyphs(GlyphRun {
                        font: FontData::new(data.data.clone(), index),
                        size,
                        color: color_in(color),
                        glyphs: glyphs
                            .into_iter()
                            .map(|(id, x, y)| GlyphPos { id, x, y })
                            .collect(),
                        skew,
                        coords: coords.map_err(|_| DecodeError::InvalidValue)?,
                    })
                }
                WireItem::Path {
                    path,
                    x,
                    y,
                    color,
                    stroke,
                } => DisplayItem::Path {
                    path: path_in(&path)?,
                    x,
                    y,
                    color: color_in(color),
                    stroke,
                },
                WireItem::PushClip { rect, radius } => DisplayItem::PushClip {
                    rect: rect_in(rect),
                    radius,
                },
                WireItem::PushOpacity { rect, alpha } => DisplayItem::PushOpacity {
                    rect: rect_in(rect),
                    alpha,
                },
                WireItem::PopClip => DisplayItem::PopClip,
                WireItem::PushTransform { matrix } => DisplayItem::PushTransform { matrix },
                WireItem::PopTransform => DisplayItem::PopTransform,
                WireItem::PushScroll { id } => DisplayItem::PushScroll { id },
                WireItem::PushFixed => DisplayItem::PushFixed,
                WireItem::Scrollbar {
                    id,
                    track,
                    vertical,
                    visible,
                    total,
                } => DisplayItem::Scrollbar {
                    id,
                    track: rect_in(track),
                    vertical,
                    visible,
                    total,
                },
                WireItem::Image { rect, image } => DisplayItem::Image {
                    rect: rect_in(rect),
                    image: self
                        .images
                        .get(&image)
                        .cloned()
                        .ok_or(DecodeError::InvalidValue)?,
                },
            });
        }
        for (index, scroller) in wire.scrollers.into_iter().enumerate() {
            // Une zone ne peut désigner qu'une zone qui la précède : l'arbre
            // de défilement s'y fie.
            let parent = match scroller.parent {
                None => None,
                Some(parent) => Some(
                    usize::try_from(parent)
                        .ok()
                        .filter(|parent| *parent < index)
                        .ok_or(DecodeError::InvalidValue)?,
                ),
            };
            list.scrollers.push(Scroller {
                id: scroller.id,
                parent,
                port: rect_in(scroller.port),
                content: (scroller.content[0], scroller.content[1]),
                wheel: (scroller.wheel_x, scroller.wheel_y),
            });
        }
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_ipc::{from_bytes, to_bytes};

    fn painted() -> DisplayList {
        let doc = oxyde_html::parse_document(
            "<style>body{margin:0} #z{height:40px;overflow:auto;border:1px solid red}</style>\
             <div id=z><p style='transform:rotate(10deg);opacity:.5'>Bonjour Oxyde</p>\
             <div style='height:200px'></div></div>",
        );
        let viewport = oxyde_css::Viewport {
            width: 300.0,
            height: 200.0,
        };
        let sheets = oxyde_css::collect_style_elements(&doc);
        let styles = oxyde_css::compute_styles(&doc, &sheets, viewport);
        let mut text = oxyde_text::TextContext::new();
        let tree = oxyde_layout::layout_document(&doc, &styles, &mut text, viewport, &|_| None);
        let image = Image::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255]).expect("image");
        let mut list = crate::paint(&tree, viewport, &|_| None);
        list.items.push(DisplayItem::Image {
            rect: Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            },
            image,
        });
        list
    }

    /// Forme comparable d'une liste : ses primitives, polices et images
    /// réduites à leur contenu.
    fn shape(list: &DisplayList) -> Vec<String> {
        list.items
            .iter()
            .map(|item| match item {
                DisplayItem::Glyphs(run) => format!(
                    "glyphes {} octets, index {}, {:?} {:?} {:?}",
                    run.font.data.data().len(),
                    run.font.index,
                    run.size,
                    run.color,
                    run.glyphs
                ),
                DisplayItem::Image { rect, image } => {
                    format!("image {rect:?} {:?}", image.data().data.data())
                }
                other => format!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn display_list_survives_the_wire() {
        let list = painted();
        assert!(
            list.items
                .iter()
                .any(|item| matches!(item, DisplayItem::Glyphs(_))),
            "la page doit contenir du texte"
        );
        let mut sender = ResourceSender::new();
        let mut store = ResourceStore::new();
        let wire = sender.encode(&list);
        assert_eq!(wire.resources.len(), 2, "une police et une image");
        let bytes = to_bytes(&wire);
        let received: WireList = from_bytes(&bytes).expect("décodage");
        let rebuilt = store.decode(received).expect("liste");
        assert_eq!(shape(&rebuilt), shape(&list));
        assert_eq!(rebuilt.scrollers, list.scrollers);
        assert_eq!(rebuilt.scrollers.len(), 1);

        // Seconde image : les ressources ne repartent pas.
        let again = sender.encode(&list);
        assert!(again.resources.is_empty());
        assert!(to_bytes(&again).len() < bytes.len() / 2);
        let rebuilt = store.decode(again).expect("liste");
        assert_eq!(shape(&rebuilt), shape(&list));
    }

    #[test]
    fn unknown_or_inconsistent_resources_are_rejected() {
        let list = painted();
        // Un destinataire neuf ne connaît pas les ressources d'une liste qui
        // ne les joint pas.
        let mut sender = ResourceSender::new();
        sender.encode(&list);
        let without = sender.encode(&list);
        assert_eq!(
            ResourceStore::new().decode(without).err(),
            Some(DecodeError::InvalidValue)
        );
        // Image dont les pixels ne font pas la taille annoncée.
        let lie = WireList {
            resources: vec![Resource::Image {
                id: 1,
                width: 100,
                height: 100,
                rgba: Bytes(vec![0; 8]),
            }],
            ..WireList::default()
        };
        assert_eq!(
            ResourceStore::new().decode(lie).err(),
            Some(DecodeError::InvalidValue)
        );
        // Zone défilante qui se désigne elle-même comme parent.
        let cycle = WireList {
            scrollers: vec![WireScroller {
                id: 1,
                parent: Some(0),
                port: [0.0; 4],
                content: [0.0; 2],
                wheel_x: true,
                wheel_y: true,
            }],
            ..WireList::default()
        };
        assert_eq!(
            ResourceStore::new().decode(cycle).err(),
            Some(DecodeError::InvalidValue)
        );
    }
}
