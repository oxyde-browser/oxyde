//! Canevas 2D : ce que le moteur fournit à `canvas.js`.
//!
//! Chaque `<canvas>` qui a un contexte de dessin a ici sa surface de pixels
//! ([`Surface`]), dessinée par `tiny-skia` (remplissages, contours,
//! dégradés, motifs, modes de fusion, découpe). Le script tient l'état du
//! contexte (transformation, styles, tracé courant) et passe à chaque
//! opération tout ce qu'il lui faut, sous forme de textes compacts :
//!
//! - un tracé : `M x y`, `L x y`, `Q x1 y1 x y`, `C x1 y1 x2 y2 x y`, `Z`,
//!   les verbes écrits `0` à `4`, en coordonnées de la surface ;
//! - une peinture : `c r g b a` (couleur), `l x0 y0 x1 y1 …arrêts`
//!   (dégradé linéaire), `r x0 y0 r0 x1 y1 r1 …arrêts` (dégradé radial), `p
//!   nature source` (motif), un arrêt étant `position r g b a` ;
//! - un état : `alpha fusion découpe a b c d e f` (transformation courante).
//!
//! Le texte est mis en forme par `parley` (une ligne), le contour de ses
//! glyphes lu par `skrifa`, puis rempli ou tracé comme un tracé ordinaire.
//!
//! LIMITES CONNUES : le texte se dessine par ses contours, sans ajustement à
//! la grille ni glyphes en couleur (émoji) ; ni ombres ni filtres ; un
//! dégradé radial dont le cercle de départ a un rayon non
//! nul est approché ; pas de dégradé conique ; un motif se répète toujours
//! dans les deux sens ; les couleurs sont en sRGB 8 bits prémultiplié (une
//! couleur très transparente perd de la précision) ; la somme des surfaces
//! d'une page est bornée.

use oxyde_js::{JsResult, Value, Vm};
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient,
    Mask, Paint, Path, PathBuilder, Pattern, Pixmap, PixmapPaint, Point, RadialGradient, Shader,
    SpreadMode, Stroke, StrokeDash, Transform,
};

use super::{arg, host};

/// Côté maximal d'une surface, en pixels, et nombre total de pixels que les
/// surfaces d'une page peuvent occuper (256 Mio).
const MAX_SIDE: u32 = 8192;
const MAX_TOTAL_PIXELS: u64 = 64 * 1024 * 1024;
/// Premier identifiant des surfaces qui n'appartiennent pas à un élément
/// (voir `canvas.js`).
const DETACHED_IDS: u32 = 3_000_000_000;
/// Nombre maximal de zones de découpe gardées par surface.
const MAX_CLIPS: usize = 4096;

/// Surface de dessin d'un canevas.
pub(super) struct Surface {
    pixmap: Pixmap,
    /// Zones de découpe créées par `clip()` ; leur rang, plus un, est
    /// l'identifiant que le script garde dans l'état du contexte.
    clips: Vec<Mask>,
    /// Vrai si les pixels ont changé depuis la dernière remise à la page.
    dirty: bool,
}

/// Identifiant de surface passé par le script : l'index du nœud `<canvas>`.
fn surface_id(vm: &mut Vm, value: &Value) -> JsResult<u32> {
    let number = vm.to_number(value)?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée par le filtre"
    )]
    Ok(if number.is_finite() && (0.0..4.0e9).contains(&number) {
        number as u32
    } else {
        u32::MAX
    })
}

/// Nombres d'un texte séparés par des blancs ; un nombre illisible ou non
/// fini vaut `None`.
fn numbers(text: &str) -> Option<Vec<f32>> {
    text.split_ascii_whitespace()
        .map(|part| part.parse::<f32>().ok().filter(|n| n.is_finite()))
        .collect()
}

/// Tracé décrit par `text` (voir l'en-tête du module).
fn parse_path(text: &str) -> Option<Path> {
    let values = numbers(text)?;
    let mut builder = PathBuilder::new();
    let mut rest = values.as_slice();
    while let [verb, tail @ ..] = rest {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "verbe : un petit entier"
        )]
        let verb = *verb as u8;
        rest = match (verb, tail) {
            (0, [x, y, tail @ ..]) => {
                builder.move_to(*x, *y);
                tail
            }
            (1, [x, y, tail @ ..]) => {
                builder.line_to(*x, *y);
                tail
            }
            (2, [x1, y1, x, y, tail @ ..]) => {
                builder.quad_to(*x1, *y1, *x, *y);
                tail
            }
            (3, [x1, y1, x2, y2, x, y, tail @ ..]) => {
                builder.cubic_to(*x1, *y1, *x2, *y2, *x, *y);
                tail
            }
            (4, tail) => {
                builder.close();
                tail
            }
            _ => return None,
        };
    }
    builder.finish()
}

/// État du contexte passé avec une opération.
struct State {
    alpha: f32,
    blend: BlendMode,
    /// La fusion touche aussi les pixels hors de la forme dessinée.
    whole_surface: bool,
    clip: usize,
    transform: Transform,
}

