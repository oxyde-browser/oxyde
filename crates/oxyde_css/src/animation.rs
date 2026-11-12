//! Animations et transitions CSS (CSS Animations 1, CSS Transitions 1).
//!
//! L'[`Animator`] garde, d'un calcul de style à l'autre, l'état des
//! animations et des transitions de chaque élément. À chaque calcul, la
//! cascade produit le style « de base » d'un élément ; l'animateur y
//! superpose la valeur courante des transitions, puis celle des animations,
//! et note les événements à distribuer (`animationstart`, `transitionend`…).
//!
//! LIMITES CONNUES : pas d'API Web Animations (`getAnimations`,
//! `Element.animate`) ; `animation-composition` et les fonctions de
//! temporisation propres à une image clé ne sont pas pris en charge ; deux
//! transformations de formes différentes sont interpolées coefficient par
//! coefficient plutôt que par décomposition de matrice ; l'inversion d'une
//! transition en cours repart de la valeur courante sans raccourcir sa durée.

use std::collections::HashMap;
use std::sync::Arc;

use oxyde_dom::NodeId;

use crate::properties::{Declaration, Longhand, Side, parse_declarations};
use crate::style::{ComputedStyle, Context, LineHeight, Size, Visibility, apply, copy_property};
use crate::stylesheet::Stylesheet;
use crate::syntax::{Token, matching_close, split_top_level, tokenize, trim};
use crate::transform::TransformFunction;
use crate::values::Color;

/// Fonction de temporisation (`ease`, `cubic-bezier()`, `steps()`…).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Easing {
    /// `linear`
    Linear,
    /// `cubic-bezier(x1, y1, x2, y2)`
    CubicBezier(f32, f32, f32, f32),
    /// `steps(n, …)` ; le booléen est vrai si le saut a lieu au début de
    /// chaque palier (`start`, `jump-start`).
    Steps(u32, bool),
}

impl Easing {
    /// `ease`, la valeur initiale.
    pub const EASE: Self = Self::CubicBezier(0.25, 0.1, 0.25, 1.0);

    /// Analyse une fonction de temporisation.
    #[must_use]
    pub fn parse(tokens: &[Token]) -> Option<Self> {
        let tokens = trim(tokens);
        match tokens {
            [Token::Ident(word)] => Some(match word.to_ascii_lowercase().as_str() {
                "linear" => Self::Linear,
                "ease" => Self::EASE,
                "ease-in" => Self::CubicBezier(0.42, 0.0, 1.0, 1.0),
                "ease-out" => Self::CubicBezier(0.0, 0.0, 0.58, 1.0),
                "ease-in-out" => Self::CubicBezier(0.42, 0.0, 0.58, 1.0),
                "step-start" => Self::Steps(1, true),
                "step-end" => Self::Steps(1, false),
                _ => return None,
            }),
            [Token::Function(name), ..] => {
                let close = matching_close(tokens, 0);
                if close + 1 != tokens.len() {
                    return None;
                }
                let parts: Vec<&[Token]> = split_top_level(tokens.get(1..close)?, &Token::Comma)
                    .into_iter()
                    .map(trim)
                    .collect();
                match (name.as_str(), parts.as_slice()) {
                    ("cubic-bezier", [x1, y1, x2, y2]) => {
                        let number = |part: &[Token]| match part {
                            [Token::Number(v)] => Some(*v),
                            _ => None,
                        };
                        let (x1, x2) = (number(x1)?, number(x2)?);
                        if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
                            return None;
                        }
                        Some(Self::CubicBezier(x1, number(y1)?, x2, number(y2)?))
                    }
                    ("steps", [count, rest @ ..]) if rest.len() <= 1 => {
                        let [Token::Number(count)] = count else {
                            return None;
                        };
                        if *count < 1.0 || count.fract() != 0.0 {
                            return None;
                        }
                        let at_start = match rest {
                            [] => false,
                            [[Token::Ident(position)]] => {
                                match position.to_ascii_lowercase().as_str() {
                                    "start" | "jump-start" | "jump-both" => true,
                                    "end" | "jump-end" | "jump-none" => false,
                                    _ => return None,
                                }
                            }
                            _ => return None,
                        };
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_sign_loss,
                            reason = "entier positif vérifié ci-dessus"
                        )]
                        Some(Self::Steps(*count as u32, at_start))
                    }
                    ("linear", _) => Some(Self::Linear),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Avancement de l'effet pour une fraction `t` du temps écoulé, de 0 à 1.
    #[must_use]
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Steps(count, at_start) => {
                #[expect(clippy::cast_precision_loss, reason = "nombre de paliers petit")]
                let count = count.max(1) as f32;
                let step = if at_start {
                    (t * count).floor() + 1.0
                } else {
                    (t * count).floor()
                };
                (step / count).clamp(0.0, 1.0)
            }
            Self::CubicBezier(x1, y1, x2, y2) => {
                if t <= 0.0 || t >= 1.0 {
                    return t;
                }
                // Courbe de Bézier d'extrémités (0, 0) et (1, 1) : on cherche
                // par dichotomie le paramètre dont l'abscisse vaut `t`.
                let bezier = |p1: f32, p2: f32, s: f32| {
                    let inv = 1.0 - s;
                    3.0 * inv * inv * s * p1 + 3.0 * inv * s * s * p2 + s * s * s
                };
                let (mut low, mut high) = (0.0_f32, 1.0_f32);
                let mut s = t;
                for _ in 0..24 {
                    let x = bezier(x1, x2, s);
                    if (x - t).abs() < 1.0e-5 {
                        break;
                    }
                    if x < t {
                        low = s;
                    } else {
                        high = s;
                    }
                    s = (low + high) / 2.0;
                }
                bezier(y1, y2, s)
            }
        }
    }
}

/// Valeur de `animation-direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    /// `normal`
    #[default]
    Normal,
    /// `reverse`
    Reverse,
    /// `alternate`
    Alternate,
    /// `alternate-reverse`
    AlternateReverse,
}

/// Valeur de `animation-fill-mode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FillMode {
    /// `none`
    #[default]
    None,
    /// `forwards`
    Forwards,
    /// `backwards`
    Backwards,
    /// `both`
    Both,
}

/// Valeurs calculées des propriétés `animation-*` : une liste par
/// propriété, dans l'ordre des animations.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AnimationStyle {
    /// `animation-name` ; `none` pour une entrée sans animation.
    pub names: Vec<String>,
    /// `animation-duration`, en millisecondes.
    pub durations: Vec<f32>,
    /// `animation-delay`, en millisecondes.
    pub delays: Vec<f32>,
    /// `animation-timing-function`
    pub easings: Vec<Easing>,
    /// `animation-iteration-count` ; l'infini pour `infinite`.
    pub iterations: Vec<f32>,
    /// `animation-direction`
    pub directions: Vec<Direction>,
    /// `animation-fill-mode`
    pub fill_modes: Vec<FillMode>,
    /// `animation-play-state` : vrai pour `paused`.
    pub paused: Vec<bool>,
}

