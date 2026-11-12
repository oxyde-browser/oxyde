//! Styles calculés et cascade (CSS Cascading and Inheritance Level 4).
//!
//! Équivalent de `StyleResolver` / `ComputedStyle` dans Blink, en version
//! directe : pour chaque élément, on rassemble les déclarations applicables,
//! on les trie (origine, importance, spécificité, ordre) et on les applique.
//!
//! LIMITES CONNUES : pas d'index de règles (chaque élément teste toutes les
//! règles), pas de `var()`, pas de `calc()`.

use std::sync::{Arc, LazyLock};

use oxyde_dom::{Document, ElementData, NodeId};

use crate::grid::{GridAreas, GridLine, TrackList, TrackSize};
use crate::properties::{Declaration, Longhand, Side, Value, parse_declarations};
use crate::stylesheet::{Stylesheet, Viewport, parse_stylesheet};
use crate::syntax::tokenize;
use crate::values::{Color, Length};

/// Valeurs par côté.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Edges<T> {
    /// Haut.
    pub top: T,
    /// Droite.
    pub right: T,
    /// Bas.
    pub bottom: T,
    /// Gauche.
    pub left: T,
}

impl<T: Copy> Edges<T> {
    /// Même valeur sur les quatre côtés.
    pub const fn all(v: T) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    /// Valeur du côté `side`.
    pub fn get(&self, side: Side) -> T {
        match side {
            Side::Top => self.top,
            Side::Right => self.right,
            Side::Bottom => self.bottom,
            Side::Left => self.left,
        }
    }

    fn set(&mut self, side: Side, v: T) {
        match side {
            Side::Top => self.top = v,
            Side::Right => self.right = v,
            Side::Bottom => self.bottom = v,
            Side::Left => self.left = v,
        }
    }
}

/// Taille calculée : `auto` (ou `none` pour les maximums), pixels ou pourcentage.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Size {
    /// `auto` / `none`.
    #[default]
    Auto,
    /// Pixels CSS.
    Px(f32),
    /// Pourcentage du bloc conteneur.
    Percent(f32),
    /// Pixels plus un pourcentage du bloc conteneur (`calc(50% - 10px)`).
    Calc {
        /// Part fixe, en pixels.
        px: f32,
        /// Part relative, en pourcentage du bloc conteneur.
        percent: f32,
    },
}

impl Size {
    /// Résout par rapport à `base` ; `None` pour `auto`.
    #[must_use]
    pub fn resolve(self, base: f32) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Px(v) => Some(v),
            Self::Percent(p) => Some(base * p / 100.0),
            Self::Calc { px, percent } => Some(px + base * percent / 100.0),
        }
    }
}

/// Valeur calculée de `display`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Display {
    None,
    Block,
    Inline,
    InlineBlock,
    ListItem,
    Flex,
    Grid,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    TableCaption,
    Contents,
}

/// Valeur calculée de `position`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

/// Valeur calculée de `float`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Float {
    #[default]
    None,
    Left,
    Right,
}

/// Valeur calculée de `clear`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Clear {
    #[default]
    None,
    Left,
    Right,
    Both,
}

/// Valeur calculée de `overflow-x` ou `overflow-y`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Clip,
    Scroll,
    Auto,
}

/// Valeur calculée de `visibility`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    Collapse,
}

/// Valeur calculée de `flex-direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    /// Vrai si l'axe principal est horizontal.
    #[must_use]
    pub fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::RowReverse)
    }

    /// Vrai si l'axe principal est parcouru à rebours.
    #[must_use]
    pub fn is_reverse(self) -> bool {
        matches!(self, Self::RowReverse | Self::ColumnReverse)
    }
}

/// Valeur calculée de `flex-wrap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

/// Alignement (`justify-content`, `align-content`, `align-items`,
/// `align-self`), une fois les synonymes réunis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Alignment {
    /// `normal` ou `auto` : le comportement par défaut du conteneur.
    #[default]
    Normal,
    /// `stretch`
    Stretch,
    /// `flex-start`, `start`, `self-start`, `left`
    Start,
    /// `flex-end`, `end`, `self-end`, `right`
    End,
    /// `center`
    Center,
    /// `baseline`
    Baseline,
    /// `space-between`
    SpaceBetween,
    /// `space-around`
    SpaceAround,
    /// `space-evenly`
    SpaceEvenly,
}

impl Alignment {
    fn parse(word: &str) -> Self {
        match word {
            "stretch" => Self::Stretch,
            "flex-start" | "start" | "self-start" | "left" => Self::Start,
            "flex-end" | "end" | "self-end" | "right" => Self::End,
            "center" => Self::Center,
            "baseline" => Self::Baseline,
            "space-between" => Self::SpaceBetween,
            "space-around" => Self::SpaceAround,
            "space-evenly" => Self::SpaceEvenly,
            _ => Self::Normal,
        }
    }
}

/// Valeur calculée de `line-height`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// `normal`
    Normal,
    /// Multiple de la taille de police (hérité tel quel).
    Number(f32),
    /// Pixels.
    Px(f32),
}

/// Valeur calculée de `text-align`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum TextAlign {
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
}

/// Valeur calculée de `white-space`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum WhiteSpace {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}

impl WhiteSpace {
    /// Vrai si les suites de blancs sont fusionnées.
    #[must_use]
    pub fn collapses(self) -> bool {
        matches!(self, Self::Normal | Self::Nowrap | Self::PreLine)
    }

    /// Vrai si les lignes peuvent être coupées automatiquement.
    #[must_use]
    pub fn wraps(self) -> bool {
        matches!(self, Self::Normal | Self::PreWrap | Self::PreLine)
    }

    /// Vrai si les sauts de ligne du source sont conservés.
    #[must_use]
    pub fn preserves_newlines(self) -> bool {
        matches!(self, Self::Pre | Self::PreWrap | Self::PreLine)
    }
}

/// Style de bordure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
    Dotted,
}