fn parse_state(text: &str) -> Option<State> {
    let values = numbers(text)?;
    let [alpha, blend, clip, a, b, c, d, e, f] = values.as_slice() else {
        return None;
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rangs : de petits entiers positifs"
    )]
    let (blend, clip) = (*blend as u8, *clip as usize);
    let (blend, whole_surface) = match blend {
        0 => (BlendMode::SourceOver, false),
        1 => (BlendMode::SourceIn, true),
        2 => (BlendMode::SourceOut, true),
        3 => (BlendMode::SourceAtop, false),
        4 => (BlendMode::DestinationOver, false),
        5 => (BlendMode::DestinationIn, true),
        6 => (BlendMode::DestinationOut, false),
        7 => (BlendMode::DestinationAtop, true),
        8 => (BlendMode::Plus, false),
        9 => (BlendMode::Source, true),
        10 => (BlendMode::Xor, false),
        11 => (BlendMode::Multiply, false),
        12 => (BlendMode::Screen, false),
        13 => (BlendMode::Overlay, false),
        14 => (BlendMode::Darken, false),
        15 => (BlendMode::Lighten, false),
        16 => (BlendMode::ColorDodge, false),
        17 => (BlendMode::ColorBurn, false),
        18 => (BlendMode::HardLight, false),
        19 => (BlendMode::SoftLight, false),
        20 => (BlendMode::Difference, false),
        21 => (BlendMode::Exclusion, false),
        22 => (BlendMode::Hue, false),
        23 => (BlendMode::Saturation, false),
        24 => (BlendMode::Color, false),
        25 => (BlendMode::Luminosity, false),
        // `clearRect` : efface sous la forme.
        26 => (BlendMode::Clear, false),
        _ => return None,
    };
    Some(State {
        alpha: alpha.clamp(0.0, 1.0),
        blend,
        whole_surface,
        clip,
        transform: Transform::from_row(*a, *b, *c, *d, *e, *f),
    })
}

/// Couleur `r g b a` (composantes de 0 à 255, opacité de 0 à 1).
fn color(values: &[f32]) -> Option<Color> {
    let [r, g, b, a] = values else {
        return None;
    };
    Color::from_rgba(
        (r / 255.0).clamp(0.0, 1.0),
        (g / 255.0).clamp(0.0, 1.0),
        (b / 255.0).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
}

/// Arrêts d'un dégradé : `position r g b a`, répétés. La position est
/// ramenée de `[0, 1]` à `[start, 1]` (cercle de départ d'un dégradé radial).
fn stops(values: &[f32], start: f32) -> Option<Vec<GradientStop>> {
    let (chunks, rest) = values.as_chunks::<5>();
    if !rest.is_empty() {
        return None;
    }
    chunks
        .iter()
        .map(|[position, rgba @ ..]| {
            Some(GradientStop::new(
                start + position.clamp(0.0, 1.0) * (1.0 - start),
                color(rgba)?,
            ))
        })
        .collect()
}

/// Peinture décrite par `text` ; les motifs lisent leur source dans
/// `sources` (une copie des pixels, prise avant de dessiner). `None` si la
/// peinture ne dessine rien (dégradé sans arrêt, source absente).
fn parse_shader<'a>(
    text: &str,
    transform: Transform,
    source: Option<&'a Pixmap>,
) -> Option<Shader<'a>> {
    let (kind, rest) = text
        .trim_start()
        .split_once(' ')
        .unwrap_or((text.trim(), ""));
    match kind {
        "c" => Some(Shader::SolidColor(color(&numbers(rest)?)?)),
        "l" => {
            let values = numbers(rest)?;
            let [x0, y0, x1, y1, tail @ ..] = values.as_slice() else {
                return None;
            };
            // Début et fin confondus : le dégradé ne peint rien.
            if x0 == x1 && y0 == y1 {
                return None;
            }
            let stops = stops(tail, 0.0)?;
            LinearGradient::new(
                Point::from_xy(*x0, *y0),
                Point::from_xy(*x1, *y1),
                stops,
                SpreadMode::Pad,
                transform,
            )
        }
        "r" => {
            let values = numbers(rest)?;
            let [x0, y0, r0, x1, y1, r1, tail @ ..] = values.as_slice() else {
                return None;
            };
            // Deux cercles confondus : le dégradé ne peint rien.
            if x0 == x1 && y0 == y1 && r0 == r1 {
                return None;
            }
            let start = if *r1 > 0.0 {
                (r0 / r1).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let stops = stops(tail, start)?;
            RadialGradient::new(
                Point::from_xy(*x0, *y0),
                Point::from_xy(*x1, *y1),
                *r1,
                stops,
                SpreadMode::Pad,
                transform,
            )
        }
        "p" => Some(Pattern::new(
            source?.as_ref(),
            SpreadMode::Repeat,
            FilterQuality::Bilinear,
            1.0,
            transform,
        )),
        _ => None,
    }
}

/// Source de pixels que nomme une peinture `p nature id` ou un `drawImage` :
/// une copie des pixels d'un autre canevas (`c`) ou d'une image de la page
/// (`i`).
fn source_pixmap(vm: &mut Vm, kind: &str, id: u32) -> Option<Pixmap> {
    let h = host(vm)?;
    match kind {
        "c" => h.canvases.get(&id).map(|surface| surface.pixmap.clone()),
        "i" => {
            let node = h.doc.id_from_index(id as usize)?;
            let image = h.layout_env.as_ref()?.pictures.get(&node)?;
            let data = image.data();
            let mut pixmap = Pixmap::new(data.width, data.height)?;
            // Les images de la page ont un alpha non prémultiplié.
            for (out, rgba) in pixmap
                .data_mut()
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(data.data.data().as_chunks::<4>().0)
            {
                *out = premultiply(*rgba);
            }
            Some(pixmap)
        }
        _ => None,
    }
}

