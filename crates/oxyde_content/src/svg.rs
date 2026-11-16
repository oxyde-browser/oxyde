//! Dessin des éléments `<svg>` : le sous-arbre est rendu en pixels par
//! `tiny-skia`, puis peint dans la page comme une image.
//!
//! Sont dessinés : `path`, `rect`, `circle`, `ellipse`, `line`, `polyline`,
//! `polygon`, les groupes (`g`, `a`, `svg` imbriqué), `use` (vers un élément
//! du même document, `symbol` compris), les dégradés linéaires et radiaux,
//! `viewBox` et `preserveAspectRatio`, l'attribut `transform`, l'opacité de
//! groupe. Les propriétés de peinture viennent du moteur de style
//! ([`oxyde_css::SvgStyle`]) : attributs de présentation, attribut `style`
//! et feuilles de style comptent donc tous.
//!
//! Le texte (`text`, `tspan`) est mis en forme sur une ligne et dessiné
//! par le contour de ses glyphes.
//!
//! Une découpe (`clip-path` vers un `clipPath`) ou un masque (`mask`)
//! s'appliquent au contenu dessiné à part, comme l'opacité de groupe.
//!
//! Un motif (`pattern`) est dessiné une fois dans une tuile, répétée par
//! le remplissage.
//!
//! LIMITES CONNUES : pas d'image (`image`), de marqueur ni de filtre ;
//! `transform-box` ne vaut que pour une forme (un groupe se rapporte
//! toujours à la fenêtre) ; une découpe ou un masque
//! exprimés dans la boîte englobante
//! (`objectBoundingBox`) sont ignorés, de même que la zone d'un masque et
//! le texte dans une découpe ; pour le texte, ni
//! `textPath`, ni positions glyphe par glyphe, ni `textLength`, ni
//! écriture de droite à gauche soignée, et pas de texte du tout dans une
//! image SVG chargée par `<img>` ; un
//! `use` ne suit pas une adresse vers un autre fichier, et son contenu
//! hérite de lui par approximation (une propriété qui vaut celle du
//! contexte où le modèle est rangé est tenue pour héritée) ; la
//! couleur d'un arrêt de dégradé se lit dans ses attributs, pas dans les
//! feuilles de style ; `transform-origin` est ignoré ; le rendu se fait à
//! deux pixels par pixel CSS, quel que soit l'écran ; une image SVG chargée
//! par `<img>` est rendue une fois, à sa taille propre.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::script::canvas::{FontSpec, shape_text};
use oxyde_css::{
    ComputedStyle, Display, StyleMap, SvgLineCap, SvgLineJoin, SvgPaint, SvgStyle, Viewport,
    Visibility,
};
use oxyde_dom::{Document, ElementData, Namespace, NodeId};
use oxyde_paint::Image;
use oxyde_text::TextContext;
use tiny_skia::{
    Color, FillRule, FilterQuality, GradientStop, LineCap, LineJoin, LinearGradient, Mask,
    MaskType, Paint, Path, PathBuilder, Pattern, Pixmap, PixmapPaint, Point, RadialGradient, Rect,
    Shader, SpreadMode, Stroke, StrokeDash, Transform,
};

/// Côté maximal de la surface de rendu, en pixels.
const MAX_SIDE: f32 = 4096.0;
/// Profondeur maximale de l'arbre dessiné, `use` compris.
const MAX_DEPTH: usize = 48;
/// Nombre maximal de dégradés suivis par `href`.
const MAX_CHAIN: usize = 8;
/// Nombre maximal de surfaces intermédiaires ouvertes à la fois (opacité de
/// groupe, découpe, masque) : chacune a la taille du rendu.
const MAX_LAYERS: usize = 6;
/// Côté maximal de la tuile d'un motif, en pixels.
const MAX_TILE_SIDE: f32 = 2048.0;
/// Nombre maximal d'éléments dessinés par rendu : des `use` imbriqués
/// peuvent multiplier un petit document sans fin.
const MAX_ELEMENTS: u32 = 200_000;
/// Pixels de rendu par pixel CSS pour un `<svg>` de la page.
pub(crate) const PAGE_SCALE: f32 = 2.0;
/// Taille d'un `<svg>` sans dimensions ni `viewBox`.
const DEFAULT_SIZE: (f32, f32) = (300.0, 150.0);

/// Éléments SVG du document, par identifiant (le premier l'emporte).
pub(crate) struct Index<'a>(HashMap<&'a str, NodeId>);

impl<'a> Index<'a> {
    /// Recense les identifiants des éléments SVG de `doc`.
    pub(crate) fn new(doc: &'a Document) -> Self {
        let mut map = HashMap::new();
        for node in doc.descendants(doc.root()) {
            if let Some(el) = doc.element(node)
                && el.ns == Namespace::Svg
                && let Some(id) = el.id()
            {
                map.entry(id).or_insert(node);
            }
        }
        Self(map)
    }

    /// Élément visé par une adresse `#id`.
    fn target(&self, href: &str) -> Option<NodeId> {
        self.0.get(href.trim().strip_prefix('#')?).copied()
    }
}

/// Vrai si `node` est un élément SVG nommé `name`.
fn is_svg(doc: &Document, node: NodeId, name: &str) -> bool {
    doc.element(node)
        .is_some_and(|el| el.ns == Namespace::Svg && el.name == name)
}

/// Vrai si `node` est un élément `<svg>` sans ancêtre SVG direct : la page
/// le met en page comme un élément remplacé.
pub(crate) fn is_outer_root(doc: &Document, node: NodeId) -> bool {
    is_svg(doc, node, "svg")
        && !doc
            .node(node)
            .parent
            .and_then(|parent| doc.element(parent))
            .is_some_and(|parent| parent.ns == Namespace::Svg)
}

/// Éléments `<svg>` les plus extérieurs de `doc`.
pub(crate) fn outer_roots(doc: &Document) -> Vec<NodeId> {
    doc.descendants(doc.root())
        .filter(|node| is_outer_root(doc, *node))
        .collect()
}

// ---- Nombres et longueurs ---------------------------------------------------------------------

/// Lecteur de nombres dans un attribut (`d`, `points`, `viewBox`…).
struct Scanner<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            at: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Passe les blancs et les virgules.
    fn skip_separators(&mut self) {
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_whitespace() || b == b',')
        {
            self.at += 1;
        }
    }

    /// Nombre suivant, séparateurs passés ; `None` s'il n'y en a pas.
    fn number(&mut self) -> Option<f32> {
        self.skip_separators();
        let start = self.at;
        let mut at = self.at;
        let get = |at: usize| self.bytes.get(at).copied();
        if matches!(get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let digits_start = at;
        while get(at).is_some_and(|b| b.is_ascii_digit()) {
            at += 1;
        }
        let mut digits = at - digits_start;
        if get(at) == Some(b'.') {
            at += 1;
            let fraction_start = at;
            while get(at).is_some_and(|b| b.is_ascii_digit()) {
                at += 1;
            }
            digits += at - fraction_start;
        }
        if digits == 0 {
            return None;
        }
        if matches!(get(at), Some(b'e' | b'E')) {
            let mut exponent = at + 1;
            if matches!(get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            if get(exponent).is_some_and(|b| b.is_ascii_digit()) {
                while get(exponent).is_some_and(|b| b.is_ascii_digit()) {
                    exponent += 1;
                }
                at = exponent;
            }
        }
        let text = std::str::from_utf8(self.bytes.get(start..at)?).ok()?;
        let value = text.parse::<f32>().ok().filter(|v| v.is_finite())?;
        self.at = at;
        Some(value)
    }

    /// Drapeau d'un arc : un seul chiffre, `0` ou `1`, parfois collé au suivant.
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let value = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.at += 1;
        Some(value)
    }
}

/// Tous les nombres de `text`, jusqu'au premier jeton qui n'en est pas un.
fn numbers(text: &str) -> Vec<f32> {
    let mut scanner = Scanner::new(text);
    let mut out = Vec::new();
    while let Some(value) = scanner.number() {
        out.push(value);
    }
    out
}

/// Longueur d'un attribut, en unités de l'utilisateur. Un pourcentage se
/// rapporte à `base` ; `em` au corps du texte `font_size`.
fn length(value: &str, base: f32, font_size: f32) -> Option<f32> {
    let value = value.trim();
    let mut scanner = Scanner::new(value);
    let number = scanner.number()?;
    let unit = value.get(scanner.at..)?.trim();
    let factor = match unit.to_ascii_lowercase().as_str() {
        "" | "px" => 1.0,
        "%" => base / 100.0,
        "em" | "rem" => font_size,
        "ex" => font_size / 2.0,
        "pt" => 4.0 / 3.0,
        "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "q" => 96.0 / 101.6,
        _ => return None,
    };
    Some(number * factor)
}

/// Fenêtre courante : la taille à laquelle se rapportent les pourcentages.
#[derive(Clone, Copy)]
struct Port {
    width: f32,
    height: f32,
}

impl Port {
    /// Longueur de référence pour un pourcentage qui n'est ni horizontal ni
    /// vertical (un rayon).
    fn diagonal(self) -> f32 {
        (self.width.hypot(self.height)) / std::f32::consts::SQRT_2
    }
}

/// Attribut de longueur `name` de `el`, `default` s'il manque ou est invalide.
fn attr_length(el: &ElementData, name: &str, base: f32, font_size: f32, default: f32) -> f32 {
    el.attr(name)
        .and_then(|value| length(value, base, font_size))
        .unwrap_or(default)
}

