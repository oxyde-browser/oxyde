//! Interface d'Oxyde, dessinée d'après la maquette « Oxyde — navigateur Rust ».
//!
//! Interface en mode immédiat : [`build_chrome`] reçoit l'état du navigateur
//! et renvoie la liste d'affichage de l'interface, les zones cliquables, les
//! éléments exposés aux technologies d'assistance et l'emplacement des volets
//! de page. Aucun état n'est conservé entre deux images.
//!
//! Écrans couverts : « Fenêtre principale · vue scindée » (`Main`), « Nouvel
//! onglet » (`NewTab`), « Pages d'erreur » (`Errors`), « Historique »
//! (`History`), « Favoris » (`Bookmarks`), « Paramètres » (`Settings`) et
//! « Palette de commandes » (`Palette`), ainsi que la fenêtre de « Navigation
//! privée » (`Private`).
//!
//! Les polices de la maquette (Inter, Space Grotesk, JetBrains Mono) sont
//! embarquées par `oxyde_text` ; Segoe UI et Consolas ne servent que de repli.

mod find;
mod horizontal;
mod library;
mod onboarding;
mod palette;
mod panes;
mod settings;
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
    /// Bibliothèque : « Retour aux onglets ».
    LibraryBack,
    /// Bibliothèque : rubrique.
    LibrarySection(LibrarySection),
    /// Bibliothèque : ligne d'index donné (ouvre la page).
    LibraryRow(usize),
    /// Bibliothèque : bouton de retrait de la ligne d'index donné.
    LibraryRemove(usize),
    /// Bibliothèque : champ de recherche.
    LibrarySearch,
    /// Bibliothèque : effacer tout l'historique (à confirmer).
    LibraryClear,
    /// Recherche dans la page : champ de saisie.
    FindInput,
    /// Recherche dans la page : occurrence précédente.
    FindPrevious,
    /// Recherche dans la page : occurrence suivante.
    FindNext,
    /// Recherche dans la page : fermer la barre.
    FindClose,
    /// Premier lancement : navigateur d'où importer, d'index donné.
    OnboardingSource(usize),
    /// Premier lancement : couleur d'accent d'index donné.
    OnboardingAccent(usize),
    /// Premier lancement : passer l'étape.
    OnboardingSkip,
    /// Premier lancement : valider l'étape.
    OnboardingNext,
    /// Palette : ligne d'index donné.
    PaletteRow(usize),
    /// Palette : filtre d'index donné.
    PaletteFilter(usize),
    /// Palette : la boîte elle-même, hors de ses lignes.
    PaletteInput,
    /// Palette : le reste de la fenêtre (un clic la ferme).
    PaletteDismiss,
    /// Paramètres : rubrique d'index donné.
    SettingsGroup(usize),
    /// Paramètres : commande du réglage `row` (index parmi tous les
    /// réglages) ; `option` : valeur choisie, pour un choix ou des pastilles.
    Setting {
        /// Index du réglage.
        row: usize,
        /// Index de la valeur choisie.
        option: usize,
    },
}

/// Barre de recherche dans la page telle que l'interface l'affiche.
#[derive(Clone, Copy, Debug)]
pub struct FindView<'a> {
    /// Texte cherché, affiché quand la barre n'a pas la saisie.
    pub query: &'a str,
    /// La barre a la saisie : le texte est alors [`ChromeState::address`].
    pub focused: bool,
    /// Nombre d'occurrences.
    pub count: usize,
    /// Rang de l'occurrence courante, à partir de 1.
    pub current: usize,
}

/// Premier lancement tel que l'interface l'affiche.
#[derive(Clone, Copy, Debug)]
pub struct OnboardingView<'a> {
    /// Étape en cours : 0, l'import ; 1, la couleur d'accent.
    pub step: usize,
    /// Navigateurs d'où importer ; le dernier choix est de ne rien importer.
    pub sources: &'a [&'a str],
    /// Index du choix retenu dans `sources`.
    pub selected: usize,
    /// Nombre de favoris importés à l'étape précédente, s'il y a eu import.
    pub imported: Option<usize>,
    /// Couleurs d'accent proposées.
    pub accents: &'a [Color],
    /// Nom accessible de chaque couleur.
    pub accent_names: &'a [&'a str],
    /// Index de la couleur retenue.
    pub accent: usize,
}

/// Dessin placé en tête d'une ligne de la palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteGlyph {
    /// Pastille portant l'initiale du site.
    Badge,
    /// Commande du navigateur.
    Command,
    /// Recherche sur le web.
    Search,
    /// Adresse à ouvrir.
    Page,
}