/// Pixel prémultiplié par son alpha.
fn premultiply([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let scale = |c: u8| {
        let product = u16::from(c) * u16::from(a) + 127;
        u8::try_from(product / 255).unwrap_or(u8::MAX)
    };
    [scale(r), scale(g), scale(b), a]
}

/// Pixel rendu à un alpha non prémultiplié.
fn unpremultiply([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    if a == 0 {
        return [0, 0, 0, 0];
    }
    let scale = |c: u8| {
        let value = (u16::from(c) * 255 + u16::from(a) / 2) / u16::from(a);
        u8::try_from(value.min(255)).unwrap_or(u8::MAX)
    };
    [scale(r), scale(g), scale(b), a]
}

/// `canvasResize(id, largeur, hauteur)` : crée la surface du canevas, ou la
/// remet à zéro (transparente, sans découpe). Rend `false` si la taille est
/// refusée.
pub(super) fn resize(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let width = vm.to_number(&arg(args, 1))?;
    let height = vm.to_number(&arg(args, 2))?;
    let side = |n: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "valeur bornée par le filtre"
        )]
        Some(n)
            .filter(|n| n.is_finite() && (1.0..=f64::from(MAX_SIDE)).contains(n))
            .map(|n| n as u32)
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    h.canvases.remove(&id);
    let (Some(width), Some(height)) = (side(width), side(height)) else {
        return Ok(Value::Bool(false));
    };
    let used: u64 = h
        .canvases
        .values()
        .map(|s| u64::from(s.pixmap.width()) * u64::from(s.pixmap.height()))
        .sum();
    if used + u64::from(width) * u64::from(height) > MAX_TOTAL_PIXELS {
        return Ok(Value::Bool(false));
    }
    let Some(pixmap) = Pixmap::new(width, height) else {
        return Ok(Value::Bool(false));
    };
    h.canvases.insert(
        id,
        Surface {
            pixmap,
            clips: Vec::new(),
            dirty: true,
        },
    );
    h.dirty = true;
    Ok(Value::Bool(true))
}

/// Dessine sur la surface `id` : `shape` trace la forme sur le canevas qu'on
/// lui donne, avec la peinture et la découpe qu'on lui donne. Quand la
/// fusion touche toute la surface, ou qu'une opacité s'applique à une
/// peinture qui n'est pas une couleur, la forme est tracée sur un calque,
/// fondu ensuite dans la surface.
fn draw(
    vm: &mut Vm,
    id: u32,
    state: &State,
    paint_text: &str,
    shape: impl Fn(&mut Pixmap, &Paint<'_>, Option<&Mask>),
) {
    // Source d'un motif : copiée avant d'emprunter la surface.
    let source = paint_text
        .trim_start()
        .strip_prefix("p ")
        .and_then(|rest| {
            let (kind, source) = rest.trim().split_once(' ')?;
            Some((kind.to_owned(), source.trim().parse::<u32>().ok()?))
        })
        .and_then(|(kind, source)| source_pixmap(vm, &kind, source));
    let Some(shader) = parse_shader(paint_text, state.transform, source.as_ref()) else {
        return;
    };
    let Some(h) = host(vm) else {
        return;
    };
    let Some(surface) = h.canvases.get_mut(&id) else {
        return;
    };
    let clip = state
        .clip
        .checked_sub(1)
        .and_then(|at| surface.clips.get(at));
    // Une découpe demandée mais introuvable ne laisse rien passer.
    if state.clip > 0 && clip.is_none() {
        return;
    }
    let solid = matches!(shader, Shader::SolidColor(_));
    let layered = state.whole_surface || (state.alpha < 1.0 && !solid);
    let mut paint = Paint {
        shader,
        anti_alias: true,
        ..Paint::default()
    };
    if layered {
        let Some(mut layer) = Pixmap::new(surface.pixmap.width(), surface.pixmap.height()) else {
            return;
        };
        shape(&mut layer, &paint, None);
        surface.pixmap.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint {
                opacity: state.alpha,
                blend_mode: state.blend,
                quality: FilterQuality::Nearest,
            },
            Transform::identity(),
            clip,
        );
    } else {
        if let Shader::SolidColor(color) = &mut paint.shader {
            color.apply_opacity(state.alpha);
        }
        paint.blend_mode = state.blend;
        shape(&mut surface.pixmap, &paint, clip);
    }
    surface.dirty = true;
    h.dirty = true;
}

/// `canvasFill(id, tracé, peinture, état, pairImpair)` : remplit le tracé.
pub(super) fn fill(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let path = vm.to_string(&arg(args, 1))?;
    let paint = vm.to_string(&arg(args, 2))?;
    let state = vm.to_string(&arg(args, 3))?;
    let rule = if arg(args, 4).truthy() {
        FillRule::EvenOdd
    } else {
        FillRule::Winding
    };
    let (Some(path), Some(state)) = (parse_path(&path), parse_state(&state)) else {
        return Ok(Value::Undefined);
    };
    draw(vm, id, &state, &paint, |pixmap, paint, clip| {
        pixmap.fill_path(&path, paint, rule, Transform::identity(), clip);
    });
    Ok(Value::Undefined)
}

/// `canvasStroke(id, tracé, peinture, état, trait)` : trace le contour du
/// tracé ; `trait` vaut `largeur bout jonction onglet décalage …tirets`.
pub(super) fn stroke(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let path = vm.to_string(&arg(args, 1))?;
    let paint = vm.to_string(&arg(args, 2))?;
    let state = vm.to_string(&arg(args, 3))?;
    let line = vm.to_string(&arg(args, 4))?;
    let (Some(path), Some(state), Some(line)) =
        (parse_path(&path), parse_state(&state), numbers(&line))
    else {
        return Ok(Value::Undefined);
    };
    let [width, cap, join, miter, offset, dashes @ ..] = line.as_slice() else {
        return Ok(Value::Undefined);
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rangs : de petits entiers positifs"
    )]
    let stroke = Stroke {
        width: *width,
        miter_limit: *miter,
        line_cap: match *cap as u8 {
            1 => LineCap::Round,
            2 => LineCap::Square,
            _ => LineCap::Butt,
        },
        line_join: match *join as u8 {
            1 => LineJoin::Round,
            2 => LineJoin::Bevel,
            _ => LineJoin::Miter,
        },
        dash: if dashes.is_empty() {
            None
        } else {
            StrokeDash::new(dashes.to_vec(), *offset)
        },
    };
    // Le trait a son épaisseur dans le repère courant : le tracé, tenu dans
    // le repère de la surface, y est ramené, puis dessiné transformé.
    let Some(local) = state
        .transform
        .invert()
        .and_then(|inverse| path.transform(inverse))
    else {
        return Ok(Value::Undefined);
    };
    let transform = state.transform;
    draw(vm, id, &state, &paint, |pixmap, paint, clip| {
        pixmap.stroke_path(&local, paint, &stroke, transform, clip);
    });
    Ok(Value::Undefined)
}

