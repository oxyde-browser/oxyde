//! Interface d'Oxyde, dessinée d'après la maquette « Oxyde — navigateur Rust ».
//!
//! Interface en mode immédiat : [`build_chrome`] reçoit l'état du navigateur
//! et renvoie la liste d'affichage de l'interface, les zones cliquables, les
//! éléments exposés aux technologies d'assistance et l'emplacement des volets
//! de page. Aucun état n'est conservé entre deux images.
//!
//! Écrans couverts : « Fenêtre principale · vue scindée » (`Main`), « Nouvel
//! onglet » (`NewTab`) et « Pages d'erreur » (`Errors`).
//!
//! Les polices de la maquette (Inter, Space Grotesk, JetBrains Mono) sont
//! embarquées par `oxyde_text` ; Segoe UI et Consolas ne servent que de repli.

mod panes;
pub mod theme;

use kurbo::{BezPath, Circle, Shape};
use oxyde_css::Color;
use oxyde_paint::{DisplayItem, DisplayList, Rect};
use oxyde_text::parley::{
    Alignment, AlignmentOptions, FontFamily, FontWeight, LineHeight, TextStyle, TextWrapMode,
};
use oxyde_text::{TextBrush, TextContext, TextLayout};

/// Élément de l'interface sous le pointeur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Page précédente.
    Back,
    /// Page suivante.
    Forward,
    /// Actualiser.
    Reload,
    /// Activer ou quitter la vue scindée.
    SplitToggle,
    /// Barre d'adresse, ou champ de recherche du nouvel onglet.
    Address,
    /// Onglet d'index donné (rangée, tuile épinglée).
    Tab(usize),
    /// Bouton de fermeture de l'onglet d'index donné.
    CloseTab(usize),
    /// « Nouvel onglet ».
    NewTab,
    /// Paramètres.
    Settings,
    /// Espace d'index donné.
    Space(usize),
    /// Réduire la fenêtre.
    Minimize,
    /// Agrandir ou restaurer la fenêtre.
    Maximize,
    /// Fermer la fenêtre.
    CloseWindow,
    /// Zone de déplacement de la fenêtre.
    TitleBar,
    /// Contenu du volet de l'onglet d'index donné.
    Pane(usize),
    /// Bouton de fermeture du volet de l'onglet d'index donné.
    ClosePane(usize),
    /// Onglet fermé récemment, d'index donné (0 = le plus récent).
    Reopen(usize),
    /// Bouton principal de la page d'erreur de l'onglet donné.
    ErrorPrimary(usize),
    /// Bouton secondaire de la page d'erreur de l'onglet donné.
    ErrorSecondary(usize),
}

/// Nature d'une page d'erreur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorStyle {
    /// Site injoignable : « Réessayer » et diagnostic réseau.
    Network,
    /// Certificat refusé : fond rouge, « Revenir en lieu sûr ».
    Certificate,
    /// Autre échec : « Réessayer » seul.
    Other,
}

/// Page d'erreur telle que l'interface l'affiche.
#[derive(Clone, Copy, Debug)]
pub struct ErrorView<'a> {
    /// Nature de l'erreur.
    pub style: ErrorStyle,
    /// Titre.
    pub title: &'a str,
    /// Explication.
    pub message: &'a str,
    /// Code d'erreur (`ERR_…`).
    pub code: &'a str,
    /// Détail technique, affiché quand l'utilisateur l'a demandé.
    pub detail: Option<&'a str>,
}

/// Ce qu'affiche le volet d'un onglet.
#[derive(Clone, Copy, Debug)]
pub enum PaneContent<'a> {
    /// Page web, rendue par le moteur.
    Page,
    /// Page « Nouvel onglet », dessinée par l'interface.
    NewTab,
    /// Page d'erreur, dessinée par l'interface.
    Error(ErrorView<'a>),
}

/// Onglet tel que l'interface l'affiche.
#[derive(Clone, Copy, Debug)]
pub struct TabView<'a> {
    /// Titre de la page (ou adresse à défaut).
    pub title: &'a str,
    /// Nom d'hôte, pour la pastille et l'en-tête de volet.
    pub host: &'a str,
    /// Chargement en cours.
    pub loading: bool,
    /// Onglet épinglé (affiché dans la grille, commun à tous les espaces).
    pub pinned: bool,
    /// Index de l'espace auquel l'onglet appartient.
    pub space: usize,
    /// Contenu du volet.
    pub content: PaneContent<'a>,
}

/// Espace (groupe d'onglets) tel que l'interface l'affiche.
#[derive(Clone, Copy, Debug)]
pub struct SpaceView<'a> {
    /// Nom.
    pub name: &'a str,
    /// Couleur de la pastille.
    pub color: Color,
}

/// Onglet fermé récemment.
#[derive(Clone, Copy, Debug)]
pub struct ClosedView<'a> {
    /// Titre.
    pub title: &'a str,
    /// Nom d'hôte.
    pub host: &'a str,
}

/// État du navigateur nécessaire au dessin de l'interface.
#[derive(Clone, Debug)]
pub struct ChromeState<'a> {
    /// Taille de la fenêtre, en pixels logiques.
    pub width: f32,
    /// Taille de la fenêtre, en pixels logiques.
    pub height: f32,
    /// Tous les onglets ; leur index est celui porté par [`Hit::Tab`].
    pub tabs: &'a [TabView<'a>],
    /// Index de l'onglet actif (volet principal).
    pub active: usize,
    /// Index de l'onglet affiché dans le second volet, si la vue est scindée.
    pub split: Option<usize>,
    /// Texte de la barre d'adresse.
    pub address: &'a str,
    /// Vrai pendant la saisie d'une adresse.
    pub editing: bool,
    /// Vrai si tout le texte saisi est sélectionné.
    pub select_all: bool,
    /// Vrai si la page active est servie en HTTPS.
    pub secure: bool,
    /// Une page précédente existe.
    pub can_back: bool,
    /// Une page suivante existe.
    pub can_forward: bool,
    /// Élément survolé.
    pub hover: Option<Hit>,
    /// Espaces.
    pub spaces: &'a [SpaceView<'a>],
    /// Index de l'espace courant.
    pub space: usize,
    /// Onglets fermés récemment, du plus récent au plus ancien.
    pub closed: &'a [ClosedView<'a>],
    /// Défilement de la liste d'onglets, en pixels (borné par [`Chrome::tab_scroll_max`]).
    pub tab_scroll: f32,
    /// Position du curseur dans `address` pendant la saisie, en octets.
    pub caret: usize,
}