/// Puce de liste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs, reason = "les variantes portent le nom de la valeur CSS")]
pub enum ListStyle {
    None,
    Disc,
    Circle,
    Square,
    Decimal,
}

/// Style calculé d'un élément.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyle {
    /// `display`
    pub display: Display,
    /// `color`
    pub color: Color,
    /// `background-color`
    pub background_color: Color,
    /// `font-size`, en pixels.
    pub font_size: f32,
    /// `font-weight` (1–1000).
    pub font_weight: f32,
    /// `font-style: italic | oblique`
    pub italic: bool,
    /// `font-family`
    pub font_family: Arc<[String]>,
    /// `line-height`
    pub line_height: LineHeight,
    /// `text-align`
    pub text_align: TextAlign,
    /// `white-space`
    pub white_space: WhiteSpace,
    /// `text-decoration-line: underline`
    pub underline: bool,
    /// `text-decoration-line: line-through`
    pub line_through: bool,
    /// `margin-*`
    pub margin: Edges<Size>,
    /// `padding-*`
    pub padding: Edges<Size>,
    /// `border-*-width` spécifiées ; voir [`ComputedStyle::border_widths`].
    pub border_width: Edges<f32>,
    /// `border-*-style`
    pub border_style: Edges<BorderStyle>,
    /// `border-*-color` ; `None` signifie `currentcolor`.
    pub border_color: Edges<Option<Color>>,
    /// `border-radius` (uniforme), en pixels.
    pub border_radius: f32,
    /// `width`
    pub width: Size,
    /// `height`
    pub height: Size,
    /// `min-width`
    pub min_width: Size,
    /// `min-height`
    pub min_height: Size,
    /// `max-width` (`Auto` = `none`)
    pub max_width: Size,
    /// `max-height` (`Auto` = `none`)
    pub max_height: Size,
    /// `box-sizing: border-box`
    pub border_box: bool,
    /// `list-style-type`
    pub list_style: ListStyle,
    /// `position`
    pub position: Position,
    /// `top`, `right`, `bottom`, `left`
    pub inset: Edges<Size>,
    /// `float`
    pub float: Float,
    /// `clear`
    pub clear: Clear,
    /// `z-index` ; `None` : `auto`.
    pub z_index: Option<i32>,
    /// `overflow-x`
    pub overflow_x: Overflow,
    /// `overflow-y`
    pub overflow_y: Overflow,
    /// `visibility`
    pub visibility: Visibility,
    /// `opacity`
    pub opacity: f32,
    /// `flex-direction`
    pub flex_direction: FlexDirection,
    /// `flex-wrap`
    pub flex_wrap: FlexWrap,
    /// `flex-grow`
    pub flex_grow: f32,
    /// `flex-shrink`
    pub flex_shrink: f32,
    /// `flex-basis` ; `None` : `content`, et [`Size::Auto`] : `auto`.
    pub flex_basis: Option<Size>,
    /// `justify-content`
    pub justify_content: Alignment,
    /// `align-items`
    pub align_items: Alignment,
    /// `align-self` ; [`Alignment::Normal`] : celui du conteneur.
    pub align_self: Alignment,
    /// `align-content`
    pub align_content: Alignment,
    /// `order`
    pub order: i32,
    /// `row-gap` (`Auto` : `normal`)
    pub row_gap: Size,
    /// `column-gap` (`Auto` : `normal`)
    pub column_gap: Size,
    /// Vrai si `display` vaut `flex` ou `inline-flex` : le contenu suit la
    /// mise en page flexible, que la boîte soit de niveau bloc ou en ligne.
    pub flex_container: bool,
    /// Vrai si `display` vaut `grid` ou `inline-grid`.
    pub grid_container: bool,
    /// `grid-template-columns`
    pub grid_template_columns: Arc<TrackList<Size>>,
    /// `grid-template-rows`
    pub grid_template_rows: Arc<TrackList<Size>>,
    /// `grid-auto-columns` ; vide : `auto`.
    pub grid_auto_columns: Arc<[TrackSize<Size>]>,
    /// `grid-auto-rows` ; vide : `auto`.
    pub grid_auto_rows: Arc<[TrackSize<Size>]>,
    /// `grid-auto-flow: column`
    pub grid_auto_flow_column: bool,
    /// `grid-auto-flow: dense`
    pub grid_auto_flow_dense: bool,
    /// `grid-row-start`
    pub grid_row_start: GridLine,
    /// `grid-row-end`
    pub grid_row_end: GridLine,
    /// `grid-column-start`
    pub grid_column_start: GridLine,
    /// `grid-column-end`
    pub grid_column_end: GridLine,
    /// `grid-template-areas`
    pub grid_template_areas: Arc<GridAreas>,
    /// `justify-items`
    pub justify_items: Alignment,
    /// `justify-self` ; [`Alignment::Normal`] : celui du conteneur.
    pub justify_self: Alignment,
}