/// Valeurs calculées des propriétés `transition-*`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TransitionStyle {
    /// `transition-property` (`all`, `none` ou un nom de propriété).
    pub properties: Vec<String>,
    /// `transition-duration`, en millisecondes.
    pub durations: Vec<f32>,
    /// `transition-delay`, en millisecondes.
    pub delays: Vec<f32>,
    /// `transition-timing-function`
    pub easings: Vec<Easing>,
}

/// Élément de rang `index` d'une liste répétée à la demande, comme le veut
/// CSS quand les listes d'une famille de propriétés n'ont pas la même
/// longueur.
fn cycled<T: Copy>(list: &[T], index: usize, default: T) -> T {
    if list.is_empty() {
        default
    } else {
        list[index % list.len()]
    }
}

/// Image clé d'une règle `@keyframes`.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    /// Position dans l'animation, de 0 à 1.
    pub offset: f32,
    /// Déclarations de l'image clé.
    pub declarations: Vec<Declaration>,
}

/// Règle `@keyframes`.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframes {
    /// Nom de l'animation.
    pub name: String,
    /// Images clés, triées par position.
    pub frames: Vec<Keyframe>,
}

/// Analyse une règle `@keyframes` : `prelude` porte son nom, `body` ses
/// images clés.
#[must_use]
pub fn parse_keyframes(prelude: &[Token], body: &[Token]) -> Option<Keyframes> {
    let name = match trim(prelude) {
        [Token::Ident(name) | Token::Str(name)] => name.clone(),
        _ => return None,
    };
    let mut frames = Vec::new();
    let mut at = 0;
    while at < body.len() {
        let Some(open) = body
            .iter()
            .skip(at)
            .position(|t| *t == Token::OpenBrace)
            .map(|p| p + at)
        else {
            break;
        };
        let close = matching_close(body, open);
        let declarations = parse_declarations(body.get(open + 1..close).unwrap_or(&[]));
        let selectors = body.get(at..open).unwrap_or(&[]);
        let offsets: Option<Vec<f32>> = split_top_level(selectors, &Token::Comma)
            .into_iter()
            .map(|selector| match trim(selector) {
                [Token::Ident(word)] if word.eq_ignore_ascii_case("from") => Some(0.0),
                [Token::Ident(word)] if word.eq_ignore_ascii_case("to") => Some(1.0),
                [Token::Percentage(p)] if (0.0..=100.0).contains(p) => Some(p / 100.0),
                _ => None,
            })
            .collect();
        // Un sélecteur invalide écarte toute l'image clé.
        if let Some(offsets) = offsets {
            for offset in offsets {
                frames.push(Keyframe {
                    offset,
                    declarations: declarations.clone(),
                });
            }
        }
        at = close + 1;
    }
    frames.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    Some(Keyframes { name, frames })
}

/// Événement d'animation ou de transition à distribuer aux scripts.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEvent {
    /// Élément cible.
    pub node: NodeId,
    /// Type de l'événement (`animationstart`, `transitionend`…).
    pub kind: &'static str,
    /// Nom de l'animation, ou de la propriété en transition.
    pub name: String,
    /// Temps écoulé, en secondes.
    pub elapsed: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Idle,
    Before,
    Active,
    After,
}

struct Running {
    name: String,
    /// Instant de départ (début du délai), en millisecondes.
    start: f64,
    /// Temps écoulé au moment de la mise en pause.
    held: Option<f64>,
    phase: Phase,
    iteration: u32,
}

struct Transition {
    property: Longhand,
    name: String,
    from: Arc<ComputedStyle>,
    to: Arc<ComputedStyle>,
    /// Instant de création, en millisecondes.
    start: f64,
    delay: f32,
    duration: f32,
    easing: Easing,
    started: bool,
}

#[derive(Default)]
struct ElementState {
    animations: Vec<Running>,
    transitions: Vec<Transition>,
    /// Dernier style rendu, animations et transitions comprises.
    output: Option<Arc<ComputedStyle>>,
}

/// État des animations et transitions d'un document.
#[derive(Default)]
pub struct Animator {
    /// Style de base de chaque élément au calcul précédent, par index de nœud.
    previous: Vec<Option<Arc<ComputedStyle>>>,
    /// Styles de base du calcul en cours.
    current: Vec<Option<Arc<ComputedStyle>>>,
    elements: HashMap<NodeId, ElementState>,
    events: Vec<AnimationEvent>,
    /// Vrai si une animation ou une transition avance avec le temps.
    running: bool,
    /// Vrai si le dernier calcul a superposé au moins un effet.
    effective: bool,
    /// Noms de propriété déjà résolus en propriétés élémentaires.
    resolved: HashMap<String, Vec<Longhand>>,
}

impl std::fmt::Debug for Animator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animator")
            .field("elements", &self.elements.len())
            .field("running", &self.running)
            .finish_non_exhaustive()
    }
}

/// Propriétés élémentaires que `transition-property: all` couvre.
fn all_longhands() -> Vec<Longhand> {
    let sides = [Side::Top, Side::Right, Side::Bottom, Side::Left];
    let mut all = vec![
        Longhand::Color,
        Longhand::BackgroundColor,
        Longhand::FontSize,
        Longhand::FontWeight,
        Longhand::LineHeight,
        Longhand::BorderRadius,
        Longhand::Width,
        Longhand::Height,
        Longhand::MinWidth,
        Longhand::MaxWidth,
        Longhand::MinHeight,
        Longhand::MaxHeight,
        Longhand::ZIndex,
        Longhand::Visibility,
        Longhand::Opacity,
        Longhand::FlexGrow,
        Longhand::FlexShrink,
        Longhand::FlexBasis,
        Longhand::Order,
        Longhand::RowGap,
        Longhand::ColumnGap,
        Longhand::BorderSpacingH,
        Longhand::BorderSpacingV,
        Longhand::VerticalAlign,
        Longhand::AspectRatio,
        Longhand::Transform,
        Longhand::TransformOriginX,
        Longhand::TransformOriginY,
    ];
    for side in sides {
        all.extend([
            Longhand::Margin(side),
            Longhand::Padding(side),
            Longhand::BorderWidth(side),
            Longhand::BorderColor(side),
            Longhand::Inset(side),
        ]);
    }
    all
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    // Interpolation en couleurs prémultipliées : une couleur transparente
    // ne teinte pas le résultat.
    let alpha = |c: Color| f32::from(c.a) / 255.0;
    let (aa, ba) = (alpha(a), alpha(b));
    let out_alpha = lerp(aa, ba, t);
    let channel = |x: u8, y: u8| {
        if out_alpha <= 0.0 {
            return 0;
        }
        let value = lerp(f32::from(x) * aa, f32::from(y) * ba, t) / out_alpha;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "valeur bornée à 0..=255"
        )]
        {
            value.round().clamp(0.0, 255.0) as u8
        }
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "valeur bornée à 0..=255"
    )]
    let a_out = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    Color {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: a_out,
    }
}

/// Interpole deux tailles ; `None` si l'une vaut `auto` (pas d'interpolation).
fn lerp_size(a: Size, b: Size, t: f32) -> Option<Size> {
    let parts = |size: Size| match size {
        Size::Auto => None,
        Size::Px(v) => Some((v, 0.0)),
        Size::Percent(p) => Some((0.0, p)),
        Size::Calc { px, percent } => Some((px, percent)),
    };
    match (a, b) {
        (Size::Px(x), Size::Px(y)) => Some(Size::Px(lerp(x, y, t))),
        (Size::Percent(x), Size::Percent(y)) => Some(Size::Percent(lerp(x, y, t))),
        _ => {
            let ((ax, ap), (bx, bp)) = (parts(a)?, parts(b)?);
            Some(Size::Calc {
                px: lerp(ax, bx, t),
                percent: lerp(ap, bp, t),
            })
        }
    }
}

