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
//! LIMITES CONNUES : pas de texte (`text`, `tspan`), d'image (`image`), de
//! découpe (`clipPath`), de masque, de motif, de marqueur ni de filtre ; un
//! `use` ne suit pas une adresse vers un autre fichier, et son contenu
//! hérite de lui par approximation (une propriété qui vaut celle du
//! contexte où le modèle est rangé est tenue pour héritée) ; la
//! couleur d'un arrêt de dégradé se lit dans ses attributs, pas dans les
//! feuilles de style ; `transform-origin` est ignoré ; le rendu se fait à
//! deux pixels par pixel CSS, quel que soit l'écran ; une image SVG chargée
//! par `<img>` est rendue une fois, à sa taille propre.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use oxyde_css::{
    ComputedStyle, Display, StyleMap, SvgLineCap, SvgLineJoin, SvgPaint, SvgStyle, Viewport,
    Visibility,
};
use oxyde_dom::{Document, ElementData, Namespace, NodeId};
use oxyde_paint::Image;
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke, StrokeDash,
    Transform,
};

/// Côté maximal de la surface de rendu, en pixels.
const MAX_SIDE: f32 = 4096.0;
/// Profondeur maximale de l'arbre dessiné, `use` compris.
const MAX_DEPTH: usize = 48;
/// Nombre maximal de dégradés suivis par `href`.
const MAX_CHAIN: usize = 8;
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

/// Éléments `<svg>` les plus extérieurs de `doc` : ceux que la page met en
/// page comme des éléments remplacés.
pub(crate) fn outer_roots(doc: &Document) -> Vec<NodeId> {
    doc.descendants(doc.root())
        .filter(|node| {
            is_svg(doc, *node, "svg")
                && !doc
                    .node(*node)
                    .parent
                    .and_then(|parent| doc.element(parent))
                    .is_some_and(|parent| parent.ns == Namespace::Svg)
        })
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

/// Valeur de l'attribut `transform` (ou `gradientTransform`).
fn parse_transform(text: &str) -> Transform {
    let mut out = Transform::identity();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|at| open + at) else {
            break;
        };
        let name = rest[..open].trim_matches(|c: char| c.is_whitespace() || c == ',');
        let args = numbers(&rest[open + 1..close]);
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

/// Transformation propre à un élément : son attribut `transform`, à défaut
/// la propriété CSS du même nom.
fn own_transform(el: &ElementData, style: &ComputedStyle) -> Transform {
    if let Some(text) = el.attr("transform") {
        return parse_transform(text);
    }
    match style.transform_matrix(0.0, 0.0) {
        Some(matrix) => {
            let [a, b, c, d, e, f] = matrix.0;
            Transform::from_row(a, b, c, d, e, f)
        }
        None => Transform::identity(),
    }
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

    /// Peinture `paint` pour une forme de boîte englobante `bounds`.
    fn shader(
        &self,
        paint: &SvgPaint,
        opacity: f32,
        current: oxyde_css::values::Color,
        bounds: Rect,
        port: Port,
    ) -> Option<Shader<'static>> {
        let solid = |color: oxyde_css::values::Color| {
            (color.a > 0 && opacity > 0.0).then(|| Shader::SolidColor(skia_color(color, opacity)))
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
        if let Some(shader) = self.shader(&svg.fill, svg.fill_opacity, current, bounds, port) {
            let paint = Paint {
                shader,
                anti_alias: true,
                ..Paint::default()
            };
            let rule = if svg.even_odd {
                FillRule::EvenOdd
            } else {
                FillRule::Winding
            };
            target.fill_path(path, &paint, rule, transform, None);
        }
        if svg.stroke_width > 0.0
            && let Some(shader) =
                self.shader(&svg.stroke, svg.stroke_opacity, current, bounds, port)
        {
            let paint = Paint {
                shader,
                anti_alias: true,
                ..Paint::default()
            };
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
        if !group && !shape {
            return;
        }
        let transform = transform.pre_concat(own_transform(el, style));
        // Une opacité de groupe se compose d'un bloc : le contenu est dessiné
        // à part, puis fondu dans la cible.
        if style.opacity < 1.0 {
            if style.opacity <= 0.0 {
                return;
            }
            let Some(mut layer) = Pixmap::new(target.width(), target.height()) else {
                return;
            };
            self.draw_content(&mut layer, node, el, style, transform, port, rebase, depth);
            let paint = PixmapPaint {
                opacity: style.opacity,
                ..PixmapPaint::default()
            };
            target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
        } else {
            self.draw_content(target, node, el, style, transform, port, rebase, depth);
        }
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

// ---- Entrées ----------------------------------------------------------------------------------

/// Taille propre de l'élément `<svg>` `node`, en pixels CSS : ses attributs
/// `width` et `height`, à défaut ceux que donne son `viewBox`.
pub(crate) fn intrinsic_size(doc: &Document, node: NodeId) -> (f32, f32) {
    let Some(el) = doc.element(node) else {
        return DEFAULT_SIZE;
    };
    // Un pourcentage n'est pas une taille propre.
    let absolute = |name: &str| {
        el.attr(name)
            .filter(|value| !value.contains('%'))
            .and_then(|value| length(value, 0.0, 16.0))
            .filter(|value| *value >= 0.0)
    };
    let view = view_box(el);
    match (absolute("width"), absolute("height"), view) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some([_, _, vw, vh])) => (w, w * vh / vw),
        (None, Some(h), Some([_, _, vw, vh])) => (h * vw / vh, h),
        (None, None, Some([_, _, vw, vh])) => (vw, vh),
        (Some(w), None, None) => (w, DEFAULT_SIZE.1),
        (None, Some(h), None) => (DEFAULT_SIZE.0, h),
        (None, None, None) => DEFAULT_SIZE,
    }
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

/// Rend l'élément `<svg>` `root` dans une image de `width` × `height`
/// pixels CSS, à `scale` pixels par pixel CSS.
pub(crate) fn render(
    doc: &Document,
    styles: &StyleMap,
    index: &Index<'_>,
    root: NodeId,
    width: f32,
    height: f32,
    scale: f32,
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
    let scene = Scene { doc, styles, index };
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
    let (width, height) = intrinsic_size(&doc, root);
    let sheets = oxyde_css::collect_style_elements(&doc);
    let styles = oxyde_css::compute_styles(&doc, &sheets, Viewport { width, height });
    render(&doc, &styles, &Index::new(&doc), root, width, height, 1.0)
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
        assert!(decode(b"pas du SVG").is_none());
    }
}