/// `canvasClip(id, tracé, pairImpair, découpe)` : restreint la découpe
/// `découpe` (0 : aucune) au tracé et rend l'identifiant de la nouvelle
/// découpe ; 0 si elle n'a pas pu être créée.
pub(super) fn clip(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let path = vm.to_string(&arg(args, 1))?;
    let rule = if arg(args, 2).truthy() {
        FillRule::EvenOdd
    } else {
        FillRule::Winding
    };
    let previous = vm.to_number(&arg(args, 3))?;
    let Some(surface) = host(vm).and_then(|h| h.canvases.get_mut(&id)) else {
        return Ok(Value::Number(0.0));
    };
    if surface.clips.len() >= MAX_CLIPS {
        return Ok(Value::Number(0.0));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rang : un petit entier positif"
    )]
    let inherited = (previous.max(0.0) as usize)
        .checked_sub(1)
        .and_then(|at| surface.clips.get(at))
        .cloned();
    let Some(mut mask) =
        inherited.or_else(|| Mask::new(surface.pixmap.width(), surface.pixmap.height()))
    else {
        return Ok(Value::Number(0.0));
    };
    let first = previous < 1.0;
    match parse_path(&path) {
        Some(path) if first => mask.fill_path(&path, rule, true, Transform::identity()),
        Some(path) => mask.intersect_path(&path, rule, true, Transform::identity()),
        // Tracé vide : plus rien ne passe.
        None => mask.data_mut().fill(0),
    }
    surface.clips.push(mask);
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre de découpes, borné par MAX_CLIPS"
    )]
    Ok(Value::Number(surface.clips.len() as f64))
}

/// `canvasImage(id, nature, source, géométrie, état, lissage)` : dessine le
/// rectangle `sx sy sw sh` de la source dans le rectangle `dx dy dw dh` du
/// repère courant. Rend `false` si la source est absente.
pub(super) fn image(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let kind = vm.to_string(&arg(args, 1))?.to_string();
    let source = surface_id(vm, &arg(args, 2))?;
    let geometry = vm.to_string(&arg(args, 3))?;
    let state = vm.to_string(&arg(args, 4))?;
    let smooth = arg(args, 5).truthy();
    let (Some(geometry), Some(state)) = (numbers(&geometry), parse_state(&state)) else {
        return Ok(Value::Bool(false));
    };
    let [sx, sy, sw, sh, dx, dy, dw, dh] = geometry.as_slice() else {
        return Ok(Value::Bool(false));
    };
    let Some(pixels) = source_pixmap(vm, &kind, source) else {
        return Ok(Value::Bool(false));
    };
    if *sw == 0.0 || *sh == 0.0 {
        return Ok(Value::Bool(true));
    }
    // La source, déplacée et mise à l'échelle, peint le rectangle visé.
    let place = Transform::from_row(
        dw / sw,
        0.0,
        0.0,
        dh / sh,
        dx - sx * dw / sw,
        dy - sy * dh / sh,
    );
    let Some(target) =
        tiny_skia::Rect::from_xywh(dx.min(dx + dw), dy.min(dy + dh), dw.abs(), dh.abs())
    else {
        return Ok(Value::Bool(true));
    };
    let transform = state.transform;
    let quality = if smooth {
        FilterQuality::Bilinear
    } else {
        FilterQuality::Nearest
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Bool(false));
    };
    let Some(surface) = h.canvases.get_mut(&id) else {
        return Ok(Value::Bool(false));
    };
    let clip = state
        .clip
        .checked_sub(1)
        .and_then(|at| surface.clips.get(at));
    if state.clip > 0 && clip.is_none() {
        return Ok(Value::Bool(true));
    }
    let paint = Paint {
        shader: Pattern::new(
            pixels.as_ref(),
            SpreadMode::Pad,
            quality,
            state.alpha,
            place,
        ),
        blend_mode: state.blend,
        anti_alias: true,
        ..Paint::default()
    };
    if state.whole_surface {
        if let Some(mut layer) = Pixmap::new(surface.pixmap.width(), surface.pixmap.height()) {
            let plain = Paint {
                blend_mode: BlendMode::SourceOver,
                ..paint.clone()
            };
            layer.fill_rect(target, &plain, transform, None);
            surface.pixmap.draw_pixmap(
                0,
                0,
                layer.as_ref(),
                &PixmapPaint {
                    opacity: 1.0,
                    blend_mode: state.blend,
                    quality: FilterQuality::Nearest,
                },
                Transform::identity(),
                clip,
            );
        }
    } else {
        surface.pixmap.fill_rect(target, &paint, transform, clip);
    }
    surface.dirty = true;
    h.dirty = true;
    Ok(Value::Bool(true))
}