type Functions = [TransformFunction<Size>];

/// Fonction neutre de même forme que `function`.
fn neutral(function: &TransformFunction<Size>) -> TransformFunction<Size> {
    match function {
        TransformFunction::Translate(..) => {
            TransformFunction::Translate(Size::Px(0.0), Size::Px(0.0))
        }
        TransformFunction::Scale(..) => TransformFunction::Scale(1.0, 1.0),
        TransformFunction::Rotate(_) => TransformFunction::Rotate(0.0),
        TransformFunction::Skew(..) => TransformFunction::Skew(0.0, 0.0),
        TransformFunction::Matrix(_) => {
            TransformFunction::Matrix(crate::transform::Affine::IDENTITY.0)
        }
    }
}

fn lerp_function(
    a: &TransformFunction<Size>,
    b: &TransformFunction<Size>,
    t: f32,
) -> Option<TransformFunction<Size>> {
    Some(match (a, b) {
        (TransformFunction::Translate(ax, ay), TransformFunction::Translate(bx, by)) => {
            TransformFunction::Translate(lerp_size(*ax, *bx, t)?, lerp_size(*ay, *by, t)?)
        }
        (TransformFunction::Scale(ax, ay), TransformFunction::Scale(bx, by)) => {
            TransformFunction::Scale(lerp(*ax, *bx, t), lerp(*ay, *by, t))
        }
        (TransformFunction::Rotate(a), TransformFunction::Rotate(b)) => {
            TransformFunction::Rotate(lerp(*a, *b, t))
        }
        (TransformFunction::Skew(ax, ay), TransformFunction::Skew(bx, by)) => {
            TransformFunction::Skew(lerp(*ax, *bx, t), lerp(*ay, *by, t))
        }
        (TransformFunction::Matrix(a), TransformFunction::Matrix(b)) => {
            let mut out = [0.0; 6];
            for (slot, (x, y)) in out.iter_mut().zip(a.iter().zip(b)) {
                *slot = lerp(*x, *y, t);
            }
            TransformFunction::Matrix(out)
        }
        _ => return None,
    })
}

fn lerp_transform(a: Option<&Functions>, b: Option<&Functions>, t: f32) -> Option<Arc<Functions>> {
    let (a, b): (Vec<_>, Vec<_>) = match (a, b) {
        (None, None) => return None,
        (Some(a), None) => (a.to_vec(), a.iter().map(neutral).collect()),
        (None, Some(b)) => (b.iter().map(neutral).collect(), b.to_vec()),
        (Some(a), Some(b)) => (a.to_vec(), b.to_vec()),
    };
    if a.len() == b.len() {
        let pairs: Option<Vec<_>> = a
            .iter()
            .zip(&b)
            .map(|(x, y)| lerp_function(x, y, t))
            .collect();
        if let Some(list) = pairs {
            return Some(list.into());
        }
    }
    // Formes différentes : chaque liste est ramenée à sa matrice (les
    // pourcentages de translation comptant pour zéro).
    let matrix = |list: &[TransformFunction<Size>]| {
        let style = ComputedStyle {
            transform: Some(list.into()),
            ..ComputedStyle::initial()
        };
        style
            .transform_functions_matrix(0.0, 0.0)
            .unwrap_or(crate::transform::Affine::IDENTITY)
            .0
    };
    let (ma, mb) = (matrix(&a), matrix(&b));
    let mut out = [0.0; 6];
    for (slot, (x, y)) in out.iter_mut().zip(ma.iter().zip(&mb)) {
        *slot = lerp(*x, *y, t);
    }
    Some(vec![TransformFunction::Matrix(out)].into())
}