impl ComputedStyle {
    /// Valeurs initiales de toutes les propriétés.
    #[must_use]
    pub fn initial() -> Self {
        static SERIF: LazyLock<Arc<[String]>> = LazyLock::new(|| Arc::from(["serif".to_owned()]));
        // Valeurs vides partagées : la plupart des éléments n'ont pas de grille.
        static NO_TRACKS: LazyLock<Arc<TrackList<Size>>> =
            LazyLock::new(|| Arc::new(TrackList::default()));
        static NO_SIZES: LazyLock<Arc<[TrackSize<Size>]>> = LazyLock::new(|| Arc::from([]));
        static NO_AREAS: LazyLock<Arc<GridAreas>> =
            LazyLock::new(|| Arc::new(GridAreas::default()));
        Self {
            display: Display::Inline,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            font_size: 16.0,
            font_weight: 400.0,
            italic: false,
            font_family: Arc::clone(&SERIF),
            line_height: LineHeight::Normal,
            text_align: TextAlign::Start,
            white_space: WhiteSpace::Normal,
            underline: false,
            line_through: false,
            margin: Edges::all(Size::Px(0.0)),
            padding: Edges::all(Size::Px(0.0)),
            border_width: Edges::all(3.0),
            border_style: Edges::all(BorderStyle::None),
            border_color: Edges::all(None),
            border_radius: 0.0,
            width: Size::Auto,
            height: Size::Auto,
            min_width: Size::Auto,
            min_height: Size::Auto,
            max_width: Size::Auto,
            max_height: Size::Auto,
            border_box: false,
            list_style: ListStyle::Disc,
            position: Position::Static,
            inset: Edges::all(Size::Auto),
            float: Float::None,
            clear: Clear::None,
            z_index: None,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            visibility: Visibility::Visible,
            opacity: 1.0,
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Some(Size::Auto),
            justify_content: Alignment::Normal,
            align_items: Alignment::Normal,
            align_self: Alignment::Normal,
            align_content: Alignment::Normal,
            order: 0,
            row_gap: Size::Auto,
            column_gap: Size::Auto,
            flex_container: false,
            grid_container: false,
            grid_template_columns: Arc::clone(&NO_TRACKS),
            grid_template_rows: Arc::clone(&NO_TRACKS),
            grid_auto_columns: Arc::clone(&NO_SIZES),
            grid_auto_rows: Arc::clone(&NO_SIZES),
            grid_auto_flow_column: false,
            grid_auto_flow_dense: false,
            grid_row_start: GridLine::Auto,
            grid_row_end: GridLine::Auto,
            grid_column_start: GridLine::Auto,
            grid_column_end: GridLine::Auto,
            grid_template_areas: Arc::clone(&NO_AREAS),
            justify_items: Alignment::Normal,
            justify_self: Alignment::Normal,
        }
    }

    /// Vrai si la boîte est positionnée (`position` autre que `static`).
    #[must_use]
    pub fn is_positioned(&self) -> bool {
        self.position != Position::Static
    }

    /// Vrai si la boîte sort du flux (`position: absolute` ou `fixed`).
    #[must_use]
    pub fn is_out_of_flow(&self) -> bool {
        matches!(self.position, Position::Absolute | Position::Fixed)
    }

    /// Nom CSS de la valeur de `position`.
    #[must_use]
    pub fn position_name(&self) -> &'static str {
        match self.position {
            Position::Static => "static",
            Position::Relative => "relative",
            Position::Absolute => "absolute",
            Position::Fixed => "fixed",
            Position::Sticky => "sticky",
        }
    }

    /// Style d'un enfant avant application de ses propres déclarations :
    /// propriétés héritées copiées, les autres à leur valeur initiale.
    #[must_use]
    pub fn inherit_from(parent: &Self) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            font_weight: parent.font_weight,
            italic: parent.italic,
            font_family: Arc::clone(&parent.font_family),
            line_height: parent.line_height,
            text_align: parent.text_align,
            white_space: parent.white_space,
            // `text-decoration` n'est pas hérité mais se propage aux boîtes
            // descendantes ; on modélise cette propagation par un héritage.
            underline: parent.underline,
            line_through: parent.line_through,
            list_style: parent.list_style,
            visibility: parent.visibility,
            ..Self::initial()
        }
    }

    /// Largeurs de bordure utilisées (nulles si le style est `none`).
    #[must_use]
    pub fn border_widths(&self) -> Edges<f32> {
        let used = |side: Side| {
            if self.border_style.get(side) == BorderStyle::None {
                0.0
            } else {
                self.border_width.get(side)
            }
        };
        Edges {
            top: used(Side::Top),
            right: used(Side::Right),
            bottom: used(Side::Bottom),
            left: used(Side::Left),
        }
    }

    /// Couleur de bordure utilisée pour `side` (`currentcolor` résolu).
    #[must_use]
    pub fn border_color_of(&self, side: Side) -> Color {
        self.border_color.get(side).unwrap_or(self.color)
    }

    /// Hauteur de ligne utilisée, en pixels.
    #[must_use]
    pub fn line_height_px(&self) -> f32 {
        match self.line_height {
            LineHeight::Normal => self.font_size * 1.2,
            LineHeight::Number(n) => self.font_size * n,
            LineHeight::Px(v) => v,
        }
    }
}

struct Context<'a> {
    parent: &'a ComputedStyle,
    root_font_size: f32,
    viewport: Viewport,
}

fn length_px(length: Length, font_size: f32, cx: &Context<'_>) -> Size {
    match length {
        Length::Px(v) => Size::Px(v),
        Length::Em(v) => Size::Px(v * font_size),
        Length::Rem(v) => Size::Px(v * cx.root_font_size),
        Length::Vw(v) => Size::Px(v * cx.viewport.width / 100.0),
        Length::Vh(v) => Size::Px(v * cx.viewport.height / 100.0),
        Length::Percent(v) => Size::Percent(v),
        Length::Calc(l) => {
            let px = l.px
                + l.em * font_size
                + l.rem * cx.root_font_size
                + l.vw * cx.viewport.width / 100.0
                + l.vh * cx.viewport.height / 100.0;
            if !l.has_percent {
                Size::Px(px)
            } else {
                Size::Calc {
                    px,
                    percent: l.percent,
                }
            }
        }
    }
}

fn is_inherited(property: Longhand) -> bool {
    matches!(
        property,
        Longhand::Color
            | Longhand::FontSize
            | Longhand::FontWeight
            | Longhand::FontStyle
            | Longhand::FontFamily
            | Longhand::LineHeight
            | Longhand::TextAlign
            | Longhand::WhiteSpace
            | Longhand::ListStyleType
            | Longhand::Visibility
    )
}