/// Volet de page web : emplacement de la page et de son onglet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pane {
    /// Index de l'onglet affiché.
    pub tab: usize,
    /// Zone de la page, en pixels logiques.
    pub rect: Rect,
    /// Zone du volet entier (en-tête compris), pour la découpe arrondie.
    pub frame: Rect,
}

/// Rôle d'un élément pour les technologies d'assistance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Bouton.
    Button,
    /// Champ de saisie.
    TextInput,
    /// Onglet.
    Tab,
    /// Contenu d'un volet.
    Document,
    /// Titre.
    Heading,
    /// Texte.
    Text,
}

/// Élément de l'interface exposé aux technologies d'assistance.
#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    /// Action déclenchée par un clic, s'il y en a une.
    pub hit: Option<Hit>,
    /// Rôle.
    pub role: Role,
    /// Nom accessible.
    pub label: String,
    /// Zone, en pixels logiques.
    pub rect: Rect,
    /// Sélectionné (onglet actif, espace courant, bascule activée).
    pub selected: bool,
    /// Désactivé.
    pub disabled: bool,
    /// Valeur (champ de saisie).
    pub value: Option<String>,
}

/// Résultat de [`build_chrome`].
#[derive(Clone, Debug, Default)]
pub struct Chrome {
    /// Liste d'affichage de l'interface (sous les pages).
    pub list: DisplayList,
    /// Liste d'affichage à dessiner par-dessus les pages (anneau du volet actif).
    pub overlay: DisplayList,
    /// Zones cliquables, de la plus prioritaire à la moins prioritaire.
    pub hits: Vec<(Rect, Hit)>,
    /// Volets de page web (les pages internes sont déjà dans `list`).
    pub panes: Vec<Pane>,
    /// Défilement maximal de la liste d'onglets (nul si tout tient à l'écran).
    pub tab_scroll_max: f32,
    /// Éléments exposés aux technologies d'assistance, dans l'ordre de lecture.
    pub widgets: Vec<Widget>,
}

impl Chrome {
    /// Élément de l'interface situé au point (`x`, `y`).
    #[must_use]
    pub fn hit(&self, x: f32, y: f32) -> Option<Hit> {
        self.hits
            .iter()
            .find(|(rect, _)| rect.contains(x, y))
            .map(|(_, hit)| *hit)
    }
}

pub(crate) const UI_FONT: &str = "\"Inter\", \"Segoe UI\", system-ui, sans-serif";
pub(crate) const DISPLAY_FONT: &str =
    "\"Space Grotesk\", \"Inter\", \"Segoe UI\", system-ui, sans-serif";
pub(crate) const MONO_FONT: &str = "\"JetBrains Mono\", \"Consolas\", monospace";

pub(crate) struct Painter<'a> {
    text: &'a mut TextContext,
    pub(crate) list: DisplayList,
    pub(crate) hits: Vec<(Rect, Hit)>,
    pub(crate) widgets: Vec<Widget>,
    pub(crate) hover: Option<Hit>,
}

#[derive(Clone, Copy)]
pub(crate) struct Label<'a> {
    pub(crate) text: &'a str,
    pub(crate) size: f32,
    pub(crate) weight: f32,
    pub(crate) color: Color,
    pub(crate) family: &'a str,
    /// Hauteur de ligne, en multiple de la taille.
    pub(crate) line: f32,
}

impl<'a> Label<'a> {
    pub(crate) fn new(text: &'a str, size: f32, color: Color) -> Self {
        Label {
            text,
            size,
            weight: 400.0,
            color,
            family: UI_FONT,
            line: 1.4,
        }
    }

    pub(crate) fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub(crate) fn family(mut self, family: &'a str) -> Self {
        self.family = family;
        self
    }

    pub(crate) fn line(mut self, line: f32) -> Self {
        self.line = line;
        self
    }
}

pub(crate) fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn svg(data: &str) -> BezPath {
    // Les tracés sont des constantes du programme, validées par les tests.
    BezPath::from_svg(data).unwrap_or_default()
}