/// Écrit dans `out` la valeur de `property` interpolée entre `from` et `to`
/// à l'avancement `t` ; une propriété qui ne s'interpole pas bascule à
/// mi-course.
fn interpolate(
    property: Longhand,
    from: &ComputedStyle,
    to: &ComputedStyle,
    t: f32,
    out: &mut ComputedStyle,
) {
    let discrete = |out: &mut ComputedStyle| {
        copy_property(out, if t < 0.5 { from } else { to }, property);
    };
    let size = |a: Size, b: Size| lerp_size(a, b, t);
    match property {
        Longhand::Opacity => out.opacity = lerp(from.opacity, to.opacity, t).clamp(0.0, 1.0),
        Longhand::Color => out.color = lerp_color(from.color, to.color, t),
        Longhand::BackgroundColor => {
            out.background_color = lerp_color(from.background_color, to.background_color, t);
        }
        Longhand::BorderColor(side) => {
            let color = lerp_color(from.border_color_of(side), to.border_color_of(side), t);
            out.border_color.set(side, Some(color));
        }
        Longhand::FontSize => out.font_size = lerp(from.font_size, to.font_size, t).max(0.0),
        Longhand::FontWeight => out.font_weight = lerp(from.font_weight, to.font_weight, t),
        Longhand::BorderRadius => {
            out.border_radius = lerp(from.border_radius, to.border_radius, t).max(0.0);
        }
        Longhand::FlexGrow => out.flex_grow = lerp(from.flex_grow, to.flex_grow, t).max(0.0),
        Longhand::FlexShrink => {
            out.flex_shrink = lerp(from.flex_shrink, to.flex_shrink, t).max(0.0);
        }
        Longhand::BorderSpacingH => {
            out.border_spacing.0 = lerp(from.border_spacing.0, to.border_spacing.0, t);
        }
        Longhand::BorderSpacingV => {
            out.border_spacing.1 = lerp(from.border_spacing.1, to.border_spacing.1, t);
        }
        Longhand::BorderWidth(side) => {
            // La largeur utilisée : une bordure sans style compte pour zéro.
            let width = lerp(
                from.border_widths().get(side),
                to.border_widths().get(side),
                t,
            );
            copy_property(out, to, Longhand::BorderStyle(side));
            if to.border_widths().get(side) == 0.0 && t < 1.0 {
                copy_property(out, from, Longhand::BorderStyle(side));
            }
            out.border_width.set(side, width.max(0.0));
        }
        Longhand::LineHeight => match (from.line_height, to.line_height) {
            (LineHeight::Number(a), LineHeight::Number(b)) => {
                out.line_height = LineHeight::Number(lerp(a, b, t));
            }
            (LineHeight::Px(a), LineHeight::Px(b)) => {
                out.line_height = LineHeight::Px(lerp(a, b, t))
            }
            _ => discrete(out),
        },
        Longhand::Margin(side) => match size(from.margin.get(side), to.margin.get(side)) {
            Some(value) => out.margin.set(side, value),
            None => discrete(out),
        },
        Longhand::Padding(side) => match size(from.padding.get(side), to.padding.get(side)) {
            Some(value) => out.padding.set(side, value),
            None => discrete(out),
        },
        Longhand::Inset(side) => match size(from.inset.get(side), to.inset.get(side)) {
            Some(value) => out.inset.set(side, value),
            None => discrete(out),
        },
        Longhand::Width => match size(from.width, to.width) {
            Some(value) => {
                out.width = value;
                out.width_keyword = crate::style::SizeKeyword::None;
            }
            None => discrete(out),
        },
        Longhand::Height => match size(from.height, to.height) {
            Some(value) => out.height = value,
            None => discrete(out),
        },
        Longhand::MinWidth => match size(from.min_width, to.min_width) {
            Some(value) => out.min_width = value,
            None => discrete(out),
        },
        Longhand::MaxWidth => match size(from.max_width, to.max_width) {
            Some(value) => out.max_width = value,
            None => discrete(out),
        },
        Longhand::MinHeight => match size(from.min_height, to.min_height) {
            Some(value) => out.min_height = value,
            None => discrete(out),
        },
        Longhand::MaxHeight => match size(from.max_height, to.max_height) {
            Some(value) => out.max_height = value,
            None => discrete(out),
        },
        Longhand::RowGap => match size(from.row_gap, to.row_gap) {
            Some(value) => out.row_gap = value,
            None => discrete(out),
        },
        Longhand::ColumnGap => match size(from.column_gap, to.column_gap) {
            Some(value) => out.column_gap = value,
            None => discrete(out),
        },
        Longhand::TransformOriginX => match size(from.transform_origin.0, to.transform_origin.0) {
            Some(value) => out.transform_origin.0 = value,
            None => discrete(out),
        },
        Longhand::TransformOriginY => match size(from.transform_origin.1, to.transform_origin.1) {
            Some(value) => out.transform_origin.1 = value,
            None => discrete(out),
        },
        Longhand::FlexBasis => match (from.flex_basis, to.flex_basis) {
            (Some(a), Some(b)) => match size(a, b) {
                Some(value) => out.flex_basis = Some(value),
                None => discrete(out),
            },
            _ => discrete(out),
        },
        Longhand::ZIndex => match (from.z_index, to.z_index) {
            (Some(a), Some(b)) => {
                #[expect(clippy::cast_precision_loss, reason = "indices d'empilement modestes")]
                let value = lerp(a as f32, b as f32, t).round();
                #[expect(clippy::cast_possible_truncation, reason = "valeur entre deux i32")]
                {
                    out.z_index = Some(value as i32);
                }
            }
            _ => discrete(out),
        },
        Longhand::Order => {
            #[expect(clippy::cast_precision_loss, reason = "ordres modestes")]
            let value = lerp(from.order as f32, to.order as f32, t).round();
            #[expect(clippy::cast_possible_truncation, reason = "valeur entre deux i32")]
            {
                out.order = value as i32;
            }
        }
        Longhand::AspectRatio => match (from.aspect_ratio, to.aspect_ratio) {
            (Some(a), Some(b)) => out.aspect_ratio = Some(lerp(a.ln(), b.ln(), t).exp()),
            _ => discrete(out),
        },
        Longhand::Visibility => {
            // `visible` l'emporte tant que la transition n'est pas à son terme.
            let visible = |style: &ComputedStyle| style.visibility == Visibility::Visible;
            out.visibility = if t <= 0.0 {
                from.visibility
            } else if t >= 1.0 {
                to.visibility
            } else if visible(from) || visible(to) {
                Visibility::Visible
            } else if t < 0.5 {
                from.visibility
            } else {
                to.visibility
            };
        }
        Longhand::Transform => {
            out.transform = lerp_transform(from.transform.as_deref(), to.transform.as_deref(), t);
        }
        _ => discrete(out),
    }
}

/// Vrai si `property` n'a pas la même valeur dans `a` et dans `b`.
fn differs(property: Longhand, a: &ComputedStyle, b: &ComputedStyle) -> bool {
    let mut probe = a.clone();
    copy_property(&mut probe, b, property);
    probe != *a
}

/// Nom CSS d'une propriété élémentaire (pour `propertyName`).
fn longhand_name(property: Longhand) -> String {
    let side = |side: Side| match side {
        Side::Top => "top",
        Side::Right => "right",
        Side::Bottom => "bottom",
        Side::Left => "left",
    };
    match property {
        Longhand::Margin(s) => format!("margin-{}", side(s)),
        Longhand::Padding(s) => format!("padding-{}", side(s)),
        Longhand::BorderWidth(s) => format!("border-{}-width", side(s)),
        Longhand::BorderColor(s) => format!("border-{}-color", side(s)),
        Longhand::BorderStyle(s) => format!("border-{}-style", side(s)),
        Longhand::Inset(s) => side(s).to_owned(),
        Longhand::BorderSpacingH | Longhand::BorderSpacingV => "border-spacing".to_owned(),
        Longhand::TransformOriginX | Longhand::TransformOriginY => "transform-origin".to_owned(),
        other => {
            // `BackgroundColor` → `background-color`.
            let debug = format!("{other:?}");
            let mut name = String::new();
            for c in debug.chars() {
                if c.is_ascii_uppercase() && !name.is_empty() {
                    name.push('-');
                }
                name.push(c.to_ascii_lowercase());
            }
            name
        }
    }
}

/// Règle `@keyframes` de nom `name` : la dernière déclarée l'emporte.
fn find_keyframes<'a>(sheets: &'a [Stylesheet], name: &str) -> Option<&'a Keyframes> {
    sheets
        .iter()
        .rev()
        .flat_map(|sheet| sheet.keyframes.iter().rev())
        .find(|keyframes| keyframes.name == name)
}

impl Animator {
    /// Animateur sans animation en cours.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Vrai si une animation ou une transition avance avec le temps : le
    /// style doit être recalculé à chaque image.
    #[must_use]
    pub fn needs_frames(&self) -> bool {
        self.running
    }

    /// Vrai si le dernier calcul de style dépendait de l'instant : une
    /// animation ou une transition y a contribué.
    #[must_use]
    pub fn is_effective(&self) -> bool {
        self.effective || self.running
    }

    /// Événements produits depuis le dernier appel, dans l'ordre.
    pub fn take_events(&mut self) -> Vec<AnimationEvent> {
        std::mem::take(&mut self.events)
    }

    /// Prépare un calcul de style sur un document de `len` nœuds.
    pub(crate) fn begin(&mut self, len: usize) {
        self.current = vec![None; len];
        self.running = false;
        self.effective = false;
    }

    /// Clôt un calcul de style : oublie les éléments disparus.
    pub(crate) fn end(&mut self) {
        let current = std::mem::take(&mut self.current);
        self.elements
            .retain(|node, _| current.get(node.index()).is_some_and(Option::is_some));
        self.previous = current;
    }

    /// Propriétés élémentaires que désigne un nom de `transition-property`.
    fn longhands_of(&mut self, name: &str) -> Vec<Longhand> {
        if let Some(known) = self.resolved.get(name) {
            return known.clone();
        }
        let list: Vec<Longhand> = if name == "all" {
            all_longhands()
        } else {
            parse_declarations(&tokenize(&format!("{name}:inherit")))
                .into_iter()
                .map(|declaration| declaration.property)
                .collect()
        };
        self.resolved.insert(name.to_owned(), list.clone());
        list
    }