/// Copie la propriété `property` de `from` vers `style`.
fn copy_property(style: &mut ComputedStyle, from: &ComputedStyle, property: Longhand) {
    match property {
        Longhand::Display => {
            style.display = from.display;
            style.flex_container = from.flex_container;
            style.grid_container = from.grid_container;
        }
        Longhand::Color => style.color = from.color,
        Longhand::BackgroundColor => style.background_color = from.background_color,
        Longhand::FontSize => style.font_size = from.font_size,
        Longhand::FontWeight => style.font_weight = from.font_weight,
        Longhand::FontStyle => style.italic = from.italic,
        Longhand::FontFamily => style.font_family = Arc::clone(&from.font_family),
        Longhand::LineHeight => style.line_height = from.line_height,
        Longhand::TextAlign => style.text_align = from.text_align,
        Longhand::WhiteSpace => style.white_space = from.white_space,
        Longhand::TextDecorationLine => {
            style.underline = from.underline;
            style.line_through = from.line_through;
        }
        Longhand::Margin(s) => style.margin.set(s, from.margin.get(s)),
        Longhand::Padding(s) => style.padding.set(s, from.padding.get(s)),
        Longhand::BorderWidth(s) => style.border_width.set(s, from.border_width.get(s)),
        Longhand::BorderColor(s) => style.border_color.set(s, from.border_color.get(s)),
        Longhand::BorderStyle(s) => style.border_style.set(s, from.border_style.get(s)),
        Longhand::BorderRadius => style.border_radius = from.border_radius,
        Longhand::Width => style.width = from.width,
        Longhand::Height => style.height = from.height,
        Longhand::MinWidth => style.min_width = from.min_width,
        Longhand::MaxWidth => style.max_width = from.max_width,
        Longhand::MinHeight => style.min_height = from.min_height,
        Longhand::MaxHeight => style.max_height = from.max_height,
        Longhand::BoxSizing => style.border_box = from.border_box,
        Longhand::ListStyleType => style.list_style = from.list_style,
        Longhand::Position => style.position = from.position,
        Longhand::Inset(s) => style.inset.set(s, from.inset.get(s)),
        Longhand::Float => style.float = from.float,
        Longhand::Clear => style.clear = from.clear,
        Longhand::ZIndex => style.z_index = from.z_index,
        Longhand::OverflowX => style.overflow_x = from.overflow_x,
        Longhand::OverflowY => style.overflow_y = from.overflow_y,
        Longhand::Visibility => style.visibility = from.visibility,
        Longhand::Opacity => style.opacity = from.opacity,
        Longhand::FlexDirection => style.flex_direction = from.flex_direction,
        Longhand::FlexWrap => style.flex_wrap = from.flex_wrap,
        Longhand::FlexGrow => style.flex_grow = from.flex_grow,
        Longhand::FlexShrink => style.flex_shrink = from.flex_shrink,
        Longhand::FlexBasis => style.flex_basis = from.flex_basis,
        Longhand::JustifyContent => style.justify_content = from.justify_content,
        Longhand::AlignItems => style.align_items = from.align_items,
        Longhand::AlignSelf => style.align_self = from.align_self,
        Longhand::AlignContent => style.align_content = from.align_content,
        Longhand::Order => style.order = from.order,
        Longhand::RowGap => style.row_gap = from.row_gap,
        Longhand::ColumnGap => style.column_gap = from.column_gap,
        Longhand::GridTemplateColumns => {
            style.grid_template_columns = Arc::clone(&from.grid_template_columns);
        }
        Longhand::GridTemplateRows => {
            style.grid_template_rows = Arc::clone(&from.grid_template_rows);
        }
        Longhand::GridAutoColumns => style.grid_auto_columns = Arc::clone(&from.grid_auto_columns),
        Longhand::GridAutoRows => style.grid_auto_rows = Arc::clone(&from.grid_auto_rows),
        Longhand::GridAutoFlow => {
            style.grid_auto_flow_column = from.grid_auto_flow_column;
            style.grid_auto_flow_dense = from.grid_auto_flow_dense;
        }
        Longhand::GridRowStart => style.grid_row_start = from.grid_row_start.clone(),
        Longhand::GridRowEnd => style.grid_row_end = from.grid_row_end.clone(),
        Longhand::GridColumnStart => style.grid_column_start = from.grid_column_start.clone(),
        Longhand::GridColumnEnd => style.grid_column_end = from.grid_column_end.clone(),
        Longhand::GridTemplateAreas => {
            style.grid_template_areas = Arc::clone(&from.grid_template_areas);
        }
        Longhand::JustifyItems => style.justify_items = from.justify_items,
        Longhand::JustifySelf => style.justify_self = from.justify_self,
    }
}

fn font_size_px(value: &Value, cx: &Context<'_>) -> Option<f32> {
    let parent = cx.parent.font_size;
    Some(match value {
        Value::Length(Length::Percent(p)) => parent * p / 100.0,
        Value::Length(Length::Em(v)) => parent * v,
        Value::Length(l) => match length_px(*l, parent, cx) {
            Size::Px(v) => v,
            _ => return None,
        },
        Value::Ident(word) => match word.as_str() {
            "xx-small" => 9.0,
            "x-small" => 10.0,
            "small" => 13.0,
            "medium" => 16.0,
            "large" => 18.0,
            "x-large" => 24.0,
            "xx-large" => 32.0,
            "xxx-large" => 48.0,
            "smaller" => parent / 1.2,
            "larger" => parent * 1.2,
            _ => return None,
        },
        _ => return None,
    })
}

fn size_of(value: &Value, font_size: f32, cx: &Context<'_>) -> Size {
    match value {
        Value::Length(l) => length_px(*l, font_size, cx),
        _ => Size::Auto,
    }
}