impl Painter<'_> {
    pub(crate) fn fill(&mut self, r: Rect, color: Color, radius: f32) {
        self.list.rect(r, color, radius);
    }

    pub(crate) fn outline(&mut self, r: Rect, color: Color, radius: f32, width: f32) {
        self.list.items.push(DisplayItem::StrokeRect {
            rect: r,
            color,
            width,
            radius,
        });
    }

    pub(crate) fn stroke_path(&mut self, data: &str, x: f32, y: f32, color: Color, width: f32) {
        self.list.items.push(DisplayItem::Path {
            path: svg(data),
            x,
            y,
            color,
            stroke: Some(width),
        });
    }

    pub(crate) fn dot(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        let path = Circle::new((f64::from(cx), f64::from(cy)), f64::from(radius)).to_path(0.05);
        self.list.items.push(DisplayItem::Path {
            path,
            x: 0.0,
            y: 0.0,
            color,
            stroke: None,
        });
    }

    /// Met en forme `label` ; coupe les lignes à `wrap` si une largeur est donnée.
    fn shape(&mut self, label: &Label<'_>, wrap: Option<f32>) -> TextLayout {
        let c = label.color;
        let style = TextStyle {
            font_family: FontFamily::from(label.family),
            font_size: label.size,
            font_weight: FontWeight::new(label.weight),
            brush: TextBrush {
                color: [c.r, c.g, c.b, c.a],
                background: [0; 4],
                node: 0,
            },
            line_height: LineHeight::FontSizeRelative(label.line),
            text_wrap_mode: if wrap.is_some() {
                TextWrapMode::Wrap
            } else {
                TextWrapMode::NoWrap
            },
            ..TextStyle::default()
        };
        let mut builder = self
            .text
            .layouts
            .tree_builder(&mut self.text.fonts, 1.0, true, &style);
        builder.push_text(label.text);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(wrap);
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout
    }

    /// Largeur de `label` sur une ligne.
    pub(crate) fn measure(&mut self, label: &Label<'_>) -> f32 {
        if label.text.is_empty() {
            0.0
        } else {
            self.shape(label, None).width()
        }
    }

    /// Dessine une ligne de texte centrée verticalement dans une bande de
    /// hauteur `h` commençant en `y` ; renvoie la largeur dessinée. Le texte
    /// est coupé (sans points de suspension) au-delà de `max_w`.
    pub(crate) fn label(&mut self, label: &Label<'_>, x: f32, y: f32, h: f32, max_w: f32) -> f32 {
        if label.text.is_empty() || max_w <= 0.0 {
            return 0.0;
        }
        let layout = self.shape(label, None);
        let width = layout.width();
        let top = (y + (h - layout.height()) / 2.0).round();
        let clipped = width > max_w;
        if clipped {
            self.list.items.push(DisplayItem::PushClip {
                rect: rect(x, y, max_w, h),
                radius: 0.0,
            });
        }
        self.list.text(&layout, x, top);
        if clipped {
            self.list.items.push(DisplayItem::PopClip);
        }
        width.min(max_w)
    }

    /// Comme [`Painter::label`], centré horizontalement dans `r`.
    pub(crate) fn label_centered(&mut self, label: &Label<'_>, r: Rect) {
        let w = self.measure(label).min(r.w);
        self.label(label, r.x + (r.w - w) / 2.0, r.y, r.h, r.w);
    }

    /// Dessine un paragraphe coupé à `max_w` à partir de (`x`, `y`) ; renvoie sa hauteur.
    pub(crate) fn paragraph(&mut self, label: &Label<'_>, x: f32, y: f32, max_w: f32) -> f32 {
        if label.text.is_empty() || max_w <= 0.0 {
            return 0.0;
        }
        let layout = self.shape(label, Some(max_w));
        self.list.text(&layout, x, y);
        layout.height()
    }

    /// Déclare une zone cliquable et l'élément accessible correspondant.
    pub(crate) fn target(&mut self, r: Rect, hit: Hit, role: Role, label: &str) {
        self.hits.push((r, hit));
        self.widgets.push(Widget {
            hit: Some(hit),
            role,
            label: label.to_owned(),
            rect: r,
            selected: false,
            disabled: false,
            value: None,
        });
    }

    /// Déclare un élément accessible non cliquable (titre, texte).
    pub(crate) fn describe(&mut self, r: Rect, role: Role, label: &str) {
        self.widgets.push(Widget {
            hit: None,
            role,
            label: label.to_owned(),
            rect: r,
            selected: false,
            disabled: false,
            value: None,
        });
    }

    /// Modifie le dernier élément accessible déclaré.
    pub(crate) fn last_widget(&mut self, edit: impl FnOnce(&mut Widget)) {
        if let Some(widget) = self.widgets.last_mut() {
            edit(widget);
        }
    }

    /// Bouton carré à icône ; le fond s'éclaire au survol.
    #[expect(
        clippy::too_many_arguments,
        reason = "description complète d'un bouton"
    )]
    fn icon_button(
        &mut self,
        r: Rect,
        hit: Hit,
        name: &str,
        icon: &str,
        icon_size: f32,
        color: Color,
        stroke: f32,
    ) {
        if self.hover == Some(hit) {
            self.fill(r, theme::HOVER, 8.0);
        }
        self.stroke_path(
            icon,
            r.x + (r.w - icon_size) / 2.0,
            r.y + (r.h - icon_size) / 2.0,
            color,
            stroke,
        );
        self.target(r, hit, Role::Button, name);
    }

    /// Indication de raccourci clavier (« Ctrl L ») alignée à droite en `right`.
    pub(crate) fn key_hint(&mut self, text: &str, right: f32, y: f32, h: f32) {
        let label = Label::new(text, 11.0, theme::TEXT_MUTED);
        let w = self.measure(&label);
        let frame = rect(right - w - 12.0, y + (h - 18.0) / 2.0, w + 12.0, 18.0);
        self.outline(frame, theme::BORDER_HINT, 5.0, 1.0);
        self.label(&label, frame.x + 6.0, frame.y, frame.h, w + 1.0);
    }

    /// Pastille carrée portant l'initiale d'un site.
    pub(crate) fn badge(&mut self, r: Rect, radius: f32, host: &str, title: &str, size: f32) {
        self.fill(r, favicon_color(host), radius);
        let letter = initial(host, title);
        self.label_centered(&Label::new(&letter, size, theme::WHITE).weight(600.0), r);
    }
}

const ICON_BACK: &str = "M10 3 5 8l5 5";
const ICON_FORWARD: &str = "M6 3l5 5-5 5";
const ICON_RELOAD: &str = "M13 8a5 5 0 1 1-1.5-3.5M13 3v2.5h-2.5";
const ICON_SPLIT: &str =
    "M4 3h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM8 3v10";
const ICON_LOCK: &str = "M5 7h6a1.5 1.5 0 0 1 1.5 1.5v3.5a1.5 1.5 0 0 1-1.5 1.5H5a1.5 1.5 0 0 1-1.5-1.5V8.5A1.5 1.5 0 0 1 5 7zM5.5 7V5a2.5 2.5 0 0 1 5 0v2";
const ICON_GLOBE: &str =
    "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM2.5 8h11M8 2.5c-2 2-2 9 0 11M8 2.5c2 2 2 9 0 11";
pub(crate) const ICON_CLOSE: &str = "M4 4l8 8M12 4l-8 8";
const ICON_PLUS: &str = "M8 3v10M3 8h10";
const ICON_SETTINGS: &str = "M3 5h5.5M12.5 5h.5M3 11h1M7.5 11H13M12.25 5a1.75 1.75 0 1 1-3.5 0 1.75 1.75 0 0 1 3.5 0zM7.25 11a1.75 1.75 0 1 1-3.5 0 1.75 1.75 0 0 1 3.5 0z";
const ICON_WIN_MIN: &str = "M1.5 6h9";
const ICON_WIN_MAX: &str =
    "M2.5 1.5h7a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1z";