    /// Superpose au style de base `base` de `node` ses transitions et ses
    /// animations à l'instant `now` (millisecondes). `important` : les
    /// propriétés fixées par une déclaration `!important`, sur lesquelles
    /// une animation n'a pas prise.
    pub(crate) fn animate(
        &mut self,
        node: NodeId,
        base: ComputedStyle,
        sheets: &[Stylesheet],
        cx: &Context<'_>,
        important: &[Longhand],
        now: f64,
    ) -> Arc<ComputedStyle> {
        let base = Arc::new(base);
        let previous = self.previous.get(node.index()).cloned().flatten();
        if let Some(slot) = self.current.get_mut(node.index()) {
            *slot = Some(Arc::clone(&base));
        }
        let has_state = self.elements.contains_key(&node);
        if !has_state && base.animation.is_none() && base.transition.is_none() {
            return base;
        }
        let mut state = self.elements.remove(&node).unwrap_or_default();
        self.update_transitions(node, &mut state, previous.as_ref(), &base, now);
        self.update_animations(node, &mut state, &base, now);

        let mut out: Option<ComputedStyle> = None;
        // Transitions : valeur courante entre le départ et l'arrivée.
        let mut finished = Vec::new();
        for (index, transition) in state.transitions.iter_mut().enumerate() {
            let elapsed = now - transition.start - f64::from(transition.delay);
            if elapsed >= 0.0 && !transition.started {
                transition.started = true;
                self.events.push(AnimationEvent {
                    node,
                    kind: "transitionstart",
                    name: transition.name.clone(),
                    elapsed: f64::from((-transition.delay).max(0.0)) / 1000.0,
                });
            }
            if elapsed >= f64::from(transition.duration) {
                finished.push(index);
                continue;
            }
            self.running = true;
            #[expect(clippy::cast_possible_truncation, reason = "fraction entre 0 et 1")]
            let fraction = if transition.duration > 0.0 {
                (elapsed / f64::from(transition.duration)).clamp(0.0, 1.0) as f32
            } else {
                1.0
            };
            let progress = transition.easing.at(fraction);
            let target = out.get_or_insert_with(|| ComputedStyle::clone(&base));
            interpolate(
                transition.property,
                &transition.from,
                &transition.to,
                progress,
                target,
            );
        }
        for index in finished.into_iter().rev() {
            let transition = state.transitions.remove(index);
            self.events.push(AnimationEvent {
                node,
                kind: "transitionend",
                name: transition.name,
                elapsed: f64::from(transition.duration) / 1000.0,
            });
        }

        // Animations : elles l'emportent sur les transitions.
        if let Some(spec) = base.animation.clone() {
            for (index, running) in state.animations.iter().enumerate() {
                let Some(progress) = Self::animation_progress(&spec, index, running, now) else {
                    continue;
                };
                let Some(keyframes) = find_keyframes(sheets, &running.name) else {
                    continue;
                };
                let easing = cycled(&spec.easings, index, Easing::EASE);
                let target = out.get_or_insert_with(|| ComputedStyle::clone(&base));
                Self::apply_keyframes(keyframes, &base, cx, important, progress, easing, target);
            }
        }

        let result = match out {
            Some(style) => {
                self.effective = true;
                Arc::new(style)
            }
            None => Arc::clone(&base),
        };
        if state.animations.is_empty() && state.transitions.is_empty() && base.transition.is_none()
        {
            // Plus rien à suivre pour cet élément.
            return result;
        }
        state.output = Some(Arc::clone(&result));
        self.elements.insert(node, state);
        result
    }

    /// Démarre, remplace ou annule les transitions de l'élément d'après le
    /// changement de son style de base.
    fn update_transitions(
        &mut self,
        node: NodeId,
        state: &mut ElementState,
        previous: Option<&Arc<ComputedStyle>>,
        base: &Arc<ComputedStyle>,
        now: f64,
    ) {
        let Some(previous) = previous else {
            // Premier style de l'élément : rien ne transite.
            return;
        };
        if Arc::ptr_eq(previous, base) || **previous == **base {
            return;
        }
        // Un élément qui n'était pas rendu n'a pas de valeur d'où partir.
        if previous.display == crate::style::Display::None {
            state.transitions.clear();
            return;
        }
        let spec = base.transition.clone();
        // Valeur de départ : ce que l'élément affichait à l'instant.
        let shown = state.output.clone().unwrap_or_else(|| Arc::clone(previous));
        // Propriétés couvertes, la dernière entrée de la liste l'emportant.
        let mut covered: Vec<(Longhand, usize)> = Vec::new();
        if let Some(spec) = &spec {
            for (index, name) in spec.properties.iter().enumerate() {
                if name == "none" {
                    continue;
                }
                for property in self.longhands_of(name) {
                    covered.retain(|(known, _)| *known != property);
                    covered.push((property, index));
                }
            }
        }
        // Une transition dont la cible a changé, ou que plus rien ne couvre,
        // est annulée.
        let mut kept = Vec::new();
        for transition in std::mem::take(&mut state.transitions) {
            let property = transition.property;
            let retargeted = differs(property, &transition.to, base);
            let still_covered = covered.iter().any(|(known, _)| *known == property);
            if retargeted || !still_covered {
                self.events.push(AnimationEvent {
                    node,
                    kind: "transitioncancel",
                    name: transition.name.clone(),
                    elapsed: (now - transition.start - f64::from(transition.delay)).max(0.0)
                        / 1000.0,
                });
            } else {
                kept.push(transition);
            }
        }
        state.transitions = kept;
        let Some(spec) = spec else {
            return;
        };
        for (property, index) in covered {
            if !differs(property, previous, base) {
                continue;
            }
            if state.transitions.iter().any(|t| t.property == property) {
                continue;
            }
            let duration = cycled(&spec.durations, index, 0.0).max(0.0);
            let delay = cycled(&spec.delays, index, 0.0);
            if duration + delay <= 0.0 || !differs(property, &shown, base) {
                continue;
            }
            let name = longhand_name(property);
            self.events.push(AnimationEvent {
                node,
                kind: "transitionrun",
                name: name.clone(),
                elapsed: f64::from((-delay).max(0.0)) / 1000.0,
            });
            state.transitions.push(Transition {
                property,
                name,
                from: Arc::clone(&shown),
                to: Arc::clone(base),
                start: now,
                delay,
                duration,
                easing: cycled(&spec.easings, index, Easing::EASE),
                started: false,
            });
        }
    }