fn apply(style: &mut ComputedStyle, decl: &Declaration, cx: &Context<'_>) {
    let property = decl.property;
    match &decl.value {
        Value::Inherit => return copy_property(style, cx.parent, property),
        Value::Initial => return copy_property(style, &ComputedStyle::initial(), property),
        Value::Unset => {
            let from = if is_inherited(property) {
                cx.parent.clone()
            } else {
                ComputedStyle::initial()
            };
            return copy_property(style, &from, property);
        }
        _ => {}
    }
    let fs = style.font_size;
    let ident = match &decl.value {
        Value::Ident(s) => s.as_str(),
        _ => "",
    };
    match property {
        Longhand::Display => {
            style.flex_container = matches!(ident, "flex" | "inline-flex");
            style.grid_container = matches!(ident, "grid" | "inline-grid");
            style.display = match ident {
                "none" => Display::None,
                "block" | "flow-root" => Display::Block,
                "inline" => Display::Inline,
                "inline-block" | "inline-flex" | "inline-grid" | "inline-table" => {
                    Display::InlineBlock
                }
                "list-item" => Display::ListItem,
                "flex" => Display::Flex,
                "grid" => Display::Grid,
                "table" => Display::Table,
                "table-row-group" | "table-header-group" | "table-footer-group" => {
                    Display::TableRowGroup
                }
                "table-row" => Display::TableRow,
                "table-cell" => Display::TableCell,
                "table-caption" => Display::TableCaption,
                "table-column" | "table-column-group" => Display::None,
                "contents" => Display::Contents,
                _ => return,
            };
        }
        Longhand::Color => match &decl.value {
            Value::Color(c) => style.color = *c,
            // `color: currentcolor` équivaut à `inherit`.
            Value::CurrentColor => style.color = cx.parent.color,
            _ => {}
        },
        Longhand::BackgroundColor => match &decl.value {
            Value::Color(c) => style.background_color = *c,
            Value::CurrentColor => style.background_color = style.color,
            _ => {}
        },
        Longhand::FontSize => {
            if let Some(px) = font_size_px(&decl.value, cx) {
                style.font_size = px.max(0.0);
            }
        }
        Longhand::FontWeight => match &decl.value {
            Value::Number(n) => style.font_weight = *n,
            Value::Ident(_) => {
                let w = cx.parent.font_weight;
                style.font_weight = if ident == "bolder" {
                    if w < 350.0 {
                        400.0
                    } else if w < 550.0 {
                        700.0
                    } else {
                        900.0
                    }
                } else if w < 550.0 {
                    100.0
                } else if w < 750.0 {
                    400.0
                } else {
                    700.0
                };
            }
            _ => {}
        },
        Longhand::FontStyle => style.italic = ident != "normal",
        Longhand::FontFamily => {
            if let Value::FontFamily(list) = &decl.value {
                style.font_family = Arc::from(list.as_slice());
            }
        }
        Longhand::LineHeight => {
            style.line_height = match &decl.value {
                Value::Number(n) => LineHeight::Number(*n),
                Value::Length(Length::Percent(p)) => LineHeight::Px(fs * p / 100.0),
                Value::Length(l) => match length_px(*l, fs, cx) {
                    Size::Px(v) => LineHeight::Px(v),
                    _ => return,
                },
                _ => LineHeight::Normal,
            };
        }
        Longhand::TextAlign => {
            style.text_align = match ident {
                "left" => TextAlign::Left,
                "right" => TextAlign::Right,
                "center" | "-webkit-center" => TextAlign::Center,
                "justify" => TextAlign::Justify,
                "end" => TextAlign::End,
                _ => TextAlign::Start,
            };
        }
        Longhand::WhiteSpace => {
            style.white_space = match ident {
                "nowrap" => WhiteSpace::Nowrap,
                "pre" => WhiteSpace::Pre,
                "pre-wrap" | "break-spaces" => WhiteSpace::PreWrap,
                "pre-line" => WhiteSpace::PreLine,
                _ => WhiteSpace::Normal,
            };
        }
        Longhand::TextDecorationLine => {
            style.underline = ident.split(' ').any(|w| w == "underline");
            style.line_through = ident.split(' ').any(|w| w == "line-through");
        }
        Longhand::Margin(side) => style.margin.set(side, size_of(&decl.value, fs, cx)),
        Longhand::Padding(side) => style.padding.set(side, size_of(&decl.value, fs, cx)),
        Longhand::BorderWidth(side) => {
            let px = match &decl.value {
                Value::Ident(_) => match ident {
                    "thin" => 1.0,
                    "thick" => 5.0,
                    _ => 3.0,
                },
                Value::Length(l) => match length_px(*l, fs, cx) {
                    Size::Px(v) => v,
                    _ => return,
                },
                _ => return,
            };
            style.border_width.set(side, px);
        }
        Longhand::BorderStyle(side) => style.border_style.set(
            side,
            match ident {
                "none" | "hidden" => BorderStyle::None,
                "dashed" => BorderStyle::Dashed,
                "dotted" => BorderStyle::Dotted,
                _ => BorderStyle::Solid,
            },
        ),
        Longhand::BorderColor(side) => match &decl.value {
            Value::Color(c) => style.border_color.set(side, Some(*c)),
            Value::CurrentColor => style.border_color.set(side, None),
            _ => {}
        },
        Longhand::BorderRadius => {
            if let Value::Length(l) = &decl.value
                && let Size::Px(v) = length_px(*l, fs, cx)
            {
                style.border_radius = v;
            }
        }
        Longhand::Width => style.width = size_of(&decl.value, fs, cx),
        Longhand::Height => style.height = size_of(&decl.value, fs, cx),
        Longhand::MinWidth => style.min_width = size_of(&decl.value, fs, cx),
        Longhand::MaxWidth => style.max_width = size_of(&decl.value, fs, cx),
        Longhand::MinHeight => style.min_height = size_of(&decl.value, fs, cx),
        Longhand::MaxHeight => style.max_height = size_of(&decl.value, fs, cx),
        Longhand::BoxSizing => style.border_box = ident == "border-box",
        Longhand::Position => {
            style.position = match ident {
                "relative" => Position::Relative,
                "absolute" => Position::Absolute,
                "fixed" => Position::Fixed,
                "sticky" => Position::Sticky,
                _ => Position::Static,
            };
        }
        Longhand::Inset(side) => style.inset.set(side, size_of(&decl.value, fs, cx)),
        Longhand::Float => {
            style.float = match ident {
                "left" | "inline-start" => Float::Left,
                "right" | "inline-end" => Float::Right,
                _ => Float::None,
            };
        }
        Longhand::Clear => {
            style.clear = match ident {
                "left" | "inline-start" => Clear::Left,
                "right" | "inline-end" => Clear::Right,
                "both" => Clear::Both,
                _ => Clear::None,
            };
        }
        Longhand::ZIndex => {
            style.z_index = match &decl.value {
                #[expect(clippy::cast_possible_truncation, reason = "entier validé à l'analyse")]
                Value::Number(n) => Some(*n as i32),
                _ => None,
            };
        }
        Longhand::Order => {
            if let Value::Number(n) = &decl.value {
                #[expect(clippy::cast_possible_truncation, reason = "entier validé à l'analyse")]
                {
                    style.order = *n as i32;
                }
            }
        }
        Longhand::OverflowX | Longhand::OverflowY => {
            let overflow = match ident {
                "hidden" => Overflow::Hidden,
                "clip" => Overflow::Clip,
                "scroll" => Overflow::Scroll,
                "auto" => Overflow::Auto,
                _ => Overflow::Visible,
            };
            if property == Longhand::OverflowX {
                style.overflow_x = overflow;
            } else {
                style.overflow_y = overflow;
            }
        }
        Longhand::Visibility => {
            style.visibility = match ident {
                "hidden" => Visibility::Hidden,
                "collapse" => Visibility::Collapse,
                _ => Visibility::Visible,
            };
        }
        Longhand::Opacity => {
            if let Value::Number(n) = &decl.value {
                style.opacity = *n;
            }
        }
        Longhand::FlexDirection => {
            style.flex_direction = match ident {
                "row-reverse" => FlexDirection::RowReverse,
                "column" => FlexDirection::Column,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }
        Longhand::FlexWrap => {
            style.flex_wrap = match ident {
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::NoWrap,
            };
        }
        Longhand::FlexGrow => {
            if let Value::Number(n) = &decl.value {
                style.flex_grow = *n;
            }
        }
        Longhand::FlexShrink => {
            if let Value::Number(n) = &decl.value {
                style.flex_shrink = *n;
            }
        }
        Longhand::FlexBasis => {
            style.flex_basis = match ident {
                "content" => None,
                _ => Some(size_of(&decl.value, fs, cx)),
            };
        }
        Longhand::JustifyContent => style.justify_content = Alignment::parse(ident),
        Longhand::AlignItems => style.align_items = Alignment::parse(ident),
        Longhand::AlignSelf => style.align_self = Alignment::parse(ident),
        Longhand::AlignContent => style.align_content = Alignment::parse(ident),
        Longhand::RowGap => style.row_gap = size_of(&decl.value, fs, cx),
        Longhand::ColumnGap => style.column_gap = size_of(&decl.value, fs, cx),
        Longhand::GridTemplateColumns | Longhand::GridTemplateRows => {
            if let Value::Tracks(list) = &decl.value {
                let computed = Arc::new(list.map(|l| length_px(l, fs, cx)));
                if property == Longhand::GridTemplateColumns {
                    style.grid_template_columns = computed;
                } else {
                    style.grid_template_rows = computed;
                }
            }
        }
        Longhand::GridAutoColumns | Longhand::GridAutoRows => {
            if let Value::TrackSizes(sizes) = &decl.value {
                let list = TrackList {
                    entries: sizes
                        .iter()
                        .map(|t| crate::grid::TrackEntry::Track(*t))
                        .collect(),
                }
                .map(|l| length_px(l, fs, cx));
                let computed: Arc<[TrackSize<Size>]> = list
                    .entries
                    .into_iter()
                    .filter_map(|entry| match entry {
                        crate::grid::TrackEntry::Track(track) => Some(track),
                        _ => None,
                    })
                    .collect();
                if property == Longhand::GridAutoColumns {
                    style.grid_auto_columns = computed;
                } else {
                    style.grid_auto_rows = computed;
                }
            }
        }
        Longhand::GridAutoFlow => {
            style.grid_auto_flow_column = ident.starts_with("column");
            style.grid_auto_flow_dense = ident.ends_with("dense");
        }
        Longhand::GridRowStart
        | Longhand::GridRowEnd
        | Longhand::GridColumnStart
        | Longhand::GridColumnEnd => {
            if let Value::Line(line) = &decl.value {
                let target = match property {
                    Longhand::GridRowStart => &mut style.grid_row_start,
                    Longhand::GridRowEnd => &mut style.grid_row_end,
                    Longhand::GridColumnStart => &mut style.grid_column_start,
                    _ => &mut style.grid_column_end,
                };
                *target = line.clone();
            }
        }
        Longhand::GridTemplateAreas => {
            if let Value::Areas(areas) = &decl.value {
                style.grid_template_areas = Arc::new(areas.clone());
            }
        }
        Longhand::JustifyItems => style.justify_items = Alignment::parse(ident),
        Longhand::JustifySelf => style.justify_self = Alignment::parse(ident),
        Longhand::ListStyleType => {
            style.list_style = match ident {
                "none" => ListStyle::None,
                "circle" => ListStyle::Circle,
                "square" => ListStyle::Square,
                "decimal" => ListStyle::Decimal,
                _ => ListStyle::Disc,
            };
        }
    }
}

static UA_SHEET: LazyLock<Stylesheet> = LazyLock::new(|| parse_stylesheet(include_str!("ua.css")));

/// Indications de présentation issues des attributs HTML (`bgcolor`, `align`…).
fn presentational_hints(el: &ElementData) -> Vec<Declaration> {
    let mut css = String::new();
    let mut push = |property: &str, value: &str| {
        // Les valeurs passent par l'analyseur CSS : rien d'invalide n'est accepté.
        if !value.contains([';', '{', '}']) {
            css.push_str(property);
            css.push(':');
            css.push_str(value);
            css.push(';');
        }
    };
    let dimension = |v: &str| {
        let v = v.trim();
        if v.ends_with('%') {
            v.to_owned()
        } else {
            format!("{}px", v.trim_end_matches("px"))
        }
    };
    if let Some(v) = el.attr("bgcolor") {
        push("background-color", v);
    }
    match el.name.as_str() {
        "font" => {
            if let Some(v) = el.attr("color") {
                push("color", v);
            }
        }
        "body" => {
            if let Some(v) = el.attr("text") {
                push("color", v);
            }
        }
        "img" | "table" | "td" | "th" | "canvas" | "video" | "iframe" | "hr" => {
            if let Some(v) = el.attr("width") {
                push("width", &dimension(v));
            }
            if let Some(v) = el.attr("height") {
                push("height", &dimension(v));
            }
        }
        _ => {}
    }
    if matches!(
        el.name.as_str(),
        "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "td" | "th"
    ) && let Some(v) = el.attr("align")
    {
        push("text-align", &v.to_ascii_lowercase());
    }
    if css.is_empty() {
        Vec::new()
    } else {
        parse_declarations(&tokenize(&css))
    }
}

/// Entrée de cascade : (rang d'origine, spécificité, ordre, déclaration).
type Matched<'a> = (u8, u32, u32, &'a Declaration);