/// `canvasId(élément)` : identifiant de surface de l'élément (l'index de son
/// nœud), ou `null` si ce n'est pas un nœud.
pub(super) fn id_of(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    Ok(match super::node_of(vm, &arg(args, 0)) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "index de nœud, très en deçà de 2^53"
        )]
        Some(node) => Value::Number(node.index() as f64),
        None => Value::Null,
    })
}

/// `canvasSourceSize(nature, id)` : `largeur hauteur` de la source de pixels
/// (canevas `c` ou image `i`), ou `null` si elle n'en a pas.
pub(super) fn source_size(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let kind = vm.to_string(&arg(args, 0))?.to_string();
    let id = surface_id(vm, &arg(args, 1))?;
    let size = host(vm).and_then(|h| match kind.as_str() {
        "c" => h
            .canvases
            .get(&id)
            .map(|surface| (surface.pixmap.width(), surface.pixmap.height())),
        "i" => {
            let node = h.doc.id_from_index(id as usize)?;
            let image = h.layout_env.as_ref()?.pictures.get(&node)?;
            Some((image.width(), image.height()))
        }
        _ => None,
    });
    Ok(match size {
        Some((width, height)) => Value::string(&format!("{width} {height}")),
        None => Value::Null,
    })
}

/// `canvasDecode(id, octets)` : décode l'image `octets` (PNG, JPEG, WebP,
/// GIF…) dans une surface neuve d'identifiant `id` et rend sa taille,
/// `largeur hauteur` ; `null` si l'image est illisible ou trop grande.
pub(super) fn decode(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let binary = vm.to_string(&arg(args, 1))?;
    let bytes: Vec<u8> = binary
        .chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect();
    let Some(picture) = crate::decode_image(&bytes) else {
        return Ok(Value::Null);
    };
    let (width, height) = (picture.width(), picture.height());
    if width > MAX_SIDE || height > MAX_SIDE {
        return Ok(Value::Null);
    }
    let Some(mut pixmap) = Pixmap::new(width, height) else {
        return Ok(Value::Null);
    };
    for (out, rgba) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(picture.data().data.data().as_chunks::<4>().0)
    {
        *out = premultiply(*rgba);
    }
    let Some(h) = host(vm) else {
        return Ok(Value::Null);
    };
    h.canvases.insert(
        id,
        Surface {
            pixmap,
            clips: Vec::new(),
            dirty: false,
        },
    );
    Ok(Value::string(&format!("{width} {height}")))
}

/// Coordonnée entière passée par le script.
fn integer(vm: &mut Vm, value: &Value) -> JsResult<i64> {
    let number = vm.to_number(value)?;
    #[expect(clippy::cast_possible_truncation, reason = "valeur bornée juste avant")]
    Ok(if number.is_finite() {
        number.clamp(-1.0e9, 1.0e9) as i64
    } else {
        0
    })
}

/// `canvasGetPixels(id, x, y, largeur, hauteur)` : les pixels du rectangle,
/// en RGBA non prémultiplié (chaîne binaire) ; transparents hors de la
/// surface. `null` si le rectangle est trop grand.
pub(super) fn get_pixels(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let x = integer(vm, &arg(args, 1))?;
    let y = integer(vm, &arg(args, 2))?;
    let width = integer(vm, &arg(args, 3))?;
    let height = integer(vm, &arg(args, 4))?;
    let side = i64::from(MAX_SIDE);
    if !(1..=side).contains(&width) || !(1..=side).contains(&height) {
        return Ok(Value::Null);
    }
    let Some(surface) = host(vm).and_then(|h| h.canvases.get(&id)) else {
        return Ok(Value::Null);
    };
    let (surface_width, surface_height) = (
        i64::from(surface.pixmap.width()),
        i64::from(surface.pixmap.height()),
    );
    let data = surface.pixmap.data();
    let mut out = String::with_capacity(usize::try_from(width * height * 4).unwrap_or(0));
    for row in y..y + height {
        for column in x..x + width {
            let inside = (0..surface_width).contains(&column) && (0..surface_height).contains(&row);
            let pixel = usize::try_from((row * surface_width + column) * 4)
                .ok()
                .filter(|_| inside)
                .and_then(|at| data.get(at..at + 4))
                .and_then(|rgba| <[u8; 4]>::try_from(rgba).ok())
                .map_or([0; 4], unpremultiply);
            out.extend(pixel.iter().map(|byte| char::from(*byte)));
        }
    }
    Ok(Value::string(&out))
}

/// `canvasPutPixels(id, pixels, largeur, x, y, sx, sy, sl, sh)` : écrit dans
/// la surface, au point (`x`, `y`), le rectangle (`sx`, `sy`, `sl`, `sh`) de
/// l'image `pixels` (RGBA non prémultiplié, `largeur` pixels par ligne), sans
/// fusion ni découpe.
pub(super) fn put_pixels(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let pixels = vm.to_string(&arg(args, 1))?;
    let bytes: Vec<u8> = pixels
        .chars()
        .filter_map(|c| u8::try_from(u32::from(c)).ok())
        .collect();
    let image_width = integer(vm, &arg(args, 2))?;
    let x = integer(vm, &arg(args, 3))?;
    let y = integer(vm, &arg(args, 4))?;
    let from_x = integer(vm, &arg(args, 5))?;
    let from_y = integer(vm, &arg(args, 6))?;
    let width = integer(vm, &arg(args, 7))?;
    let height = integer(vm, &arg(args, 8))?;
    if image_width < 1 {
        return Ok(Value::Undefined);
    }
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let Some(surface) = h.canvases.get_mut(&id) else {
        return Ok(Value::Undefined);
    };
    let (surface_width, surface_height) = (
        i64::from(surface.pixmap.width()),
        i64::from(surface.pixmap.height()),
    );
    let data = surface.pixmap.data_mut();
    for row in from_y..from_y + height {
        for column in from_x..from_x + width {
            let (to_x, to_y) = (x + column, y + row);
            if !(0..image_width).contains(&column)
                || row < 0
                || !(0..surface_width).contains(&to_x)
                || !(0..surface_height).contains(&to_y)
            {
                continue;
            }
            let source = usize::try_from((row * image_width + column) * 4)
                .ok()
                .and_then(|at| bytes.get(at..at + 4))
                .and_then(|rgba| <[u8; 4]>::try_from(rgba).ok());
            let target = usize::try_from((to_y * surface_width + to_x) * 4)
                .ok()
                .and_then(|at| data.get_mut(at..at + 4));
            if let (Some(source), Some(target)) = (source, target) {
                target.copy_from_slice(&premultiply(source));
            }
        }
    }
    surface.dirty = true;
    h.dirty = true;
    Ok(Value::Undefined)
}