/// Ligne de la palette de commandes.
#[derive(Clone, Copy, Debug)]
pub struct PaletteRow<'a> {
    /// Texte principal.
    pub title: &'a str,
    /// Nom d'hôte, pour la pastille.
    pub host: &'a str,
    /// Indication à droite (vide s'il n'y en a pas).
    pub hint: &'a str,
    /// L'indication est un raccourci clavier, à encadrer.
    pub shortcut: bool,
    /// Dessin de tête.
    pub glyph: PaletteGlyph,
    /// Titre de groupe à afficher avant la ligne.
    pub heading: Option<&'a str>,
}

/// Palette de commandes telle que l'interface l'affiche. Le texte saisi
/// est celui de [`ChromeState::address`].
#[derive(Clone, Copy, Debug)]
pub struct PaletteView<'a> {
    /// Lignes ; leur index est celui porté par [`Hit::PaletteRow`].
    pub rows: &'a [PaletteRow<'a>],
    /// Index de la ligne choisie au clavier.
    pub selected: usize,
    /// Noms des filtres ; leur index est celui porté par [`Hit::PaletteFilter`].
    pub filters: &'a [&'a str],
    /// Index du filtre actif.
    pub filter: usize,
}

/// Commande d'un réglage.
#[derive(Clone, Copy, Debug)]
pub enum SettingControl<'a> {
    /// Interrupteur.
    Switch {
        /// Activé.
        on: bool,
    },
    /// Choix parmi quelques libellés.
    Choice {
        /// Libellés.
        options: &'a [&'a str],
        /// Index du libellé choisi.
        selected: usize,
    },
    /// Pastilles de couleur.
    Swatches {
        /// Couleurs.
        colors: &'a [Color],
        /// Nom accessible de chaque couleur.
        names: &'a [&'a str],
        /// Index de la couleur choisie.
        selected: usize,
    },
    /// Bouton d'action.
    Button {
        /// Libellé.
        label: &'a str,
        /// Action destructrice qui attend sa confirmation.
        danger: bool,
    },
    /// Valeur en lecture seule.
    Value {
        /// Texte.
        text: &'a str,
    },
}

/// Réglage : un titre, une explication, une commande.
#[derive(Clone, Copy, Debug)]
pub struct SettingRow<'a> {
    /// Titre.
    pub title: &'a str,
    /// Explication (vide s'il n'y en a pas).
    pub description: &'a str,
    /// Commande.
    pub control: SettingControl<'a>,
}

/// Rubrique des paramètres.
#[derive(Clone, Debug)]
pub struct SettingsGroup<'a> {
    /// Titre.
    pub title: &'a str,
    /// Réglages.
    pub rows: Vec<SettingRow<'a>>,
}

/// Paramètres tels que l'interface les affiche.
#[derive(Clone, Copy, Debug)]
pub struct SettingsView<'a> {
    /// Rubriques ; l'index d'un réglage, porté par [`Hit::Setting`], compte
    /// à travers elles.
    pub groups: &'a [SettingsGroup<'a>],
    /// Rubrique mise en avant dans la colonne de gauche.
    pub selected: usize,
    /// Défilement, en pixels (borné par [`Chrome::library_scroll_max`]).
    pub scroll: f32,
    /// Rubrique à amener en haut du panneau ; le défilement retenu est
    /// rendu par [`Chrome::settings_scroll`].
    pub jump: Option<usize>,
}

/// Rubrique de la bibliothèque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibrarySection {
    /// Pages visitées.
    History,
    /// Pages mises en favori.
    Bookmarks,
}

/// Ligne de la bibliothèque.
#[derive(Clone, Copy, Debug)]
pub struct LibraryRow<'a> {
    /// Titre de la page.
    pub title: &'a str,
    /// Nom d'hôte, pour la pastille et la colonne de droite.
    pub host: &'a str,
    /// Heure de la visite (historique) ou adresse (favoris).
    pub detail: &'a str,
    /// Titre de groupe à afficher avant la ligne (« Aujourd'hui »).
    pub heading: Option<&'a str>,
}