// ---- viewBox et transformations ---------------------------------------------------------------

/// `viewBox` de `el` : (x, y, largeur, hauteur), largeur et hauteur positives.
fn view_box(el: &ElementData) -> Option<[f32; 4]> {
    let values = numbers(el.attr("viewBox")?);
    match values[..] {
        [x, y, w, h] if w > 0.0 && h > 0.0 => Some([x, y, w, h]),
        _ => None,
    }
}

/// Transformation qui loge le `viewBox` dans une fenêtre de `width` ×
/// `height`, selon `preserveAspectRatio`.
fn fit(view: [f32; 4], ratio: Option<&str>, width: f32, height: f32) -> Transform {
    let [x, y, w, h] = view;
    let (sx, sy) = (width / w, height / h);
    let mut words = ratio.unwrap_or("").split_ascii_whitespace();
    let align = words.next().unwrap_or("xMidYMid");
    if align == "none" {
        return Transform::from_row(sx, 0.0, 0.0, sy, -x * sx, -y * sy);
    }
    let scale = if words.next() == Some("slice") {
        sx.max(sy)
    } else {
        sx.min(sy)
    };
    let share = |key: &str| {
        if align.contains(&format!("{key}Min")) {
            0.0
        } else if align.contains(&format!("{key}Max")) {
            1.0
        } else {
            0.5
        }
    };
    let dx = (width - w * scale) * share("x");
    let dy = (height - h * scale) * share("Y");
    Transform::from_row(scale, 0.0, 0.0, scale, dx - x * scale, dy - y * scale)
}

/// Arguments d'une fonction de l'attribut `transform` : des nombres sans
/// unité, séparés par des blancs ou par une virgule. `None` s'il y a autre
/// chose.
fn arguments(text: &str) -> Option<Vec<f32>> {
    let mut scanner = Scanner::new(text);
    let mut out = Vec::new();
    let blanks = |scanner: &mut Scanner<'_>| {
        while scanner.peek().is_some_and(|b| b.is_ascii_whitespace()) {
            scanner.at += 1;
        }
    };
    loop {
        blanks(&mut scanner);
        if scanner.peek().is_none() {
            return Some(out);
        }
        // `number` passerait les virgules : elles sont comptées ici.
        if scanner.peek() == Some(b',') {
            return None;
        }
        out.push(scanner.number()?);
        blanks(&mut scanner);
        match scanner.peek() {
            None => return Some(out),
            Some(b',') => {
                scanner.at += 1;
                blanks(&mut scanner);
                // Une virgule annonce un nombre.
                if scanner.peek().is_none() || scanner.peek() == Some(b',') {
                    return None;
                }
            }
            Some(b) if b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.') => {}
            Some(_) => return None,
        }
    }
}

/// Valeur de l'attribut `transform` (ou `gradientTransform`).
fn parse_transform(text: &str) -> Transform {
    let mut out = Transform::identity();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|at| open + at) else {
            break;
        };
        let name = rest[..open].trim_matches(|c: char| c.is_whitespace() || c == ',');
        // Des unités ou une virgule en trop : la valeur entière est invalide.
        let Some(args) = arguments(&rest[open + 1..close]) else {
            return Transform::identity();
        };
        let step = match (name, args.as_slice()) {
            ("matrix", [a, b, c, d, e, f]) => Transform::from_row(*a, *b, *c, *d, *e, *f),
            ("translate", [x]) => Transform::from_translate(*x, 0.0),
            ("translate", [x, y]) => Transform::from_translate(*x, *y),
            ("scale", [s]) => Transform::from_scale(*s, *s),
            ("scale", [x, y]) => Transform::from_scale(*x, *y),
            ("rotate", [angle]) => Transform::from_rotate(*angle),
            ("rotate", [angle, cx, cy]) => Transform::from_rotate_at(*angle, *cx, *cy),
            ("skewX", [angle]) => Transform::from_skew(angle.to_radians().tan(), 0.0),
            ("skewY", [angle]) => Transform::from_skew(0.0, angle.to_radians().tan()),
            // Fonction inconnue ou mal formée : la valeur entière est invalide.
            _ => return Transform::identity(),
        };
        out = out.pre_concat(step);
        rest = &rest[close + 1..];
    }
    out
}

/// Transformation propre à un élément : `translate`, `rotate`, `scale`,
/// puis la propriété `transform` (à défaut l'attribut du même nom, qui n'est
/// qu'un attribut de présentation), autour de `transform-origin`. L'origine
/// et les pourcentages se rapportent à la boîte que désigne
/// `transform-box` : la fenêtre SVG `port`, ou la boîte englobante `bounds`
/// de la forme quand elle est connue.
fn own_transform(
    el: &ElementData,
    style: &ComputedStyle,
    port: Port,
    bounds: Option<Rect>,
) -> Transform {
    let attribute = el.attr("transform").map(parse_transform);
    if attribute.is_none() && !style.is_transformed() {
        return Transform::identity();
    }
    // Boîte de référence : (x, y, largeur, hauteur).
    let reference = match (style.transform_box, bounds) {
        (oxyde_css::TransformBox::FillBox, Some(b)) => (b.x(), b.y(), b.width(), b.height()),
        (oxyde_css::TransformBox::StrokeBox, Some(b)) => {
            // Le contour déborde d'une demi-épaisseur de chaque côté.
            let half = if style.svg.stroke == SvgPaint::None {
                0.0
            } else {
                style.svg.stroke_width / 2.0
            };
            (
                b.x() - half,
                b.y() - half,
                b.width() + half * 2.0,
                b.height() + half * 2.0,
            )
        }
        _ => (0.0, 0.0, port.width, port.height),
    };
    let port = Port {
        width: reference.2,
        height: reference.3,
    };
    let skia = |matrix: oxyde_css::Affine| {
        let [a, b, c, d, e, f] = matrix.0;
        Transform::from_row(a, b, c, d, e, f)
    };
    let main = style
        .transform_functions_matrix(port.width, port.height)
        .map(skia)
        .or(attribute)
        .unwrap_or_else(Transform::identity);
    let individual = skia(style.individual_matrix(port.width, port.height));
    let x = reference.0 + style.transform_origin.0.resolve(port.width).unwrap_or(0.0);
    let y = reference.1 + style.transform_origin.1.resolve(port.height).unwrap_or(0.0);
    Transform::from_translate(x, y)
        .pre_concat(individual)
        .pre_concat(main)
        .pre_concat(Transform::from_translate(-x, -y))
}

// ---- Tracés -----------------------------------------------------------------------------------

/// Ajoute à `builder` l'arc d'ellipse de (`x0`, `y0`) à (`x`, `y`), découpé
/// en courbes de Bézier (SVG 1.1, annexe F.6).
#[expect(
    clippy::too_many_arguments,
    reason = "les paramètres d'un arc SVG, tels que la syntaxe les donne"
)]
fn arc_to(
    builder: &mut PathBuilder,
    x0: f32,
    y0: f32,
    rx: f32,
    ry: f32,
    rotation: f32,
    large: bool,
    sweep: bool,
    x: f32,
    y: f32,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if (x0 == x && y0 == y) || !(rx > 0.0 && ry > 0.0) {
        // Rayon nul : un segment de droite.
        if rx == 0.0 || ry == 0.0 {
            builder.line_to(x, y);
        }
        return;
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let (dx, dy) = ((x0 - x) / 2.0, (y0 - y) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    // Rayons trop petits : agrandis jusqu'à joindre les deux points.
    let lambda = (x1 / rx).powi(2) + (y1 / ry).powi(2);
    if lambda > 1.0 {
        let grow = lambda.sqrt();
        rx *= grow;
        ry *= grow;
    }
    let numerator = (rx * ry).powi(2) - (rx * y1).powi(2) - (ry * x1).powi(2);
    let denominator = (rx * y1).powi(2) + (ry * x1).powi(2);
    let mut factor = if denominator > 0.0 {
        (numerator.max(0.0) / denominator).sqrt()
    } else {
        0.0
    };
    if large == sweep {
        factor = -factor;
    }
    let (cx1, cy1) = (factor * rx * y1 / ry, -factor * ry * x1 / rx);
    let (cx, cy) = (
        cos * cx1 - sin * cy1 + (x0 + x) / 2.0,
        sin * cx1 + cos * cy1 + (y0 + y) / 2.0,
    );
    let angle = |ux: f32, uy: f32| uy.atan2(ux);
    let start = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let end = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry);
    let mut sweep_angle = end - start;
    if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    } else if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    }
    if !sweep_angle.is_finite() {
        builder.line_to(x, y);
        return;
    }
    // Un quart de tour au plus par courbe.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "entre 1 et 4 morceaux"
    )]
    let pieces = (sweep_angle.abs() / std::f32::consts::FRAC_PI_2)
        .ceil()
        .clamp(1.0, 4.0) as u32;
    #[expect(clippy::cast_precision_loss, reason = "au plus 4")]
    let step = sweep_angle / pieces as f32;
    let handle = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |theta: f32| {
        let (s, c) = theta.sin_cos();
        (
            cx + rx * c * cos - ry * s * sin,
            cy + rx * c * sin + ry * s * cos,
        )
    };
    let tangent = |theta: f32| {
        let (s, c) = theta.sin_cos();
        (-rx * s * cos - ry * c * sin, -rx * s * sin + ry * c * cos)
    };
    let mut theta = start;
    for index in 0..pieces {
        let next = theta + step;
        let (px, py) = point(theta);
        let (tx, ty) = tangent(theta);
        // Le dernier point est celui que la syntaxe donne, sans erreur d'arrondi.
        let (qx, qy) = if index + 1 == pieces {
            (x, y)
        } else {
            point(next)
        };
        let (ux, uy) = tangent(next);
        builder.cubic_to(
            px + handle * tx,
            py + handle * ty,
            qx - handle * ux,
            qy - handle * uy,
            qx,
            qy,
        );
        theta = next;
    }
}