/// Pixels de la surface en RGBA non prémultiplié.
fn straight_pixels(pixmap: &Pixmap) -> Vec<u8> {
    let (pixels, _) = pixmap.data().as_chunks::<4>();
    pixels
        .iter()
        .flat_map(|rgba| unpremultiply(*rgba))
        .collect()
}

/// `canvasPng(id)` : l'image de la surface au format PNG (chaîne binaire),
/// ou `null`.
pub(super) fn png(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let Some(surface) = host(vm).and_then(|h| h.canvases.get(&id)) else {
        return Ok(Value::Null);
    };
    let (width, height) = (surface.pixmap.width(), surface.pixmap.height());
    let Some(picture) = image::RgbaImage::from_raw(width, height, straight_pixels(&surface.pixmap))
    else {
        return Ok(Value::Null);
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    if picture
        .write_to(&mut encoded, image::ImageFormat::Png)
        .is_err()
    {
        return Ok(Value::Null);
    }
    let binary: String = encoded
        .into_inner()
        .iter()
        .map(|byte| char::from(*byte))
        .collect();
    Ok(Value::string(&binary))
}

/// `parseCssColor(texte)` : `r g b a` (composantes de 0 à 255, opacité de 0
/// à 1) de la couleur CSS `texte`, ou `null` si ce n'en est pas une.
pub(super) fn parse_css_color(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let tokens: Vec<_> = oxyde_css::syntax::tokenize(text.trim());
    let parsed = oxyde_css::values::parse_color(&tokens).filter(|(_, used)| *used == tokens.len());
    Ok(match parsed {
        Some((oxyde_css::values::ParsedColor::Color(color), _)) => Value::string(&format!(
            "{} {} {} {}",
            color.r,
            color.g,
            color.b,
            f32::from(color.a) / 255.0
        )),
        // `currentcolor` : la couleur du texte du canevas, noire à défaut.
        Some((oxyde_css::values::ParsedColor::CurrentColor, _)) => Value::string("0 0 0 1"),
        None => Value::Null,
    })
}

/// Police que décrit la valeur de `ctx.font` (`italic bold 20px Arial`).
struct FontSpec {
    /// Corps, en pixels.
    size: f32,
    /// Liste de familles, écrite comme en CSS.
    family: String,
    weight: f32,
    italic: bool,
}

/// Lit une valeur de `font` ; `None` si elle n'a pas de corps lisible.
fn parse_font(text: &str) -> Option<FontSpec> {
    let mut weight = 400.0;
    let mut italic = false;
    let mut rest = text.trim();
    loop {
        let (word, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        // Le corps : un nombre suivi d'une unité, avec ou sans `/interligne`.
        let measure = word.split('/').next().unwrap_or(word);
        let unit_at = measure
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(measure.len());
        let (number, unit) = measure.split_at(unit_at);
        if let Ok(value) = number.parse::<f32>() {
            let size = match unit.to_ascii_lowercase().as_str() {
                "px" => value,
                "pt" => value * 4.0 / 3.0,
                "em" | "rem" => value * 16.0,
                "%" => value * 0.16,
                // Un nombre sans unité est une graisse (`700`).
                "" if (1.0..=1000.0).contains(&value) => {
                    weight = value;
                    rest = tail.trim_start();
                    continue;
                }
                _ => return None,
            };
            let family = tail.trim();
            return (size.is_finite() && size > 0.0 && !family.is_empty()).then(|| FontSpec {
                size: size.min(10_000.0),
                family: family.to_owned(),
                weight,
                italic,
            });
        }
        match word.to_ascii_lowercase().as_str() {
            "italic" | "oblique" => italic = true,
            "bold" | "bolder" => weight = 700.0,
            "lighter" => weight = 300.0,
            "" => return None,
            // `normal`, `small-caps`, largeurs de police : sans effet ici.
            _ => {}
        }
        if tail.is_empty() {
            return None;
        }
        rest = tail.trim_start();
    }
}

/// Texte mis en forme sur une ligne : le contour de ses glyphes, dans un
/// repère dont l'origine est le coin supérieur gauche de la ligne, et ses
/// mesures.
struct Shaped {
    outline: Option<Path>,
    width: f32,
    /// Distance du haut de la ligne à la ligne de base.
    baseline: f32,
    ascent: f32,
    descent: f32,
}

/// Plume qui recopie le contour d'un glyphe dans un tracé, à l'origine
/// (`x`, `y`) du glyphe ; l'axe vertical des polices monte, celui du canevas
/// descend.
struct GlyphPen<'a> {
    builder: &'a mut PathBuilder,
    x: f32,
    y: f32,
}

impl skrifa::outline::OutlinePen for GlyphPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(self.x + x, self.y - y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(self.x + x, self.y - y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder
            .quad_to(self.x + cx0, self.y - cy0, self.x + x, self.y - y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            self.x + cx0,
            self.y - cy0,
            self.x + cx1,
            self.y - cy1,
            self.x + x,
            self.y - y,
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

/// Met en forme `text` dans la police `font`.
fn shape_text(context: &mut oxyde_text::TextContext, text: &str, font: &FontSpec) -> Shaped {
    use oxyde_text::parley::{
        Alignment, AlignmentOptions, FontFamily, FontStyle, FontWeight, PositionedLayoutItem,
        StyleProperty,
    };
    use skrifa::MetadataProvider;
    let mut builder = context
        .layouts
        .ranged_builder(&mut context.fonts, text, 1.0, true);
    builder.push_default(StyleProperty::FontSize(font.size));
    builder.push_default(StyleProperty::FontFamily(FontFamily::from(
        font.family.as_str(),
    )));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(font.weight)));
    if font.italic {
        builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
    }
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout.align(Alignment::Start, AlignmentOptions::default());
    let mut path = PathBuilder::new();
    let (mut baseline, mut ascent, mut descent) =
        (font.size * 0.8, font.size * 0.8, font.size * 0.2);
    let mut measured = false;
    for (rank, line) in layout.lines().enumerate() {
        if rank == 0 {
            baseline = line.metrics().baseline;
        }
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyphs) = item else {
                continue;
            };
            let run = glyphs.run();
            if !measured {
                // Montée et descente de la première police employée.
                let metrics = run.font_metrics();
                (ascent, descent) = (metrics.ascent, metrics.descent);
                measured = true;
            }
            let data = run.font();
            let Ok(face) = skrifa::FontRef::from_index(data.data.as_ref(), data.index) else {
                continue;
            };
            let outlines = face.outline_glyphs();
            let coords: Vec<skrifa::instance::NormalizedCoord> = run
                .normalized_coords()
                .iter()
                .map(|coord| skrifa::instance::NormalizedCoord::from_bits(coord.to_bits()))
                .collect();
            let size = skrifa::instance::Size::new(run.font_size());
            for glyph in glyphs.positioned_glyphs() {
                let Some(outline) = outlines.get(skrifa::GlyphId::new(glyph.id)) else {
                    continue;
                };
                let mut pen = GlyphPen {
                    builder: &mut path,
                    x: glyph.x,
                    y: glyph.y,
                };
                let settings = skrifa::outline::DrawSettings::unhinted(
                    size,
                    skrifa::instance::LocationRef::new(&coords),
                );
                // Un glyphe sans contour (espace) n'ajoute rien.
                let _ = outline.draw(settings, &mut pen);
            }
        }
    }
    Shaped {
        outline: path.finish(),
        // Un texte vide n'occupe aucune largeur.
        width: if text.is_empty() {
            0.0
        } else {
            layout.full_width()
        },
        baseline,
        ascent,
        descent,
    }
}