    /// Accorde les animations en cours à `animation-name`, fait avancer
    /// leur phase et note les événements correspondants.
    fn update_animations(
        &mut self,
        node: NodeId,
        state: &mut ElementState,
        base: &Arc<ComputedStyle>,
        now: f64,
    ) {
        let spec = base.animation.clone().unwrap_or_default();
        let mut old = std::mem::take(&mut state.animations);
        for (index, name) in spec.names.iter().enumerate() {
            let mut running = match old.iter().position(|r| r.name == *name) {
                Some(position) => old.remove(position),
                None => Running {
                    name: name.clone(),
                    start: now,
                    held: None,
                    phase: Phase::Idle,
                    iteration: 0,
                },
            };
            if name == "none" {
                state.animations.push(running);
                continue;
            }
            // Pause : le temps écoulé est figé, puis repris où il en était.
            let paused = cycled(&spec.paused, index, false);
            match (paused, running.held) {
                (true, None) => running.held = Some(now - running.start),
                (false, Some(held)) => {
                    running.start = now - held;
                    running.held = None;
                }
                _ => {}
            }
            let duration = f64::from(cycled(&spec.durations, index, 0.0).max(0.0));
            let delay = f64::from(cycled(&spec.delays, index, 0.0));
            let iterations = f64::from(cycled(&spec.iterations, index, 1.0).max(0.0));
            let total = duration * iterations;
            let active_time = running.held.unwrap_or(now - running.start) - delay;
            let (phase, iteration) = if active_time < 0.0 {
                (Phase::Before, 0)
            } else if active_time >= total {
                (Phase::After, 0)
            } else {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "rang d'itération positif"
                )]
                let iteration = if duration > 0.0 {
                    (active_time / duration).floor() as u32
                } else {
                    0
                };
                (Phase::Active, iteration)
            };
            let seconds = |ms: f64| ms / 1000.0;
            let mut emit = |kind: &'static str, elapsed: f64| {
                self.events.push(AnimationEvent {
                    node,
                    kind,
                    name: name.clone(),
                    elapsed,
                });
            };
            let was_started = matches!(running.phase, Phase::Active | Phase::After);
            let is_started = matches!(phase, Phase::Active | Phase::After);
            if !was_started && is_started {
                emit("animationstart", seconds((-delay).max(0.0)));
            }
            if running.phase == Phase::Active
                && phase == Phase::Active
                && iteration != running.iteration
            {
                emit(
                    "animationiteration",
                    seconds(duration * f64::from(iteration)),
                );
            }
            if running.phase != Phase::After && phase == Phase::After {
                emit("animationend", seconds(total));
            }
            if phase != Phase::After && running.held.is_none() {
                self.running = true;
            }
            running.phase = phase;
            running.iteration = iteration;
            state.animations.push(running);
        }
        // Animations retirées de `animation-name`.
        for gone in old {
            if gone.name != "none" && matches!(gone.phase, Phase::Before | Phase::Active) {
                let elapsed = (gone.held.unwrap_or(now - gone.start)).max(0.0) / 1000.0;
                self.events.push(AnimationEvent {
                    node,
                    kind: "animationcancel",
                    name: gone.name,
                    elapsed,
                });
            }
        }
    }

    /// Avancement, de 0 à 1 dans le sens de lecture, de l'animation de rang
    /// `index` ; `None` si elle ne produit aucun effet à cet instant.
    fn animation_progress(
        spec: &AnimationStyle,
        index: usize,
        running: &Running,
        now: f64,
    ) -> Option<f32> {
        if running.name == "none" {
            return None;
        }
        let duration = f64::from(cycled(&spec.durations, index, 0.0).max(0.0));
        let delay = f64::from(cycled(&spec.delays, index, 0.0));
        let iterations = f64::from(cycled(&spec.iterations, index, 1.0).max(0.0));
        let fill = cycled(&spec.fill_modes, index, FillMode::None);
        let direction = cycled(&spec.directions, index, Direction::Normal);
        let total = duration * iterations;
        let active_time = running.held.unwrap_or(now - running.start) - delay;
        // (itération, fraction de l'itération)
        let (iteration, fraction) = if active_time < 0.0 {
            if !matches!(fill, FillMode::Backwards | FillMode::Both) {
                return None;
            }
            (0.0, 0.0)
        } else if active_time >= total {
            if !matches!(fill, FillMode::Forwards | FillMode::Both) {
                return None;
            }
            if iterations == 0.0 {
                (0.0, 0.0)
            } else if iterations.is_finite() && iterations.fract() == 0.0 {
                (iterations - 1.0, 1.0)
            } else if iterations.is_finite() {
                (iterations.floor(), iterations.fract())
            } else {
                (0.0, 1.0)
            }
        } else {
            let overall = active_time / duration;
            (overall.floor(), overall.fract())
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "parité d'un rang d'itération"
        )]
        let odd = (iteration as i64) % 2 == 1;
        let reversed = match direction {
            Direction::Normal => false,
            Direction::Reverse => true,
            Direction::Alternate => odd,
            Direction::AlternateReverse => !odd,
        };
        #[expect(clippy::cast_possible_truncation, reason = "fraction entre 0 et 1")]
        let fraction = fraction as f32;
        Some(if reversed { 1.0 - fraction } else { fraction })
    }

    /// Écrit dans `out` les valeurs que les images clés donnent à
    /// l'avancement `progress`.
    fn apply_keyframes(
        keyframes: &Keyframes,
        base: &ComputedStyle,
        cx: &Context<'_>,
        important: &[Longhand],
        progress: f32,
        easing: Easing,
        out: &mut ComputedStyle,
    ) {
        // Style de chaque image clé : le style de base, ses déclarations en plus.
        let styles: Vec<ComputedStyle> = keyframes
            .frames
            .iter()
            .map(|frame| {
                let mut style = base.clone();
                for declaration in &frame.declarations {
                    apply(&mut style, declaration, cx);
                }
                style
            })
            .collect();
        let mut properties: Vec<Longhand> = Vec::new();
        for frame in &keyframes.frames {
            for declaration in &frame.declarations {
                let property = declaration.property;
                if !properties.contains(&property) && !important.contains(&property) {
                    properties.push(property);
                }
            }
        }
        for property in properties {
            // Images clés qui fixent cette propriété ; le style de base tient
            // lieu de début et de fin s'ils manquent.
            let mut stops: Vec<(f32, &ComputedStyle)> = keyframes
                .frames
                .iter()
                .zip(&styles)
                .filter(|(frame, _)| frame.declarations.iter().any(|d| d.property == property))
                .map(|(frame, style)| (frame.offset, style))
                .collect();
            if stops.first().is_none_or(|(offset, _)| *offset > 0.0) {
                stops.insert(0, (0.0, base));
            }
            if stops.last().is_none_or(|(offset, _)| *offset < 1.0) {
                stops.push((1.0, base));
            }
            // Dernier arrêt situé à `progress` ou avant, et l'arrêt suivant.
            let after = stops
                .iter()
                .position(|(offset, _)| *offset > progress)
                .unwrap_or(stops.len() - 1)
                .max(1);
            let (from_offset, from) = stops[after - 1];
            let (to_offset, to) = stops[after];
            let span = to_offset - from_offset;
            let local = if span > 0.0 {
                ((progress - from_offset) / span).clamp(0.0, 1.0)
            } else {
                1.0
            };
            interpolate(property, from, to, easing.at(local), out);
        }
    }
}

/// Durée en millisecondes (`2s`, `150ms`).
fn time(token: &Token) -> Option<f32> {
    match token {
        Token::Dimension(v, unit) if unit == "s" => Some(v * 1000.0),
        Token::Dimension(v, unit) if unit == "ms" => Some(*v),
        _ => None,
    }
}