/// Tracé décrit par l'attribut `d` ; ce qui précède une erreur est gardé.
fn parse_path(data: &str) -> Option<Path> {
    let mut scanner = Scanner::new(data);
    let mut builder = PathBuilder::new();
    // Point courant, début du sous-tracé, dernier point de contrôle.
    let (mut x, mut y) = (0.0_f32, 0.0_f32);
    let (mut start_x, mut start_y) = (0.0_f32, 0.0_f32);
    let mut control: Option<(f32, f32, bool)> = None;
    let mut command = 0_u8;
    let mut started = false;
    'segments: loop {
        scanner.skip_separators();
        let Some(next) = scanner.peek() else { break };
        if next.is_ascii_alphabetic() {
            command = next;
            scanner.at += 1;
        } else if command == 0 {
            break;
        } else if command == b'M' {
            // Des coordonnées qui suivent un `M` sont des `L`.
            command = b'L';
        } else if command == b'm' {
            command = b'l';
        }
        let relative = command.is_ascii_lowercase();
        let (ox, oy) = if relative { (x, y) } else { (0.0, 0.0) };
        // Tout tracé commence par un déplacement.
        if !started && !matches!(command, b'M' | b'm') {
            break;
        }
        let mut read = |count: usize| -> Option<[f32; 6]> {
            let mut values = [0.0; 6];
            for slot in values.iter_mut().take(count) {
                *slot = scanner.number()?;
            }
            Some(values)
        };
        let mut next_control = None;
        match command.to_ascii_uppercase() {
            b'M' => {
                let Some([px, py, ..]) = read(2) else {
                    break 'segments;
                };
                (x, y) = (ox + px, oy + py);
                (start_x, start_y) = (x, y);
                builder.move_to(x, y);
                started = true;
            }
            b'L' => {
                let Some([px, py, ..]) = read(2) else {
                    break 'segments;
                };
                (x, y) = (ox + px, oy + py);
                builder.line_to(x, y);
            }
            b'H' => {
                let Some([px, ..]) = read(1) else {
                    break 'segments;
                };
                x = ox + px;
                builder.line_to(x, y);
            }
            b'V' => {
                let Some([py, ..]) = read(1) else {
                    break 'segments;
                };
                y = oy + py;
                builder.line_to(x, y);
            }
            b'C' => {
                let Some([x1, y1, x2, y2, px, py]) = read(6) else {
                    break 'segments;
                };
                builder.cubic_to(ox + x1, oy + y1, ox + x2, oy + y2, ox + px, oy + py);
                next_control = Some((ox + x2, oy + y2, true));
                (x, y) = (ox + px, oy + py);
            }
            b'S' => {
                let Some([x2, y2, px, py, ..]) = read(4) else {
                    break 'segments;
                };
                // Premier point de contrôle : le reflet du précédent.
                let (x1, y1) = match control {
                    Some((cx, cy, true)) => (2.0 * x - cx, 2.0 * y - cy),
                    _ => (x, y),
                };
                builder.cubic_to(x1, y1, ox + x2, oy + y2, ox + px, oy + py);
                next_control = Some((ox + x2, oy + y2, true));
                (x, y) = (ox + px, oy + py);
            }
            b'Q' => {
                let Some([x1, y1, px, py, ..]) = read(4) else {
                    break 'segments;
                };
                builder.quad_to(ox + x1, oy + y1, ox + px, oy + py);
                next_control = Some((ox + x1, oy + y1, false));
                (x, y) = (ox + px, oy + py);
            }
            b'T' => {
                let Some([px, py, ..]) = read(2) else {
                    break 'segments;
                };
                let (x1, y1) = match control {
                    Some((cx, cy, false)) => (2.0 * x - cx, 2.0 * y - cy),
                    _ => (x, y),
                };
                builder.quad_to(x1, y1, ox + px, oy + py);
                next_control = Some((x1, y1, false));
                (x, y) = (ox + px, oy + py);
            }
            b'A' => {
                let Some([rx, ry, rotation, ..]) = read(3) else {
                    break 'segments;
                };
                let (Some(large), Some(sweep)) = (scanner.flag(), scanner.flag()) else {
                    break 'segments;
                };
                let (Some(px), Some(py)) = (scanner.number(), scanner.number()) else {
                    break 'segments;
                };
                let (px, py) = (ox + px, oy + py);
                arc_to(&mut builder, x, y, rx, ry, rotation, large, sweep, px, py);
                (x, y) = (px, py);
            }
            b'Z' => {
                // Ce qui suit sans `M` repart du début du sous-tracé.
                builder.close();
                (x, y) = (start_x, start_y);
            }
            _ => break 'segments,
        }
        control = next_control;
    }
    builder.finish()
}

/// Ellipse de centre (`cx`, `cy`) : quatre courbes.
fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<Path> {
    if !(rx > 0.0 && ry > 0.0) {
        return None;
    }
    PathBuilder::from_oval(Rect::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0)?)
}

/// Rectangle, aux coins arrondis de rayons (`rx`, `ry`).
fn rect_path(x: f32, y: f32, w: f32, h: f32, rx: f32, ry: f32) -> Option<Path> {
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    let (rx, ry) = (rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0));
    if rx == 0.0 || ry == 0.0 {
        return Some(PathBuilder::from_rect(Rect::from_xywh(x, y, w, h)?));
    }
    // Longueur des poignées d'un quart d'ellipse.
    const KAPPA: f32 = 0.552_284_8;
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let mut b = PathBuilder::new();
    b.move_to(x + rx, y);
    b.line_to(x + w - rx, y);
    b.cubic_to(x + w - rx + kx, y, x + w, y + ry - ky, x + w, y + ry);
    b.line_to(x + w, y + h - ry);
    b.cubic_to(
        x + w,
        y + h - ry + ky,
        x + w - rx + kx,
        y + h,
        x + w - rx,
        y + h,
    );
    b.line_to(x + rx, y + h);
    b.cubic_to(x + rx - kx, y + h, x, y + h - ry + ky, x, y + h - ry);
    b.line_to(x, y + ry);
    b.cubic_to(x, y + ry - ky, x + rx - kx, y, x + rx, y);
    b.close();
    b.finish()
}

/// Tracé d'une forme de base ; `None` pour un autre élément ou une forme
/// sans surface.
fn shape_path(el: &ElementData, port: Port, font_size: f32) -> Option<Path> {
    let horizontal = |name: &str| attr_length(el, name, port.width, font_size, 0.0);
    let vertical = |name: &str| attr_length(el, name, port.height, font_size, 0.0);
    let radius = |name: &str| {
        el.attr(name)
            .and_then(|value| length(value, port.diagonal(), font_size))
    };
    match el.name.as_str() {
        "path" => parse_path(el.attr("d")?),
        "rect" => {
            // Un seul rayon donné vaut pour les deux.
            let rx = el.attr("rx").and_then(|v| length(v, port.width, font_size));
            let ry = el
                .attr("ry")
                .and_then(|v| length(v, port.height, font_size));
            let (rx, ry) = match (rx, ry) {
                (Some(rx), Some(ry)) => (rx, ry),
                (Some(r), None) | (None, Some(r)) => (r, r),
                (None, None) => (0.0, 0.0),
            };
            rect_path(
                horizontal("x"),
                vertical("y"),
                horizontal("width"),
                vertical("height"),
                rx,
                ry,
            )
        }
        "circle" => {
            let r = radius("r")?;
            ellipse_path(horizontal("cx"), vertical("cy"), r, r)
        }
        "ellipse" => {
            let rx = el.attr("rx").and_then(|v| length(v, port.width, font_size));
            let ry = el
                .attr("ry")
                .and_then(|v| length(v, port.height, font_size));
            let (rx, ry) = match (rx, ry) {
                (Some(rx), Some(ry)) => (rx, ry),
                (Some(r), None) | (None, Some(r)) => (r, r),
                (None, None) => return None,
            };
            ellipse_path(horizontal("cx"), vertical("cy"), rx, ry)
        }
        "line" => {
            let mut b = PathBuilder::new();
            b.move_to(horizontal("x1"), vertical("y1"));
            b.line_to(horizontal("x2"), vertical("y2"));
            b.finish()
        }
        "polyline" | "polygon" => {
            let values = numbers(el.attr("points")?);
            let mut b = PathBuilder::new();
            for (index, pair) in values.as_chunks::<2>().0.iter().enumerate() {
                if index == 0 {
                    b.move_to(pair[0], pair[1]);
                } else {
                    b.line_to(pair[0], pair[1]);
                }
            }
            if el.name == "polygon" {
                b.close();
            }
            b.finish()
        }
        _ => None,
    }
}