const ICON_WIN_CLOSE: &str = "M1.5 1.5l9 9M10.5 1.5l-9 9";
const ICON_LOGO: &str = "M9 2.5a6.5 6.5 0 1 0 6.5 6.5";

/// Couleur stable pour un hôte donné, prise dans `palette`.
fn host_color(host: &str, palette: &[Color; 4]) -> Color {
    let hash = host
        .bytes()
        .fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    palette[hash as usize % palette.len()]
}

pub(crate) fn favicon_color(host: &str) -> Color {
    host_color(host, &theme::FAVICONS)
}

pub(crate) fn initial(host: &str, title: &str) -> String {
    let source = if host.is_empty() { title } else { host };
    source
        .trim_start_matches("www.")
        .chars()
        .find(|c| c.is_alphanumeric())
        .map_or_else(|| "·".to_owned(), |c| c.to_uppercase().collect())
}

/// Titre affiché pour un onglet.
pub(crate) fn tab_title<'a>(tab: &TabView<'a>) -> &'a str {
    match tab.content {
        PaneContent::NewTab => "Nouvel onglet",
        _ if tab.loading && tab.title.is_empty() => "Chargement…",
        _ => tab.title,
    }
}

/// Construit l'interface pour l'état `state`.
pub fn build_chrome(state: &ChromeState<'_>, text: &mut TextContext) -> Chrome {
    let mut p = Painter {
        text,
        list: DisplayList::default(),
        hits: Vec::new(),
        widgets: Vec::new(),
        hover: state.hover,
    };
    let (w, h) = (state.width, state.height);
    p.fill(rect(0.0, 0.0, w, h), theme::WINDOW, 0.0);

    // ---- Barre de titre ----------------------------------------------------
    let title_h = theme::TITLE_BAR;
    p.stroke_path(ICON_LOGO, 16.0, (title_h - 18.0) / 2.0, theme::ACCENT, 2.5);
    p.dot(16.0 + 13.6, (title_h - 18.0) / 2.0 + 4.4, 1.6, theme::AMBER);
    let name = Label::new("Oxyde", 13.0, theme::TEXT)
        .weight(600.0)
        .family(DISPLAY_FONT);
    p.label(&name, 44.0, 0.0, title_h, 120.0);
    let buttons = [
        (Hit::Minimize, ICON_WIN_MIN, "Réduire"),
        (Hit::Maximize, ICON_WIN_MAX, "Agrandir"),
        (Hit::CloseWindow, ICON_WIN_CLOSE, "Fermer"),
    ];
    for (i, (hit, icon, label)) in buttons.into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "trois boutons")]
        let r = rect(w - 46.0 * (3 - i) as f32, 0.0, 46.0, title_h);
        if p.hover == Some(hit) {
            let color = if hit == Hit::CloseWindow {
                theme::DANGER
            } else {
                theme::HOVER
            };
            p.fill(r, color, 0.0);
        }
        p.stroke_path(icon, r.x + 17.0, r.y + 14.0, theme::ICON, 1.0);
        p.target(r, hit, Role::Button, label);
    }
    p.hits
        .push((rect(0.0, 0.0, w - 138.0, title_h), Hit::TitleBar));

    // ---- Barre latérale ----------------------------------------------------
    let side_x = 8.0 + 4.0;
    let side_w = theme::SIDEBAR - 8.0;
    let side_right = side_x + side_w;
    let mut y = title_h + 4.0;

    let nav = [
        (Hit::Back, ICON_BACK, state.can_back, "Page précédente"),
        (
            Hit::Forward,
            ICON_FORWARD,
            state.can_forward,
            "Page suivante",
        ),
        (Hit::Reload, ICON_RELOAD, true, "Actualiser"),
    ];
    for (i, (hit, icon, enabled, label)) in nav.into_iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "trois boutons")]
        let r = rect(side_x + 38.0 * i as f32, y, 36.0, 36.0);
        if enabled {
            p.icon_button(r, hit, label, icon, 16.0, theme::ICON, 1.5);
        } else {
            p.stroke_path(icon, r.x + 10.0, r.y + 10.0, theme::ICON_DISABLED, 1.5);
            p.describe(r, Role::Button, label);
            p.last_widget(|w| w.disabled = true);
        }
    }
    let split_r = rect(side_right - 36.0, y, 36.0, 36.0);
    if state.split.is_some() {
        p.fill(split_r, theme::PILL, 8.0);
    }
    let split_color = if state.split.is_some() {
        theme::ACCENT
    } else {
        theme::ICON
    };
    p.icon_button(
        split_r,
        Hit::SplitToggle,
        "Vue scindée",
        ICON_SPLIT,
        16.0,
        split_color,
        1.5,
    );
    p.last_widget(|w| w.selected = state.split.is_some());
    y += 36.0 + 16.0;

    // Barre d'adresse. Sur la page « Nouvel onglet », la saisie se fait dans
    // le champ de recherche de la page : la barre n'affiche pas le curseur.
    let active_is_new_tab = matches!(
        state.tabs.get(state.active).map(|t| t.content),
        Some(PaneContent::NewTab)
    );
    let editing_here = state.editing && !active_is_new_tab;
    let addr = rect(side_x, y, side_w, 40.0);
    p.fill(addr, theme::SURFACE, 10.0);
    p.outline(
        addr,
        if editing_here {
            theme::ACCENT
        } else {
            theme::BORDER_SOFT
        },
        10.0,
        1.0,
    );
    p.target(addr, Hit::Address, Role::TextInput, "Adresse");
    p.last_widget(|w| w.value = Some(state.address.to_owned()));
    let icon = if state.secure { ICON_LOCK } else { ICON_GLOBE };
    p.stroke_path(icon, addr.x + 12.0, addr.y + 12.0, theme::TEXT_MUTED, 1.3);
    let text_x = addr.x + 12.0 + 16.0 + 8.0;
    if editing_here {
        let max_w = addr.x + addr.w - 12.0 - text_x;
        panes::edit_field(&mut p, state, text_x, addr.y, addr.h, max_w, 14.0, "");
    } else {
        let max_w = addr.x + addr.w - 10.0 - 52.0 - text_x;
        let shown = if state.editing { "" } else { state.address };
        p.label(
            &Label::new(shown, 14.0, theme::TEXT),
            text_x,
            addr.y,
            addr.h,
            max_w,
        );
        p.key_hint("Ctrl L", addr.x + addr.w - 10.0, addr.y, addr.h);
    }
    y += 40.0 + 16.0;

    // Onglets épinglés : grille de quatre tuiles par rangée.
    let pinned: Vec<usize> = (0..state.tabs.len())
        .filter(|i| state.tabs[*i].pinned)
        .collect();
    if !pinned.is_empty() {
        let tile_w = (side_w - 3.0 * 8.0) / 4.0;
        for (n, &i) in pinned.iter().enumerate() {
            let tab = &state.tabs[i];
            #[expect(clippy::cast_precision_loss, reason = "peu d'onglets épinglés")]
            let r = rect(
                side_x + (tile_w + 8.0) * (n % 4) as f32,
                y + 52.0 * (n / 4) as f32,
                tile_w,
                44.0,
            );
            let shown = i == state.active || Some(i) == state.split;
            let background = if shown {
                theme::ACTIVE
            } else if p.hover == Some(Hit::Tab(i)) {
                theme::HOVER_STRONG
            } else {
                theme::SURFACE
            };
            p.fill(r, background, 10.0);
            let letter = initial(tab.host, tab.title);
            let color = host_color(tab.host, &theme::PINNED_LETTERS);
            p.label_centered(&Label::new(&letter, 14.0, color).weight(600.0), r);
            p.target(
                r,
                Hit::Tab(i),
                Role::Tab,
                &format!("{}, épinglé", tab_title(tab)),
            );
            p.last_widget(|w| w.selected = i == state.active);
        }
        #[expect(clippy::cast_precision_loss, reason = "peu d'onglets épinglés")]
        let rows = pinned.len().div_ceil(4) as f32;
        y += rows * 52.0 - 8.0 + 16.0;
    }

    // Titre de l'espace.
    let space_name = state.spaces.get(state.space).map_or("", |s| s.name);
    let caption = format!("ESPACE · {}", space_name.to_uppercase());
    p.label(
        &Label::new(&caption, 11.0, theme::TEXT_MUTED),
        side_x + 8.0,
        y,
        16.0,
        side_w - 16.0,
    );
    y += 16.0 + 6.0;

    // Onglets de l'espace courant. Ceux de la vue scindée sont regroupés dans
    // un cadre quand ils se suivent dans la liste.
    let visible: Vec<usize> = (0..state.tabs.len())
        .filter(|i| !state.tabs[*i].pinned && state.tabs[*i].space == state.space)
        .collect();
    let list_bottom = h - 8.0 - 48.0 - 42.0;
    let group = state.split.and_then(|s| {
        let a = visible.iter().position(|i| *i == state.active)?;
        let b = visible.iter().position(|i| *i == s)?;
        (a.abs_diff(b) == 1).then_some((a.min(b), a.max(b)))
    });
    // Quand la liste dépasse la place disponible, elle défile dans une zone
    // découpée et la rangée « Nouvel onglet » reste ancrée dessous.
    let list_top = y;
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre d'onglets petit devant 2^24"
    )]
    let content_h = visible.len() as f32 * 42.0 + if group.is_some() { 36.0 } else { 0.0 };
    let list_h = (list_bottom - list_top).max(0.0);
    let tab_scroll_max = (content_h - list_h).max(0.0);
    let scrolling = tab_scroll_max > 0.0;
    let list_clip = rect(side_x - 4.0, list_top, side_w + 8.0, list_h);
    let first_list_hit = p.hits.len();
    if scrolling {
        p.list.items.push(DisplayItem::PushClip {
            rect: list_clip,
            radius: 0.0,
        });
        y -= state.tab_scroll.clamp(0.0, tab_scroll_max);
    }
    for (n, &i) in visible.iter().enumerate() {
        let tab = &state.tabs[i];
        let grouped = group.is_some_and(|(first, last)| n == first || n == last);
        if group.is_some_and(|(first, _)| n == first) {
            let frame = rect(side_x, y, side_w, 4.0 + 40.0 + 2.0 + 40.0 + 22.0 + 4.0);
            p.fill(frame, theme::GROUP, 12.0);
            p.outline(frame, theme::BORDER_SOFT, 12.0, 1.0);
            y += 4.0;
        }
        let inset = if grouped { 4.0 } else { 0.0 };
        let row = rect(side_x + inset, y, side_w - 2.0 * inset, 40.0);
        let is_active = i == state.active;
        let hovered = p.hover == Some(Hit::Tab(i)) || p.hover == Some(Hit::CloseTab(i));
        if is_active {
            p.fill(row, theme::ACTIVE, 8.0);
        } else if hovered {
            p.fill(row, theme::HOVER, 8.0);
        }
        let pad_left = if grouped { 10.0 } else { 14.0 };
        let badge = rect(row.x + pad_left, row.y + 11.0, 18.0, 18.0);
        if matches!(tab.content, PaneContent::NewTab) {
            p.stroke_path(ICON_PLUS, badge.x + 1.0, badge.y + 1.0, theme::ICON, 1.5);
        } else {
            p.badge(badge, 5.0, tab.host, tab.title, 11.0);
        }

        let show_close = is_active || hovered;
        let close = rect(row.x + row.w - 8.0 - 22.0, row.y + 9.0, 22.0, 22.0);
        let title_x = badge.x + 18.0 + 10.0;
        let title_max = if show_close {
            close.x - 6.0 - title_x
        } else {
            row.x + row.w - 8.0 - title_x
        };
        let title_text = tab_title(tab);
        let color = if is_active {
            theme::WHITE
        } else {
            theme::TEXT_TAB
        };
        let title =
            Label::new(title_text, 14.0, color).weight(if is_active { 500.0 } else { 400.0 });
        p.label(&title, title_x, row.y, row.h, title_max);
        if show_close {
            if p.hover == Some(Hit::CloseTab(i)) {
                p.fill(close, theme::HOVER_STRONG, 6.0);
            }
            p.stroke_path(ICON_CLOSE, close.x + 4.0, close.y + 4.0, theme::ICON, 1.5);
            p.target(
                close,
                Hit::CloseTab(i),
                Role::Button,
                &format!("Fermer {title_text}"),
            );
        }
        p.target(row, Hit::Tab(i), Role::Tab, title_text);
        p.last_widget(|w| w.selected = is_active);
        y += 40.0 + 2.0;

        if group.is_some_and(|(_, last)| n == last) {
            p.label(
                &Label::new("Vue scindée · 2 pages", 11.0, theme::TEXT_MUTED),
                side_x + 14.0,
                y,
                18.0,
                side_w - 28.0,
            );
            y += 22.0 + 4.0 + 6.0;
        }
    }
    if scrolling {
        p.list.items.push(DisplayItem::PopClip);
        // Les rangées sorties de la zone visible ne doivent plus être cliquables.
        for (r, _) in p.hits.iter_mut().skip(first_list_hit) {
            let top = r.y.max(list_clip.y);
            let bottom = (r.y + r.h).min(list_clip.y + list_clip.h);
            r.y = top;
            r.h = (bottom - top).max(0.0);
        }
        y = list_bottom + 2.0;
    }

    // « Nouvel onglet ».
    let new_tab = rect(side_x, y, side_w, 40.0);
    if p.hover == Some(Hit::NewTab) {
        p.fill(new_tab, theme::HOVER, 8.0);
    }
    p.stroke_path(
        ICON_PLUS,
        new_tab.x + 14.0,
        new_tab.y + 12.0,
        theme::TEXT_MUTED,
        1.5,
    );
    p.label(
        &Label::new("Nouvel onglet", 14.0, theme::TEXT_MUTED),
        new_tab.x + 40.0,
        new_tab.y,
        40.0,
        140.0,
    );
    p.key_hint("Ctrl T", new_tab.x + new_tab.w - 8.0, new_tab.y, 40.0);
    p.target(new_tab, Hit::NewTab, Role::Button, "Nouvel onglet");

    // Bas de la barre : espaces et paramètres.
    let bottom_y = h - 8.0 - 4.0 - 36.0;
    let mut space_x = side_x;
    for (i, space) in state.spaces.iter().enumerate() {
        let current = i == state.space;
        let label = Label::new(space.name, 13.0, theme::TEXT);
        let r = if current {
            let sw = p.measure(&label).min(120.0);
            rect(space_x, bottom_y, 12.0 + 8.0 + 8.0 + sw + 12.0, 36.0)
        } else {
            rect(space_x, bottom_y, 36.0, 36.0)
        };
        if current {
            p.fill(r, theme::PILL, 18.0);
            p.dot(r.x + 16.0, r.y + 18.0, 4.0, space.color);
            p.label(&label, r.x + 28.0, r.y, 36.0, 120.0);
        } else {
            if p.hover == Some(Hit::Space(i)) {
                p.fill(r, theme::HOVER, 18.0);
            }
            p.dot(r.x + 18.0, r.y + 18.0, 4.0, space.color);
        }
        p.target(
            r,
            Hit::Space(i),
            Role::Button,
            &format!("Espace {}", space.name),
        );
        p.last_widget(|w| w.selected = current);
        space_x += r.w + 4.0;
    }
    let settings = rect(side_right - 36.0, bottom_y, 36.0, 36.0);
    p.icon_button(
        settings,
        Hit::Settings,
        "Paramètres",
        ICON_SETTINGS,
        16.0,
        theme::ICON,
        1.5,
    );

    // ---- Volets de contenu -------------------------------------------------
    let mut overlay = DisplayList::default();
    let mut web_panes = Vec::new();
    let area = rect(
        8.0 + theme::SIDEBAR + 8.0,
        title_h,
        (w - theme::SIDEBAR - 24.0).max(0.0),
        (h - title_h - 8.0).max(0.0),
    );
    let shown: Vec<usize> = match state.split {
        Some(second) if second != state.active && second < state.tabs.len() => {
            vec![state.active.min(second), state.active.max(second)]
        }
        _ if state.active < state.tabs.len() => vec![state.active],
        _ => Vec::new(),
    };
    let count = shown.len();
    #[expect(clippy::cast_precision_loss, reason = "au plus deux volets")]
    let pane_w = if count == 0 {
        0.0
    } else {
        (area.w - 8.0 * (count as f32 - 1.0)) / count as f32
    };
    for (slot, tab_index) in shown.into_iter().enumerate() {
        let tab = &state.tabs[tab_index];
        #[expect(clippy::cast_precision_loss, reason = "au plus deux volets")]
        let frame = rect(
            area.x + (pane_w + 8.0) * slot as f32,
            area.y,
            pane_w,
            area.h,
        );
        let mut page = frame;
        if count > 1 {
            // En-tête de volet : hôte et bouton de fermeture.
            let header = rect(frame.x, frame.y, frame.w, 32.0);
            p.list.items.push(DisplayItem::PushClip {
                rect: frame,
                radius: 12.0,
            });
            p.fill(header, theme::PANE_HEADER, 0.0);
            p.list.items.push(DisplayItem::PopClip);
            let host = if tab.host.is_empty() {
                tab_title(tab)
            } else {
                tab.host
            };
            p.label(
                &Label::new(host, 12.0, theme::PANE_HEADER_TEXT),
                header.x + 14.0,
                header.y,
                32.0,
                header.w - 60.0,
            );
            let close = rect(header.x + header.w - 6.0 - 28.0, header.y + 2.0, 28.0, 28.0);
            if p.hover == Some(Hit::ClosePane(tab_index)) {
                p.fill(close, theme::PANE_HEADER_HOVER, 6.0);
            }
            p.stroke_path(
                ICON_CLOSE,
                close.x + 8.0,
                close.y + 8.0,
                theme::PANE_HEADER_TEXT,
                1.2,
            );
            p.target(
                close,
                Hit::ClosePane(tab_index),
                Role::Button,
                "Fermer ce volet",
            );
            page = rect(frame.x, frame.y + 32.0, frame.w, (frame.h - 32.0).max(0.0));
            if tab_index == state.active {
                overlay.items.push(DisplayItem::StrokeRect {
                    rect: frame,
                    color: theme::ACCENT,
                    width: 2.0,
                    radius: 12.0,
                });
            }
        }
        // Les pages internes sont découpées aux coins arrondis du volet.
        match tab.content {
            PaneContent::Page => web_panes.push(Pane {
                tab: tab_index,
                rect: page,
                frame,
            }),
            PaneContent::NewTab => {
                p.list.items.push(DisplayItem::PushClip {
                    rect: frame,
                    radius: 12.0,
                });
                panes::new_tab(&mut p, state, tab_index, page, count > 1);
                p.list.items.push(DisplayItem::PopClip);
            }
            PaneContent::Error(error) => {
                p.list.items.push(DisplayItem::PushClip {
                    rect: frame,
                    radius: 12.0,
                });
                panes::error(&mut p, &error, tab_index, page, count > 1);
                p.list.items.push(DisplayItem::PopClip);
            }
        }
        p.target(page, Hit::Pane(tab_index), Role::Document, tab_title(tab));
    }

    Chrome {
        list: p.list,
        overlay,
        hits: p.hits,
        panes: web_panes,
        tab_scroll_max,
        widgets: p.widgets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACES: [SpaceView<'static>; 3] = [
        SpaceView {
            name: "Travail",
            color: theme::ACCENT,
        },
        SpaceView {
            name: "Perso",
            color: theme::AMBER,
        },
        SpaceView {
            name: "Veille",
            color: theme::BLUE,
        },
    ];

    fn tab(title: &'static str, host: &'static str) -> TabView<'static> {
        TabView {
            title,
            host,
            loading: false,
            pinned: false,
            space: 0,
            content: PaneContent::Page,
        }
    }

    fn state<'a>(tabs: &'a [TabView<'a>], split: Option<usize>) -> ChromeState<'a> {
        ChromeState {
            width: 1440.0,
            height: 900.0,
            tabs,
            active: 0,
            split,
            address: "docs.rs/wgpu",
            editing: false,
            select_all: false,
            secure: true,
            can_back: true,
            can_forward: false,
            hover: None,
            spaces: &SPACES,
            space: 0,
            closed: &[],
            tab_scroll: 0.0,
            caret: 0,
        }
    }

    fn tabs() -> [TabView<'static>; 3] {
        [
            tab("wgpu — docs.rs", "docs.rs"),
            tab("Dessiner un triangle", "wgpu.rs"),
            tab("", ""),
        ]
    }

    fn hit_rect(chrome: &Chrome, hit: Hit) -> Option<Rect> {
        chrome.hits.iter().find(|(_, h)| *h == hit).map(|(r, _)| *r)
    }

    #[test]
    fn icons_are_valid_svg_paths() {
        for icon in [
            ICON_BACK,
            ICON_FORWARD,
            ICON_RELOAD,
            ICON_SPLIT,
            ICON_LOCK,
            ICON_GLOBE,
            ICON_CLOSE,
            ICON_PLUS,
            ICON_SETTINGS,
            ICON_WIN_MIN,
            ICON_WIN_MAX,
            ICON_WIN_CLOSE,
            ICON_LOGO,
            panes::ICON_SEARCH,
            panes::ICON_OFFLINE,
            panes::ICON_WARNING,
        ] {
            let path = BezPath::from_svg(icon).unwrap_or_else(|e| panic!("{icon}: {e}"));
            assert!(!path.elements().is_empty());
        }
    }

    #[test]
    fn single_pane_fills_content_area() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        assert_eq!(chrome.panes.len(), 1);
        let pane = chrome.panes[0];
        assert_eq!(pane.tab, 0);
        assert_eq!(
            pane.rect,
            Rect {
                x: 280.0,
                y: 40.0,
                w: 1152.0,
                h: 852.0
            }
        );
        assert_eq!(chrome.hit(300.0, 400.0), Some(Hit::Pane(0)));
        assert_eq!(chrome.hit(100.0, 10.0), Some(Hit::TitleBar));
        assert_eq!(chrome.hit(1430.0, 10.0), Some(Hit::CloseWindow));
        assert_eq!(chrome.hit(30.0, 60.0), Some(Hit::Back));
        assert_eq!(chrome.hit(70.0, 60.0), None, "« suivant » est désactivé");
        assert_eq!(chrome.hit(100.0, 110.0), Some(Hit::Address));
        assert!(chrome.overlay.items.is_empty());
    }

    #[test]
    fn split_view_shows_two_panes_with_headers() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let chrome = build_chrome(&state(&tabs, Some(1)), &mut text);
        assert_eq!(chrome.panes.len(), 2);
        let (a, b) = (chrome.panes[0], chrome.panes[1]);
        assert_eq!((a.tab, b.tab), (0, 1));
        assert_eq!(a.rect.w, b.rect.w);
        assert_eq!(a.rect.y, 72.0, "page sous l'en-tête de 32 px");
        assert_eq!(b.frame.x, a.frame.x + a.frame.w + 8.0);
        assert_eq!(chrome.overlay.items.len(), 1, "anneau du volet actif");
        let close_b = hit_rect(&chrome, Hit::ClosePane(1)).expect("fermeture du second volet");
        assert!(close_b.x > b.frame.x);
    }

    #[test]
    fn tabs_are_clickable_and_closable() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let mut s = state(&tabs, None);
        s.hover = Some(Hit::Tab(1));
        let chrome = build_chrome(&s, &mut text);
        let row = |i: usize| hit_rect(&chrome, Hit::Tab(i)).expect("onglet");
        assert_eq!(row(1).y, row(0).y + 42.0);
        // Le bouton de fermeture passe avant la rangée.
        let close = hit_rect(&chrome, Hit::CloseTab(1)).expect("fermeture de l'onglet survolé");
        assert_eq!(
            chrome.hit(close.x + 5.0, close.y + 5.0),
            Some(Hit::CloseTab(1))
        );
        assert_eq!(
            chrome.hit(row(1).x + 40.0, row(1).y + 20.0),
            Some(Hit::Tab(1))
        );
        assert!(hit_rect(&chrome, Hit::NewTab).is_some());
    }

    #[test]
    fn pinned_tabs_and_spaces() {
        let mut text = TextContext::new();
        let mut tabs = tabs();
        tabs[1].pinned = true;
        tabs[2].space = 1;
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        let tile = hit_rect(&chrome, Hit::Tab(1)).expect("tuile épinglée");
        assert_eq!((tile.w, tile.h), (58.0, 44.0));
        let row = hit_rect(&chrome, Hit::Tab(0)).expect("onglet de l'espace courant");
        assert!(row.y > tile.y + tile.h, "la liste commence sous la grille");
        assert!(
            hit_rect(&chrome, Hit::Tab(2)).is_none(),
            "onglet d'un autre espace masqué"
        );
        // Trois espaces : la pastille de l'espace courant porte son nom.
        let current = hit_rect(&chrome, Hit::Space(0)).expect("espace courant");
        let other = hit_rect(&chrome, Hit::Space(1)).expect("autre espace");
        assert!(current.w > 36.0);
        assert_eq!(other.w, 36.0);
        assert_eq!(other.x, current.x + current.w + 4.0);
    }

    #[test]
    fn long_tab_list_scrolls_inside_its_area() {
        let mut text = TextContext::new();
        let many: Vec<TabView<'static>> = (0..40).map(|_| tab("Onglet", "site.test")).collect();
        let mut s = state(&many, None);
        let chrome = build_chrome(&s, &mut text);
        assert!(
            chrome.tab_scroll_max > 0.0,
            "quarante onglets ne tiennent pas dans 900 px"
        );
        let new_tab = hit_rect(&chrome, Hit::NewTab).expect("rangée Nouvel onglet");
        assert!(
            new_tab.y + new_tab.h <= 900.0 - 48.0,
            "ancrée au-dessus des espaces"
        );
        // Les rangées hors de la zone visible ne sont pas cliquables.
        let last = hit_rect(&chrome, Hit::Tab(39)).expect("dernier onglet");
        assert_eq!(last.h, 0.0);
        assert_eq!(chrome.hit(100.0, new_tab.y + 20.0), Some(Hit::NewTab));

        // Défilé à fond, le dernier onglet devient visible et le premier disparaît.
        s.tab_scroll = chrome.tab_scroll_max;
        let scrolled = build_chrome(&s, &mut text);
        let last = hit_rect(&scrolled, Hit::Tab(39)).expect("dernier onglet");
        assert_eq!(last.h, 40.0);
        assert!(last.y + last.h <= new_tab.y);
        assert_eq!(
            hit_rect(&scrolled, Hit::Tab(0)).expect("premier onglet").h,
            0.0
        );

        // Une courte liste ne défile pas.
        let few = tabs();
        assert_eq!(
            build_chrome(&state(&few, None), &mut text).tab_scroll_max,
            0.0
        );
    }

    #[test]
    fn new_tab_page_is_drawn_by_the_chrome() {
        let mut text = TextContext::new();
        let mut tabs = tabs();
        tabs[0].content = PaneContent::NewTab;
        tabs[1].pinned = true;
        let closed = [ClosedView {
            title: "Cartes",
            host: "cartes.test",
        }];
        let mut s = state(&tabs, None);
        s.closed = &closed;
        let chrome = build_chrome(&s, &mut text);
        assert!(chrome.panes.is_empty(), "aucune page web à rendre");
        let search = chrome
            .hits
            .iter()
            .filter(|(_, h)| *h == Hit::Address)
            .map(|(r, _)| *r)
            .find(|r| r.x > 280.0)
            .expect("champ de recherche");
        assert_eq!(search.h, 56.0);
        assert!(search.w <= 720.0);
        assert!(hit_rect(&chrome, Hit::Reopen(0)).is_some());
        let tiles = chrome
            .hits
            .iter()
            .filter(|(_, h)| *h == Hit::Tab(1))
            .count();
        assert_eq!(tiles, 2, "tuile de la barre latérale et tuile de la page");
        assert!(
            chrome
                .widgets
                .iter()
                .any(|w| w.role == Role::Heading && w.label == "Où allons-nous ?")
        );
    }

    #[test]
    fn error_pages_offer_their_actions() {
        let mut text = TextContext::new();
        let mut tabs = tabs();
        let view = |style| ErrorView {
            style,
            title: "Cette connexion n'est pas sécurisée",
            message: "Le certificat de exemple.fr a expiré.",
            code: "NET::ERR_CERT_DATE_INVALID",
            detail: None,
        };
        tabs[0].content = PaneContent::Error(view(ErrorStyle::Certificate));
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        assert!(chrome.panes.is_empty());
        let primary = hit_rect(&chrome, Hit::ErrorPrimary(0)).expect("bouton principal");
        let secondary = hit_rect(&chrome, Hit::ErrorSecondary(0)).expect("bouton secondaire");
        assert_eq!(primary.h, 44.0);
        assert!(secondary.x > primary.x + primary.w);
        let labels: Vec<&str> = chrome
            .widgets
            .iter()
            .filter(|w| matches!(w.hit, Some(Hit::ErrorPrimary(_) | Hit::ErrorSecondary(_))))
            .map(|w| w.label.as_str())
            .collect();
        assert_eq!(labels, ["Revenir en lieu sûr", "Afficher les détails"]);

        tabs[0].content = PaneContent::Error(view(ErrorStyle::Other));
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        assert!(hit_rect(&chrome, Hit::ErrorPrimary(0)).is_some());
        assert!(hit_rect(&chrome, Hit::ErrorSecondary(0)).is_none());
    }

    #[test]
    fn widgets_describe_the_interface() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        let find = |label: &str| chrome.widgets.iter().find(|w| w.label == label);
        assert!(find("Page suivante").is_some_and(|w| w.disabled && w.hit.is_none()));
        assert!(find("Page précédente").is_some_and(|w| !w.disabled && w.hit == Some(Hit::Back)));
        let address = find("Adresse").expect("barre d'adresse");
        assert_eq!(
            (address.role, address.value.as_deref()),
            (Role::TextInput, Some("docs.rs/wgpu"))
        );
        assert!(find("wgpu — docs.rs").is_some_and(|w| w.role == Role::Tab && w.selected));
        assert!(find("Espace Travail").is_some_and(|w| w.selected));
    }
}