/// `canvasMeasure(texte, police)` : `largeur montée descente` du texte mis
/// en forme sur une ligne, ou `null` si la police est illisible.
pub(super) fn measure(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    let font = vm.to_string(&arg(args, 1))?;
    let (Some(font), Some(h)) = (parse_font(&font), host(vm)) else {
        return Ok(Value::Null);
    };
    let context = h.text.get_or_insert_with(oxyde_text::TextContext::new);
    let shaped = shape_text(context, &text, &font);
    Ok(Value::string(&format!(
        "{} {} {}",
        shaped.width, shaped.ascent, shaped.descent
    )))
}

/// `canvasText(id, texte, police, placement, peinture, état, trait)` :
/// dessine le texte, rempli (`trait` vide) ou en contour. `placement` vaut
/// `x y largeurMax alignement ligneDeBase` : largeur maximale négative s'il
/// n'y en a pas ; alignement 0 (début), 1 (centre) ou 2 (fin) ; ligne de base
/// 0 (alphabétique), 1 (haut), 2 (milieu), 3 (bas).
pub(super) fn text(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let id = surface_id(vm, &arg(args, 0))?;
    let content = vm.to_string(&arg(args, 1))?;
    let font = vm.to_string(&arg(args, 2))?;
    let placement = vm.to_string(&arg(args, 3))?;
    let paint = vm.to_string(&arg(args, 4))?;
    let state = vm.to_string(&arg(args, 5))?;
    let line = vm.to_string(&arg(args, 6))?;
    let (Some(font), Some(placement), Some(state), Some(line)) = (
        parse_font(&font),
        numbers(&placement),
        parse_state(&state),
        numbers(&line),
    ) else {
        return Ok(Value::Undefined);
    };
    let [x, y, max_width, align, baseline] = placement.as_slice() else {
        return Ok(Value::Undefined);
    };
    let Some(h) = host(vm) else {
        return Ok(Value::Undefined);
    };
    let context = h.text.get_or_insert_with(oxyde_text::TextContext::new);
    let shaped = shape_text(context, &content, &font);
    let Some(outline) = shaped.outline else {
        return Ok(Value::Undefined);
    };
    // Un texte plus large que la largeur maximale est resserré.
    let squeeze = if *max_width >= 0.0 && shaped.width > *max_width && shaped.width > 0.0 {
        max_width / shaped.width
    } else {
        1.0
    };
    if squeeze <= 0.0 {
        return Ok(Value::Undefined);
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "rangs : de petits entiers positifs"
    )]
    let (align, baseline) = (*align as u8, *baseline as u8);
    let anchor_x = match align {
        1 => shaped.width / 2.0,
        2 => shaped.width,
        _ => 0.0,
    };
    let anchor_y = match baseline {
        1 => shaped.baseline - shaped.ascent,
        2 => shaped.baseline - (shaped.ascent - shaped.descent) / 2.0,
        3 => shaped.baseline + shaped.descent,
        _ => shaped.baseline,
    };
    // Dans le repère courant : l'ancre du texte au point (`x`, `y`).
    let place = Transform::from_row(squeeze, 0.0, 0.0, 1.0, x - anchor_x * squeeze, y - anchor_y);
    let Some(local) = outline.transform(place) else {
        return Ok(Value::Undefined);
    };
    let transform = state.transform;
    if let [width, cap, join, miter, offset, dashes @ ..] = line.as_slice() {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "rangs : de petits entiers positifs"
        )]
        let stroke = Stroke {
            width: *width,
            miter_limit: *miter,
            line_cap: match *cap as u8 {
                1 => LineCap::Round,
                2 => LineCap::Square,
                _ => LineCap::Butt,
            },
            line_join: match *join as u8 {
                1 => LineJoin::Round,
                2 => LineJoin::Bevel,
                _ => LineJoin::Miter,
            },
            dash: if dashes.is_empty() {
                None
            } else {
                StrokeDash::new(dashes.to_vec(), *offset)
            },
        };
        draw(vm, id, &state, &paint, |pixmap, paint, clip| {
            pixmap.stroke_path(&local, paint, &stroke, transform, clip);
        });
    } else {
        draw(vm, id, &state, &paint, |pixmap, paint, clip| {
            pixmap.fill_path(&local, paint, FillRule::Winding, transform, clip);
        });
    }
    Ok(Value::Undefined)
}