// ---- Peintures --------------------------------------------------------------------------------

/// Couleur `tiny-skia` d'une couleur CSS, son opacité multipliée par `opacity`.
fn skia_color(color: oxyde_css::values::Color, opacity: f32) -> Color {
    let alpha = (f32::from(color.a) / 255.0 * opacity).clamp(0.0, 1.0);
    Color::from_rgba(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        alpha,
    )
    .unwrap_or(Color::TRANSPARENT)
}

/// Couleur écrite dans un attribut ; `current` pour `currentcolor`.
fn parse_color(text: &str, current: oxyde_css::values::Color) -> Option<oxyde_css::values::Color> {
    let tokens = oxyde_css::syntax::tokenize(text.trim());
    let (color, used) = oxyde_css::values::parse_color(&tokens)?;
    (used == tokens.len()).then_some(match color {
        oxyde_css::values::ParsedColor::Color(c) => c,
        oxyde_css::values::ParsedColor::CurrentColor => current,
    })
}

/// Valeur de la propriété `name` d'un élément, lue dans son attribut
/// `style` puis dans son attribut de présentation.
fn declared<'a>(el: &'a ElementData, name: &str) -> Option<&'a str> {
    el.attr("style")
        .and_then(|style| {
            style.split(';').find_map(|declaration| {
                let (property, value) = declaration.split_once(':')?;
                property
                    .trim()
                    .eq_ignore_ascii_case(name)
                    .then_some(value.trim())
            })
        })
        .or_else(|| el.attr(name))
}

/// Adresse `href` (ou `xlink:href`) d'un élément.
fn href(el: &ElementData) -> Option<&str> {
    el.attr("href").or_else(|| {
        el.attrs
            .iter()
            .find(|attr| attr.name == "href" || attr.name == "xlink:href")
            .map(|attr| attr.value.as_str())
    })
}

/// De quoi remplir ou tracer une forme.
enum Brush {
    /// Couleur unie ou dégradé.
    Shader(Shader<'static>),
    /// Motif : une tuile de pixels, répétée ; `place` la pose dans le
    /// repère de la forme.
    Pattern {
        tile: Pixmap,
        place: Transform,
        opacity: f32,
    },
}

impl Brush {
    /// Peinture `tiny-skia` de ce pinceau.
    fn paint(&self) -> Paint<'_> {
        let shader = match self {
            Self::Shader(shader) => shader.clone(),
            Self::Pattern {
                tile,
                place,
                opacity,
            } => Pattern::new(
                tile.as_ref(),
                SpreadMode::Repeat,
                FilterQuality::Bilinear,
                *opacity,
                *place,
            ),
        };
        Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        }
    }
}

/// Héritage du contenu d'un `<use>` : ce qui vaut encore la valeur du
/// contexte où le modèle est rangé (`from`) prend celle du `use` (`onto`).
struct Rebase {
    from: SvgStyle,
    from_color: oxyde_css::values::Color,
    onto: SvgStyle,
    onto_color: oxyde_css::values::Color,
}

/// Style de peinture et couleur courante d'un élément, héritage d'un
/// `<use>` compris.
fn effective(
    style: &ComputedStyle,
    rebase: Option<&Rebase>,
) -> (SvgStyle, oxyde_css::values::Color) {
    match rebase {
        None => ((*style.svg).clone(), style.color),
        Some(rebase) => (
            style.svg.rebased(&rebase.from, &rebase.onto),
            if style.color == rebase.from_color {
                rebase.onto_color
            } else {
                style.color
            },
        ),
    }
}

/// Ce qu'il faut pour dessiner : le document, ses styles et ses identifiants.
struct Scene<'a> {
    doc: &'a Document,
    styles: &'a StyleMap,
    index: &'a Index<'a>,
    /// De quoi mettre le texte en forme ; sans cela, il n'est pas dessiné.
    text: Option<RefCell<&'a mut TextContext>>,
    /// Surfaces intermédiaires ouvertes en ce moment.
    layers: Cell<usize>,
    /// Éléments qu'il reste permis de dessiner (voir [`MAX_ELEMENTS`]).
    budget: Cell<u32>,
}