/// Ajoute à `out` les déclarations des règles actives de `rules` qui
/// s'appliquent à `node`.
fn collect_matching<'a>(
    doc: &Document,
    node: NodeId,
    rules: &'a [crate::stylesheet::Rule],
    active: &[usize],
    author: bool,
    order: &mut u32,
    out: &mut Vec<Matched<'a>>,
) {
    for rule in active.iter().filter_map(|i| rules.get(*i)) {
        let Some(spec) = rule
            .selectors
            .iter()
            .filter(|s| s.matches(doc, node))
            .map(crate::selector::Selector::specificity)
            .max()
        else {
            continue;
        };
        for d in &rule.declarations {
            *order = order.saturating_add(1);
            let rank = match (author, d.important) {
                (false, false) => 0,
                (true, false) => 1,
                (true, true) => 2,
                (false, true) => 3,
            };
            out.push((rank, spec, *order, d));
        }
    }
}

/// Styles calculés de tous les éléments d'un document.
#[derive(Clone, Debug, Default)]
pub struct StyleMap {
    styles: Vec<Option<Arc<ComputedStyle>>>,
}

impl StyleMap {
    /// Style de l'élément `id` (`None` pour un nœud qui n'est pas un élément).
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&Arc<ComputedStyle>> {
        self.styles.get(id.index()).and_then(Option::as_ref)
    }
}