/// Surfaces dont les pixels ont changé depuis le dernier appel : identifiant
/// (l'index du nœud du canevas, pour un canevas de la page), largeur, hauteur
/// et pixels RGBA non prémultipliés.
pub(super) fn take_frames(vm: &mut Vm) -> Vec<(u32, u32, u32, Vec<u8>)> {
    let Some(h) = host(vm) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    for (id, surface) in &mut h.canvases {
        if !std::mem::take(&mut surface.dirty) {
            continue;
        }
        // Les surfaces sans élément (`OffscreenCanvas`, `ImageBitmap`) ne
        // s'affichent pas.
        if *id < DETACHED_IDS {
            frames.push((
                *id,
                surface.pixmap.width(),
                surface.pixmap.height(),
                straight_pixels(&surface.pixmap),
            ));
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_states_and_paints_are_parsed() {
        let path = parse_path("0 0 0 1 10 0 1 10 10 4").expect("tracé");
        assert_eq!(path.bounds().width(), 10.0);
        assert!(parse_path("").is_none());
        assert!(parse_path("1").is_none());
        assert!(parse_path("0 0 0 9 1 1").is_none());
        assert!(parse_path("0 0 0 1 NaN 2").is_none());
        let state = parse_state("0.5 9 2 1 0 0 1 10 20").expect("état");
        assert!(state.whole_surface && state.clip == 2);
        assert_eq!(state.transform, Transform::from_translate(10.0, 20.0));
        assert!(parse_state("1 99 0 1 0 0 1 0 0").is_none());
        assert!(matches!(
            parse_shader("c 255 0 0 1", Transform::identity(), None),
            Some(Shader::SolidColor(_))
        ));
        assert!(
            parse_shader(
                "l 0 0 10 0 0 255 0 0 1 1 0 0 255 1",
                Transform::identity(),
                None
            )
            .is_some()
        );
        // Dégradé sans arrêt, ou motif sans source : rien à peindre.
        assert!(parse_shader("l 0 0 10 0", Transform::identity(), None).is_none());
        assert!(parse_shader("p c 3", Transform::identity(), None).is_none());
    }

    #[test]
    fn fonts_are_parsed_and_text_is_shaped() {
        let font = parse_font("italic bold 20px Arial, sans-serif").expect("police");
        assert_eq!((font.size, font.weight, font.italic), (20.0, 700.0, true));
        assert_eq!(font.family, "Arial, sans-serif");
        let font = parse_font("300 12pt/1.5 serif").expect("police");
        assert_eq!((font.size, font.weight, font.italic), (16.0, 300.0, false));
        assert!(parse_font("bold").is_none());
        assert!(parse_font("10px").is_none());
        assert!(parse_font("").is_none());
        let mut context = oxyde_text::TextContext::new();
        let font = parse_font("20px Inter").expect("police");
        let word = shape_text(&mut context, "Oxyde", &font);
        let longer = shape_text(&mut context, "Oxyde Oxyde", &font);
        assert!(word.width > 20.0 && longer.width > word.width * 1.9);
        assert!(word.ascent > word.descent && word.baseline > 0.0);
        let outline = word.outline.expect("contours");
        assert!(outline.bounds().width() > 20.0 && outline.bounds().height() > 8.0);
        // Une espace seule a une largeur, pas de contour.
        let space = shape_text(&mut context, " ", &font);
        assert!(space.outline.is_none() && space.width > 0.0);
    }

    #[test]
    fn alpha_is_premultiplied_and_restored() {
        assert_eq!(premultiply([255, 128, 0, 128]), [128, 64, 0, 128]);
        assert_eq!(unpremultiply([128, 64, 0, 128]), [255, 128, 0, 128]);
        assert_eq!(unpremultiply([9, 9, 9, 0]), [0, 0, 0, 0]);
        for value in [0u8, 1, 127, 200, 255] {
            assert_eq!(
                unpremultiply(premultiply([value, value, value, 255]))[0],
                value
            );
        }
    }
}