impl Scene<'_> {
    /// Dégradé `node` et ceux dont il hérite par `href`, du plus proche au
    /// plus lointain.
    fn gradient_chain(&self, node: NodeId) -> Vec<&ElementData> {
        let mut chain = Vec::new();
        let mut at = Some(node);
        while let Some(node) = at
            && chain.len() < MAX_CHAIN
        {
            let Some(el) = self.doc.element(node) else {
                break;
            };
            if el.ns != Namespace::Svg
                || !matches!(el.name.as_str(), "linearGradient" | "radialGradient")
            {
                break;
            }
            chain.push(el);
            at = href(el).and_then(|target| self.index.target(target));
        }
        chain
    }

    /// Arrêts du premier dégradé de la chaîne qui en a.
    fn stops(
        &self,
        node: NodeId,
        current: oxyde_css::values::Color,
        opacity: f32,
    ) -> Vec<(f32, Color)> {
        let mut at = Some(node);
        for _ in 0..MAX_CHAIN {
            let Some(node) = at else { break };
            let mut stops = Vec::new();
            let mut last = 0.0_f32;
            for child in self.doc.children(node) {
                let Some(el) = self.doc.element(child) else {
                    continue;
                };
                if el.ns != Namespace::Svg || el.name != "stop" {
                    continue;
                }
                let offset = el.attr("offset").map_or(0.0, |text| {
                    let text = text.trim();
                    match text.strip_suffix('%') {
                        Some(percent) => percent.trim().parse::<f32>().unwrap_or(0.0) / 100.0,
                        None => text.parse::<f32>().unwrap_or(0.0),
                    }
                });
                // Les positions ne reculent jamais.
                last = offset.clamp(0.0, 1.0).max(last);
                let color = declared(el, "stop-color")
                    .and_then(|text| parse_color(text, current))
                    .unwrap_or(oxyde_css::values::Color::BLACK);
                let stop_opacity = declared(el, "stop-opacity")
                    .and_then(|text| text.trim().parse::<f32>().ok())
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0);
                stops.push((last, skia_color(color, stop_opacity * opacity)));
            }
            if !stops.is_empty() {
                return stops;
            }
            at = self
                .doc
                .element(node)
                .and_then(href)
                .and_then(|target| self.index.target(target));
        }
        Vec::new()
    }

    /// Peinture du serveur `node` pour une forme de boîte englobante
    /// `bounds`. `None` s'il ne peint rien ou n'est pas un dégradé.
    fn gradient(
        &self,
        node: NodeId,
        bounds: Rect,
        port: Port,
        opacity: f32,
        current: oxyde_css::values::Color,
    ) -> Option<Shader<'static>> {
        let chain = self.gradient_chain(node);
        let first = chain.first()?;
        let attr = |name: &str| chain.iter().find_map(|el| el.attr(name));
        let colors = self.stops(node, current, opacity);
        let last = colors.last()?.1;
        if colors.len() == 1 {
            return Some(Shader::SolidColor(last));
        }
        let stops: Vec<GradientStop> = colors
            .iter()
            .map(|(position, color)| GradientStop::new(*position, *color))
            .collect();
        let user_space = attr("gradientUnits") == Some("userSpaceOnUse");
        // Dans la boîte englobante, les coordonnées sont des fractions.
        let coordinate = |name: &str, base: f32, default: f32| {
            let Some(text) = attr(name) else {
                return default;
            };
            let text = text.trim();
            if user_space {
                length(text, base, 16.0).unwrap_or(default)
            } else {
                match text.strip_suffix('%') {
                    Some(percent) => percent.trim().parse::<f32>().map_or(default, |v| v / 100.0),
                    None => text.parse::<f32>().unwrap_or(default),
                }
            }
        };
        let (width, height, diagonal) = if user_space {
            (port.width, port.height, port.diagonal())
        } else {
            (1.0, 1.0, 1.0)
        };
        let mut transform = if user_space {
            Transform::identity()
        } else {
            // Une forme sans épaisseur n'a pas de boîte où loger le dégradé.
            if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                return None;
            }
            Transform::from_row(
                bounds.width(),
                0.0,
                0.0,
                bounds.height(),
                bounds.x(),
                bounds.y(),
            )
        };
        if let Some(text) = attr("gradientTransform") {
            transform = transform.pre_concat(parse_transform(text));
        }
        let spread = match attr("spreadMethod") {
            Some("reflect") => SpreadMode::Reflect,
            Some("repeat") => SpreadMode::Repeat,
            _ => SpreadMode::Pad,
        };
        if first.name == "linearGradient" {
            let start = Point::from_xy(coordinate("x1", width, 0.0), coordinate("y1", height, 0.0));
            let end = Point::from_xy(
                coordinate("x2", width, width),
                coordinate("y2", height, 0.0),
            );
            if start == end {
                // Début et fin confondus : la couleur du dernier arrêt.
                return Some(Shader::SolidColor(last));
            }
            LinearGradient::new(start, end, stops, spread, transform)
        } else {
            let cx = coordinate("cx", width, width / 2.0);
            let cy = coordinate("cy", height, height / 2.0);
            let r = coordinate("r", diagonal, diagonal / 2.0);
            if r <= 0.0 {
                return Some(Shader::SolidColor(last));
            }
            let fx = coordinate("fx", width, cx);
            let fy = coordinate("fy", height, cy);
            RadialGradient::new(
                Point::from_xy(fx, fy),
                Point::from_xy(cx, cy),
                r,
                stops,
                spread,
                transform,
            )
        }
    }

    /// Motif `node` et ceux dont il hérite par `href`, du plus proche au
    /// plus lointain.
    fn pattern_chain(&self, node: NodeId) -> Vec<(NodeId, &ElementData)> {
        let mut chain = Vec::new();
        let mut at = Some(node);
        while let Some(node) = at
            && chain.len() < MAX_CHAIN
        {
            let Some(el) = self.doc.element(node) else {
                break;
            };
            if el.ns != Namespace::Svg || el.name != "pattern" {
                break;
            }
            chain.push((node, el));
            at = href(el).and_then(|target| self.index.target(target));
        }
        chain
    }

    /// Pinceau du motif `node` pour une forme de boîte englobante `bounds`,
    /// dessinée avec la transformation `transform` (elle fixe la finesse de
    /// la tuile). `None` s'il ne peint rien.
    fn pattern(
        &self,
        node: NodeId,
        bounds: Rect,
        port: Port,
        opacity: f32,
        transform: Transform,
    ) -> Option<Brush> {
        if self.layers.get() >= MAX_LAYERS {
            return None;
        }
        let chain = self.pattern_chain(node);
        let attr = |name: &str| chain.iter().find_map(|(_, el)| el.attr(name));
        // Le contenu : celui du premier motif de la chaîne qui en a.
        let content = chain.iter().map(|(node, _)| *node).find(|node| {
            self.doc
                .children(*node)
                .any(|child| self.doc.element(child).is_some())
        })?;
        let user_space = attr("patternUnits") == Some("userSpaceOnUse");
        // Dans la boîte englobante, les longueurs en sont des fractions.
        let measure = |name: &str, base: f32, side: f32| -> f32 {
            let Some(text) = attr(name) else { return 0.0 };
            let text = text.trim();
            if user_space {
                length(text, base, 16.0).unwrap_or(0.0)
            } else {
                let fraction = match text.strip_suffix('%') {
                    Some(percent) => percent.trim().parse::<f32>().map_or(0.0, |v| v / 100.0),
                    None => text.parse::<f32>().unwrap_or(0.0),
                };
                fraction * side
            }
        };
        let (mut x, mut y) = (
            measure("x", port.width, bounds.width()),
            measure("y", port.height, bounds.height()),
        );
        if !user_space {
            x += bounds.x();
            y += bounds.y();
        }
        let width = measure("width", port.width, bounds.width());
        let height = measure("height", port.height, bounds.height());
        if !(width > 0.0 && height > 0.0) {
            return None;
        }
        let mut place = Transform::from_translate(x, y);
        if let Some(text) = attr("patternTransform") {
            place = parse_transform(text).pre_concat(place);
        }
        // Finesse de la tuile : celle du dessin final, bornée.
        let total = transform.pre_concat(place);
        // Par axe : un motif étiré dans un sens garde des bords nets.
        let scale_x = total.sx.hypot(total.ky).clamp(0.01, 64.0);
        let scale_y = total.kx.hypot(total.sy).clamp(0.01, 64.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "côtés entre 1 et MAX_TILE_SIDE"
        )]
        let side =
            |length: f32, scale: f32| (length * scale).ceil().clamp(1.0, MAX_TILE_SIDE) as u32;
        let mut tile = Pixmap::new(side(width, scale_x), side(height, scale_y))?;
        #[expect(clippy::cast_precision_loss, reason = "côtés d'au plus MAX_TILE_SIDE")]
        let (pixels_x, pixels_y) = (tile.width() as f32 / width, tile.height() as f32 / height);
        // Repère du contenu : celui du motif, son `viewBox`, ou la boîte
        // englobante de la forme.
        let mut inner = Transform::from_scale(pixels_x, pixels_y);
        let mut inner_port = Port { width, height };
        let view = attr("viewBox").map(numbers);
        if let Some([vx, vy, vw, vh]) = view.as_deref().and_then(|v| <[f32; 4]>::try_from(v).ok())
            && vw > 0.0
            && vh > 0.0
        {
            inner = inner.pre_concat(fit(
                [vx, vy, vw, vh],
                attr("preserveAspectRatio"),
                width,
                height,
            ));
            inner_port = Port {
                width: vw,
                height: vh,
            };
        } else if attr("patternContentUnits") == Some("objectBoundingBox") {
            inner = inner.pre_concat(Transform::from_scale(bounds.width(), bounds.height()));
        }
        self.layers.set(self.layers.get() + 1);
        self.draw_children(&mut tile, content, inner, inner_port, None, 0);
        self.layers.set(self.layers.get().saturating_sub(1));
        Some(Brush::Pattern {
            tile,
            place: place.pre_concat(Transform::from_scale(1.0 / pixels_x, 1.0 / pixels_y)),
            opacity,
        })
    }

    /// Pinceau de la peinture `paint` pour une forme de boîte englobante
    /// `bounds`, dessinée avec la transformation `transform`.
    fn brush(
        &self,
        paint: &SvgPaint,
        opacity: f32,
        current: oxyde_css::values::Color,
        bounds: Rect,
        (port, transform): (Port, Transform),
    ) -> Option<Brush> {
        let solid = |color: oxyde_css::values::Color| {
            (color.a > 0 && opacity > 0.0)
                .then(|| Brush::Shader(Shader::SolidColor(skia_color(color, opacity))))
        };
        match paint {
            SvgPaint::None => None,
            SvgPaint::Color(color) => solid(*color),
            SvgPaint::CurrentColor => solid(current),
            SvgPaint::Server(id, fallback) => match self.index.0.get(&**id) {
                Some(node)
                    if self.doc.element(*node).is_some_and(|el| {
                        matches!(el.name.as_str(), "linearGradient" | "radialGradient")
                    }) =>
                {
                    self.gradient(*node, bounds, port, opacity, current)
                        .map(Brush::Shader)
                }
                Some(node) if is_svg(self.doc, *node, "pattern") => {
                    self.pattern(*node, bounds, port, opacity, transform)
                }
                // Serveur absent ou d'un genre non géré : la couleur de repli.
                _ => solid((*fallback)?),
            },
        }
    }

    /// Remplit et trace `path` selon le style de `node`.
    fn paint_shape(
        &self,
        target: &mut Pixmap,
        path: &Path,
        svg: &SvgStyle,
        current: oxyde_css::values::Color,
        transform: Transform,
        port: Port,
    ) {
        let bounds = path.bounds();
        let place = (port, transform);
        if let Some(brush) = self.brush(&svg.fill, svg.fill_opacity, current, bounds, place) {
            let paint = brush.paint();
            let rule = if svg.even_odd {
                FillRule::EvenOdd
            } else {
                FillRule::Winding
            };
            target.fill_path(path, &paint, rule, transform, None);
        }
        if svg.stroke_width > 0.0
            && let Some(brush) = self.brush(&svg.stroke, svg.stroke_opacity, current, bounds, place)
        {
            let paint = brush.paint();
            let stroke = Stroke {
                width: svg.stroke_width,
                miter_limit: svg.miter_limit,
                line_cap: match svg.line_cap {
                    SvgLineCap::Butt => LineCap::Butt,
                    SvgLineCap::Round => LineCap::Round,
                    SvgLineCap::Square => LineCap::Square,
                },
                line_join: match svg.line_join {
                    SvgLineJoin::Miter => LineJoin::Miter,
                    SvgLineJoin::Round => LineJoin::Round,
                    SvgLineJoin::Bevel => LineJoin::Bevel,
                },
                dash: dash(&svg.dash_array, svg.dash_offset),
            };
            target.stroke_path(path, &paint, &stroke, transform, None);
        }
    }

    /// Dessine les enfants de `node`.
    fn draw_children(
        &self,
        target: &mut Pixmap,
        node: NodeId,
        transform: Transform,
        port: Port,
        rebase: Option<&Rebase>,
        depth: usize,
    ) {
        for child in self.doc.children(node) {
            self.draw(target, child, transform, port, rebase, depth + 1);
        }
    }

    /// Dessine l'élément `node` et sa descendance. `rebase` déplace la
    /// couleur courante des éléments atteints par un `use`.
    fn draw(
        &self,
        target: &mut Pixmap,
        node: NodeId,
        transform: Transform,
        port: Port,
        rebase: Option<&Rebase>,
        depth: usize,
    ) {
        if depth > MAX_DEPTH {
            return;
        }
        let Some(left) = self.budget.get().checked_sub(1) else {
            return;
        };
        self.budget.set(left);
        let (Some(el), Some(style)) = (self.doc.element(node), self.styles.get(node)) else {
            return;
        };
        if el.ns != Namespace::Svg || style.display == Display::None {
            return;
        }
        let group = matches!(el.name.as_str(), "g" | "a" | "svg" | "switch" | "use");
        let shape = matches!(
            el.name.as_str(),
            "path" | "rect" | "circle" | "ellipse" | "line" | "polyline" | "polygon"
        );
        if !group && !shape && el.name != "text" {
            return;
        }
        let bounds = (shape && style.transform_box != oxyde_css::TransformBox::ViewBox)
            .then(|| shape_path(el, port, style.font_size))
            .flatten()
            .map(|path| path.bounds());
        let transform = transform.pre_concat(own_transform(el, style, port, bounds));
        // Une opacité de groupe, une découpe ou un masque valent pour le
        // contenu pris d'un bloc : il est dessiné à part, puis fondu dans la
        // cible.
        let clip = self.referenced(el, "clip-path", "clipPath");
        let mask = self.referenced(el, "mask", "mask");
        let layered = style.opacity < 1.0 || clip.is_some() || mask.is_some();
        if !layered || self.layers.get() >= MAX_LAYERS {
            self.draw_content(target, node, el, style, transform, port, rebase, depth);
            return;
        }
        if style.opacity <= 0.0 {
            return;
        }
        let Some(mut layer) = Pixmap::new(target.width(), target.height()) else {
            return;
        };
        self.layers.set(self.layers.get() + 1);
        self.draw_content(&mut layer, node, el, style, transform, port, rebase, depth);
        if let Some(clip) = clip
            && let Some(mask) = self.clip_mask(clip, &layer, transform, port, depth)
        {
            layer.apply_mask(&mask);
        }
        if let Some(mask) = mask
            && let Some(mask) = self.luminance_mask(mask, &layer, transform, port, depth)
        {
            layer.apply_mask(&mask);
        }
        self.layers.set(self.layers.get().saturating_sub(1));
        let paint = PixmapPaint {
            opacity: style.opacity,
            ..PixmapPaint::default()
        };
        target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
    }

    /// Élément `kind` que désigne la propriété `name` de `el` (`url(#id)`).
    fn referenced(&self, el: &ElementData, name: &str, kind: &str) -> Option<NodeId> {
        let value = declared(el, name)?.trim();
        let address = value
            .strip_prefix("url(")?
            .split(')')
            .next()?
            .trim()
            .trim_matches(['"', '\'']);
        let node = self.index.target(address)?;
        is_svg(self.doc, node, kind).then_some(node)
    }

    /// Masque de la découpe `clip`, dans le repère `transform` de l'élément
    /// découpé. `None` si elle ne s'applique pas (unités non gérées).
    fn clip_mask(
        &self,
        clip: NodeId,
        like: &Pixmap,
        transform: Transform,
        port: Port,
        depth: usize,
    ) -> Option<Mask> {
        let el = self.doc.element(clip)?;
        let style = self.styles.get(clip)?;
        if el.attr("clipPathUnits") == Some("objectBoundingBox") {
            return None;
        }
        let transform = transform.pre_concat(own_transform(el, style, port, None));
        let mut mask = Mask::new(like.width(), like.height())?;
        for child in self.doc.children(clip) {
            self.clip_shape(&mut mask, child, transform, port, depth + 1);
        }
        Some(mask)
    }

    /// Ajoute au masque la forme `node` d'une découpe (ou celle que vise un
    /// `use`).
    fn clip_shape(
        &self,
        mask: &mut Mask,
        node: NodeId,
        transform: Transform,
        port: Port,
        depth: usize,
    ) {
        if depth > MAX_DEPTH {
            return;
        }
        let (Some(el), Some(style)) = (self.doc.element(node), self.styles.get(node)) else {
            return;
        };
        if el.ns != Namespace::Svg || style.display == Display::None {
            return;
        }
        let transform = transform.pre_concat(own_transform(el, style, port, None));
        if el.name == "use" {
            if let Some(target) = href(el).and_then(|target| self.index.target(target)) {
                let x = attr_length(el, "x", port.width, style.font_size, 0.0);
                let y = attr_length(el, "y", port.height, style.font_size, 0.0);
                let transform = transform.pre_concat(Transform::from_translate(x, y));
                self.clip_shape(mask, target, transform, port, depth + 1);
            }
            return;
        }
        if let Some(path) = shape_path(el, port, style.font_size) {
            let rule = match self.inherited(node, "clip-rule") {
                Some("evenodd") => FillRule::EvenOdd,
                _ => FillRule::Winding,
            };
            mask.fill_path(&path, rule, true, transform);
        }
    }

    /// Masque que dessine l'élément `mask`, dans le repère `transform` de
    /// l'élément masqué : la luminance de son contenu (ou son opacité avec
    /// `mask-type: alpha`).
    fn luminance_mask(
        &self,
        mask: NodeId,
        like: &Pixmap,
        transform: Transform,
        port: Port,
        depth: usize,
    ) -> Option<Mask> {
        let el = self.doc.element(mask)?;
        if el.attr("maskContentUnits") == Some("objectBoundingBox") {
            return None;
        }
        let mut content = Pixmap::new(like.width(), like.height())?;
        self.layers.set(self.layers.get() + 1);
        self.draw_children(&mut content, mask, transform, port, None, depth + 1);
        self.layers.set(self.layers.get().saturating_sub(1));
        let kind = if declared(el, "mask-type") == Some("alpha") {
            MaskType::Alpha
        } else {
            MaskType::Luminance
        };
        Some(Mask::from_pixmap(content.as_ref(), kind))
    }

    /// Dessine `node`, sa transformation et son opacité déjà prises en compte.
    #[expect(
        clippy::too_many_arguments,
        reason = "l'état de dessin, passé tel quel de proche en proche"
    )]
    fn draw_content(
        &self,
        target: &mut Pixmap,
        node: NodeId,
        el: &ElementData,
        style: &ComputedStyle,
        transform: Transform,
        port: Port,
        rebase: Option<&Rebase>,
        depth: usize,
    ) {
        let font_size = style.font_size;
        match el.name.as_str() {
            "text" => self.draw_text(target, node, transform, port, rebase, depth),
            "g" | "a" | "switch" => {
                self.draw_children(target, node, transform, port, rebase, depth)
            }
            "svg" => {
                let x = attr_length(el, "x", port.width, font_size, 0.0);
                let y = attr_length(el, "y", port.height, font_size, 0.0);
                let width = attr_length(el, "width", port.width, font_size, port.width);
                let height = attr_length(el, "height", port.height, font_size, port.height);
                let (inner, port) = viewport_of(el, Transform::from_translate(x, y), width, height);
                self.draw_children(
                    target,
                    node,
                    transform.pre_concat(inner),
                    port,
                    rebase,
                    depth,
                );
            }
            "use" => {
                let Some(referenced) = href(el).and_then(|target| self.index.target(target)) else {
                    return;
                };
                let Some(target_el) = self.doc.element(referenced) else {
                    return;
                };
                let x = attr_length(el, "x", port.width, font_size, 0.0);
                let y = attr_length(el, "y", port.height, font_size, 0.0);
                let transform = transform.pre_concat(Transform::from_translate(x, y));
                // Le contenu hérite du `use`, pas de l'endroit où le modèle est rangé.
                let (onto, onto_color) = effective(style, rebase);
                let context = self
                    .doc
                    .node(referenced)
                    .parent
                    .and_then(|parent| self.styles.get(parent));
                let inner = Rebase {
                    from: context.map_or_else(Default::default, |c| (*c.svg).clone()),
                    from_color: context.map_or(oxyde_css::values::Color::BLACK, |c| c.color),
                    onto,
                    onto_color,
                };
                let rebase = Some(&inner);
                if matches!(target_el.name.as_str(), "symbol" | "svg") {
                    let size = |name: &str, base: f32| {
                        el.attr(name)
                            .or_else(|| target_el.attr(name))
                            .and_then(|value| length(value, base, font_size))
                            .unwrap_or(base)
                    };
                    let (inner, port) = viewport_of(
                        target_el,
                        Transform::identity(),
                        size("width", port.width),
                        size("height", port.height),
                    );
                    self.draw_children(
                        target,
                        referenced,
                        transform.pre_concat(inner),
                        port,
                        rebase,
                        depth,
                    );
                } else {
                    self.draw(target, referenced, transform, port, rebase, depth + 1);
                }
            }
            _ => {
                if style.visibility != Visibility::Visible {
                    return;
                }
                if let Some(path) = shape_path(el, port, font_size) {
                    let (svg, current) = effective(style, rebase);
                    self.paint_shape(target, &path, &svg, current, transform, port);
                }
            }
        }
    }
}