/// Feuilles d'auteur contenues dans les éléments `<style>` du document.
#[must_use]
pub fn collect_style_elements(doc: &Document) -> Vec<Stylesheet> {
    doc.descendants(doc.root())
        .filter(|n| doc.element(*n).is_some_and(|e| e.is_html("style")))
        .map(|n| parse_stylesheet(&doc.text_content(n)))
        .collect()
}

/// Calcule le style de chaque élément de `doc`.
///
/// `author_sheets` : feuilles d'auteur dans l'ordre du document.
#[must_use]
pub fn compute_styles(
    doc: &Document,
    author_sheets: &[Stylesheet],
    viewport: Viewport,
) -> StyleMap {
    let initial = ComputedStyle::initial();
    let mut map = StyleMap {
        styles: vec![None; doc.len()],
    };
    let mut root_font_size = initial.font_size;

    // Règles actives, avec leur rang d'origine (0 = navigateur, 1 = auteur).
    let active = |sheet: &'_ Stylesheet| -> Vec<usize> {
        sheet
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.media.iter().all(|m| m.matches(viewport)))
            .map(|(i, _)| i)
            .collect()
    };
    let ua_active = active(&UA_SHEET);
    let author_active: Vec<Vec<usize>> = author_sheets.iter().map(active).collect();

    for node in doc.descendants(doc.root()) {
        let Some(el) = doc.element(node) else {
            continue;
        };
        let parent_style = doc.node(node).parent.and_then(|p| map.get(p)).cloned();
        let is_root = parent_style.is_none();
        let parent = parent_style.as_deref().unwrap_or(&initial);

        // (rang, spécificité, ordre, déclaration)
        let mut matched: Vec<(u8, u32, u32, &Declaration)> = Vec::new();
        let mut order = 0u32;
        collect_matching(
            doc,
            node,
            &UA_SHEET.rules,
            &ua_active,
            false,
            &mut order,
            &mut matched,
        );
        let hints = presentational_hints(el);
        let hint_base = order;
        for (sheet, rules) in author_sheets.iter().zip(&author_active) {
            collect_matching(
                doc,
                node,
                &sheet.rules,
                rules,
                true,
                &mut order,
                &mut matched,
            );
        }
        // Indications de présentation : origine auteur, spécificité nulle, avant toute règle.
        for d in &hints {
            matched.push((1, 0, hint_base, d));
        }
        let inline = el
            .attr("style")
            .map(|s| parse_declarations(&tokenize(s)))
            .unwrap_or_default();
        for d in &inline {
            order = order.saturating_add(1);
            matched.push((if d.important { 2 } else { 1 }, u32::MAX, order, d));
        }
        matched.sort_by_key(|(rank, spec, ord, _)| (*rank, *spec, *ord));

        let mut style = ComputedStyle::inherit_from(parent);
        let cx = Context {
            parent,
            root_font_size,
            viewport,
        };
        // La taille de police d'abord : les unités `em` des autres propriétés en dépendent.
        for (_, _, _, d) in matched
            .iter()
            .filter(|m| m.3.property == Longhand::FontSize)
        {
            apply(&mut style, d, &cx);
        }
        if is_root {
            root_font_size = style.font_size;
        }
        let cx = Context {
            parent,
            root_font_size,
            viewport,
        };
        // La couleur ensuite : `currentcolor` des autres propriétés en dépend.
        for (_, _, _, d) in matched.iter().filter(|m| m.3.property == Longhand::Color) {
            apply(&mut style, d, &cx);
        }
        for (_, _, _, d) in matched
            .iter()
            .filter(|m| !matches!(m.3.property, Longhand::FontSize | Longhand::Color))
        {
            apply(&mut style, d, &cx);
        }
        if let Some(slot) = map.styles.get_mut(node.index()) {
            *slot = Some(Arc::new(style));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEWPORT: Viewport = Viewport {
        width: 1000.0,
        height: 800.0,
    };

    fn styled(html: &str) -> (Document, StyleMap) {
        let doc = oxyde_html::parse_document(html);
        let sheets = collect_style_elements(&doc);
        let map = compute_styles(&doc, &sheets, VIEWPORT);
        (doc, map)
    }

    fn style_of(doc: &Document, map: &StyleMap, tag: &str) -> Arc<ComputedStyle> {
        let node = doc
            .find_html_element(doc.root(), tag)
            .expect("élément présent");
        Arc::clone(map.get(node).expect("style calculé"))
    }

    #[test]
    fn ua_defaults() {
        let (doc, map) = styled("<h1>t</h1><p>a <b>b</b> <a href=x>l</a></p><script>x</script>");
        let h1 = style_of(&doc, &map, "h1");
        assert_eq!(h1.display, Display::Block);
        assert_eq!(h1.font_size, 32.0);
        assert_eq!(h1.font_weight, 700.0);
        assert_eq!(h1.margin.top, Size::Px(32.0 * 0.67));
        assert_eq!(style_of(&doc, &map, "body").margin.left, Size::Px(8.0));
        assert_eq!(style_of(&doc, &map, "b").font_weight, 700.0);
        let a = style_of(&doc, &map, "a");
        assert_eq!(a.color, Color::rgb(0, 0, 238));
        assert!(a.underline);
        assert_eq!(style_of(&doc, &map, "script").display, Display::None);
    }

    #[test]
    fn cascade_order() {
        let (doc, map) = styled(
            "<style>
               p { color: red; margin: 10px }
               #x { color: green }
               .c { color: blue !important }
               p { margin-left: 2em; font-size: 20px }
             </style>
             <p id=x class=c style='color: black; padding: 1em'>t</p>",
        );
        let p = style_of(&doc, &map, "p");
        assert_eq!(
            p.color,
            Color::rgb(0, 0, 255),
            "!important l'emporte sur le style en ligne"
        );
        assert_eq!(p.margin.top, Size::Px(10.0));
        assert_eq!(
            p.margin.left,
            Size::Px(40.0),
            "em relatif à la taille de police de l'élément"
        );
        assert_eq!(p.padding.top, Size::Px(20.0));
    }

    #[test]
    fn inheritance_and_keywords() {
        let (doc, map) = styled(
            "<style>
               div { color: #123456; font-size: 10px; border: 1px solid; background: red }
               span { font-size: 150%; border-top: inherit; background: inherit }
               em { color: initial; font-size: 2rem }
             </style>
             <div><span>a<em>b</em></span></div>",
        );
        let div = style_of(&doc, &map, "div");
        let span = style_of(&doc, &map, "span");
        let em = style_of(&doc, &map, "em");
        assert_eq!(span.color, div.color);
        assert_eq!(span.font_size, 15.0);
        assert_eq!(span.background_color, Color::rgb(255, 0, 0));
        assert_eq!(span.border_widths().top, 1.0);
        assert_eq!(span.border_widths().left, 0.0);
        assert_eq!(span.border_color_of(Side::Top), div.color);
        assert_eq!(em.color, Color::BLACK);
        assert_eq!(em.font_size, 32.0);
        assert!(em.italic);
    }

    #[test]
    fn calc_and_new_properties() {
        let (doc, map) = styled(
            "<style>
               p { width: calc(100% - 20px); margin-left: calc(2 * 5px + 1em);
                   padding-top: calc((10px + 6px) / 4); height: calc(1px + ); }
               div { position: absolute; inset: 1px 2px; float: right; flex: 2 1 0;
                     z-index: 3; overflow: hidden auto; gap: 4px 8px; }
             </style><p>a</p><div>b</div>",
        );
        let p = style_of(&doc, &map, "p");
        assert_eq!(
            p.width,
            Size::Calc {
                px: -20.0,
                percent: 100.0
            }
        );
        assert_eq!(p.width.resolve(300.0), Some(280.0));
        assert_eq!(p.margin.left, Size::Px(26.0));
        assert_eq!(p.padding.top, Size::Px(4.0));
        // Une expression invalide écarte la déclaration.
        assert_eq!(p.height, Size::Auto);
        let div = style_of(&doc, &map, "div");
        assert_eq!(div.position, Position::Absolute);
        assert_eq!(div.inset.top, Size::Px(1.0));
        assert_eq!(div.inset.left, Size::Px(2.0));
        assert_eq!(div.float, Float::Right);
        assert_eq!((div.flex_grow, div.flex_shrink), (2.0, 1.0));
        assert_eq!(div.flex_basis, Some(Size::Px(0.0)));
        assert_eq!(div.z_index, Some(3));
        assert_eq!(
            (div.overflow_x, div.overflow_y),
            (Overflow::Hidden, Overflow::Auto)
        );
        assert_eq!(
            (div.row_gap, div.column_gap),
            (Size::Px(4.0), Size::Px(8.0))
        );
    }

    #[test]
    fn media_and_hints() {
        let (doc, map) = styled(
            "<style>
               @media (max-width: 500px) { p { color: red } }
               @media (min-width: 500px) { p { color: green } }
             </style>
             <p align=CENTER>t</p><table bgcolor='#ff0000' width=50%></table>",
        );
        let p = style_of(&doc, &map, "p");
        assert_eq!(p.color, Color::rgb(0, 128, 0));
        assert_eq!(p.text_align, TextAlign::Center);
        let table = style_of(&doc, &map, "table");
        assert_eq!(table.background_color, Color::rgb(255, 0, 0));
        assert_eq!(table.width, Size::Percent(50.0));
    }
}