/// Éléments d'une liste séparée par des virgules, chacun analysé par `item`.
fn comma_list<T>(tokens: &[Token], item: impl Fn(&[Token]) -> Option<T>) -> Option<Vec<T>> {
    split_top_level(tokens, &Token::Comma)
        .into_iter()
        .map(|part| item(trim(part)))
        .collect()
}

/// Liste de durées ; `allow_negative` pour les délais.
#[must_use]
pub fn parse_times(tokens: &[Token], allow_negative: bool) -> Option<Vec<f32>> {
    comma_list(tokens, |part| match part {
        [token] => time(token).filter(|v| allow_negative || *v >= 0.0),
        _ => None,
    })
}

/// Liste de fonctions de temporisation.
#[must_use]
pub fn parse_easings(tokens: &[Token]) -> Option<Vec<Easing>> {
    comma_list(tokens, Easing::parse)
}

/// Liste de nombres d'itérations (`infinite` : l'infini).
#[must_use]
pub fn parse_iterations(tokens: &[Token]) -> Option<Vec<f32>> {
    comma_list(tokens, |part| match part {
        [Token::Number(v)] if *v >= 0.0 => Some(*v),
        [Token::Ident(word)] if word.eq_ignore_ascii_case("infinite") => Some(f32::INFINITY),
        _ => None,
    })
}

/// Liste d'identifiants pris dans `allowed` (tous, si `allowed` est vide).
#[must_use]
pub fn parse_names(tokens: &[Token], allowed: &[&str]) -> Option<Vec<String>> {
    comma_list(tokens, |part| match part {
        [Token::Ident(word)] => {
            let lower = word.to_ascii_lowercase();
            if allowed.is_empty() {
                // Les noms d'animation gardent leur casse ; `none` est un mot-clé.
                Some(if lower == "none" { lower } else { word.clone() })
            } else {
                allowed.contains(&lower.as_str()).then_some(lower)
            }
        }
        [Token::Str(name)] if allowed.is_empty() => Some(name.clone()),
        _ => None,
    })
}

/// Valeurs de `animation-direction`.
pub const DIRECTIONS: &[&str] = &["normal", "reverse", "alternate", "alternate-reverse"];
/// Valeurs de `animation-fill-mode`.
pub const FILL_MODES: &[&str] = &["none", "forwards", "backwards", "both"];
/// Valeurs de `animation-play-state`.
pub const PLAY_STATES: &[&str] = &["running", "paused"];

/// Une animation du raccourci `animation`.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimationEntry {
    /// `animation-name`
    pub name: String,
    /// `animation-duration`, en millisecondes.
    pub duration: f32,
    /// `animation-delay`, en millisecondes.
    pub delay: f32,
    /// `animation-timing-function`
    pub easing: Easing,
    /// `animation-iteration-count`
    pub iterations: f32,
    /// `animation-direction`
    pub direction: String,
    /// `animation-fill-mode`
    pub fill_mode: String,
    /// `animation-play-state`
    pub play_state: String,
}

/// Analyse le raccourci `animation` : une entrée par animation.
#[must_use]
pub fn parse_animation_shorthand(tokens: &[Token]) -> Option<Vec<AnimationEntry>> {
    comma_list(tokens, |part| {
        let mut entry = AnimationEntry {
            name: "none".to_owned(),
            duration: 0.0,
            delay: 0.0,
            easing: Easing::EASE,
            iterations: 1.0,
            direction: "normal".to_owned(),
            fill_mode: "none".to_owned(),
            play_state: "running".to_owned(),
        };
        // Chaque composante ne se donne qu'une fois ; un mot-clé déjà pris
        // devient le nom de l'animation.
        let (mut times, mut easing, mut iterations) = (0, false, false);
        let (mut direction, mut fill, mut state, mut named) = (false, false, false, false);
        let mut at = 0;
        while at < part.len() {
            let token = &part[at];
            if *token == Token::Whitespace {
                at += 1;
                continue;
            }
            let end = if matches!(token, Token::Function(_)) {
                matching_close(part, at) + 1
            } else {
                at + 1
            };
            let piece = part.get(at..end)?;
            at = end;
            if let [single] = piece
                && let Some(value) = time(single)
            {
                match times {
                    0 if value >= 0.0 => entry.duration = value,
                    1 => entry.delay = value,
                    _ => return None,
                }
                times += 1;
                continue;
            }
            if !easing && let Some(value) = Easing::parse(piece) {
                entry.easing = value;
                easing = true;
                continue;
            }
            if !iterations
                && let Some(value) = parse_iterations(piece)
                && let [value] = value.as_slice()
            {
                entry.iterations = *value;
                iterations = true;
                continue;
            }
            let word = match piece {
                [Token::Ident(word)] => word.clone(),
                [Token::Str(name)] if !named => {
                    entry.name = name.clone();
                    named = true;
                    continue;
                }
                _ => return None,
            };
            let lower = word.to_ascii_lowercase();
            if !direction && DIRECTIONS.contains(&lower.as_str()) {
                entry.direction = lower;
                direction = true;
            } else if !fill && FILL_MODES.contains(&lower.as_str()) {
                entry.fill_mode = lower;
                fill = true;
            } else if !state && PLAY_STATES.contains(&lower.as_str()) {
                entry.play_state = lower;
                state = true;
            } else if !named {
                entry.name = if lower == "none" { lower } else { word };
                named = true;
            } else {
                return None;
            }
        }
        Some(entry)
    })
    .filter(|entries| !entries.is_empty())
}

/// Une transition du raccourci `transition`.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionEntry {
    /// `transition-property`
    pub property: String,
    /// `transition-duration`, en millisecondes.
    pub duration: f32,
    /// `transition-delay`, en millisecondes.
    pub delay: f32,
    /// `transition-timing-function`
    pub easing: Easing,
}