/// Fenêtre que crée un élément `svg` ou `symbol` de taille `width` ×
/// `height` : la transformation de son contenu et la nouvelle référence des
/// pourcentages.
fn viewport_of(el: &ElementData, place: Transform, width: f32, height: f32) -> (Transform, Port) {
    match view_box(el) {
        Some(view) => (
            place.pre_concat(fit(view, el.attr("preserveAspectRatio"), width, height)),
            Port {
                width: view[2],
                height: view[3],
            },
        ),
        None => (place, Port { width, height }),
    }
}

/// Pointillé d'un contour ; `None` pour un trait continu.
fn dash(array: &[f32], offset: f32) -> Option<StrokeDash> {
    if array.is_empty() || array.iter().sum::<f32>() <= 0.0 {
        return None;
    }
    // Une liste impaire est répétée une fois.
    let mut list = array.to_vec();
    if list.len() % 2 == 1 {
        list.extend_from_slice(array);
    }
    StrokeDash::new(list, offset)
}

// ---- Texte ------------------------------------------------------------------------------------

/// Morceau de texte mis en forme, en attente de l'ancrage de sa ligne.
struct Piece {
    /// Contour des glyphes, l'origine sur la ligne de base au début du morceau.
    outline: Path,
    /// Début du morceau sur la ligne.
    x: f32,
    /// Ligne de base.
    y: f32,
    svg: SvgStyle,
    color: oxyde_css::values::Color,
}