/// Bibliothèque telle que l'interface l'affiche. Le texte de recherche est
/// celui de [`ChromeState::address`].
#[derive(Clone, Copy, Debug)]
pub struct LibraryView<'a> {
    /// Rubrique affichée.
    pub section: LibrarySection,
    /// Lignes ; leur index est celui porté par [`Hit::LibraryRow`].
    pub rows: &'a [LibraryRow<'a>],
    /// Défilement de la liste, en pixels (borné par [`Chrome::library_scroll_max`]).
    pub scroll: f32,
    /// L'effacement de l'historique attend sa confirmation.
    pub confirm_clear: bool,
    /// D'autres lignes existent, qui ne sont pas affichées.
    pub truncated: bool,
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
    /// Le processus de rendu de l'onglet s'est arrêté : « Recharger la
    /// page » et « Fermer l'onglet ».
    Crashed,
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
    /// Texte de la barre d'adresse, ou texte en cours de saisie (adresse,
    /// recherche dans l'historique, palette de commandes).
    pub address: &'a str,
    /// Adresse de la page active, affichée dans la barre d'adresse quand la
    /// saisie se fait ailleurs.
    pub page_address: &'a str,
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
    /// Bibliothèque ouverte à la place des onglets.
    pub library: Option<LibraryView<'a>>,
    /// Paramètres ouverts à la place des onglets.
    pub settings: Option<SettingsView<'a>>,
    /// Palette de commandes ouverte par-dessus les pages.
    pub palette: Option<PaletteView<'a>>,
    /// Barre de recherche dans la page active.
    pub find: Option<FindView<'a>>,
    /// Premier lancement, affiché à la place des onglets.
    pub onboarding: Option<OnboardingView<'a>>,
    /// Couleur d'accent.
    pub accent: Color,
    /// Fenêtre de navigation privée.
    pub private: bool,
    /// Onglets horizontaux, dans la barre de titre, plutôt que la barre latérale.
    pub horizontal: bool,
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
    /// Interrupteur (`selected` : activé).
    Switch,
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
    /// Défilement maximal de la bibliothèque ou des paramètres (nul si
    /// l'écran tient dans la fenêtre).
    pub library_scroll_max: f32,
    /// Défilement retenu par les paramètres quand l'état demandait de
    /// sauter à une rubrique.
    pub settings_scroll: Option<f32>,
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
    /// Couleur d'accent.
    pub(crate) accent: Color,
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
    pub(crate) fn icon_button(
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

pub(crate) const ICON_BACK: &str = "M10 3 5 8l5 5";
pub(crate) const ICON_FORWARD: &str = "M6 3l5 5-5 5";
pub(crate) const ICON_RELOAD: &str = "M13 8a5 5 0 1 1-1.5-3.5M13 3v2.5h-2.5";
pub(crate) const ICON_SPLIT: &str =
    "M4 3h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM8 3v10";
pub(crate) const ICON_LOCK: &str = "M5 7h6a1.5 1.5 0 0 1 1.5 1.5v3.5a1.5 1.5 0 0 1-1.5 1.5H5a1.5 1.5 0 0 1-1.5-1.5V8.5A1.5 1.5 0 0 1 5 7zM5.5 7V5a2.5 2.5 0 0 1 5 0v2";
pub(crate) const ICON_GLOBE: &str =
    "M8 2.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11zM2.5 8h11M8 2.5c-2 2-2 9 0 11M8 2.5c2 2 2 9 0 11";
pub(crate) const ICON_CLOSE: &str = "M4 4l8 8M12 4l-8 8";
pub(crate) const ICON_PLUS: &str = "M8 3v10M3 8h10";
pub(crate) const ICON_SETTINGS: &str = "M3 5h5.5M12.5 5h.5M3 11h1M7.5 11H13M12.25 5a1.75 1.75 0 1 1-3.5 0 1.75 1.75 0 0 1 3.5 0zM7.25 11a1.75 1.75 0 1 1-3.5 0 1.75 1.75 0 0 1 3.5 0z";
pub(crate) const ICON_WIN_MIN: &str = "M1.5 6h9";
pub(crate) const ICON_WIN_MAX: &str =
    "M2.5 1.5h7a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1z";
pub(crate) const ICON_WIN_CLOSE: &str = "M1.5 1.5l9 9M10.5 1.5l-9 9";
pub(crate) const ICON_LOGO: &str = "M9 2.5a6.5 6.5 0 1 0 6.5 6.5";

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
    // La palette ou la barre de recherche a la saisie : dessous, la fenêtre
    // s'affiche comme au repos.
    let typed = state;
    let resting;
    let state = if typed.palette.is_some() || typed.find.is_some_and(|find| find.focused) {
        resting = ChromeState {
            editing: false,
            address: typed.page_address,
            ..typed.clone()
        };
        &resting
    } else {
        typed
    };
    let mut p = Painter {
        text,
        list: DisplayList::default(),
        hits: Vec::new(),
        widgets: Vec::new(),
        hover: state.hover,
        accent: state.accent,
    };
    let (w, h) = (state.width, state.height);
    p.fill(rect(0.0, 0.0, w, h), theme::WINDOW, 0.0);

    // ---- Barre de titre ----------------------------------------------------
    // Les onglets horizontaux vivent dans la barre de titre ; les écrans qui
    // remplacent les onglets gardent la barre ordinaire.
    let horizontal = state.horizontal
        && state.library.is_none()
        && state.settings.is_none()
        && state.onboarding.is_none();
    let title_h = theme::TITLE_BAR;
    if horizontal {
        horizontal::header(&mut p, state);
    } else {
        p.stroke_path(ICON_LOGO, 16.0, (title_h - 18.0) / 2.0, p.accent, 2.5);
        p.dot(16.0 + 13.6, (title_h - 18.0) / 2.0 + 4.4, 1.6, theme::AMBER);
        let name = Label::new("Oxyde", 13.0, theme::TEXT)
            .weight(600.0)
            .family(DISPLAY_FONT);
        let name_w = p.label(&name, 44.0, 0.0, title_h, 120.0);
        if state.private {
            let text = "Navigation privée";
            let label = Label::new(text, 12.0, theme::PRIVATE_CHIP_TEXT);
            let chip = rect(
                44.0 + name_w + 10.0,
                (title_h - 22.0) / 2.0,
                p.measure(&label) + 20.0,
                22.0,
            );
            p.fill(chip, theme::PRIVATE_BORDER, 10.0);
            p.label_centered(&label, chip);
            p.describe(chip, Role::Text, text);
        }
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
    }

    // ---- Bibliothèque : elle remplace les onglets et les volets -------------
    if let Some(view) = &state.library {
        let library_scroll_max = library::build(&mut p, state, view);
        return Chrome {
            list: p.list,
            hits: p.hits,
            widgets: p.widgets,
            library_scroll_max,
            ..Chrome::default()
        };
    }
    if let Some(view) = &state.onboarding {
        onboarding::build(&mut p, state, view);
        return Chrome {
            list: p.list,
            hits: p.hits,
            widgets: p.widgets,
            ..Chrome::default()
        };
    }
    if let Some(view) = &state.settings {
        let (library_scroll_max, settings_scroll) = settings::build(&mut p, state, view);
        return Chrome {
            list: p.list,
            hits: p.hits,
            widgets: p.widgets,
            library_scroll_max,
            settings_scroll,
            ..Chrome::default()
        };
    }

    // ---- Barre latérale ----------------------------------------------------
    let tab_scroll_max = if horizontal {
        horizontal::toolbar(&mut p, state);
        0.0
    } else {
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
            p.accent
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
                p.accent
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
        let caption = if state.private {
            "ONGLETS PRIVÉS".to_owned()
        } else {
            format!("ESPACE · {}", space_name.to_uppercase())
        };
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
        tab_scroll_max
    };

    // ---- Volets de contenu -------------------------------------------------
    let mut overlay = DisplayList::default();
    let mut web_panes = Vec::new();
    let area = if horizontal {
        let top = horizontal::HEADER + horizontal::TOOLBAR + 8.0;
        rect(8.0, top, (w - 16.0).max(0.0), (h - top - 8.0).max(0.0))
    } else {
        rect(
            8.0 + theme::SIDEBAR + 8.0,
            title_h,
            (w - theme::SIDEBAR - 24.0).max(0.0),
            (h - title_h - 8.0).max(0.0),
        )
    };
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
                    color: state.accent,
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

    // ---- Palette de commandes : par-dessus les pages, et avant tout le
    // reste pour les clics comme pour les technologies d'assistance.
    let Painter {
        text,
        list,
        mut hits,
        mut widgets,
        hover,
        accent,
    } = p;
    // La barre de recherche d'abord : la palette la recouvre.
    let mut front = Painter {
        text,
        list: DisplayList::default(),
        hits: Vec::new(),
        widgets: Vec::new(),
        hover,
        accent,
    };
    if let Some(view) = &typed.find {
        let pane = web_panes
            .iter()
            .find(|pane| pane.tab == state.active)
            .map_or(area, |pane| pane.rect);
        find::build(&mut front, typed, view, pane);
        overlay.items.append(&mut front.list.items);
        front.hits.append(&mut hits);
        hits = std::mem::take(&mut front.hits);
        front.widgets.append(&mut widgets);
        widgets = std::mem::take(&mut front.widgets);
    }
    if let Some(view) = &state.palette {
        palette::build(&mut front, typed, view);
        overlay.items.append(&mut front.list.items);
        front.hits.append(&mut hits);
        hits = front.hits;
        front.widgets.append(&mut widgets);
        widgets = front.widgets;
    }

    Chrome {
        list,
        overlay,
        hits,
        panes: web_panes,
        tab_scroll_max,
        widgets,
        library_scroll_max: 0.0,
        settings_scroll: None,
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
            page_address: "docs.rs/wgpu",
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
            library: None,
            settings: None,
            palette: None,
            find: None,
            onboarding: None,
            accent: theme::ACCENT,
            private: false,
            horizontal: false,
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
            panes::ICON_CRASHED,
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

        // Onglet planté : recharger, ou fermer l'onglet.
        tabs[0].content = PaneContent::Error(view(ErrorStyle::Crashed));
        let chrome = build_chrome(&state(&tabs, None), &mut text);
        let labels: Vec<&str> = chrome
            .widgets
            .iter()
            .filter(|w| matches!(w.hit, Some(Hit::ErrorPrimary(_) | Hit::ErrorSecondary(_))))
            .map(|w| w.label.as_str())
            .collect();
        assert_eq!(labels, ["Recharger la page", "Fermer l'onglet"]);
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

    fn library_rows() -> [LibraryRow<'static>; 2] {
        [
            LibraryRow {
                title: "wgpu — docs.rs",
                host: "docs.rs",
                detail: "14:32",
                heading: Some("Aujourd'hui"),
            },
            LibraryRow {
                title: "Cartes",
                host: "cartes.example",
                detail: "17:41",
                heading: Some("Hier"),
            },
        ]
    }

    fn find_hit(chrome: &Chrome, hit: Hit) -> Option<Rect> {
        chrome.hits.iter().find(|(_, h)| *h == hit).map(|(r, _)| *r)
    }

    #[test]
    fn library_replaces_tabs_and_lists_rows() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let rows = library_rows();
        let mut s = state(&tabs, None);
        s.address = "";
        s.editing = true;
        s.library = Some(LibraryView {
            section: LibrarySection::History,
            rows: &rows,
            scroll: 0.0,
            confirm_clear: false,
            truncated: false,
        });
        let chrome = build_chrome(&s, &mut text);
        assert!(chrome.panes.is_empty(), "aucune page sous la bibliothèque");
        assert!(find_hit(&chrome, Hit::Tab(0)).is_none());
        assert_eq!(chrome.library_scroll_max, 0.0);

        let row = find_hit(&chrome, Hit::LibraryRow(0)).expect("première ligne");
        assert_eq!(
            chrome.hit(row.x + 100.0, row.y + 10.0),
            Some(Hit::LibraryRow(0))
        );
        assert_eq!(chrome.hit(20.0, 60.0), Some(Hit::LibraryBack));
        for hit in [
            Hit::LibrarySection(LibrarySection::Bookmarks),
            Hit::LibrarySearch,
            Hit::LibraryClear,
            Hit::CloseWindow,
        ] {
            assert!(find_hit(&chrome, hit).is_some(), "{hit:?}");
        }
        assert!(
            chrome
                .widgets
                .iter()
                .any(|w| w.role == Role::Heading && w.label == "Historique")
        );
        // Le bouton de retrait n'apparaît que sur la ligne survolée, et prend le clic.
        assert!(find_hit(&chrome, Hit::LibraryRemove(0)).is_none());
        s.hover = Some(Hit::LibraryRow(0));
        let chrome = build_chrome(&s, &mut text);
        let remove = find_hit(&chrome, Hit::LibraryRemove(0)).expect("retrait");
        assert_eq!(
            chrome.hit(remove.x + 5.0, remove.y + 5.0),
            Some(Hit::LibraryRemove(0))
        );
    }

    #[test]
    fn long_library_scrolls_inside_its_panel() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let rows: Vec<LibraryRow<'_>> = (0..100)
            .map(|i| LibraryRow {
                title: "Page",
                host: "site.example",
                detail: "site.example/page",
                heading: (i == 0).then_some("Tous les favoris"),
            })
            .collect();
        let mut s = state(&tabs, None);
        let view = |scroll| LibraryView {
            section: LibrarySection::Bookmarks,
            rows: &rows,
            scroll,
            confirm_clear: false,
            truncated: false,
        };
        s.library = Some(view(0.0));
        let chrome = build_chrome(&s, &mut text);
        let max = chrome.library_scroll_max;
        assert!(max > 1000.0, "la liste dépasse la fenêtre : {max}");
        assert!(find_hit(&chrome, Hit::LibraryRow(99)).is_none());
        // Les zones cliquables ne débordent jamais du panneau.
        let inside = |chrome: &Chrome| {
            chrome
                .hits
                .iter()
                .filter(|(_, hit)| matches!(hit, Hit::LibraryRow(_)))
                .all(|(r, _)| r.y >= 40.0 && r.y + r.h <= 892.0)
        };
        assert!(inside(&chrome));
        // En bas, la dernière ligne est visible et la première ne l'est plus.
        s.library = Some(view(max));
        let chrome = build_chrome(&s, &mut text);
        assert!(find_hit(&chrome, Hit::LibraryRow(99)).is_some());
        assert!(find_hit(&chrome, Hit::LibraryRow(0)).is_none());
        assert!(inside(&chrome));
        assert_eq!(chrome.library_scroll_max, max);
    }

    #[test]
    fn settings_screen_exposes_its_controls() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let colors = [theme::ACCENT, theme::AMBER];
        let names = ["Vert", "Ambre"];
        let options = ["Un", "Deux", "Trois"];
        let row = |title, description, control| SettingRow {
            title,
            description,
            control,
        };
        let groups = vec![
            SettingsGroup {
                title: "Apparence",
                rows: vec![row(
                    "Couleur",
                    "",
                    SettingControl::Swatches {
                        colors: &colors,
                        names: &names,
                        selected: 1,
                    },
                )],
            },
            SettingsGroup {
                title: "Divers",
                rows: vec![
                    row(
                        "Choix",
                        "Explication",
                        SettingControl::Choice {
                            options: &options,
                            selected: 0,
                        },
                    ),
                    row("Bascule", "", SettingControl::Switch { on: true }),
                    row(
                        "Action",
                        "",
                        SettingControl::Button {
                            label: "Faire",
                            danger: false,
                        },
                    ),
                    row("Version", "", SettingControl::Value { text: "1.0" }),
                ],
            },
        ];
        let mut s = state(&tabs, None);
        s.settings = Some(SettingsView {
            groups: &groups,
            selected: 0,
            scroll: 0.0,
            jump: None,
        });
        let chrome = build_chrome(&s, &mut text);
        assert!(chrome.panes.is_empty());
        assert_eq!(chrome.settings_scroll, None);
        assert_eq!(chrome.library_scroll_max, 0.0);
        for hit in [
            Hit::LibraryBack,
            Hit::SettingsGroup(1),
            Hit::Setting { row: 0, option: 1 },
            Hit::Setting { row: 1, option: 2 },
            Hit::Setting { row: 2, option: 0 },
            Hit::Setting { row: 3, option: 0 },
        ] {
            assert!(find_hit(&chrome, hit).is_some(), "{hit:?}");
        }
        // Une valeur en lecture seule n'est pas cliquable.
        assert!(find_hit(&chrome, Hit::Setting { row: 4, option: 0 }).is_none());
        let switch = chrome
            .widgets
            .iter()
            .find(|w| w.role == Role::Switch)
            .expect("interrupteur");
        assert!(switch.selected);
        assert_eq!(switch.label, "Bascule");
        assert!(switch.rect.h >= 44.0, "cible tactile de 44 px");

        // Dans une fenêtre basse, sauter à une rubrique fait défiler jusqu'à elle.
        s.height = 300.0;
        s.settings = Some(SettingsView {
            groups: &groups,
            selected: 1,
            scroll: 0.0,
            jump: Some(1),
        });
        let chrome = build_chrome(&s, &mut text);
        let applied = chrome.settings_scroll.expect("saut");
        assert!(applied > 0.0 && applied <= chrome.library_scroll_max);
    }

    #[test]
    fn palette_sits_above_everything() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let row = |title, glyph, heading| PaletteRow {
            title,
            host: "docs.rs",
            hint: "Ctrl T",
            shortcut: glyph == PaletteGlyph::Command,
            glyph,
            heading,
        };
        let rows = [
            row(
                "wgpu — docs.rs",
                PaletteGlyph::Badge,
                Some("Onglets ouverts"),
            ),
            row("Nouvel onglet", PaletteGlyph::Command, Some("Commandes")),
            row("Rechercher « wg »", PaletteGlyph::Search, Some("Web")),
        ];
        let filters = ["Tout", "Onglets"];
        let mut s = state(&tabs, None);
        s.address = "wg";
        s.editing = true;
        s.caret = 2;
        s.palette = Some(PaletteView {
            rows: &rows,
            selected: 1,
            filters: &filters,
            filter: 0,
        });
        let chrome = build_chrome(&s, &mut text);
        // La page reste dessous, mais les clics vont à la palette.
        assert_eq!(chrome.panes.len(), 1);
        assert!(!chrome.overlay.items.is_empty());
        let target = find_hit(&chrome, Hit::PaletteRow(1)).expect("ligne");
        assert_eq!(
            chrome.hit(target.x + 5.0, target.y + 5.0),
            Some(Hit::PaletteRow(1))
        );
        assert_eq!(
            chrome.hit(target.x + 5.0, target.y - 20.0),
            Some(Hit::PaletteInput)
        );
        assert_eq!(chrome.hit(5.0, 5.0), Some(Hit::PaletteDismiss));
        assert_eq!(chrome.hit(1420.0, 880.0), Some(Hit::PaletteDismiss));
        assert!(find_hit(&chrome, Hit::PaletteFilter(1)).is_some());

        // Le champ de la palette vient en premier et porte le texte saisi ;
        // la barre d'adresse, dessous, garde l'adresse de la page.
        let first = chrome.widgets.first().expect("champ");
        assert_eq!(
            (first.role, first.value.as_deref()),
            (Role::TextInput, Some("wg"))
        );
        let address = chrome
            .widgets
            .iter()
            .find(|w| w.label == "Adresse")
            .expect("barre d'adresse");
        assert_eq!(address.value.as_deref(), Some("docs.rs/wgpu"));
        let chosen: Vec<&str> = chrome
            .widgets
            .iter()
            .filter(|w| matches!(w.hit, Some(Hit::PaletteRow(_))) && w.selected)
            .map(|w| w.label.as_str())
            .collect();
        assert_eq!(chosen, ["Nouvel onglet"]);

        // Dans une fenêtre basse, les lignes qui ne tiennent pas ne sont pas cliquables.
        s.height = 330.0;
        let chrome = build_chrome(&s, &mut text);
        assert!(find_hit(&chrome, Hit::PaletteRow(2)).is_none());
    }

    #[test]
    fn private_window_says_so() {
        let mut text = TextContext::new();
        let mut tabs = tabs();
        tabs[0].content = PaneContent::NewTab;
        let mut s = state(&tabs, None);
        let labels = |chrome: &Chrome| -> Vec<String> {
            chrome.widgets.iter().map(|w| w.label.clone()).collect()
        };
        let normal = labels(&build_chrome(&s, &mut text));
        assert!(!normal.iter().any(|label| label == "Navigation privée"));
        s.private = true;
        let chrome = build_chrome(&s, &mut text);
        let private = labels(&chrome);
        // La barre de titre et la page « Nouvel onglet » l'annoncent.
        assert_eq!(
            private
                .iter()
                .filter(|label| *label == "Navigation privée")
                .count(),
            2
        );
        assert!(
            private
                .iter()
                .any(|label| label == "Oxyde n'enregistre pas")
        );
        // Le champ de la page reste la barre d'adresse de l'onglet.
        let field = chrome
            .widgets
            .iter()
            .find(|w| w.role == Role::TextInput && w.label == "Rechercher ou saisir une adresse")
            .expect("champ de recherche");
        assert_eq!(field.hit, Some(Hit::Address));
    }

    #[test]
    fn find_bar_floats_over_the_active_pane() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let mut s = state(&tabs, None);
        s.find = Some(FindView {
            query: "wgpu",
            focused: false,
            count: 12,
            current: 3,
        });
        let chrome = build_chrome(&s, &mut text);
        let pane = chrome.panes[0].rect;
        let bar = find_hit(&chrome, Hit::FindNext).expect("suivante");
        // Dans le volet, en haut à droite, et devant lui pour les clics.
        assert!(bar.x > pane.x + pane.w / 2.0 && bar.y < pane.y + 60.0);
        assert_eq!(chrome.hit(bar.x + 4.0, bar.y + 4.0), Some(Hit::FindNext));
        for hit in [Hit::FindInput, Hit::FindPrevious, Hit::FindClose] {
            assert!(find_hit(&chrome, hit).is_some(), "{hit:?}");
        }
        assert_eq!(
            chrome.hit(pane.x + 20.0, pane.y + 200.0),
            Some(Hit::Pane(0))
        );
        let labels: Vec<&str> = chrome.widgets.iter().map(|w| w.label.as_str()).collect();
        assert!(labels.contains(&"3 sur 12"), "{labels:?}");
        // Sans la saisie, la barre d'adresse garde la sienne.
        let field = chrome
            .widgets
            .iter()
            .find(|w| w.label == "Rechercher dans la page")
            .expect("champ");
        assert_eq!(field.value.as_deref(), Some("wgpu"));

        // Avec la saisie, le texte tapé est celui de l'état ; rien de trouvé se dit.
        s.editing = true;
        s.address = "absent";
        s.find = Some(FindView {
            query: "wgpu",
            focused: true,
            count: 0,
            current: 0,
        });
        let chrome = build_chrome(&s, &mut text);
        let labels: Vec<&str> = chrome.widgets.iter().map(|w| w.label.as_str()).collect();
        assert!(labels.contains(&"Aucun résultat"), "{labels:?}");
        let address = chrome
            .widgets
            .iter()
            .find(|w| w.label == "Adresse")
            .expect("barre d'adresse");
        assert_eq!(address.value.as_deref(), Some("docs.rs/wgpu"));
    }

    #[test]
    fn onboarding_walks_through_its_two_steps() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let sources = ["Chrome", "Edge", "Ne rien importer"];
        let colors = [theme::ACCENT, theme::AMBER];
        let names = ["Vert-de-gris", "Ambre"];
        let view = |step, selected, imported| OnboardingView {
            step,
            sources: &sources,
            selected,
            imported,
            accents: &colors,
            accent_names: &names,
            accent: 1,
        };
        let labels = |chrome: &Chrome| -> Vec<String> {
            chrome.widgets.iter().map(|w| w.label.clone()).collect()
        };
        let mut s = state(&tabs, None);
        s.onboarding = Some(view(0, 1, None));
        let chrome = build_chrome(&s, &mut text);
        assert!(chrome.panes.is_empty() && find_hit(&chrome, Hit::Tab(0)).is_none());
        for hit in [
            Hit::OnboardingSource(0),
            Hit::OnboardingSource(2),
            Hit::OnboardingSkip,
            Hit::OnboardingNext,
            Hit::CloseWindow,
        ] {
            assert!(find_hit(&chrome, hit).is_some(), "{hit:?}");
        }
        let found = labels(&chrome);
        assert!(found.iter().any(|l| l == "Bienvenue dans Oxyde"));
        assert!(found.iter().any(|l| l == "Importer et continuer"));
        // « Ne rien importer » : le bouton ne promet plus d'import.
        s.onboarding = Some(view(0, 2, None));
        assert!(
            labels(&build_chrome(&s, &mut text))
                .iter()
                .any(|l| l == "Continuer")
        );

        // Seconde étape : le compte des favoris importés, puis les couleurs.
        s.onboarding = Some(view(1, 0, Some(12)));
        let chrome = build_chrome(&s, &mut text);
        let found = labels(&chrome);
        assert!(
            found.iter().any(|l| l == "12 favoris importés."),
            "{found:?}"
        );
        assert!(found.iter().any(|l| l == "Terminer"));
        assert!(find_hit(&chrome, Hit::OnboardingAccent(1)).is_some());
        assert!(find_hit(&chrome, Hit::OnboardingSkip).is_none());
        // Fenêtre étroite : la carte seule, toujours utilisable.
        s.width = 700.0;
        let chrome = build_chrome(&s, &mut text);
        assert!(find_hit(&chrome, Hit::OnboardingNext).is_some());
        assert!(!labels(&chrome).iter().any(|l| l == "Bienvenue dans Oxyde"));
    }

    #[test]
    fn horizontal_tabs_live_in_the_title_bar() {
        let mut text = TextContext::new();
        let tabs = tabs();
        let mut s = state(&tabs, None);
        s.horizontal = true;
        let chrome = build_chrome(&s, &mut text);
        // Les onglets sont dans la barre de titre, et leur clic passe avant
        // le déplacement de la fenêtre.
        let tab = find_hit(&chrome, Hit::Tab(0)).expect("onglet");
        assert!(tab.y + tab.h <= horizontal::HEADER + 0.5);
        assert_eq!(chrome.hit(tab.x + 60.0, tab.y + 20.0), Some(Hit::Tab(0)));
        assert_eq!(chrome.hit(20.0, 20.0), Some(Hit::TitleBar));
        for hit in [
            Hit::NewTab,
            Hit::Back,
            Hit::Reload,
            Hit::Address,
            Hit::Settings,
            Hit::Space(1),
            Hit::CloseWindow,
        ] {
            assert!(find_hit(&chrome, hit).is_some(), "{hit:?}");
        }
        // La vue scindée est au bout de la pilule d'adresse, et prend son clic.
        let split = find_hit(&chrome, Hit::SplitToggle).expect("vue scindée");
        assert_eq!(
            chrome.hit(split.x + 10.0, split.y + 10.0),
            Some(Hit::SplitToggle)
        );
        let address = find_hit(&chrome, Hit::Address).expect("adresse");
        assert_eq!(
            chrome.hit(address.x + 60.0, address.y + 10.0),
            Some(Hit::Address)
        );
        // La page prend toute la largeur, sous les deux barres.
        let pane = chrome.panes[0].rect;
        assert_eq!((pane.x, pane.w), (8.0, 1424.0));
        assert_eq!(pane.y, horizontal::HEADER + horizontal::TOOLBAR + 8.0);
        assert_eq!(chrome.tab_scroll_max, 0.0);

        // Les écrans qui remplacent les onglets gardent la barre de titre ordinaire.
        s.library = Some(LibraryView {
            section: LibrarySection::Bookmarks,
            rows: &[],
            scroll: 0.0,
            confirm_clear: false,
            truncated: false,
        });
        let chrome = build_chrome(&s, &mut text);
        assert!(find_hit(&chrome, Hit::Tab(0)).is_none());
        assert!(find_hit(&chrome, Hit::LibraryBack).is_some());
    }
}