/// Analyse le raccourci `transition` : une entrée par propriété.
#[must_use]
pub fn parse_transition_shorthand(tokens: &[Token]) -> Option<Vec<TransitionEntry>> {
    let entries = comma_list(tokens, |part| {
        let mut entry = TransitionEntry {
            property: "all".to_owned(),
            duration: 0.0,
            delay: 0.0,
            easing: Easing::EASE,
        };
        let (mut times, mut easing, mut named) = (0, false, false);
        let mut at = 0;
        while at < part.len() {
            let token = &part[at];
            if *token == Token::Whitespace {
                at += 1;
                continue;
            }
            let end = if matches!(token, Token::Function(_)) {
                matching_close(part, at) + 1
            } else {
                at + 1
            };
            let piece = part.get(at..end)?;
            at = end;
            if let [single] = piece
                && let Some(value) = time(single)
            {
                match times {
                    0 if value >= 0.0 => entry.duration = value,
                    1 => entry.delay = value,
                    _ => return None,
                }
                times += 1;
            } else if !easing && let Some(value) = Easing::parse(piece) {
                entry.easing = value;
                easing = true;
            } else if let [Token::Ident(word)] = piece
                && !named
            {
                entry.property = word.to_ascii_lowercase();
                named = true;
            } else {
                return None;
            }
        }
        Some(entry)
    })?;
    // `none` ne se combine pas avec d'autres propriétés.
    if entries.len() > 1 && entries.iter().any(|e| e.property == "none") {
        return None;
    }
    (!entries.is_empty()).then_some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_curves() {
        assert_eq!(Easing::Linear.at(0.3), 0.3);
        let ease = Easing::parse(&tokenize("ease")).expect("ease");
        assert!(ease.at(0.5) > 0.75, "ease avance vite au début");
        assert_eq!(ease.at(0.0), 0.0);
        assert_eq!(ease.at(1.0), 1.0);
        let steps = Easing::parse(&tokenize("steps(4, end)")).expect("steps");
        assert_eq!(steps.at(0.49), 0.25);
        assert_eq!(
            Easing::parse(&tokenize("steps(2, start)"))
                .expect("steps")
                .at(0.1),
            0.5
        );
        assert_eq!(Easing::parse(&tokenize("cubic-bezier(2, 0, 0, 1)")), None);
        assert_eq!(Easing::parse(&tokenize("steps(0)")), None);
    }

    #[test]
    fn shorthands() {
        let animation =
            parse_animation_shorthand(&tokenize("tourne 2s ease-in 500ms infinite alternate both"))
                .expect("animation");
        assert_eq!(animation.len(), 1);
        let entry = &animation[0];
        assert_eq!(entry.name, "tourne");
        assert_eq!((entry.duration, entry.delay), (2000.0, 500.0));
        assert!(entry.iterations.is_infinite());
        assert_eq!(entry.direction, "alternate");
        assert_eq!(entry.fill_mode, "both");
        let transition =
            parse_transition_shorthand(&tokenize("opacity 1s, width .5s linear 2s")).expect("ok");
        assert_eq!(transition.len(), 2);
        assert_eq!(transition[1].property, "width");
        assert_eq!(
            (transition[1].duration, transition[1].delay),
            (500.0, 2000.0)
        );
        assert_eq!(transition[1].easing, Easing::Linear);
        assert_eq!(
            parse_transition_shorthand(&tokenize("none, opacity 1s")),
            None
        );
    }

    #[test]
    fn keyframes_rule() {
        let body = tokenize("from { opacity: 0 } 50%, 75% { opacity: .5 } to { opacity: 1 }");
        let keyframes = parse_keyframes(&tokenize(" fondu "), &body).expect("règle");
        assert_eq!(keyframes.name, "fondu");
        let offsets: Vec<f32> = keyframes.frames.iter().map(|f| f.offset).collect();
        assert_eq!(offsets, vec![0.0, 0.5, 0.75, 1.0]);
    }

    /// Style de l'élément d'identifiant `id` à l'instant `now`.
    fn style_at(html: &str, animator: &mut Animator, now: f64, id: &str) -> Arc<ComputedStyle> {
        let doc = oxyde_html::parse_document(html);
        let sheets = crate::collect_style_elements(&doc);
        let viewport = crate::Viewport {
            width: 800.0,
            height: 600.0,
        };
        let styles = crate::compute_styles_at(&doc, &sheets, viewport, animator, now);
        let node = doc
            .descendants(doc.root())
            .find(|n| doc.element(*n).is_some_and(|e| e.id() == Some(id)))
            .expect("élément");
        Arc::clone(styles.get(node).expect("style"))
    }

    #[test]
    fn keyframe_animation_follows_the_clock() {
        let html = "<style>@keyframes fondu { from { opacity: 0 } to { opacity: 1 } }                    #a { animation: fondu 1s linear 2 alternate; opacity: 0.5 }</style><div id=a>";
        let mut animator = Animator::new();
        assert_eq!(style_at(html, &mut animator, 0.0, "a").opacity, 0.0);
        assert!(animator.needs_frames());
        assert_eq!(style_at(html, &mut animator, 250.0, "a").opacity, 0.25);
        // Seconde itération, lue à l'envers.
        assert_eq!(style_at(html, &mut animator, 1250.0, "a").opacity, 0.75);
        // Terminée, sans `fill-mode` : retour au style de base.
        assert_eq!(style_at(html, &mut animator, 2500.0, "a").opacity, 0.5);
        assert!(!animator.needs_frames());
        let kinds: Vec<&str> = animator.take_events().iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            ["animationstart", "animationiteration", "animationend"]
        );
    }

    #[test]
    fn paused_animation_with_negative_delay_holds_its_value() {
        let html = "<style>@keyframes large { to { width: 200px } }                    #a { width: 100px; animation: large 10s -5s linear paused forwards }</style>                    <div id=a>";
        let mut animator = Animator::new();
        assert_eq!(
            style_at(html, &mut animator, 0.0, "a").width,
            Size::Px(150.0)
        );
        assert_eq!(
            style_at(html, &mut animator, 4000.0, "a").width,
            Size::Px(150.0)
        );
        assert!(
            !animator.needs_frames(),
            "une animation en pause ne demande pas d'images"
        );
    }

    #[test]
    fn transition_runs_between_two_styles() {
        let page = |class: &str| {
            format!(
                "<style>#t {{ width: 100px; transition: width 1s linear 500ms }}                 #t.large {{ width: 300px }}</style><div id=t class='{class}'>"
            )
        };
        let mut animator = Animator::new();
        assert_eq!(
            style_at(&page(""), &mut animator, 0.0, "t").width,
            Size::Px(100.0)
        );
        assert!(!animator.needs_frames());
        // Le style change : la transition part, d'abord retenue par son délai.
        assert_eq!(
            style_at(&page("large"), &mut animator, 1000.0, "t").width,
            Size::Px(100.0)
        );
        assert!(animator.needs_frames());
        assert_eq!(
            style_at(&page("large"), &mut animator, 2000.0, "t").width,
            Size::Px(200.0)
        );
        assert_eq!(
            style_at(&page("large"), &mut animator, 2600.0, "t").width,
            Size::Px(300.0)
        );
        assert!(!animator.needs_frames());
        let kinds: Vec<&str> = animator.take_events().iter().map(|e| e.kind).collect();
        assert_eq!(kinds, ["transitionrun", "transitionstart", "transitionend"]);
        let last = animator.take_events();
        assert!(last.is_empty());
    }

    #[test]
    fn animated_values_are_inherited() {
        let html = "<style>@keyframes gris { to { color: rgb(100, 100, 100) } }                    #p { color: rgb(0, 0, 0); animation: gris 1s linear }</style>                    <div id=p><span id=c>x</span></div>";
        let mut animator = Animator::new();
        style_at(html, &mut animator, 0.0, "c");
        assert_eq!(style_at(html, &mut animator, 500.0, "c").color.r, 50);
    }

    #[test]
    fn interpolation() {
        assert_eq!(
            lerp_size(Size::Px(10.0), Size::Px(20.0), 0.5),
            Some(Size::Px(15.0))
        );
        assert_eq!(
            lerp_size(Size::Px(10.0), Size::Percent(50.0), 0.5),
            Some(Size::Calc {
                px: 5.0,
                percent: 25.0
            })
        );
        assert_eq!(lerp_size(Size::Auto, Size::Px(20.0), 0.5), None);
        let half = lerp_color(Color::rgb(0, 0, 0), Color::rgb(255, 255, 255), 0.5);
        assert_eq!((half.r, half.a), (128, 255));
        assert_eq!(longhand_name(Longhand::BackgroundColor), "background-color");
        assert_eq!(longhand_name(Longhand::Margin(Side::Left)), "margin-left");
    }
}