/// État du dessin d'un élément `text`.
struct TextRun {
    /// Position courante : où commence le prochain glyphe.
    x: f32,
    y: f32,
    /// Morceaux de la ligne en cours, depuis sa dernière position absolue.
    pieces: Vec<Piece>,
    /// Début de la ligne en cours et part de sa largeur à retrancher
    /// (`text-anchor` : 0, 0,5 ou 1).
    start: f32,
    anchor: f32,
    /// Vrai si le dernier caractère gardé est un blanc (ou s'il n'y en a pas).
    after_space: bool,
}

impl Scene<'_> {
    /// Valeur de la propriété `name` déclarée par `node` ou, à défaut, par
    /// son plus proche ancêtre SVG.
    fn inherited(&self, node: NodeId, name: &str) -> Option<&str> {
        let mut at = Some(node);
        for _ in 0..MAX_DEPTH {
            let el = self.doc.element(at?)?;
            if el.ns != Namespace::Svg {
                return None;
            }
            if let Some(value) = declared(el, name) {
                return Some(value);
            }
            at = self.doc.node(at?).parent;
        }
        None
    }

    /// Peint la ligne en cours, décalée selon son ancrage.
    fn flush_text(&self, target: &mut Pixmap, run: &mut TextRun, transform: Transform, port: Port) {
        let shift = -(run.x - run.start) * run.anchor;
        for piece in run.pieces.drain(..) {
            let place = Transform::from_translate(piece.x + shift, piece.y);
            if let Some(path) = piece.outline.transform(place) {
                self.paint_shape(target, &path, &piece.svg, piece.color, transform, port);
            }
        }
        run.start = run.x;
    }

    /// Met en forme le contenu de `node` (un `text`, un `tspan` ou un `a`)
    /// à la suite de la ligne en cours.
    #[expect(
        clippy::too_many_arguments,
        reason = "l'état de dessin, passé tel quel de proche en proche"
    )]
    fn text_content(
        &self,
        target: &mut Pixmap,
        node: NodeId,
        run: &mut TextRun,
        transform: Transform,
        port: Port,
        rebase: Option<&Rebase>,
        depth: usize,
    ) {
        let (Some(el), Some(style), Some(context)) = (
            self.doc.element(node),
            self.styles.get(node),
            self.text.as_ref(),
        ) else {
            return;
        };
        if depth > MAX_DEPTH || style.display == Display::None {
            return;
        }
        let font_size = style.font_size;
        // Une position absolue commence une nouvelle ligne à ancrer.
        let first = |name: &str, base: f32| {
            let value = el.attr(name)?;
            let head = value.split([',', ' ']).find(|part| !part.is_empty())?;
            length(head, base, font_size)
        };
        let (x, y) = (first("x", port.width), first("y", port.height));
        if x.is_some() || y.is_some() || el.name == "text" {
            self.flush_text(target, run, transform, port);
            run.x = x.unwrap_or(run.x);
            run.y = y.unwrap_or(run.y);
            run.start = run.x;
            run.anchor = match self.inherited(node, "text-anchor") {
                Some("middle") => 0.5,
                Some("end") => 1.0,
                _ => 0.0,
            };
        }
        run.x += first("dx", port.width).unwrap_or(0.0);
        run.y += first("dy", port.height).unwrap_or(0.0);
        let font = FontSpec {
            size: font_size,
            family: oxyde_text::css_font_family(&style.font_family),
            weight: style.font_weight,
            italic: style.italic,
        };
        let baseline = self
            .inherited(node, "dominant-baseline")
            .or_else(|| declared(el, "alignment-baseline"));
        for child in self.doc.children(node) {
            if let Some(text) = self.doc.text(child) {
                // Les blancs se fondent en un seul ; ceux du début tombent.
                let mut kept = String::with_capacity(text.len());
                for c in text.chars() {
                    if c.is_whitespace() {
                        if !run.after_space {
                            kept.push(' ');
                        }
                        run.after_space = true;
                    } else {
                        kept.push(c);
                        run.after_space = false;
                    }
                }
                if kept.is_empty() {
                    continue;
                }
                let shaped = shape_text(&mut context.borrow_mut(), &kept, &font);
                // Où poser la ligne de base par rapport à la position donnée.
                let drop = match baseline {
                    Some("middle") => font_size * 0.25,
                    Some("central") => (shaped.ascent - shaped.descent) / 2.0,
                    Some("hanging" | "text-before-edge" | "text-top") => shaped.ascent,
                    _ => 0.0,
                };
                if style.visibility == Visibility::Visible
                    && let Some(outline) = shaped.outline.and_then(|path| {
                        path.transform(Transform::from_translate(0.0, -shaped.baseline))
                    })
                {
                    let (svg, color) = effective(style, rebase);
                    run.pieces.push(Piece {
                        outline,
                        x: run.x,
                        y: run.y + drop,
                        svg,
                        color,
                    });
                }
                run.x += shaped.width;
            } else if self.doc.element(child).is_some_and(|el| {
                el.ns == Namespace::Svg && matches!(el.name.as_str(), "tspan" | "a")
            }) {
                self.text_content(target, child, run, transform, port, rebase, depth + 1);
            }
        }
    }

    /// Dessine un élément `text`.
    fn draw_text(
        &self,
        target: &mut Pixmap,
        node: NodeId,
        transform: Transform,
        port: Port,
        rebase: Option<&Rebase>,
        depth: usize,
    ) {
        let mut run = TextRun {
            x: 0.0,
            y: 0.0,
            pieces: Vec::new(),
            start: 0.0,
            anchor: 0.0,
            after_space: true,
        };
        self.text_content(target, node, &mut run, transform, port, rebase, depth);
        // Un blanc final ne compte pas dans la largeur à ancrer.
        if run.after_space
            && let (Some(context), Some(style)) = (self.text.as_ref(), self.styles.get(node))
            && !run.pieces.is_empty()
        {
            let font = FontSpec {
                size: style.font_size,
                family: oxyde_text::css_font_family(&style.font_family),
                weight: style.font_weight,
                italic: style.italic,
            };
            // Largeur d'une espace : celle de « a a » moins deux « a ».
            let wide = shape_text(&mut context.borrow_mut(), "a a", &font).width;
            let narrow = shape_text(&mut context.borrow_mut(), "aa", &font).width;
            run.x -= (wide - narrow).max(0.0);
        }
        self.flush_text(target, &mut run, transform, port);
    }
}

// ---- Entrées ----------------------------------------------------------------------------------

/// Taille propre de l'élément `<svg>` `node`, en pixels CSS : ses attributs
/// `width` et `height`, l'un déduit de l'autre par le `viewBox` au besoin.
/// `None` s'il n'a qu'un `viewBox` : il n'a alors que des proportions (que
/// le moteur de style lui donne par `aspect-ratio`) et prend la place qu'on
/// lui laisse.
pub(crate) fn intrinsic_size(doc: &Document, node: NodeId) -> Option<(f32, f32)> {
    let el = doc.element(node)?;
    // Un pourcentage n'est pas une taille propre.
    let absolute = |name: &str| {
        el.attr(name)
            .filter(|value| !value.contains('%'))
            .and_then(|value| length(value, 0.0, 16.0))
            .filter(|value| *value >= 0.0)
    };
    let view = view_box(el);
    match (absolute("width"), absolute("height"), view) {
        (Some(w), Some(h), _) => Some((w, h)),
        (Some(w), None, Some([_, _, vw, vh])) => Some((w, w * vh / vw)),
        (None, Some(h), Some([_, _, vw, vh])) => Some((h * vw / vh, h)),
        (None, None, Some(_)) => None,
        (Some(w), None, None) => Some((w, DEFAULT_SIZE.1)),
        (None, Some(h), None) => Some((DEFAULT_SIZE.0, h)),
        (None, None, None) => Some(DEFAULT_SIZE),
    }
}

/// Taille à donner à la mise en page pour l'élément `<svg>` `node` de style
/// `style` : sa taille propre. La taille par défaut d'un `<svg>` sans
/// dimensions ni `viewBox` n'est qu'un repli : elle cède devant
/// `aspect-ratio`.
pub(crate) fn layout_size(
    doc: &Document,
    node: NodeId,
    style: Option<&ComputedStyle>,
) -> Option<(f32, f32)> {
    let size = intrinsic_size(doc, node)?;
    let el = doc.element(node)?;
    let declared = ["width", "height", "viewBox"]
        .iter()
        .any(|name| el.attr(name).is_some());
    (declared || style.is_none_or(|style| style.aspect_ratio.is_none())).then_some(size)
}

/// Empreinte de ce dont dépend le rendu de `root` à la taille `width` ×
/// `height` : tant qu'elle ne change pas, l'image peut être gardée.
///
/// Elle couvre le sous-arbre de `root` et ses styles, pas les éléments
/// visés par un `use` ou un dégradé hors de ce sous-arbre.
pub(crate) fn signature(
    doc: &Document,
    styles: &StyleMap,
    root: NodeId,
    width: f32,
    height: f32,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.to_bits().hash(&mut hasher);
    height.to_bits().hash(&mut hasher);
    for node in std::iter::once(root).chain(doc.descendants(root)) {
        let Some(el) = doc.element(node) else {
            continue;
        };
        el.name.hash(&mut hasher);
        for attr in &el.attrs {
            attr.name.hash(&mut hasher);
            attr.value.hash(&mut hasher);
        }
        if let Some(style) = styles.get(node) {
            let color = style.color;
            [color.r, color.g, color.b, color.a].hash(&mut hasher);
            style.opacity.to_bits().hash(&mut hasher);
            (style.display == Display::None).hash(&mut hasher);
            (style.visibility == Visibility::Visible).hash(&mut hasher);
            format!("{:?}{:?}", style.svg, style.transform).hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Rend l'élément `<svg>` `root` dans une image de la taille donnée
/// (largeur, hauteur) en pixels CSS, à `scale` pixels par pixel CSS.
pub(crate) fn render(
    doc: &Document,
    styles: &StyleMap,
    index: &Index<'_>,
    root: NodeId,
    (width, height): (f32, f32),
    scale: f32,
    text: Option<&mut TextContext>,
) -> Option<Image> {
    let el = doc.element(root)?;
    if !(width > 0.0 && height > 0.0 && scale > 0.0) {
        return None;
    }
    // La surface est bornée : au-delà, le rendu perd en finesse.
    let scale = scale.min(MAX_SIDE / width).min(MAX_SIDE / height);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "côtés entre 1 et MAX_SIDE"
    )]
    let side = |value: f32| (value * scale).ceil().clamp(1.0, MAX_SIDE) as u32;
    let mut pixmap = Pixmap::new(side(width), side(height))?;
    let (inner, port) = viewport_of(el, Transform::from_scale(scale, scale), width, height);
    let scene = Scene {
        doc,
        styles,
        index,
        text: text.map(RefCell::new),
        layers: Cell::new(0),
        budget: Cell::new(MAX_ELEMENTS),
    };
    scene.draw_children(&mut pixmap, root, inner, port, None, 0);
    // `tiny-skia` garde des couleurs prémultipliées ; l'image ne l'est pas.
    let mut rgba = Vec::with_capacity(pixmap.data().len());
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    Image::from_rgba(pixmap.width(), pixmap.height(), rgba)
}

/// Rend un fichier SVG entier, à sa taille propre. `None` si ce n'en est
/// pas un.
pub(crate) fn decode(bytes: &[u8]) -> Option<Image> {
    let text = std::str::from_utf8(bytes).ok()?;
    if !text.contains("<svg") {
        return None;
    }
    let doc = oxyde_html::parse_xml(text).ok()?;
    let root = doc
        .children(doc.root())
        .find(|node| is_svg(&doc, *node, "svg"))?;
    // Un fichier sans dimensions prend celles de son `viewBox`.
    let (width, height) = intrinsic_size(&doc, root)
        .or_else(|| view_box(doc.element(root)?).map(|view| (view[2], view[3])))?;
    let sheets = oxyde_css::collect_style_elements(&doc);
    let styles = oxyde_css::compute_styles(&doc, &sheets, Viewport { width, height });
    render(
        &doc,
        &styles,
        &Index::new(&doc),
        root,
        (width, height),
        1.0,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * image.width() + x) * 4) as usize;
        let data = image.data().data.data();
        [data[at], data[at + 1], data[at + 2], data[at + 3]]
    }

    #[test]
    fn transform_attributes_take_plain_numbers_only() {
        let at = |text: &str| {
            let t = parse_transform(text);
            (t.tx, t.ty)
        };
        assert_eq!(at("translate(10 20)"), (10.0, 20.0));
        assert_eq!(at("translate(10,20) translate(1)"), (11.0, 20.0));
        assert_eq!(at("translate(1e1-5)"), (10.0, -5.0));
        // Unités, virgule en trop ou nombre d'arguments faux : rien ne s'applique.
        assert_eq!(at("translate(10px 20px)"), (0.0, 0.0));
        assert_eq!(at("translate(10,)"), (0.0, 0.0));
        assert_eq!(at("translate(10 20) rotate(90,)"), (0.0, 0.0));
        assert_eq!(at("translate(1 2 3)"), (0.0, 0.0));
    }

    #[test]
    fn numbers_are_read_however_they_are_packed() {
        assert_eq!(numbers("1.5-2,3e1 .5.25"), vec![1.5, -2.0, 30.0, 0.5, 0.25]);
        assert_eq!(length("50%", 200.0, 16.0), Some(100.0));
        assert_eq!(length("2em", 0.0, 10.0), Some(20.0));
        assert_eq!(length("12", 0.0, 16.0), Some(12.0));
        assert_eq!(length("1wat", 0.0, 16.0), None);
    }

    #[test]
    fn paths_follow_every_command() {
        let path = parse_path("M0 0h10v10H0zm20 0 10 10").expect("tracé");
        let bounds = path.bounds();
        assert_eq!((bounds.left(), bounds.top()), (0.0, 0.0));
        assert_eq!((bounds.right(), bounds.bottom()), (30.0, 10.0));
        // Arc à drapeaux collés : un demi-cercle de rayon 5.
        let arc = parse_path("M0 0a5 5 0 0110 0").expect("arc");
        let bounds = arc.bounds();
        assert!((bounds.top() + 5.0).abs() < 0.01, "{bounds:?}");
        assert!((bounds.right() - 10.0).abs() < 0.01, "{bounds:?}");
        // Ce qui précède une erreur est gardé.
        assert!(parse_path("M0 0L10 10 L oups").is_some());
        assert!(parse_path("L10 10").is_none());
    }

    #[test]
    fn a_file_is_rendered_with_its_view_box_styles_and_gradients() {
        let image = decode(
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 20 10">
                   <style>.b { fill: #00f }</style>
                   <defs>
                     <linearGradient id="g"><stop offset="0" stop-color="#f00"/>
                       <stop offset="1" stop-color="#0f0"/></linearGradient>
                     <rect id="carre" width="2" height="2" fill="currentColor"/>
                   </defs>
                   <rect width="5" height="5" fill="red"/>
                   <rect class="b" x="5" width="5" height="5"/>
                   <g transform="translate(10 0)" opacity="0.5"><rect width="5" height="5"/></g>
                   <rect y="5" width="20" height="5" fill="url(#g)"/>
                   <use href="#carre" x="16" y="1" color="#ff0"/>
                   <symbol id="s" viewBox="0 0 4 4"><rect width="4" height="2"/>
                     <rect y="2" width="4" height="2" fill="#0ff"/></symbol>
                   <use href="#s" x="10" y="0" width="2" height="2" fill="#f0f"/>
                   <clipPath id="c"><rect x="18.5" width="0.5" height="5"/></clipPath>
                   <mask id="m"><rect x="19" width="0.5" height="5" fill="#fff"/></mask>
                   <rect x="18" width="2" height="1" fill="#f00" clip-path="url(#c)"/>
                   <g mask="url(#m)"><rect x="18" y="3" width="2" height="1" fill="#f00"/></g>
                   <pattern id="p" patternUnits="userSpaceOnUse" width="1" height="1">
                     <rect width="0.5" height="1" fill="#0f0"/></pattern>
                   <rect x="16" y="4" width="2" height="1" fill="url(#p)"/>
                   <rect x="18" y="4" width="1" height="1" fill="#00f"
                         transform="rotate(90)" transform-origin="19 5"/>
                 </svg>"##,
        )
        .expect("image");
        assert_eq!((image.width(), image.height()), (40, 20));
        assert_eq!(pixel(&image, 5, 5), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 15, 5), [0, 0, 255, 255]);
        // Noir à moitié opaque.
        let half = pixel(&image, 25, 5);
        assert_eq!(&half[..3], &[0, 0, 0]);
        assert!((i32::from(half[3]) - 128).abs() <= 1, "{half:?}");
        // Le dégradé va du rouge au vert.
        let (left, right) = (pixel(&image, 1, 15), pixel(&image, 38, 15));
        assert!(left[0] > 230 && left[1] < 25, "{left:?}");
        assert!(right[1] > 230 && right[0] < 25, "{right:?}");
        // Le modèle prend la couleur du `use`.
        assert_eq!(pixel(&image, 34, 4), [255, 255, 0, 255]);
        // Le contenu d'un symbole hérite du `use`, sauf ce qu'il déclare.
        assert_eq!(pixel(&image, 22, 0), [255, 0, 255, 255]);
        assert_eq!(pixel(&image, 22, 3), [0, 255, 255, 255]);
        assert_eq!(pixel(&image, 30, 8)[3], 0, "le reste est transparent");
        // La découpe et le masque ne laissent passer qu'une colonne chacun.
        assert_eq!(pixel(&image, 37, 1), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 36, 1)[3], 0);
        assert_eq!(pixel(&image, 39, 1)[3], 0);
        assert_eq!(pixel(&image, 38, 7), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 36, 7)[3], 0);
        // Le motif alterne une bande verte et une bande vide, tous les deux pixels.
        assert_eq!(pixel(&image, 32, 9), [0, 255, 0, 255]);
        assert_eq!(pixel(&image, 33, 9)[3], 0);
        assert_eq!(pixel(&image, 34, 9), [0, 255, 0, 255]);
        // Un quart de tour autour de `transform-origin` : le carré bleu
        // passe de (18, 4) à (19, 4).
        assert_eq!(pixel(&image, 39, 9), [0, 0, 255, 255]);
        assert_eq!(pixel(&image, 37, 9)[3], 0);
        assert!(decode(b"pas du SVG").is_none());
    }
}
