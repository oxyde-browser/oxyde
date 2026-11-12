//! Modèle du navigateur : onglets, espaces, historique, vue scindée, barre d'adresse.
//!
//! Ce module ne touche ni à la fenêtre ni au réseau : il transforme les
//! actions de l'utilisateur en [`Command`] que l'application exécute.
//!
//! Le modèle porte l'historique et les favoris, et sait se résumer en une
//! [`Session`] ou en renaître ; c'est l'application qui les écrit sur disque.

use oxyde_content::{Navigation, Url};
use oxyde_css::Color;
use oxyde_platform::CivilTime;
use oxyde_storage::{Bookmarks, History, Session, SessionTab, Settings};
use oxyde_ui::{
    ChromeState, ClosedView, ErrorStyle, ErrorView, FindView, Hit, LibraryRow, LibrarySection,
    LibraryView, OnboardingView, PaletteGlyph, PaletteRow, PaletteView, PaneContent,
    SettingControl, SettingRow, SettingsGroup, SettingsView, SpaceView, TabView, theme,
};

use crate::loader::{self, ErrorKind, LoadError, Loaded};
use crate::renderer::host::{PageHost, RemotePage};

/// Identifiant d'un document chargé : deux entrées d'historique qui le
/// partagent ne diffèrent que par `history.pushState` ou par leur fragment.
fn next_document() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Espaces proposés par la maquette.
const SPACES: [(&str, Color); 3] = [
    ("Travail", theme::ACCENT),
    ("Perso", theme::AMBER),
    ("Veille", theme::BLUE),
];

/// Nombre d'onglets fermés gardés en mémoire.
const MAX_CLOSED: usize = 10;

/// Réglage : couleur d'accent (`vert`, `ambre` ou `bleu`).
const KEY_ACCENT: &str = "apparence.accent";
/// Réglage : rouvrir les onglets de la dernière session au démarrage.
pub(crate) const KEY_RESTORE: &str = "session.restaurer";
/// Réglage : moteur de recherche (nom en minuscules).
const KEY_ENGINE: &str = "recherche.moteur";
/// Réglage : noter les pages visitées dans l'historique.
const KEY_HISTORY: &str = "historique.conserver";

/// Couleurs d'accent de la maquette : valeur du réglage, nom, couleur.
const ACCENT_KEYS: [&str; 3] = ["vert", "ambre", "bleu"];
const ACCENT_NAMES: [&str; 3] = ["Vert-de-gris", "Ambre", "Bleu"];
const ACCENT_COLORS: [Color; 3] = [theme::ACCENT, theme::AMBER, theme::BLUE];

/// Index des réglages dans l'écran Paramètres (voir [`Browser::settings_groups`]).
const ROW_ACCENT: usize = 0;
const ROW_RESTORE: usize = 1;
const ROW_ENGINE: usize = 2;
const ROW_HISTORY: usize = 3;
const ROW_CLEAR: usize = 4;

/// Filtres de la palette de commandes.
const PALETTE_FILTERS: [&str; 5] = ["Tout", "Onglets", "Historique", "Favoris", "Commandes"];
const FILTER_ALL: usize = 0;
const FILTER_TABS: usize = 1;
const FILTER_HISTORY: usize = 2;
const FILTER_BOOKMARKS: usize = 3;
const FILTER_COMMANDS: usize = 4;

/// Commande du navigateur proposée par la palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaletteCommand {
    NewTab,
    CloseTab,
    Reopen,
    Split,
    Pin,
    Reload,
    Bookmark,
    History,
    Bookmarks,
    Settings,
    ClearData,
    PrivateWindow,
}

/// Commandes de la palette : commande, nom, raccourci.
const PALETTE_COMMANDS: [(PaletteCommand, &str, &str); 12] = [
    (PaletteCommand::NewTab, "Nouvel onglet", "Ctrl T"),
    (PaletteCommand::Split, "Scinder la vue", "Ctrl Maj S"),
    (PaletteCommand::History, "Ouvrir l'historique", "Ctrl H"),
    (
        PaletteCommand::Bookmarks,
        "Ouvrir les favoris",
        "Ctrl Maj O",
    ),
    (PaletteCommand::Settings, "Ouvrir les paramètres", "Ctrl ,"),
    (
        PaletteCommand::Bookmark,
        "Ajouter la page aux favoris, ou l'en retirer",
        "Ctrl D",
    ),
    (
        PaletteCommand::Pin,
        "Épingler ou désépingler l'onglet",
        "Ctrl Maj P",
    ),
    (PaletteCommand::Reload, "Actualiser la page", "Ctrl R"),
    (PaletteCommand::CloseTab, "Fermer l'onglet", "Ctrl W"),
    (
        PaletteCommand::Reopen,
        "Rouvrir le dernier onglet fermé",
        "Ctrl Maj T",
    ),
    (
        PaletteCommand::PrivateWindow,
        "Nouvelle fenêtre privée",
        "Ctrl Maj N",
    ),
    (
        PaletteCommand::ClearData,
        "Effacer les données de navigation",
        "",
    ),
];

/// Ce que fait une ligne de la palette.
#[derive(Clone, Debug)]
enum PaletteAction {
    /// Aller à l'onglet d'identifiant donné.
    Tab(u64),
    /// Ouvrir cette adresse dans un nouvel onglet.
    Open(Url),
    /// Traiter ce texte comme une saisie dans la barre d'adresse.
    Input(String),
    Run(PaletteCommand),
}

/// Ligne de la palette de commandes.
struct PaletteItem {
    title: String,
    host: String,
    hint: String,
    shortcut: bool,
    glyph: PaletteGlyph,
    heading: Option<&'static str>,
    action: PaletteAction,
}

/// Palette de commandes ouverte par-dessus les pages. Le texte saisi est
/// celui de la saisie en cours ([`Browser::editing`]).
struct Palette {
    /// Index du filtre actif dans [`PALETTE_FILTERS`].
    filter: usize,
    /// Index de la ligne choisie. INVARIANT : nul, ou inférieur à `items.len()`.
    selected: usize,
    items: Vec<PaletteItem>,
}

/// Réglage : le premier lancement a été vu.
pub(crate) const KEY_WELCOME: &str = "accueil.fait";

/// Dernier choix de l'import, au premier lancement.
const NO_IMPORT: &str = "Ne rien importer";

/// Premier lancement, affiché à la place des onglets.
struct Onboarding {
    /// Étape en cours : 0, l'import ; 1, la couleur d'accent.
    step: usize,
    /// Navigateurs d'où importer : nom, fichier de favoris.
    sources: Vec<(String, std::path::PathBuf)>,
    /// Choix retenu ; `sources.len()` : ne rien importer.
    selected: usize,
    /// Nombre de favoris importés.
    imported: Option<usize>,
}

/// Barre de recherche dans la page d'un onglet.
struct FindBar {
    /// Identifiant de l'onglet où l'on cherche.
    tab: u64,
    /// Texte cherché.
    query: String,
    /// La barre a la saisie : [`Browser::editing`] porte alors son texte.
    focused: bool,
}

/// Écran Paramètres ouvert à la place des onglets.
struct SettingsScreen {
    /// Rubrique mise en avant.
    selected: usize,
    /// Défilement, en pixels.
    scroll: f32,
    /// Rubrique à amener en haut à la prochaine image.
    jump: Option<usize>,
    /// L'effacement de l'historique attend sa confirmation.
    confirm_clear: bool,
}

/// Nombre maximal de lignes affichées par la bibliothèque.
const MAX_LIBRARY_ROWS: usize = 500;

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Jour d'une date, sans l'heure.
fn day_of(time: CivilTime) -> (i32, u8, u8) {
    (time.year, time.month, time.day)
}

/// Nom du jour `day` pour un titre de groupe de l'historique.
fn day_label(day: (i32, u8, u8), today: (i32, u8, u8), yesterday: (i32, u8, u8)) -> String {
    if day == today {
        return "Aujourd'hui".to_owned();
    }
    if day == yesterday {
        return "Hier".to_owned();
    }
    let (year, month, number) = day;
    let name = MONTHS
        .get(usize::from(month).wrapping_sub(1))
        .copied()
        .unwrap_or("");
    let ordinal = if number == 1 { "er" } else { "" };
    format!("{number}{ordinal} {name} {year}")
}

/// Ligne de la bibliothèque.
struct LibraryEntry {
    url: String,
    title: String,
    host: String,
    /// Heure de la visite, ou adresse du favori.
    detail: String,
    /// Titre du groupe que la ligne ouvre.
    heading: Option<String>,
    /// Moment de la visite (nul pour un favori).
    time: u64,
}

/// Bibliothèque ouverte à la place des onglets. Dans l'historique, le texte
/// de recherche est celui de la saisie en cours ([`Browser::editing`]).
struct Library {
    section: LibrarySection,
    /// Défilement de la liste, en pixels.
    scroll: f32,
    /// L'effacement de l'historique attend sa confirmation.
    confirm_clear: bool,
    entries: Vec<LibraryEntry>,
    /// D'autres visites correspondent, qui ne sont pas affichées.
    truncated: bool,
}

/// Action à exécuter par l'application.
#[derive(Debug)]
pub(crate) enum Command {
    /// Charger `url` dans l'onglet `tab` (en arrière-plan) ; `post` : par la
    /// soumission d'un formulaire en `POST`.
    Load {
        tab: u64,
        url: Url,
        push_history: bool,
        post: Option<oxyde_content::FormPost>,
    },
    /// Réduire la fenêtre.
    Minimize,
    /// Agrandir ou restaurer la fenêtre.
    ToggleMaximize,
    /// Fermer la fenêtre.
    CloseWindow,
    /// Ouvrir l'outil de diagnostic réseau du système.
    NetworkDiagnostics,
    /// Ouvrir une fenêtre de navigation privée.
    PrivateWindow,
    /// Lire les favoris de ce fichier d'un autre navigateur, puis les
    /// remettre par [`Browser::finish_import`].
    ImportBookmarks(std::path::PathBuf),
}

/// Touche reconnue par le navigateur, indépendante de la bibliothèque de fenêtrage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyPress {
    /// Caractère(s) saisi(s).
    Text(String),
    /// Texte collé depuis le presse-papiers.
    Paste(String),
    Enter,
    /// Entrée accompagnée de Maj.
    ShiftEnter,
    Escape,
    Backspace,
    /// Lettre accompagnée de Ctrl (en minuscule).
    Ctrl(char),
    /// Lettre accompagnée de Ctrl et Maj (en minuscule).
    CtrlShift(char),
    CtrlTab,
    /// Tabulation seule.
    Tab,
    Up,
    Down,
    AltLeft,
    AltRight,
    F5,
    Delete,
    Left,
    Right,
    Home,
    End,
}

pub(crate) struct Tab {
    id: u64,
    pub(crate) page: PageHost,
    /// Chargement confié à un processus de rendu : faut-il, à son terme,
    /// ajouter une entrée à l'historique ?
    pending_push: Option<bool>,
    history: Vec<Url>,
    /// Document de chaque entrée de `history` (voir [`next_document`]).
    documents: Vec<u64>,
    /// Index de la page courante dans `history`.
    position: usize,
    loading: bool,
    title: String,
    host: String,
    pinned: bool,
    space: usize,
    error: Option<LoadError>,
    /// Détail technique de l'erreur affiché.
    show_details: bool,
    /// Onglet repris d'une session, pas encore chargé : il le sera quand
    /// il deviendra visible.
    asleep: bool,
}

impl Tab {
    fn new(id: u64, loaded: Loaded, space: usize) -> Self {
        let mut tab = Self {
            id,
            history: vec![loaded.page.url().clone()],
            documents: vec![next_document()],
            page: loaded.page.into(),
            pending_push: None,
            position: 0,
            loading: false,
            title: String::new(),
            host: String::new(),
            pinned: false,
            space,
            error: loaded.error,
            show_details: false,
            asleep: false,
        };
        tab.refresh_labels();
        tab
    }

    fn is_new_tab(&self) -> bool {
        self.error.is_none() && self.page.url().scheme() == "about"
    }

    fn refresh_labels(&mut self) {
        let url = self.page.url();
        self.host = url.host_str().unwrap_or("").to_owned();
        let title = self.page.title();
        self.title = if title.is_empty() {
            display_url(url)
        } else {
            title
        };
    }

    fn content(&self) -> PaneContent<'_> {
        match &self.error {
            Some(error) => PaneContent::Error(ErrorView {
                style: match error.kind {
                    ErrorKind::Network => ErrorStyle::Network,
                    ErrorKind::Certificate => ErrorStyle::Certificate,
                    ErrorKind::Other => ErrorStyle::Other,
                    ErrorKind::Crashed => ErrorStyle::Crashed,
                },
                title: &error.title,
                message: &error.message,
                code: &error.code,
                detail: self.show_details.then_some(error.detail.as_str()),
            }),
            None if self.is_new_tab() => PaneContent::NewTab,
            None => PaneContent::Page,
        }
    }
}

/// Onglet fermé, rouvrable.
struct Closed {
    url: Url,
    title: String,
    host: String,
    space: usize,
}

/// Adresse telle qu'affichée dans la barre : sans schéma ni barre finale.
fn display_url(url: &Url) -> String {
    match url.scheme() {
        "about" => String::new(),
        "http" | "https" => {
            let rest = &url[url::Position::BeforeHost..];
            rest.trim_end_matches('/').to_owned()
        }
        "data" => "data:…".to_owned(),
        _ => url.to_string(),
    }
}

/// Données empruntées au navigateur pour dessiner l'interface.
pub(crate) struct Views<'a> {
    tabs: Vec<TabView<'a>>,
    closed: Vec<ClosedView<'a>>,
    spaces: [SpaceView<'static>; SPACES.len()],
    library: Vec<LibraryRow<'a>>,
    onboarding: Vec<&'a str>,
    settings: Vec<SettingsGroup<'a>>,
    palette: Vec<PaletteRow<'a>>,
}

pub(crate) struct Browser {
    tabs: Vec<Tab>,
    active: usize,
    /// Index de l'onglet du second volet.
    split: Option<usize>,
    /// Texte en cours de saisie dans la barre d'adresse.
    editing: Option<String>,
    select_all: bool,
    /// Position du curseur dans le texte saisi, en octets.
    /// INVARIANT : toujours sur une frontière de caractère de `editing`.
    caret: usize,
    /// Défilement de la liste d'onglets, en pixels.
    pub(crate) tab_scroll: f32,
    pub(crate) hover: Option<Hit>,
    next_id: u64,
    /// Texte affiché dans la barre d'adresse hors saisie (cache).
    address: String,
    /// Espace courant.
    space: usize,
    /// Dernier onglet actif de chaque espace.
    space_active: [Option<u64>; SPACES.len()],
    /// Onglets fermés, du plus récent au plus ancien.
    closed: Vec<Closed>,
    /// Historique de navigation.
    pub(crate) history: History,
    /// Favoris.
    pub(crate) bookmarks: Bookmarks,
    /// Bibliothèque (historique, favoris), quand elle est ouverte.
    library: Option<Library>,
    /// Réglages.
    pub(crate) settings: Settings,
    /// Écran Paramètres, quand il est ouvert.
    settings_screen: Option<SettingsScreen>,
    /// Palette de commandes, quand elle est ouverte.
    palette: Option<Palette>,
    /// Fenêtre de navigation privée : rien n'y est noté dans l'historique.
    private: bool,
    /// Barre de recherche dans la page, quand elle est ouverte.
    find: Option<FindBar>,
    /// Premier lancement, tant qu'il n'est pas terminé.
    onboarding: Option<Onboarding>,
}

/// Secondes écoulées depuis l'époque Unix.
fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Vrai si les visites de `url` vont dans l'historique.
fn is_recorded(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https" | "file")
}

fn blank() -> Loaded {
    Loaded {
        page: loader::blank_page(),
        error: None,
    }
}

impl Browser {
    pub(crate) fn new(first: Loaded) -> Self {
        let mut browser = Self {
            tabs: vec![Tab::new(1, first, 0)],
            active: 0,
            split: None,
            editing: None,
            select_all: false,
            hover: None,
            next_id: 2,
            address: String::new(),
            space: 0,
            space_active: [None; SPACES.len()],
            closed: Vec::new(),
            caret: 0,
            tab_scroll: 0.0,
            history: History::default(),
            bookmarks: Bookmarks::default(),
            library: None,
            settings: Settings::default(),
            settings_screen: None,
            palette: None,
            private: false,
            find: None,
            onboarding: None,
        };
        browser.refresh_address();
        browser
    }

    /// Navigateur repris de `session` ; `None` si elle n'a aucun onglet.
    /// Les pages du web ne sont pas chargées : chaque onglet attend d'être
    /// visible (voir [`Browser::wake_visible`]).
    pub(crate) fn from_session(session: &Session) -> Option<Self> {
        let mut tabs = session.tabs.iter();
        let mut browser = Self::new(loader::placeholder(&tabs.next()?.url));
        for tab in tabs {
            browser.push_tab(loader::placeholder(&tab.url), 0);
        }
        for (tab, saved) in browser.tabs.iter_mut().zip(&session.tabs) {
            tab.pinned = saved.pinned;
            tab.space = saved.space.min(SPACES.len() - 1);
            tab.asleep = tab.error.is_none() && crate::renderer::host::is_remote(tab.page.url());
            if tab.asleep && !saved.title.is_empty() {
                tab.title.clone_from(&saved.title);
            }
        }
        browser.active = session.active.min(browser.tabs.len() - 1);
        browser.split = session
            .split
            .filter(|&split| split < browser.tabs.len() && split != browser.active);
        // L'espace courant est celui de l'onglet actif, sauf s'il est épinglé.
        browser.space = match browser.tabs.get(browser.active) {
            Some(tab) if !tab.pinned => tab.space,
            _ => session.space.min(SPACES.len() - 1),
        };
        for saved in session.closed.iter().take(MAX_CLOSED) {
            if let Ok(url) = Url::parse(&saved.url) {
                browser.closed.push(Closed {
                    host: url.host_str().unwrap_or("").to_owned(),
                    url,
                    title: saved.title.clone(),
                    space: saved.space.min(SPACES.len() - 1),
                });
            }
        }
        browser.refresh_address();
        Some(browser)
    }

    /// Ouvre `loaded` dans un nouvel onglet de l'espace courant et l'active.
    pub(crate) fn open_active(&mut self, loaded: Loaded) {
        self.split = None;
        self.active = self.push_tab(loaded, self.space);
        self.refresh_address();
    }

    /// Résumé des onglets, à écrire sur disque. Les onglets sans adresse
    /// durable (nouvel onglet, page d'erreur sans adresse, `data:`) n'y
    /// figurent pas.
    pub(crate) fn session(&self) -> Session {
        let durable = |url: &Url| is_recorded(url);
        let mut session = Session {
            space: self.space,
            ..Session::default()
        };
        for (index, tab) in self.tabs.iter().enumerate() {
            let url = tab
                .history
                .get(tab.position)
                .unwrap_or_else(|| tab.page.url());
            if !durable(url) {
                continue;
            }
            if index == self.active {
                session.active = session.tabs.len();
            }
            if self.split == Some(index) {
                session.split = Some(session.tabs.len());
            }
            session.tabs.push(SessionTab {
                url: url.to_string(),
                title: tab.title.clone(),
                pinned: tab.pinned,
                space: tab.space,
            });
        }
        for closed in &self.closed {
            if durable(&closed.url) {
                session.closed.push(SessionTab {
                    url: closed.url.to_string(),
                    title: closed.title.clone(),
                    pinned: false,
                    space: closed.space,
                });
            }
        }
        session
    }

    /// Chargements des onglets repris d'une session qui viennent de devenir
    /// visibles (onglet actif, second volet).
    pub(crate) fn wake_visible(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        for index in [Some(self.active), self.split].into_iter().flatten() {
            let Some(tab) = self.tabs.get_mut(index) else {
                continue;
            };
            if std::mem::take(&mut tab.asleep) {
                let url = tab.page.url().clone();
                commands.extend(self.load(index, url, false));
            }
        }
        commands
    }

    /// Note dans l'historique la page que l'onglet `index` vient d'ouvrir.
    fn record_visit(&mut self, index: usize) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        if tab.error.is_none()
            && is_recorded(tab.page.url())
            && !self.private
            && self.settings.flag(KEY_HISTORY, true)
        {
            self.history
                .record(tab.page.url().as_str(), &tab.page.title(), now_seconds());
        }
    }

    /// Ajoute la page de l'onglet actif aux favoris, ou l'en retire.
    fn toggle_bookmark(&mut self) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        if tab.error.is_none() && is_recorded(tab.page.url()) {
            self.bookmarks
                .toggle(tab.page.url().as_str(), &tab.title, now_seconds());
        }
    }

    fn refresh_address(&mut self) {
        self.address = self
            .tabs
            .get(self.active)
            .map(|t| display_url(t.page.url()))
            .unwrap_or_default();
    }

    fn push_tab(&mut self, loaded: Loaded, space: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab::new(id, loaded, space));
        self.tabs.len() - 1
    }

    /// Ouvre `loaded` dans un nouvel onglet affiché dans le second volet.
    pub(crate) fn open_in_split(&mut self, loaded: Loaded) {
        let index = self.push_tab(loaded, self.space);
        self.split = Some(index);
    }

    /// Ouvre `loaded` dans un nouvel onglet épinglé, sans l'activer.
    pub(crate) fn open_pinned(&mut self, loaded: Loaded) {
        let index = self.push_tab(loaded, self.space);
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.pinned = true;
        }
    }

    pub(crate) fn tab_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    pub(crate) fn tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    /// Fait de cette fenêtre une fenêtre de navigation privée.
    pub(crate) fn set_private(&mut self) {
        self.private = true;
    }

    /// Vrai pour une fenêtre de navigation privée.
    pub(crate) fn is_private(&self) -> bool {
        self.private
    }

    /// Titre de la fenêtre pour le système.
    pub(crate) fn window_title(&self) -> String {
        let name = if self.private {
            "Oxyde (navigation privée)"
        } else {
            "Oxyde"
        };
        match self.tabs.get(self.active) {
            Some(tab) if !tab.title.is_empty() => format!("{} — {name}", tab.title),
            _ => name.to_owned(),
        }
    }

    /// Données d'affichage empruntées au modèle.
    pub(crate) fn views(&self) -> Views<'_> {
        Views {
            tabs: self
                .tabs
                .iter()
                .map(|t| TabView {
                    title: &t.title,
                    host: &t.host,
                    loading: t.loading,
                    pinned: t.pinned,
                    space: t.space,
                    content: t.content(),
                })
                .collect(),
            closed: self
                .closed
                .iter()
                .map(|c| ClosedView {
                    title: &c.title,
                    host: &c.host,
                })
                .collect(),
            spaces: SPACES.map(|(name, color)| SpaceView { name, color }),
            library: self
                .library
                .iter()
                .flat_map(|library| &library.entries)
                .map(|entry| LibraryRow {
                    title: &entry.title,
                    host: &entry.host,
                    detail: &entry.detail,
                    heading: entry.heading.as_deref(),
                })
                .collect(),
            onboarding: self
                .onboarding
                .iter()
                .flat_map(|onboarding| &onboarding.sources)
                .map(|(name, _)| name.as_str())
                .chain(self.onboarding.iter().map(|_| NO_IMPORT))
                .collect(),
            settings: if self.settings_screen.is_some() {
                self.settings_groups()
            } else {
                Vec::new()
            },
            palette: self
                .palette
                .iter()
                .flat_map(|palette| &palette.items)
                .map(|item| PaletteRow {
                    title: &item.title,
                    host: &item.host,
                    hint: &item.hint,
                    shortcut: item.shortcut,
                    glyph: item.glyph,
                    heading: item.heading,
                })
                .collect(),
        }
    }

    /// Ouvre la barre de recherche dans la page active, ou lui rend la
    /// saisie. Sans effet sur une page interne (nouvel onglet, erreur).
    fn open_find(&mut self) {
        let Some(tab) = self.tabs.get(self.active) else {
            return;
        };
        if tab.error.is_some() || tab.is_new_tab() {
            return;
        }
        let id = tab.id;
        let query = match &self.find {
            Some(bar) if bar.tab == id => bar.query.clone(),
            Some(_) => {
                // La barre était sur un autre onglet : elle s'y ferme.
                self.close_find();
                String::new()
            }
            None => String::new(),
        };
        self.caret = query.len();
        self.select_all = !query.is_empty();
        self.editing = Some(query.clone());
        self.find = Some(FindBar {
            tab: id,
            query,
            focused: true,
        });
    }

    /// Ouvre la barre de recherche sur `text` (ligne de commande).
    pub(crate) fn find_text(&mut self, text: &str) {
        self.open_find();
        if self.find.is_some() {
            self.editing = Some(text.to_owned());
            self.caret = text.len();
            self.select_all = false;
            self.requery_find();
        }
    }

    /// Avancement de la recherche affichée par la barre, si elle est ouverte.
    pub(crate) fn find_status(&self) -> Option<oxyde_content::FindStatus> {
        let bar = self.find.as_ref()?;
        let tab = self.tabs.iter().find(|tab| tab.id == bar.tab)?;
        Some(tab.page.find_status())
    }

    /// Ferme la barre de recherche et efface le surlignage.
    fn close_find(&mut self) {
        let Some(bar) = self.find.take() else {
            return;
        };
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == bar.tab) {
            tab.page.find("");
        }
        if bar.focused {
            self.editing = None;
            self.refresh_address();
        }
    }

    /// La barre de recherche perd la saisie ; elle reste affichée.
    fn blur_find(&mut self) {
        if let Some(bar) = &mut self.find
            && std::mem::take(&mut bar.focused)
        {
            self.editing = None;
            self.refresh_address();
        }
    }

    /// Le texte de la barre de recherche a peut-être changé : la page
    /// cherche le nouveau.
    fn requery_find(&mut self) {
        let Some(bar) = &mut self.find else {
            return;
        };
        let query = self.editing.clone().unwrap_or_default();
        if query == bar.query {
            return;
        }
        bar.query = query;
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == bar.tab) {
            tab.page.find(&bar.query);
        }
    }

    /// Passe à l'occurrence suivante ou précédente.
    fn step_find(&mut self, forward: bool) {
        if let Some(bar) = &self.find
            && let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == bar.tab)
        {
            tab.page.find_step(forward);
        }
    }

    /// Clic sur la barre de recherche ; `false` : le clic vise autre chose.
    fn find_click(&mut self, hit: Hit) -> bool {
        match hit {
            Hit::FindInput => {
                if let Some(bar) = &mut self.find
                    && !bar.focused
                {
                    bar.focused = true;
                    self.caret = bar.query.len();
                    self.select_all = false;
                    self.editing = Some(bar.query.clone());
                }
            }
            Hit::FindNext => self.step_find(true),
            Hit::FindPrevious => self.step_find(false),
            Hit::FindClose => self.close_find(),
            _ => return false,
        }
        true
    }

    /// Touche pendant que la barre de recherche est ouverte ; `false` : la
    /// touche suit son cours habituel.
    fn find_key(&mut self, key: &KeyPress) -> bool {
        let focused = self.find.as_ref().is_some_and(|bar| bar.focused);
        match key {
            KeyPress::Ctrl('f') => self.open_find(),
            // Échap annule d'abord une saisie d'adresse en cours.
            KeyPress::Escape if focused || self.editing.is_none() => self.close_find(),
            KeyPress::Enter if focused => self.step_find(true),
            KeyPress::ShiftEnter if focused => self.step_find(false),
            KeyPress::Ctrl('a') if focused => self.select_all = true,
            KeyPress::Text(typed) | KeyPress::Paste(typed) if focused => {
                self.insert_text(typed);
                self.requery_find();
            }
            KeyPress::Backspace
            | KeyPress::Delete
            | KeyPress::Left
            | KeyPress::Right
            | KeyPress::Home
            | KeyPress::End
                if focused =>
            {
                self.edit_key(key);
                self.requery_find();
            }
            KeyPress::Tab | KeyPress::Up | KeyPress::Down if focused => {}
            // Tout autre raccourci agit comme d'habitude ; la barre reste
            // ouverte tant que son onglet est affiché.
            _ => {
                self.blur_find();
                return false;
            }
        }
        true
    }

    /// Vrai si la palette de commandes est ouverte.
    pub(crate) fn palette_open(&self) -> bool {
        self.palette.is_some()
    }

    /// Ouvre la palette de commandes.
    pub(crate) fn open_palette(&mut self) {
        self.library = None;
        self.settings_screen = None;
        self.palette = Some(Palette {
            filter: FILTER_ALL,
            selected: 0,
            items: Vec::new(),
        });
        self.editing = Some(String::new());
        self.caret = 0;
        self.select_all = false;
        self.refresh_palette();
    }

    fn close_palette(&mut self) {
        self.palette = None;
        self.editing = None;
        self.refresh_address();
    }

    /// Recalcule les lignes de la palette d'après le texte saisi et le filtre.
    fn refresh_palette(&mut self) {
        let Some(filter) = self.palette.as_ref().map(|palette| palette.filter) else {
            return;
        };
        let query = self.editing.as_deref().unwrap_or("").trim().to_owned();
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let matches = |texts: &[&str]| {
            words.iter().all(|word| {
                texts
                    .iter()
                    .any(|text| text.to_lowercase().contains(word.as_str()))
            })
        };
        let wants = |kind: usize| filter == FILTER_ALL || filter == kind;
        // Dans « Tout », chaque sorte ne montre que ses premiers résultats.
        let limit = |few: usize| if filter == FILTER_ALL { few } else { 10 };
        let host_of = |url: &Url| url.host_str().unwrap_or("").to_owned();
        let mut items: Vec<PaletteItem> = Vec::new();
        let push = |items: &mut Vec<PaletteItem>, heading: &'static str, mut item: PaletteItem| {
            let first = items.last().is_none_or(|last: &PaletteItem| {
                std::mem::discriminant(&last.action) != std::mem::discriminant(&item.action)
            });
            item.heading = first.then_some(heading);
            items.push(item);
        };

        if wants(FILTER_TABS) {
            let visible = self
                .tabs
                .iter()
                .filter(|tab| tab.pinned || tab.space == self.space)
                .filter(|tab| matches(&[&tab.title, tab.page.url().as_str()]))
                .take(limit(5));
            for tab in visible {
                let title = if tab.is_new_tab() {
                    "Nouvel onglet".to_owned()
                } else {
                    tab.title.clone()
                };
                push(
                    &mut items,
                    "Onglets ouverts",
                    PaletteItem {
                        title,
                        host: tab.host.clone(),
                        hint: "Aller à l'onglet".to_owned(),
                        shortcut: false,
                        glyph: PaletteGlyph::Badge,
                        heading: None,
                        action: PaletteAction::Tab(tab.id),
                    },
                );
            }
        }
        // L'historique et les favoris n'apparaissent dans « Tout » qu'une
        // fois la recherche commencée.
        let searching = !words.is_empty();
        let mut pages: Vec<(&'static str, &str, &str)> = Vec::new();
        if filter == FILTER_HISTORY || (filter == FILTER_ALL && searching) {
            let mut seen: Vec<&str> = Vec::new();
            for visit in self.history.search(&query) {
                if seen.len() == limit(4) {
                    break;
                }
                if !seen.contains(&visit.url.as_str()) {
                    seen.push(&visit.url);
                    pages.push(("Historique", &visit.url, &visit.title));
                }
            }
        }
        let history_rows = pages.len();
        if filter == FILTER_BOOKMARKS || (filter == FILTER_ALL && searching) {
            let found = self
                .bookmarks
                .items()
                .iter()
                .filter(|item| matches(&[&item.title, &item.url]))
                .take(limit(4));
            for item in found {
                pages.push(("Favoris", &item.url, &item.title));
            }
        }
        for (index, (heading, address, title)) in pages.into_iter().enumerate() {
            let Ok(url) = Url::parse(address) else {
                continue;
            };
            let mut item = PaletteItem {
                title: if title.is_empty() {
                    display_url(&url)
                } else {
                    title.to_owned()
                },
                host: host_of(&url),
                hint: host_of(&url),
                shortcut: false,
                glyph: PaletteGlyph::Badge,
                heading: None,
                action: PaletteAction::Open(url),
            };
            // Les favoris suivent l'historique : leur titre de groupe ne
            // peut pas se déduire de la sorte d'action, qui est la même.
            let starts_group = index == 0 || index == history_rows;
            item.heading = starts_group.then_some(heading);
            items.push(item);
        }
        if wants(FILTER_COMMANDS) {
            let few = if searching { 6 } else { 5 };
            let found = PALETTE_COMMANDS
                .iter()
                .filter(|(_, name, _)| matches(&[name]))
                .take(if filter == FILTER_ALL {
                    few
                } else {
                    PALETTE_COMMANDS.len()
                });
            for (command, name, keys) in found {
                push(
                    &mut items,
                    "Commandes",
                    PaletteItem {
                        title: (*name).to_owned(),
                        host: String::new(),
                        hint: (*keys).to_owned(),
                        shortcut: true,
                        glyph: PaletteGlyph::Command,
                        heading: None,
                        action: PaletteAction::Run(*command),
                    },
                );
            }
        }
        // Dernière ligne : le texte saisi, comme dans la barre d'adresse.
        if filter == FILTER_ALL && searching {
            let engine = loader::SEARCH_ENGINE_NAMES
                .get(self.engine_index())
                .copied()
                .unwrap_or("");
            let search = loader::resolve_input_with(&query, usize::MAX)
                .ok()
                .is_none_or(|url| {
                    url.query_pairs()
                        .any(|(key, value)| key == "q" && value == query.as_str())
                });
            let (title, glyph, hint) = if search {
                (
                    format!("Rechercher « {query} »"),
                    PaletteGlyph::Search,
                    engine.to_owned(),
                )
            } else {
                (format!("Ouvrir {query}"), PaletteGlyph::Page, String::new())
            };
            push(
                &mut items,
                "Web",
                PaletteItem {
                    title,
                    host: String::new(),
                    hint,
                    shortcut: false,
                    glyph,
                    heading: None,
                    action: PaletteAction::Input(query.clone()),
                },
            );
        }
        if let Some(palette) = &mut self.palette {
            palette.selected = palette.selected.min(items.len().saturating_sub(1));
            palette.items = items;
        }
    }

    /// Exécute la ligne `index` de la palette, qui se ferme.
    fn activate_palette(&mut self, index: usize) -> Vec<Command> {
        let action = self
            .palette
            .as_ref()
            .and_then(|palette| palette.items.get(index))
            .map(|item| item.action.clone());
        let Some(action) = action else {
            return Vec::new();
        };
        self.close_palette();
        match action {
            PaletteAction::Tab(id) => {
                if let Some(position) = self.tabs.iter().position(|tab| tab.id == id) {
                    self.activate(position);
                }
                Vec::new()
            }
            PaletteAction::Open(url) => self.open_in_tab(url),
            PaletteAction::Input(text) => {
                self.editing = Some(text);
                self.key(KeyPress::Enter)
            }
            PaletteAction::Run(command) => self.run_command(command),
        }
    }

    fn run_command(&mut self, command: PaletteCommand) -> Vec<Command> {
        match command {
            PaletteCommand::NewTab => self.new_tab(),
            PaletteCommand::CloseTab => return self.close_tab(self.active),
            PaletteCommand::Reopen => return self.reopen(0),
            PaletteCommand::Split => self.toggle_split(),
            PaletteCommand::Pin => self.toggle_pin(),
            PaletteCommand::Reload => return self.reload(self.active),
            PaletteCommand::Bookmark => self.toggle_bookmark(),
            PaletteCommand::History => self.open_library(LibrarySection::History),
            PaletteCommand::Bookmarks => self.open_library(LibrarySection::Bookmarks),
            PaletteCommand::Settings => self.open_settings(),
            PaletteCommand::PrivateWindow => return vec![Command::PrivateWindow],
            PaletteCommand::ClearData => {
                // Le bouton d'effacement est dans la rubrique de la confidentialité.
                self.open_settings();
                if let Some(screen) = &mut self.settings_screen {
                    screen.selected = 3;
                    screen.jump = Some(3);
                }
            }
        }
        Vec::new()
    }

    /// Ouvre `url` dans l'onglet actif s'il est vide, sinon dans un nouvel onglet.
    fn open_in_tab(&mut self, url: Url) -> Vec<Command> {
        if !self.tabs.get(self.active).is_some_and(Tab::is_new_tab) {
            self.split = None;
            self.active = self.push_tab(blank(), self.space);
        }
        self.load(self.active, url, true)
    }

    /// Clic pendant que la palette est ouverte.
    fn palette_click(&mut self, hit: Hit) -> Vec<Command> {
        match hit {
            Hit::PaletteRow(index) => return self.activate_palette(index),
            Hit::PaletteFilter(filter) => {
                if let Some(palette) = &mut self.palette {
                    palette.filter = filter.min(PALETTE_FILTERS.len() - 1);
                    palette.selected = 0;
                }
                self.refresh_palette();
            }
            Hit::PaletteInput => {}
            _ => self.close_palette(),
        }
        Vec::new()
    }

    /// Touche pendant que la palette est ouverte ; `None` : la touche suit
    /// son cours habituel.
    fn palette_key(&mut self, key: &KeyPress) -> Option<Vec<Command>> {
        let count = self
            .palette
            .as_ref()
            .map_or(0, |palette| palette.items.len());
        match key {
            KeyPress::Escape | KeyPress::Ctrl('k') => self.close_palette(),
            KeyPress::Down | KeyPress::Up if count > 0 => {
                if let Some(palette) = &mut self.palette {
                    let step = if *key == KeyPress::Down { 1 } else { count - 1 };
                    palette.selected = (palette.selected + step) % count;
                }
            }
            KeyPress::Tab => {
                if let Some(palette) = &mut self.palette {
                    palette.filter = (palette.filter + 1) % PALETTE_FILTERS.len();
                    palette.selected = 0;
                }
                self.refresh_palette();
            }
            KeyPress::Enter => {
                let selected = self.palette.as_ref().map_or(0, |palette| palette.selected);
                return Some(self.activate_palette(selected));
            }
            KeyPress::Ctrl('a') => self.select_all = true,
            KeyPress::Text(typed) | KeyPress::Paste(typed) => {
                self.insert_text(typed);
                self.requery_palette();
            }
            KeyPress::Backspace
            | KeyPress::Delete
            | KeyPress::Left
            | KeyPress::Right
            | KeyPress::Home
            | KeyPress::End => {
                self.edit_key(key);
                self.requery_palette();
            }
            KeyPress::Down | KeyPress::Up | KeyPress::AltLeft | KeyPress::AltRight => {}
            // Tout autre raccourci ferme la palette puis fait son office.
            _ => {
                self.close_palette();
                return None;
            }
        }
        Some(Vec::new())
    }

    /// Le texte saisi a peut-être changé : la première ligne redevient la choisie.
    fn requery_palette(&mut self) {
        if let Some(palette) = &mut self.palette {
            palette.selected = 0;
        }
        self.refresh_palette();
    }

    /// Index de la couleur d'accent choisie dans [`ACCENT_COLORS`].
    fn accent_index(&self) -> usize {
        let chosen = self.settings.get(KEY_ACCENT);
        ACCENT_KEYS
            .iter()
            .position(|key| Some(*key) == chosen)
            .unwrap_or(0)
    }

    /// Index du moteur de recherche choisi dans [`loader::SEARCH_ENGINE_NAMES`].
    fn engine_index(&self) -> usize {
        let chosen = self.settings.get(KEY_ENGINE);
        loader::SEARCH_ENGINE_NAMES
            .iter()
            .position(|name| chosen.is_some_and(|c| name.eq_ignore_ascii_case(c)))
            .unwrap_or(0)
    }

    /// Contenu de l'écran Paramètres. L'ordre des réglages fixe leur index
    /// (`ROW_ACCENT`, `ROW_RESTORE`…).
    fn settings_groups(&self) -> Vec<SettingsGroup<'_>> {
        let confirming = self
            .settings_screen
            .as_ref()
            .is_some_and(|screen| screen.confirm_clear);
        let row = |title, description, control| SettingRow {
            title,
            description,
            control,
        };
        vec![
            SettingsGroup {
                title: "Apparence",
                rows: vec![row(
                    "Couleur d'accent",
                    "",
                    SettingControl::Swatches {
                        colors: &ACCENT_COLORS,
                        names: &ACCENT_NAMES,
                        selected: self.accent_index(),
                    },
                )],
            },
            SettingsGroup {
                title: "Démarrage",
                rows: vec![row(
                    "Rouvrir les onglets de la dernière session",
                    "Sinon, Oxyde s'ouvre sur un nouvel onglet.",
                    SettingControl::Switch {
                        on: self.settings.flag(KEY_RESTORE, true),
                    },
                )],
            },
            SettingsGroup {
                title: "Recherche",
                rows: vec![row(
                    "Moteur de recherche",
                    "Pour ce qui n'est pas une adresse.",
                    SettingControl::Choice {
                        options: &loader::SEARCH_ENGINE_NAMES,
                        selected: self.engine_index(),
                    },
                )],
            },
            SettingsGroup {
                title: "Confidentialité et sécurité",
                rows: vec![
                    row(
                        "Conserver l'historique",
                        "Note les pages visitées pour les retrouver avec Ctrl H.",
                        SettingControl::Switch {
                            on: self.settings.flag(KEY_HISTORY, true),
                        },
                    ),
                    row(
                        "Données de navigation",
                        "Efface toutes les pages notées dans l'historique.",
                        SettingControl::Button {
                            label: if confirming {
                                "Confirmer l'effacement"
                            } else {
                                "Effacer l'historique…"
                            },
                            danger: confirming,
                        },
                    ),
                ],
            },
            SettingsGroup {
                title: "À propos d'Oxyde",
                rows: vec![row(
                    "Version",
                    "Navigateur écrit en Rust, moteur de rendu et JavaScript compris.",
                    SettingControl::Value {
                        text: env!("CARGO_PKG_VERSION"),
                    },
                )],
            },
        ]
    }

    /// Ouvre l'écran Paramètres.
    pub(crate) fn open_settings(&mut self) {
        self.library = None;
        self.editing = None;
        self.settings_screen = Some(SettingsScreen {
            selected: 0,
            scroll: 0.0,
            jump: None,
            confirm_clear: false,
        });
    }

    fn close_settings(&mut self) {
        self.settings_screen = None;
        self.refresh_address();
    }

    /// L'interface a amené la rubrique demandée en haut : le défilement
    /// retenu devient celui de l'écran.
    pub(crate) fn settle_settings_scroll(&mut self, scroll: f32) {
        if let Some(screen) = &mut self.settings_screen {
            screen.scroll = scroll;
            screen.jump = None;
        }
    }

    /// Clic dans l'écran Paramètres.
    fn settings_click(&mut self, hit: Hit) {
        let confirming = self
            .settings_screen
            .as_mut()
            .is_some_and(|screen| std::mem::take(&mut screen.confirm_clear));
        match hit {
            Hit::LibraryBack => self.close_settings(),
            Hit::SettingsGroup(group) => {
                if let Some(screen) = &mut self.settings_screen {
                    screen.selected = group;
                    screen.jump = Some(group);
                }
            }
            Hit::Setting {
                row: ROW_ACCENT,
                option,
            } => {
                if let Some(key) = ACCENT_KEYS.get(option) {
                    self.settings.set(KEY_ACCENT, key);
                }
            }
            Hit::Setting {
                row: ROW_RESTORE, ..
            } => {
                let on = self.settings.flag(KEY_RESTORE, true);
                self.settings.set_flag(KEY_RESTORE, !on);
            }
            Hit::Setting {
                row: ROW_ENGINE,
                option,
            } => {
                if let Some(name) = loader::SEARCH_ENGINE_NAMES.get(option) {
                    self.settings.set(KEY_ENGINE, &name.to_ascii_lowercase());
                }
            }
            Hit::Setting {
                row: ROW_HISTORY, ..
            } => {
                let on = self.settings.flag(KEY_HISTORY, true);
                self.settings.set_flag(KEY_HISTORY, !on);
            }
            Hit::Setting { row: ROW_CLEAR, .. } => {
                if confirming {
                    self.history.clear();
                } else if let Some(screen) = &mut self.settings_screen {
                    screen.confirm_clear = true;
                }
            }
            _ => {}
        }
    }

    /// Touche pendant que l'écran Paramètres est ouvert ; `false` : la
    /// touche suit son cours habituel.
    fn settings_key(&mut self, key: &KeyPress) -> bool {
        if let Some(screen) = &mut self.settings_screen {
            screen.confirm_clear = false;
        }
        match key {
            KeyPress::Escape | KeyPress::Ctrl(',' | 'w') => self.close_settings(),
            // Ces raccourcis quittent les paramètres puis font leur office.
            KeyPress::Ctrl('l' | 'k' | 't' | 'h') | KeyPress::CtrlShift('t' | 'o' | 's') => {
                self.close_settings();
                return false;
            }
            _ => {}
        }
        true
    }

    /// État complet de l'interface pour une fenêtre de `width` × `height`.
    pub(crate) fn chrome_state<'a>(
        &'a self,
        views: &'a Views<'a>,
        width: f32,
        height: f32,
    ) -> ChromeState<'a> {
        let active = self.tabs.get(self.active);
        ChromeState {
            width,
            height,
            tabs: &views.tabs,
            active: self.active,
            split: self.split,
            address: self.editing.as_deref().unwrap_or(&self.address),
            page_address: &self.address,
            editing: self.editing.is_some(),
            select_all: self.select_all,
            secure: active.is_some_and(|t| t.error.is_none() && t.page.url().scheme() == "https"),
            can_back: active.is_some_and(|t| t.position > 0),
            can_forward: active.is_some_and(|t| t.position + 1 < t.history.len()),
            hover: self.hover,
            spaces: &views.spaces,
            space: self.space,
            closed: &views.closed,
            tab_scroll: self.tab_scroll,
            caret: self.caret,
            library: self.library.as_ref().map(|library| LibraryView {
                section: library.section,
                rows: &views.library,
                scroll: library.scroll,
                confirm_clear: library.confirm_clear,
                truncated: library.truncated,
            }),
            find: self.find.as_ref().map(|bar| {
                let status = self
                    .tabs
                    .iter()
                    .find(|tab| tab.id == bar.tab)
                    .map(|tab| tab.page.find_status())
                    .unwrap_or_default();
                FindView {
                    query: &bar.query,
                    focused: bar.focused,
                    count: status.count,
                    current: status.current,
                }
            }),
            onboarding: self.onboarding.as_ref().map(|onboarding| OnboardingView {
                step: onboarding.step,
                sources: &views.onboarding,
                selected: onboarding.selected,
                imported: onboarding.imported,
                accents: &ACCENT_COLORS,
                accent_names: &ACCENT_NAMES,
                accent: self.accent_index(),
            }),
            palette: self.palette.as_ref().map(|palette| PaletteView {
                rows: &views.palette,
                selected: palette.selected,
                filters: &PALETTE_FILTERS,
                filter: palette.filter,
            }),
            settings: self.settings_screen.as_ref().map(|screen| SettingsView {
                groups: &views.settings,
                selected: screen.selected,
                scroll: screen.scroll,
                jump: screen.jump,
            }),
            accent: ACCENT_COLORS
                .get(self.accent_index())
                .copied()
                .unwrap_or(theme::ACCENT),
            private: self.private,
        }
    }

    /// Vrai si la bibliothèque ou les paramètres sont ouverts à la place des
    /// onglets.
    pub(crate) fn library_open(&self) -> bool {
        self.library.is_some() || self.settings_screen.is_some() || self.onboarding.is_some()
    }

    /// Affiche le premier lancement ; `sources` : les navigateurs d'où des
    /// favoris peuvent être importés (nom, fichier).
    pub(crate) fn start_onboarding(&mut self, sources: Vec<(String, std::path::PathBuf)>) {
        self.library = None;
        self.settings_screen = None;
        self.palette = None;
        self.editing = None;
        self.onboarding = Some(Onboarding {
            step: 0,
            sources,
            selected: 0,
            imported: None,
        });
    }

    /// Favoris lus dans le fichier d'un autre navigateur (adresse, titre) :
    /// ceux qui manquent sont ajoutés.
    pub(crate) fn finish_import(&mut self, found: &[(String, String)]) {
        let now = now_seconds();
        let added = found
            .iter()
            .filter(|(url, title)| self.bookmarks.add(url, title, now))
            .count();
        if let Some(onboarding) = &mut self.onboarding {
            onboarding.imported = Some(added);
        }
    }

    /// Valide (`next`) ou passe l'étape en cours du premier lancement.
    fn advance_onboarding(&mut self, next: bool) -> Vec<Command> {
        let Some(onboarding) = &mut self.onboarding else {
            return Vec::new();
        };
        if onboarding.step == 0 {
            onboarding.step = 1;
            let source = onboarding.sources.get(onboarding.selected);
            return match source {
                Some((_, file)) if next => vec![Command::ImportBookmarks(file.clone())],
                _ => Vec::new(),
            };
        }
        // Terminé : il ne reviendra pas au prochain lancement.
        self.onboarding = None;
        self.settings.set_flag(KEY_WELCOME, true);
        self.refresh_address();
        Vec::new()
    }

    /// Clic pendant le premier lancement.
    fn onboarding_click(&mut self, hit: Hit) -> Vec<Command> {
        match hit {
            Hit::OnboardingSource(index) => {
                if let Some(onboarding) = &mut self.onboarding {
                    onboarding.selected = index.min(onboarding.sources.len());
                }
            }
            Hit::OnboardingAccent(index) => {
                if let Some(key) = ACCENT_KEYS.get(index) {
                    self.settings.set(KEY_ACCENT, key);
                }
            }
            Hit::OnboardingNext => return self.advance_onboarding(true),
            Hit::OnboardingSkip => return self.advance_onboarding(false),
            _ => {}
        }
        Vec::new()
    }

    /// Touche pendant le premier lancement.
    fn onboarding_key(&mut self, key: &KeyPress) -> Vec<Command> {
        match key {
            KeyPress::Enter => return self.advance_onboarding(true),
            KeyPress::Escape => return self.advance_onboarding(false),
            KeyPress::Up | KeyPress::Down => {
                if let Some(onboarding) = &mut self.onboarding
                    && onboarding.step == 0
                {
                    let count = onboarding.sources.len() + 1;
                    let step = if *key == KeyPress::Down { 1 } else { count - 1 };
                    onboarding.selected = (onboarding.selected + step) % count;
                }
            }
            _ => {}
        }
        Vec::new()
    }

    /// Fait défiler la bibliothèque ou les paramètres de `dy` pixels, sans
    /// dépasser `max` ; renvoie `true` si le défilement a changé.
    pub(crate) fn scroll_library(&mut self, dy: f32, max: f32) -> bool {
        let scroll = match (&mut self.library, &mut self.settings_screen) {
            (Some(library), _) => &mut library.scroll,
            (None, Some(screen)) => &mut screen.scroll,
            (None, None) => return false,
        };
        let target = (*scroll + dy).clamp(0.0, max.max(0.0));
        let changed = (target - *scroll).abs() > f32::EPSILON;
        *scroll = target;
        changed
    }

    /// Ouvre la bibliothèque sur `section` (ligne de commande).
    pub(crate) fn open_screen(&mut self, section: LibrarySection) {
        self.open_library(section);
    }

    fn open_library(&mut self, section: LibrarySection) {
        self.settings_screen = None;
        self.library = Some(Library {
            section,
            scroll: 0.0,
            confirm_clear: false,
            entries: Vec::new(),
            truncated: false,
        });
        // L'historique se filtre en tapant : son champ de recherche a la saisie.
        self.editing = (section == LibrarySection::History).then(String::new);
        self.caret = 0;
        self.select_all = false;
        self.refresh_library();
    }

    fn close_library(&mut self) {
        self.library = None;
        self.editing = None;
        self.refresh_address();
    }

    /// Ouvre la bibliothèque sur `section`, ou la referme si elle y est déjà.
    fn toggle_library(&mut self, section: LibrarySection) {
        if self.library.as_ref().is_some_and(|l| l.section == section) {
            self.close_library();
        } else {
            self.open_library(section);
        }
    }

    /// Recalcule les lignes de la bibliothèque d'après l'historique, les
    /// favoris et le texte de recherche.
    fn refresh_library(&mut self) {
        let Some(library) = &mut self.library else {
            return;
        };
        library.entries.clear();
        library.truncated = false;
        let host_of = |url: &str| {
            Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_owned))
                .unwrap_or_default()
        };
        let shown =
            |url: &str| Url::parse(url).map_or_else(|_| url.to_owned(), |u| display_url(&u));
        match library.section {
            LibrarySection::History => {
                let now = now_seconds();
                let today = day_of(oxyde_platform::local_time(now));
                let yesterday = day_of(oxyde_platform::local_time(now.saturating_sub(86_400)));
                let query = self.editing.as_deref().unwrap_or("");
                let mut last_day = None;
                for visit in self.history.search(query) {
                    if library.entries.len() == MAX_LIBRARY_ROWS {
                        library.truncated = true;
                        break;
                    }
                    let local = oxyde_platform::local_time(visit.time);
                    let day = day_of(local);
                    let heading = (last_day != Some(day)).then(|| day_label(day, today, yesterday));
                    last_day = Some(day);
                    library.entries.push(LibraryEntry {
                        title: if visit.title.is_empty() {
                            shown(&visit.url)
                        } else {
                            visit.title.clone()
                        },
                        host: host_of(&visit.url),
                        detail: format!("{:02}:{:02}", local.hour, local.minute),
                        heading,
                        url: visit.url.clone(),
                        time: visit.time,
                    });
                }
            }
            LibrarySection::Bookmarks => {
                for (index, item) in self.bookmarks.items().iter().enumerate() {
                    library.entries.push(LibraryEntry {
                        title: if item.title.is_empty() {
                            shown(&item.url)
                        } else {
                            item.title.clone()
                        },
                        host: host_of(&item.url),
                        detail: shown(&item.url),
                        heading: (index == 0).then(|| "Tous les favoris".to_owned()),
                        url: item.url.clone(),
                        time: 0,
                    });
                }
            }
        }
    }

    /// Ouvre la page de la ligne `index` de la bibliothèque : dans l'onglet
    /// actif s'il est vide, sinon dans un nouvel onglet.
    fn open_library_row(&mut self, index: usize) -> Vec<Command> {
        let url = self
            .library
            .as_ref()
            .and_then(|library| library.entries.get(index))
            .and_then(|entry| Url::parse(&entry.url).ok());
        let Some(url) = url else {
            return Vec::new();
        };
        self.close_library();
        self.open_in_tab(url)
    }

    /// Retire de l'historique ou des favoris la ligne `index` de la bibliothèque.
    fn remove_library_row(&mut self, index: usize) {
        let Some(library) = &self.library else {
            return;
        };
        let Some(entry) = library.entries.get(index) else {
            return;
        };
        match library.section {
            LibrarySection::History => {
                self.history.remove(&entry.url, entry.time);
            }
            LibrarySection::Bookmarks => {
                self.bookmarks.remove(&entry.url);
            }
        }
        self.refresh_library();
    }

    /// Clic dans la bibliothèque.
    fn library_click(&mut self, hit: Hit) -> Vec<Command> {
        let confirming = self
            .library
            .as_mut()
            .is_some_and(|library| std::mem::take(&mut library.confirm_clear));
        match hit {
            Hit::LibraryBack => self.close_library(),
            Hit::LibrarySection(section) => {
                if self.library.as_ref().is_some_and(|l| l.section != section) {
                    self.open_library(section);
                }
            }
            Hit::LibraryRow(index) => return self.open_library_row(index),
            Hit::LibraryRemove(index) => self.remove_library_row(index),
            Hit::LibraryClear => {
                if confirming {
                    self.history.clear();
                    self.refresh_library();
                } else if let Some(library) = &mut self.library {
                    library.confirm_clear = true;
                }
            }
            _ => {}
        }
        Vec::new()
    }

    /// Touche pendant que la bibliothèque est ouverte ; `None` : la touche
    /// suit son cours habituel.
    fn library_key(&mut self, key: &KeyPress) -> Option<Vec<Command>> {
        if let Some(library) = &mut self.library {
            library.confirm_clear = false;
        }
        match key {
            KeyPress::Escape => {
                if self
                    .editing
                    .as_deref()
                    .is_some_and(|query| !query.is_empty())
                {
                    self.editing = Some(String::new());
                    self.caret = 0;
                    self.select_all = false;
                    self.refresh_library();
                } else {
                    self.close_library();
                }
            }
            KeyPress::Ctrl('h') => self.toggle_library(LibrarySection::History),
            KeyPress::CtrlShift('o') => self.toggle_library(LibrarySection::Bookmarks),
            KeyPress::Ctrl('w') => self.close_library(),
            KeyPress::Enter => return Some(self.open_library_row(0)),
            KeyPress::Ctrl('a') => {
                if self.editing.is_some() {
                    self.select_all = true;
                }
            }
            KeyPress::Text(typed) | KeyPress::Paste(typed) if self.editing.is_some() => {
                self.insert_text(typed);
                self.requery_library();
            }
            KeyPress::Backspace
            | KeyPress::Delete
            | KeyPress::Left
            | KeyPress::Right
            | KeyPress::Home
            | KeyPress::End
                if self.editing.is_some() =>
            {
                self.edit_key(key);
                self.requery_library();
            }
            // Ces raccourcis quittent la bibliothèque puis font leur office.
            KeyPress::Ctrl('l' | 'k' | 't') | KeyPress::CtrlShift('t') => {
                self.close_library();
                return None;
            }
            _ => {}
        }
        Some(Vec::new())
    }

    /// Le texte de recherche a peut-être changé : la liste repart du haut.
    fn requery_library(&mut self) {
        if let Some(library) = &mut self.library {
            library.scroll = 0.0;
        }
        self.refresh_library();
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    fn start_editing(&mut self) {
        let current = self
            .tabs
            .get(self.active)
            .map(|t| match t.page.url().scheme() {
                "about" => String::new(),
                _ => t.page.url().to_string(),
            });
        let text = current.unwrap_or_default();
        self.caret = text.len();
        self.editing = Some(text);
        self.select_all = true;
    }

    fn load(&mut self, index: usize, url: Url, push_history: bool) -> Vec<Command> {
        self.load_with(index, url, push_history, None)
    }

    fn load_with(
        &mut self,
        index: usize,
        url: Url,
        push_history: bool,
        post: Option<oxyde_content::FormPost>,
    ) -> Vec<Command> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Vec::new();
        };
        tab.loading = true;
        vec![Command::Load {
            tab: tab.id,
            url,
            push_history,
            post,
        }]
    }

    fn go(&mut self, index: usize, delta: isize) -> Vec<Command> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Vec::new();
        };
        let Some(target) = tab.position.checked_add_signed(delta) else {
            return Vec::new();
        };
        let Some(url) = tab.history.get(target).cloned() else {
            return Vec::new();
        };
        // Même document (entrées de `pushState`, fragments) : la page reste
        // en place, elle change d'adresse et prévient ses scripts.
        let same_document = tab.documents.get(target).is_some()
            && tab.documents.get(target) == tab.documents.get(tab.position);
        let steps = i32::try_from(delta).unwrap_or(0);
        tab.position = target;
        if same_document {
            tab.page.traverse(&url, steps);
            tab.refresh_labels();
            if index == self.active && self.editing.is_none() {
                self.refresh_address();
            }
            return Vec::new();
        }
        self.load(index, url, false)
    }

    fn reload(&mut self, index: usize) -> Vec<Command> {
        let Some(url) = self.tabs.get(index).map(|t| t.page.url().clone()) else {
            return Vec::new();
        };
        self.load(index, url, false)
    }

    fn new_tab(&mut self) {
        self.active = self.push_tab(blank(), self.space);
        self.refresh_address();
        self.start_editing();
    }

    /// Onglet à activer dans l'espace courant quand l'actif disparaît : le
    /// plus proche de `near` dans l'espace, sinon un onglet épinglé.
    fn fallback_tab(&self, near: usize) -> Option<usize> {
        let in_space = |i: &usize| {
            self.tabs
                .get(*i)
                .is_some_and(|t| !t.pinned && t.space == self.space)
        };
        (near..self.tabs.len())
            .find(in_space)
            .or_else(|| (0..near.min(self.tabs.len())).rev().find(in_space))
            .or_else(|| self.tabs.iter().position(|t| t.pinned))
    }

    fn close_tab(&mut self, index: usize) -> Vec<Command> {
        if index >= self.tabs.len() {
            return Vec::new();
        }
        let tab = self.tabs.remove(index);
        if !tab.is_new_tab() {
            self.closed.insert(
                0,
                Closed {
                    url: tab.page.url().clone(),
                    title: tab.title,
                    host: tab.host,
                    space: tab.space,
                },
            );
            self.closed.truncate(MAX_CLOSED);
        }
        if self.tabs.is_empty() {
            return vec![Command::CloseWindow];
        }
        // Les index situés après l'onglet retiré reculent d'un cran.
        let shift = |i: usize| if i > index { i - 1 } else { i };
        self.split = match self.split {
            Some(s) if s == index => None,
            Some(s) => Some(shift(s)),
            None => None,
        };
        if self.active == index {
            self.active = match self.split.take().or_else(|| self.fallback_tab(index)) {
                Some(next) => next,
                // L'espace est vide : il reçoit un nouvel onglet.
                None => self.push_tab(blank(), self.space),
            };
        } else {
            self.active = shift(self.active);
        }
        if self.split == Some(self.active) {
            self.split = None;
        }
        self.editing = None;
        self.refresh_address();
        Vec::new()
    }

    /// Rouvre l'onglet fermé d'index `k` dans son espace d'origine.
    fn reopen(&mut self, k: usize) -> Vec<Command> {
        if k >= self.closed.len() {
            return Vec::new();
        }
        let closed = self.closed.remove(k);
        self.remember_space_tab();
        self.space = closed.space.min(SPACES.len() - 1);
        self.split = None;
        self.editing = None;
        let index = self.push_tab(blank(), self.space);
        self.active = index;
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.history = vec![closed.url.clone()];
            tab.documents = vec![next_document()];
            tab.title = closed.title;
            tab.host = closed.host;
        }
        self.load(index, closed.url, false)
    }

    fn toggle_split(&mut self) {
        if self.split.take().is_some() {
            return;
        }
        // Second volet : un autre onglet de l'espace, ou un nouvel onglet.
        let other = (self.active + 1..self.tabs.len())
            .chain(0..self.active)
            .find(|i| {
                self.tabs
                    .get(*i)
                    .is_some_and(|t| !t.pinned && t.space == self.space)
            });
        self.split = Some(other.unwrap_or_else(|| self.push_tab(blank(), self.space)));
    }

    fn remember_space_tab(&mut self) {
        if let Some(tab) = self.tabs.get(self.active)
            && !tab.pinned
            && tab.space == self.space
            && let Some(slot) = self.space_active.get_mut(self.space)
        {
            *slot = Some(tab.id);
        }
    }

    fn switch_space(&mut self, space: usize) {
        if space >= SPACES.len() || space == self.space {
            return;
        }
        self.remember_space_tab();
        self.space = space;
        self.split = None;
        self.editing = None;
        let remembered = self.space_active.get(space).copied().flatten();
        let target = self
            .tabs
            .iter()
            .position(|t| Some(t.id) == remembered && !t.pinned && t.space == space)
            .or_else(|| self.tabs.iter().position(|t| !t.pinned && t.space == space));
        self.active = match target {
            Some(index) => index,
            None => self.push_tab(blank(), space),
        };
        self.refresh_address();
    }

    fn toggle_pin(&mut self) {
        let space = self.space;
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.pinned = !tab.pinned;
            // Un onglet désépinglé rejoint l'espace courant.
            tab.space = space;
        }
    }

    fn activate(&mut self, index: usize) {
        let Some(tab) = self.tabs.get(index) else {
            return;
        };
        let (pinned, space) = (tab.pinned, tab.space);
        if index == self.active {
            return;
        }
        // Cliquer sur l'autre volet échange les rôles sans défaire la vue scindée.
        if self.split == Some(index) {
            self.split = Some(self.active);
        }
        if !pinned && space != self.space {
            self.remember_space_tab();
            self.space = space;
            self.split = None;
        }
        self.active = index;
        self.editing = None;
        self.refresh_address();
    }

    /// Bouton principal d'une page d'erreur.
    fn error_primary(&mut self, index: usize) -> Vec<Command> {
        let Some(tab) = self.tabs.get(index) else {
            return Vec::new();
        };
        match tab.error.as_ref().map(|e| e.kind) {
            // « Revenir en lieu sûr » : page précédente, sinon nouvel onglet.
            Some(ErrorKind::Certificate) => {
                if tab.position > 0 {
                    self.go(index, -1)
                } else {
                    self.finish_load_at(index, blank(), true);
                    Vec::new()
                }
            }
            Some(_) => self.reload(index),
            None => Vec::new(),
        }
    }

    /// Bouton secondaire d'une page d'erreur.
    fn error_secondary(&mut self, index: usize) -> Vec<Command> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Vec::new();
        };
        match tab.error.as_ref().map(|e| e.kind) {
            Some(ErrorKind::Network) => vec![Command::NetworkDiagnostics],
            Some(ErrorKind::Certificate) => {
                tab.show_details = !tab.show_details;
                Vec::new()
            }
            // « Fermer l'onglet ».
            Some(ErrorKind::Crashed) => self.close_tab(index),
            _ => Vec::new(),
        }
    }

    /// Clic sur un élément de l'interface. `page_point` : position dans la
    /// page (pixels CSS, défilement compris) quand le clic vise un volet.
    pub(crate) fn click(&mut self, hit: Hit, page_point: Option<(f32, f32)>) -> Vec<Command> {
        if self.onboarding.is_some()
            && !matches!(
                hit,
                Hit::Minimize | Hit::Maximize | Hit::CloseWindow | Hit::TitleBar
            )
        {
            return self.onboarding_click(hit);
        }
        if self.palette.is_some() {
            return self.palette_click(hit);
        }
        if self.find.is_some() {
            if self.find_click(hit) {
                return Vec::new();
            }
            // Un clic ailleurs retire la saisie à la barre, sans la fermer.
            self.blur_find();
        }
        if self.library_open() {
            // Seuls les boutons de la fenêtre gardent leur rôle habituel.
            let window = matches!(
                hit,
                Hit::Minimize | Hit::Maximize | Hit::CloseWindow | Hit::TitleBar
            );
            if !window && self.settings_screen.is_some() {
                self.settings_click(hit);
                return Vec::new();
            }
            if !window {
                return self.library_click(hit);
            }
        } else if hit != Hit::Address {
            self.editing = None;
        }
        match hit {
            Hit::Back => self.go(self.active, -1),
            Hit::Forward => self.go(self.active, 1),
            Hit::Reload => self.reload(self.active),
            Hit::SplitToggle => {
                self.toggle_split();
                Vec::new()
            }
            Hit::Address => {
                if self.editing.is_none() {
                    self.start_editing();
                } else {
                    self.select_all = false;
                }
                Vec::new()
            }
            Hit::Tab(i) => {
                self.activate(i);
                Vec::new()
            }
            Hit::CloseTab(i) => self.close_tab(i),
            Hit::NewTab => {
                self.new_tab();
                Vec::new()
            }
            Hit::Space(i) => {
                self.switch_space(i);
                Vec::new()
            }
            Hit::Reopen(k) => self.reopen(k),
            Hit::ClosePane(i) => {
                // Fermer le volet actif : l'autre volet prend sa place. Les
                // onglets eux-mêmes restent ouverts.
                if let Some(other) = self.split.take()
                    && i == self.active
                {
                    self.active = other;
                    self.refresh_address();
                }
                Vec::new()
            }
            Hit::Pane(i) => {
                self.activate(i);
                // Les scripts de la page voient le clic d'abord et peuvent
                // annuler la navigation.
                let link = page_point.and_then(|(x, y)| {
                    let tab = self.tabs.get_mut(i)?;
                    tab.page.click(x, y)
                });
                match link {
                    Some(url) => self.load(i, url, true),
                    None => Vec::new(),
                }
            }
            Hit::ErrorPrimary(i) => self.error_primary(i),
            Hit::ErrorSecondary(i) => self.error_secondary(i),
            Hit::Minimize => vec![Command::Minimize],
            Hit::Maximize => vec![Command::ToggleMaximize],
            Hit::CloseWindow => vec![Command::CloseWindow],
            Hit::Settings => {
                self.open_settings();
                Vec::new()
            }
            // Barre de titre : gérée par la fenêtre.
            Hit::TitleBar => Vec::new(),
            // La bibliothèque et les paramètres sont fermés : leurs éléments
            // ne sont pas à l'écran.
            Hit::SettingsGroup(_) | Hit::Setting { .. } => Vec::new(),
            Hit::PaletteRow(_)
            | Hit::PaletteFilter(_)
            | Hit::PaletteInput
            | Hit::PaletteDismiss => Vec::new(),
            // Le premier lancement est terminé.
            Hit::OnboardingSource(_)
            | Hit::OnboardingAccent(_)
            | Hit::OnboardingSkip
            | Hit::OnboardingNext => Vec::new(),
            // La barre de recherche est fermée.
            Hit::FindInput | Hit::FindPrevious | Hit::FindNext | Hit::FindClose => Vec::new(),
            Hit::LibraryBack
            | Hit::LibrarySection(_)
            | Hit::LibraryRow(_)
            | Hit::LibraryRemove(_)
            | Hit::LibrarySearch
            | Hit::LibraryClear => Vec::new(),
        }
    }

    fn insert_text(&mut self, typed: &str) {
        let Some(text) = &mut self.editing else {
            return;
        };
        if self.select_all {
            text.clear();
            self.caret = 0;
        }
        self.select_all = false;
        // Une adresse tient sur une ligne : caractères de contrôle écartés.
        let clean: String = typed.chars().filter(|c| !c.is_control()).collect();
        let at = self.caret.min(text.len());
        if text.is_char_boundary(at) {
            text.insert_str(at, &clean);
            self.caret = at + clean.len();
        }
    }

    /// Touches d'édition de la barre d'adresse : effacement et déplacement du curseur.
    fn edit_key(&mut self, key: &KeyPress) {
        let Some(text) = &mut self.editing else {
            return;
        };
        let caret = self.caret.min(text.len());
        // Frontières des caractères voisins du curseur.
        let previous = text
            .get(..caret)
            .and_then(|s| s.char_indices().next_back())
            .map(|(i, _)| i);
        let next = text
            .get(caret..)
            .and_then(|s| s.chars().next())
            .map(|c| caret + c.len_utf8());
        match key {
            KeyPress::Backspace | KeyPress::Delete if self.select_all => {
                text.clear();
                self.caret = 0;
            }
            KeyPress::Backspace => {
                if let Some(start) = previous {
                    text.replace_range(start..caret, "");
                    self.caret = start;
                }
            }
            KeyPress::Delete => {
                if let Some(end) = next {
                    text.replace_range(caret..end, "");
                }
            }
            KeyPress::Left => {
                self.caret = if self.select_all {
                    0
                } else {
                    previous.unwrap_or(0)
                }
            }
            KeyPress::Right => {
                self.caret = if self.select_all {
                    text.len()
                } else {
                    next.unwrap_or(text.len())
                };
            }
            KeyPress::Home => self.caret = 0,
            KeyPress::End => self.caret = text.len(),
            _ => return,
        }
        self.select_all = false;
    }

    /// Touche du clavier.
    pub(crate) fn key(&mut self, key: KeyPress) -> Vec<Command> {
        if self.onboarding.is_some() {
            return self.onboarding_key(&key);
        }
        // Ouvrir la palette, la bibliothèque ou les paramètres ferme la
        // barre de recherche : sa page n'est plus affichée.
        if self.find.is_some() && self.palette.is_none() && !self.library_open() {
            if self.find_key(&key) {
                return Vec::new();
            }
            if matches!(
                key,
                KeyPress::Ctrl('k' | 'h' | ',') | KeyPress::CtrlShift('o')
            ) {
                self.close_find();
            }
        }
        // Hors de la barre de recherche, Maj Entrée vaut Entrée.
        let key = if key == KeyPress::ShiftEnter {
            KeyPress::Enter
        } else {
            key
        };
        if self.palette.is_some()
            && let Some(commands) = self.palette_key(&key)
        {
            return commands;
        }
        if self.settings_screen.is_some() && self.settings_key(&key) {
            return Vec::new();
        }
        if self.library.is_some()
            && let Some(commands) = self.library_key(&key)
        {
            return commands;
        }
        match key {
            KeyPress::Ctrl(',') => self.open_settings(),
            KeyPress::Ctrl('h') => self.open_library(LibrarySection::History),
            KeyPress::CtrlShift('o') => self.open_library(LibrarySection::Bookmarks),
            KeyPress::Ctrl('l') => self.start_editing(),
            KeyPress::Ctrl('k') => self.open_palette(),
            KeyPress::Ctrl('f') => self.open_find(),
            KeyPress::CtrlShift('n') => return vec![Command::PrivateWindow],
            KeyPress::CtrlShift('s') => self.toggle_split(),
            // Sans la palette, ces touches ne concernent que la page.
            KeyPress::Tab | KeyPress::Up | KeyPress::Down => {}
            KeyPress::Ctrl('t') => self.new_tab(),
            KeyPress::Ctrl('w') => return self.close_tab(self.active),
            KeyPress::Ctrl('r') | KeyPress::F5 => return self.reload(self.active),
            KeyPress::Ctrl('a') => {
                if self.editing.is_some() {
                    self.select_all = true;
                }
            }
            KeyPress::CtrlShift('t') => return self.reopen(0),
            KeyPress::CtrlShift('p') => self.toggle_pin(),
            KeyPress::Ctrl('d') => self.toggle_bookmark(),
            KeyPress::CtrlTab => {
                // Onglet suivant parmi ceux de la barre latérale (épinglés et espace courant).
                let count = self.tabs.len();
                let next = (1..count)
                    .map(|step| (self.active + step) % count)
                    .find(|i| {
                        self.tabs
                            .get(*i)
                            .is_some_and(|t| t.pinned || t.space == self.space)
                    });
                if let Some(next) = next {
                    self.activate(next);
                }
            }
            KeyPress::AltLeft => return self.go(self.active, -1),
            KeyPress::AltRight => return self.go(self.active, 1),
            KeyPress::Escape => self.editing = None,
            KeyPress::Backspace
            | KeyPress::Delete
            | KeyPress::Left
            | KeyPress::Right
            | KeyPress::Home
            | KeyPress::End => self.edit_key(&key),
            KeyPress::Text(typed) | KeyPress::Paste(typed) => self.insert_text(&typed),
            KeyPress::Enter => {
                if let Some(input) = self.editing.take()
                    && !input.trim().is_empty()
                {
                    return match loader::resolve_input_with(&input, self.engine_index()) {
                        Ok(url) => self.load(self.active, url, true),
                        Err(error) => {
                            let failed = Loaded {
                                page: loader::blank_page(),
                                error: Some(error),
                            };
                            self.finish_load_at(self.active, failed, true);
                            Vec::new()
                        }
                    };
                }
            }
            // Maj Entrée a déjà été ramenée à Entrée.
            KeyPress::Ctrl(_) | KeyPress::CtrlShift(_) | KeyPress::ShiftEnter => {}
        }
        Vec::new()
    }

    /// Exécute les minuteurs échus de tous les onglets ; renvoie `true` si un
    /// onglet doit être redessiné.
    pub(crate) fn tick(&mut self, now: std::time::Instant) -> bool {
        // La barre de recherche suit son onglet : elle se ferme dès qu'un
        // autre est affiché, ou qu'il a disparu.
        let shown = self.tabs.get(self.active).map(|tab| tab.id);
        if self.find.as_ref().is_some_and(|bar| Some(bar.tab) != shown) {
            self.close_find();
        }
        let mut redraw = false;
        let mut commits = Vec::new();
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let changed = tab.page.tick(now);
            redraw |= changed;
            let Some(remote) = tab.page.remote_mut() else {
                continue;
            };
            for commit in remote.take_commits() {
                commits.push((index, commit));
            }
            // Le processus de rendu a disparu : l'onglet le dit, les autres
            // continuent.
            let crashed = remote.is_crashed();
            if crashed
                && !tab
                    .error
                    .as_ref()
                    .is_some_and(|e| e.kind == ErrorKind::Crashed)
            {
                tab.error = Some(LoadError::crashed(tab.page.url()));
                tab.show_details = false;
                tab.loading = false;
                tab.pending_push = None;
                redraw = true;
            }
            if changed {
                // Une image peut apporter un nouveau titre.
                tab.refresh_labels();
                let title = tab.page.title();
                if !title.is_empty()
                    && !self.private
                    && tab.error.is_none()
                    && !tab.asleep
                    && is_recorded(tab.page.url())
                {
                    self.history.set_title(tab.page.url().as_str(), &title);
                }
            }
        }
        for (index, commit) in commits {
            self.finish_remote_load(index, commit);
        }
        if redraw && self.editing.is_none() {
            self.refresh_address();
        }
        redraw
    }

    /// Confie à un processus de rendu le chargement de `url` dans l'onglet
    /// `tab_id`. `wake` réveille la boucle d'événements quand ce processus a
    /// quelque chose à dire. Renvoie `false` si le processus n'a pas pu être
    /// lancé : à l'appelant de charger la page autrement.
    pub(crate) fn load_remote(
        &mut self,
        tab_id: u64,
        url: &Url,
        post: Option<&oxyde_content::FormPost>,
        push_history: bool,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return true;
        };
        match tab
            .page
            .remote_mut()
            .filter(|remote| !remote.is_crashed() && remote.can_host(url))
        {
            // L'onglet a déjà un processus voué à ce site : il y charge la
            // nouvelle page. Pour un autre site, un processus neuf prend le
            // relais et l'ancien s'arrête.
            Some(remote) => remote.load(url.clone(), post.cloned()),
            None => {
                let viewport = loader::viewport_size();
                match RemotePage::spawn(url.clone(), post.cloned(), viewport, wake) {
                    Ok(remote) => tab.page = PageHost::Remote(Box::new(remote)),
                    Err(error) => {
                        eprintln!("processus de rendu indisponible : {error}");
                        return false;
                    }
                }
            }
        }
        // Une navigation reprise après une redirection vers un autre site
        // garde l'intention de celle qu'elle remplace.
        let pushing = tab.pending_push.take().unwrap_or(false);
        tab.pending_push = Some(push_history || pushing);
        true
    }

    /// Le processus de rendu de l'onglet `index` a terminé un chargement.
    fn finish_remote_load(&mut self, index: usize, commit: crate::renderer::host::Commit) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        let push_history = tab.pending_push.take().unwrap_or(false);
        if push_history {
            tab.history.truncate(tab.position + 1);
            tab.documents.truncate(tab.position + 1);
            tab.history.push(commit.url);
            tab.documents.push(next_document());
            tab.position = tab.history.len() - 1;
        } else {
            if let Some(entry) = tab.history.get_mut(tab.position) {
                // Une redirection peut avoir changé l'adresse finale.
                *entry = commit.url;
            }
            if let Some(document) = tab.documents.get_mut(tab.position) {
                *document = next_document();
            }
        }
        tab.error = commit.error;
        tab.show_details = false;
        tab.loading = false;
        tab.asleep = false;
        tab.refresh_labels();
        if let Some(bar) = &self.find
            && bar.tab == tab.id
        {
            tab.page.find(&bar.query);
        }
        if push_history {
            self.record_visit(index);
        }
        if index == self.active && self.editing.is_none() {
            self.refresh_address();
        }
    }

    /// Chargements à lancer pour les onglets ouverts en attente de leur page
    /// (voir [`loader::placeholder`]).
    pub(crate) fn initial_loads(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        for index in 0..self.tabs.len() {
            let pending = self.tabs.get(index).and_then(|tab| {
                let url = tab.page.url();
                (tab.error.is_none() && !tab.asleep && crate::renderer::host::is_remote(url))
                    .then(|| url.clone())
            });
            if let Some(url) = pending {
                commands.extend(self.load(index, url, false));
            }
        }
        commands
    }

    /// Requêtes `fetch` émises par les scripts des onglets : (onglet, requête).
    pub(crate) fn take_fetches(&mut self) -> Vec<(u64, oxyde_content::FetchRequest)> {
        self.tabs
            .iter_mut()
            .flat_map(|tab| {
                let id = tab.id;
                tab.page.take_fetches().into_iter().map(move |r| (id, r))
            })
            .collect()
    }

    /// Livre la réponse d'une requête `fetch` à l'onglet `tab`, s'il existe
    /// encore ; renvoie `true` s'il doit être redessiné.
    pub(crate) fn complete_fetch(
        &mut self,
        tab: u64,
        id: u64,
        result: Result<oxyde_content::FetchResponse, String>,
    ) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return false;
        };
        tab.page.complete_fetch(id, result)
    }

    /// Instant du prochain minuteur à exécuter, tous onglets confondus.
    pub(crate) fn next_wake(&mut self) -> Option<std::time::Instant> {
        self.tabs
            .iter_mut()
            .filter_map(|tab| tab.page.next_wake())
            .min()
    }

    /// Molette de `dy` au point (`x`, `y`) du volet de l'onglet `index` : la
    /// zone défilante sous le pointeur défile, ou la page. Renvoie `true` si
    /// le défilement a changé.
    pub(crate) fn scroll_by(&mut self, index: usize, x: f32, y: f32, dy: f32) -> bool {
        self.tabs
            .get_mut(index)
            .is_some_and(|tab| tab.page.wheel(x, y, 0.0, dy))
    }

    fn finish_load_at(&mut self, index: usize, loaded: Loaded, push_history: bool) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if push_history {
            tab.history.truncate(tab.position + 1);
            tab.documents.truncate(tab.position + 1);
            tab.history.push(loaded.page.url().clone());
            tab.documents.push(next_document());
            tab.position = tab.history.len() - 1;
        } else {
            if let Some(entry) = tab.history.get_mut(tab.position) {
                // Une redirection peut avoir changé l'adresse finale.
                entry.clone_from(loaded.page.url());
            }
            // Rechargée ou atteinte par l'historique : c'est un nouveau document.
            if let Some(document) = tab.documents.get_mut(tab.position) {
                *document = next_document();
            }
        }
        tab.page = loaded.page.into();
        tab.pending_push = None;
        tab.error = loaded.error;
        tab.show_details = false;
        tab.loading = false;
        tab.asleep = false;
        tab.refresh_labels();
        if let Some(bar) = &self.find
            && bar.tab == tab.id
        {
            tab.page.find(&bar.query);
        }
        if push_history {
            self.record_visit(index);
        }
        if index == self.active && self.editing.is_none() {
            self.refresh_address();
        }
    }

    /// Transmet une touche à la page de l'onglet actif, sauf pendant la
    /// saisie dans la barre d'adresse. Renvoie (action par défaut annulée,
    /// page à redessiner).
    pub(crate) fn page_key(
        &mut self,
        down: bool,
        input: &oxyde_content::KeyInput<'_>,
    ) -> (bool, bool) {
        if self.editing.is_some() || self.library_open() {
            return (false, false);
        }
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return (false, false);
        };
        tab.page.key_event(down, input)
    }

    /// Exécute les navigations que les scripts des onglets ont demandées
    /// (`location`, `history`) ; renvoie les chargements à lancer.
    pub(crate) fn drain_navigations(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        for index in 0..self.tabs.len() {
            let navigations = self
                .tabs
                .get_mut(index)
                .map(|tab| tab.page.take_navigations())
                .unwrap_or_default();
            for navigation in navigations {
                match navigation {
                    Navigation::Load { url, replace, post } => {
                        commands.extend(self.load_with(index, url, !replace, post));
                    }
                    Navigation::Reload => commands.extend(self.reload(index)),
                    Navigation::Traverse(delta) => {
                        let delta = isize::try_from(delta).unwrap_or(0);
                        commands.extend(self.go(index, delta));
                    }
                    Navigation::SameDocument { url, replace } => {
                        let Some(tab) = self.tabs.get_mut(index) else {
                            continue;
                        };
                        let document = tab
                            .documents
                            .get(tab.position)
                            .copied()
                            .unwrap_or_else(next_document);
                        if replace {
                            if let Some(entry) = tab.history.get_mut(tab.position) {
                                *entry = url;
                            }
                        } else {
                            tab.history.truncate(tab.position + 1);
                            tab.documents.truncate(tab.position + 1);
                            tab.history.push(url);
                            tab.documents.push(document);
                            tab.position = tab.history.len() - 1;
                        }
                        tab.refresh_labels();
                        if index == self.active && self.editing.is_none() {
                            self.refresh_address();
                        }
                    }
                }
            }
        }
        commands
    }

    /// Fin du chargement demandé par [`Command::Load`]. Sans effet si l'onglet
    /// a été fermé entre-temps.
    pub(crate) fn finish_load(&mut self, tab_id: u64, loaded: Loaded, push_history: bool) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.finish_load_at(index, loaded, push_history);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_content::Page;

    fn page(url: &str, html: &str) -> Loaded {
        Loaded {
            page: Page::from_html(Url::parse(url).expect("url"), html),
            error: None,
        }
    }

    fn failed(url: &str, kind: ErrorKind) -> Loaded {
        Loaded {
            page: Page::from_html(Url::parse(url).expect("url"), ""),
            error: Some(LoadError {
                kind,
                title: "Titre".into(),
                message: "Message".into(),
                code: "ERR_TEST".to_owned(),
                detail: "détail".into(),
            }),
        }
    }

    fn browser() -> Browser {
        Browser::new(page(
            "https://a.test/",
            "<title>A</title><a href='/b'>b</a>",
        ))
    }

    fn load_target(commands: &[Command]) -> Option<(u64, String, bool)> {
        commands.iter().find_map(|c| match c {
            Command::Load {
                tab,
                url,
                push_history,
                ..
            } => Some((*tab, url.to_string(), *push_history)),
            _ => None,
        })
    }

    #[test]
    fn address_bar_editing_and_navigation() {
        let mut b = browser();
        assert_eq!(b.address, "a.test");
        b.key(KeyPress::Ctrl('l'));
        assert!(b.is_editing());
        b.key(KeyPress::Text("b.test/x".into()));
        b.key(KeyPress::Backspace);
        let commands = b.key(KeyPress::Enter);
        assert_eq!(
            load_target(&commands),
            Some((1, "https://b.test/".into(), true))
        );
        assert!(!b.is_editing());

        b.finish_load(1, page("https://b.test/", "<title>B</title>"), true);
        assert_eq!(b.address, "b.test");
        assert_eq!(b.window_title(), "B — Oxyde");
        let back = b.click(Hit::Back, None);
        assert_eq!(
            load_target(&back),
            Some((1, "https://a.test/".into(), false))
        );
        b.finish_load(1, page("https://a.test/", "<title>A</title>"), false);
        assert!(b.click(Hit::Back, None).is_empty(), "plus rien avant");
        assert_eq!(
            load_target(&b.click(Hit::Forward, None)),
            Some((1, "https://b.test/".into(), false))
        );
    }

    #[test]
    fn paste_replaces_selection_and_strips_control_characters() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('l'));
        b.key(KeyPress::Paste("c.test/\r\nx\t".into()));
        assert_eq!(b.editing.as_deref(), Some("c.test/x"));
        b.key(KeyPress::Ctrl('a'));
        b.key(KeyPress::Paste("d.test".into()));
        assert_eq!(b.editing.as_deref(), Some("d.test"));
        // Hors saisie, le collage est sans effet.
        b.key(KeyPress::Escape);
        b.key(KeyPress::Paste("ignoré".into()));
        assert!(!b.is_editing());
    }

    #[test]
    fn caret_moves_and_edits_inside_the_address() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('l'));
        b.key(KeyPress::Text("abé.test".into()));
        assert_eq!(b.caret, "abé.test".len());
        b.key(KeyPress::Home);
        b.key(KeyPress::Right);
        b.key(KeyPress::Right);
        b.key(KeyPress::Right);
        assert_eq!(b.caret, "abé".len(), "le curseur saute le caractère entier");
        b.key(KeyPress::Backspace);
        assert_eq!((b.editing.as_deref(), b.caret), (Some("ab.test"), 2));
        b.key(KeyPress::Delete);
        assert_eq!(b.editing.as_deref(), Some("abtest"));
        b.key(KeyPress::Text("-".into()));
        assert_eq!((b.editing.as_deref(), b.caret), (Some("ab-test"), 3));
        b.key(KeyPress::End);
        b.key(KeyPress::Delete);
        b.key(KeyPress::Home);
        b.key(KeyPress::Backspace);
        b.key(KeyPress::Left);
        assert_eq!(
            (b.editing.as_deref(), b.caret),
            (Some("ab-test"), 0),
            "aucun effet aux extrémités"
        );
        // Une flèche lève la sélection totale et place le curseur à l'extrémité.
        b.key(KeyPress::Ctrl('a'));
        b.key(KeyPress::Right);
        assert_eq!((b.select_all, b.caret), (false, 7));
    }

    #[test]
    fn tabs_split_and_close() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('t'));
        assert_eq!((b.tabs.len(), b.active), (2, 1));
        assert!(b.is_editing(), "un nouvel onglet ouvre la saisie d'adresse");
        b.key(KeyPress::Escape);

        b.click(Hit::SplitToggle, None);
        assert_eq!(b.split, Some(0));
        b.click(Hit::Tab(0), None);
        assert_eq!(
            (b.active, b.split),
            (0, Some(1)),
            "les volets échangent leurs rôles"
        );

        assert!(b.click(Hit::CloseTab(0), None).is_empty());
        assert_eq!((b.tabs.len(), b.active, b.split), (1, 0, None));
        let commands = b.key(KeyPress::Ctrl('w'));
        assert!(matches!(commands.as_slice(), [Command::CloseWindow]));
    }

    #[test]
    fn closed_tabs_can_be_reopened() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('t'));
        b.key(KeyPress::Escape);
        b.click(Hit::CloseTab(0), None);
        assert_eq!(b.closed.len(), 1, "l'onglet « a.test » est gardé");
        let commands = b.key(KeyPress::CtrlShift('t'));
        let (tab, url, push) = load_target(&commands).expect("rechargement");
        assert_eq!((url.as_str(), push), ("https://a.test/", false));
        assert!(b.closed.is_empty());
        b.finish_load(tab, page("https://a.test/", "<title>A</title>"), false);
        assert_eq!(b.address, "a.test");
        // Un nouvel onglet vide n'est pas mémorisé à la fermeture.
        b.click(Hit::CloseTab(0), None);
        assert!(b.closed.is_empty());
    }

    #[test]
    fn spaces_keep_their_own_tabs() {
        let mut b = browser();
        b.click(Hit::Space(1), None);
        assert_eq!(
            (b.space, b.tabs.len(), b.active),
            (1, 2, 1),
            "un espace vide reçoit un onglet"
        );
        assert!(b.tabs[1].is_new_tab());
        b.click(Hit::Space(0), None);
        assert_eq!((b.space, b.active), (0, 0), "retour sur l'onglet mémorisé");
        assert_eq!(b.address, "a.test");

        // Fermer le dernier onglet d'un espace y ouvre un nouvel onglet, sans changer d'espace.
        b.click(Hit::CloseTab(0), None);
        assert_eq!(b.space, 0);
        assert!(b.tabs[b.active].is_new_tab() && b.tabs[b.active].space == 0);
    }

    #[test]
    fn pinned_tabs_are_shared_between_spaces() {
        let mut b = browser();
        b.key(KeyPress::CtrlShift('p'));
        assert!(b.tabs[0].pinned);
        b.click(Hit::Space(2), None);
        let views = b.views();
        let state = b.chrome_state(&views, 1440.0, 900.0);
        assert!(state.tabs[0].pinned);
        drop(views);
        b.click(Hit::Tab(0), None);
        assert_eq!(
            (b.space, b.active),
            (2, 0),
            "un onglet épinglé s'ouvre sans changer d'espace"
        );
        b.key(KeyPress::CtrlShift('p'));
        assert_eq!(
            (b.tabs[0].pinned, b.tabs[0].space),
            (false, 2),
            "désépinglé dans l'espace courant"
        );
    }

    #[test]
    fn error_pages_and_their_buttons() {
        let mut b = browser();
        b.finish_load(1, failed("https://b.test/", ErrorKind::Network), true);
        let views = b.views();
        assert!(
            matches!(views.tabs[0].content, PaneContent::Error(e) if e.style == ErrorStyle::Network)
        );
        drop(views);
        assert_eq!(b.address, "b.test", "l'adresse demandée reste affichée");
        assert_eq!(
            load_target(&b.click(Hit::ErrorPrimary(0), None)),
            Some((1, "https://b.test/".into(), false)),
            "« Réessayer » recharge la même adresse"
        );
        assert!(matches!(
            b.click(Hit::ErrorSecondary(0), None).as_slice(),
            [Command::NetworkDiagnostics]
        ));

        b.finish_load(1, failed("https://c.test/", ErrorKind::Certificate), true);
        b.click(Hit::ErrorSecondary(0), None);
        assert!(b.tabs[0].show_details);
        assert_eq!(
            load_target(&b.click(Hit::ErrorPrimary(0), None)),
            Some((1, "https://b.test/".into(), false)),
            "« Revenir en lieu sûr » retourne à la page précédente"
        );
    }

    #[test]
    fn stale_load_results_are_ignored() {
        let mut b = browser();
        b.finish_load(99, page("https://x.test/", ""), true);
        assert_eq!(b.address, "a.test");
    }

    #[test]
    fn session_survives_a_restart() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('t'));
        b.key(KeyPress::Text("b.test".into()));
        let (tab, ..) = load_target(&b.key(KeyPress::Enter)).expect("chargement");
        b.finish_load(tab, page("https://b.test/", "<title>B</title>"), true);
        b.key(KeyPress::CtrlShift('p'));
        // Un nouvel onglet vide n'a rien à retrouver : il n'est pas conservé.
        b.key(KeyPress::Ctrl('t'));
        let session = b.session();
        assert_eq!(
            session
                .tabs
                .iter()
                .map(|t| (t.url.as_str(), t.title.as_str(), t.pinned))
                .collect::<Vec<_>>(),
            [
                ("https://a.test/", "A", false),
                ("https://b.test/", "B", true)
            ]
        );

        // Au retour, les titres sont là avant les pages, et seul l'onglet
        // visible se charge.
        let mut restored = Browser::from_session(&session).expect("session");
        assert_eq!(restored.session().tabs, session.tabs);
        assert!(restored.initial_loads().is_empty());
        assert_eq!(
            load_target(&restored.wake_visible()).map(|(_, url, push)| (url, push)),
            Some(("https://a.test/".into(), false))
        );
        assert!(restored.wake_visible().is_empty(), "une seule fois");
        // L'onglet épinglé dort jusqu'à ce qu'on l'affiche.
        restored.click(Hit::Tab(1), None);
        assert_eq!(
            load_target(&restored.wake_visible()).map(|(_, url, _)| url),
            Some("https://b.test/".into())
        );
        assert!(Browser::from_session(&Session::default()).is_none());
    }

    #[test]
    fn visits_and_bookmarks_are_recorded() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('l'));
        b.key(KeyPress::Text("b.test".into()));
        b.key(KeyPress::Enter);
        b.finish_load(1, page("https://b.test/", "<title>B</title>"), true);
        // Revenir en arrière n'est pas une nouvelle visite ; un échec non plus.
        b.click(Hit::Back, None);
        b.finish_load(1, page("https://a.test/", "<title>A</title>"), false);
        b.finish_load(1, failed("https://c.test/", ErrorKind::Network), true);
        assert_eq!(
            b.history
                .visits()
                .iter()
                .map(|v| (v.url.as_str(), v.title.as_str()))
                .collect::<Vec<_>>(),
            [("https://b.test/", "B")]
        );

        b.finish_load(1, page("https://d.test/", "<title>D</title>"), true);
        b.key(KeyPress::Ctrl('d'));
        assert!(b.bookmarks.contains("https://d.test/"));
        assert_eq!(
            b.bookmarks.items().first().map(|i| i.title.as_str()),
            Some("D")
        );
        b.key(KeyPress::Ctrl('d'));
        assert!(b.bookmarks.items().is_empty());
    }

    fn library_titles(b: &Browser) -> Vec<String> {
        let views = b.views();
        let state = b.chrome_state(&views, 1440.0, 900.0);
        state
            .library
            .map(|library| {
                library
                    .rows
                    .iter()
                    .map(|row| row.title.to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn library_lists_searches_and_opens_history() {
        let mut b = browser();
        b.history
            .record("https://docs.test/rust", "Le langage Rust", 100);
        b.history.record("https://news.test/", "Actualités", 200);
        b.key(KeyPress::Ctrl('h'));
        assert!(b.library_open());
        assert_eq!(library_titles(&b), ["Actualités", "Le langage Rust"]);

        // Taper filtre ; Échap vide la recherche avant de fermer.
        b.key(KeyPress::Text("rust".into()));
        assert_eq!(library_titles(&b), ["Le langage Rust"]);
        b.key(KeyPress::Escape);
        assert!(b.library_open());
        assert_eq!(library_titles(&b).len(), 2);

        b.click(Hit::LibraryRemove(0), None);
        assert_eq!(library_titles(&b), ["Le langage Rust"]);
        assert_eq!(b.history.visits().len(), 1);

        // La page s'ouvre dans un nouvel onglet : l'onglet actif garde la sienne.
        let commands = b.click(Hit::LibraryRow(0), None);
        assert_eq!(
            load_target(&commands).map(|(_, url, push)| (url, push)),
            Some(("https://docs.test/rust".into(), true))
        );
        assert!(!b.library_open());
        assert_eq!(b.tabs.len(), 2);

        // Effacer demande une confirmation ; un autre clic l'annule.
        b.key(KeyPress::Ctrl('h'));
        b.click(Hit::LibraryClear, None);
        b.click(Hit::LibrarySearch, None);
        b.click(Hit::LibraryClear, None);
        assert_eq!(b.history.visits().len(), 1);
        b.click(Hit::LibraryClear, None);
        assert!(b.history.visits().is_empty());
        b.key(KeyPress::Escape);
        assert!(!b.library_open());
    }

    #[test]
    fn library_bookmarks_and_shortcuts() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('d'));
        b.key(KeyPress::CtrlShift('o'));
        assert_eq!(library_titles(&b), ["A"]);
        // Pas de champ de recherche dans les favoris : le texte est ignoré,
        // et la page cachée ne reçoit pas les touches.
        b.key(KeyPress::Text("x".into()));
        assert!(!b.is_editing());
        let input = oxyde_content::KeyInput {
            key: "a",
            code: "KeyA",
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            repeat: false,
        };
        assert_eq!(b.page_key(true, &input), (false, false));

        b.click(Hit::LibrarySection(LibrarySection::History), None);
        assert!(b.is_editing(), "le champ de recherche a la saisie");
        b.click(Hit::LibrarySection(LibrarySection::Bookmarks), None);
        b.click(Hit::LibraryRemove(0), None);
        assert!(b.bookmarks.items().is_empty());
        assert!(library_titles(&b).is_empty());

        // Ctrl T quitte la bibliothèque et ouvre un onglet.
        b.key(KeyPress::Ctrl('t'));
        assert!(!b.library_open());
        assert_eq!(b.tabs.len(), 2);
        // Un clic sur un élément d'onglet pendant que la bibliothèque est
        // ouverte ne fait rien.
        b.key(KeyPress::CtrlShift('o'));
        assert!(b.click(Hit::CloseTab(0), None).is_empty());
        assert_eq!(b.tabs.len(), 2);
    }

    #[test]
    fn history_days_are_named() {
        let today = (2026, 11, 12);
        let yesterday = (2026, 11, 11);
        assert_eq!(day_label(today, today, yesterday), "Aujourd'hui");
        assert_eq!(day_label(yesterday, today, yesterday), "Hier");
        assert_eq!(
            day_label((2026, 11, 3), today, yesterday),
            "3 novembre 2026"
        );
        assert_eq!(day_label((2025, 8, 1), today, yesterday), "1er août 2025");
        // Un mois hors bornes ne fait rien paniquer.
        assert_eq!(day_label((2025, 0, 5), today, yesterday), "5  2025");
    }

    #[test]
    fn settings_screen_changes_and_keeps_settings() {
        let mut b = browser();
        b.click(Hit::Settings, None);
        assert!(b.library_open());
        b.click(
            Hit::Setting {
                row: ROW_ACCENT,
                option: 2,
            },
            None,
        );
        {
            let views = b.views();
            let state = b.chrome_state(&views, 1440.0, 900.0);
            assert_eq!(state.accent, theme::BLUE);
            assert_eq!(state.settings.map(|s| s.groups.len()), Some(5));
        }
        let toggle = |b: &mut Browser, row| {
            b.click(Hit::Setting { row, option: 0 }, None);
        };
        b.click(
            Hit::Setting {
                row: ROW_ENGINE,
                option: 2,
            },
            None,
        );
        toggle(&mut b, ROW_HISTORY);
        toggle(&mut b, ROW_RESTORE);
        assert!(!b.settings.flag(KEY_RESTORE, true));

        // Effacer l'historique demande une confirmation.
        b.history.record("https://x.test/", "X", 1);
        toggle(&mut b, ROW_CLEAR);
        assert_eq!(b.history.visits().len(), 1);
        toggle(&mut b, ROW_CLEAR);
        assert!(b.history.visits().is_empty());

        // Une rubrique choisie à gauche vient en haut du panneau.
        b.click(Hit::SettingsGroup(3), None);
        {
            let views = b.views();
            let state = b.chrome_state(&views, 1440.0, 900.0);
            let view = state.settings.expect("paramètres");
            assert_eq!((view.selected, view.jump), (3, Some(3)));
        }
        b.settle_settings_scroll(120.0);
        {
            let views = b.views();
            let state = b.chrome_state(&views, 1440.0, 900.0);
            let view = state.settings.expect("paramètres");
            assert_eq!((view.scroll, view.jump), (120.0, None));
        }
        b.key(KeyPress::Escape);
        assert!(!b.library_open());

        // La recherche va au moteur choisi, et la visite n'est plus notée.
        b.key(KeyPress::Ctrl('l'));
        b.key(KeyPress::Text("deux mots".into()));
        let (_, url, _) = load_target(&b.key(KeyPress::Enter)).expect("recherche");
        assert!(
            url.starts_with("https://www.mojeek.com/search?q=deux+mots"),
            "{url}"
        );
        b.finish_load(1, page("https://b.test/", "<title>B</title>"), true);
        assert!(b.history.visits().is_empty());

        // Les réglages reviennent tels quels après un passage par le disque.
        let reread = Settings::decode(&b.settings.encode());
        assert_eq!(reread.get(KEY_ACCENT), Some("bleu"));
        assert_eq!(reread.get(KEY_ENGINE), Some("mojeek"));
        assert!(!reread.flag(KEY_HISTORY, true));

        // Ctrl , ouvre les paramètres ; Ctrl H passe à l'historique.
        b.key(KeyPress::Ctrl(','));
        assert!(b.settings_screen.is_some());
        b.key(KeyPress::Ctrl('h'));
        assert!(b.settings_screen.is_none() && b.library.is_some());
    }

    fn palette_titles(b: &Browser) -> Vec<String> {
        let views = b.views();
        let state = b.chrome_state(&views, 1440.0, 900.0);
        state
            .palette
            .map(|palette| {
                palette
                    .rows
                    .iter()
                    .map(|row| row.title.to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn palette_finds_tabs_pages_and_commands() {
        let mut b = browser();
        b.history
            .record("https://docs.test/rust", "Le langage Rust", 100);
        b.bookmarks
            .add("https://wgpu.test/", "Dessiner un triangle", 1);
        b.key(KeyPress::Ctrl('k'));
        assert!(b.palette_open());
        assert_eq!(
            palette_titles(&b),
            [
                "A",
                "Nouvel onglet",
                "Scinder la vue",
                "Ouvrir l'historique",
                "Ouvrir les favoris",
                "Ouvrir les paramètres"
            ]
        );

        // Taper cherche partout ; la dernière ligne reprend le texte saisi.
        b.key(KeyPress::Text("rust".into()));
        assert_eq!(
            palette_titles(&b),
            ["Le langage Rust", "Rechercher « rust »"]
        );
        b.key(KeyPress::Text(" triangle".into()));
        assert_eq!(palette_titles(&b), ["Rechercher « rust triangle »"]);
        for _ in 0.." triangle".len() {
            b.key(KeyPress::Backspace);
        }
        // Les flèches tournent en rond.
        b.key(KeyPress::Up);
        b.key(KeyPress::Down);
        let commands = b.key(KeyPress::Enter);
        assert_eq!(
            load_target(&commands).map(|(_, url, push)| (url, push)),
            Some(("https://docs.test/rust".into(), true))
        );
        assert!(!b.palette_open() && !b.is_editing());
        assert_eq!(b.tabs.len(), 2, "ouverte dans un nouvel onglet");

        // Une commande.
        b.key(KeyPress::Ctrl('k'));
        b.key(KeyPress::Text("scind".into()));
        b.key(KeyPress::Enter);
        assert!(b.split.is_some());

        // Tab change de filtre ; « Commandes » les montre toutes.
        b.key(KeyPress::Ctrl('k'));
        for _ in 0..FILTER_COMMANDS {
            b.key(KeyPress::Tab);
        }
        assert_eq!(palette_titles(&b).len(), PALETTE_COMMANDS.len());
        b.click(Hit::PaletteFilter(FILTER_BOOKMARKS), None);
        assert_eq!(palette_titles(&b), ["Dessiner un triangle"]);
        b.click(Hit::PaletteInput, None);
        assert!(b.palette_open());
        b.click(Hit::PaletteDismiss, None);
        assert!(!b.palette_open());

        // Une adresse s'ouvre telle quelle.
        b.key(KeyPress::Ctrl('k'));
        b.key(KeyPress::Text("c.test/x".into()));
        assert_eq!(palette_titles(&b), ["Ouvrir c.test/x"]);
        assert_eq!(
            load_target(&b.key(KeyPress::Enter)).map(|(_, url, _)| url),
            Some("https://c.test/x".into())
        );

        // Aller à un onglet ; un autre raccourci ferme la palette et agit.
        b.key(KeyPress::Ctrl('k'));
        b.click(Hit::PaletteRow(0), None);
        assert_eq!(b.active, 0);
        b.key(KeyPress::Ctrl('k'));
        b.key(KeyPress::Ctrl('h'));
        assert!(!b.palette_open() && b.library.is_some());
    }

    #[test]
    fn private_window_records_nothing() {
        let mut b = browser();
        b.set_private();
        assert_eq!(b.window_title(), "A — Oxyde (navigation privée)");
        b.key(KeyPress::Ctrl('l'));
        b.key(KeyPress::Text("b.test".into()));
        b.key(KeyPress::Enter);
        b.finish_load(1, page("https://b.test/", "<title>B</title>"), true);
        assert!(b.history.visits().is_empty());
        assert!(!b.history.is_dirty());
        {
            let views = b.views();
            assert!(b.chrome_state(&views, 1440.0, 900.0).private);
        }
        // Les favoris, eux, restent possibles.
        b.key(KeyPress::Ctrl('d'));
        assert!(b.bookmarks.contains("https://b.test/"));

        // Ctrl Maj N et la palette demandent une fenêtre privée.
        assert!(matches!(
            b.key(KeyPress::CtrlShift('n')).as_slice(),
            [Command::PrivateWindow]
        ));
        b.key(KeyPress::Ctrl('k'));
        b.key(KeyPress::Text("privée".into()));
        assert!(matches!(
            b.key(KeyPress::Enter).as_slice(),
            [Command::PrivateWindow]
        ));
    }

    fn find_view(b: &Browser) -> Option<(String, bool)> {
        let views = b.views();
        let state = b.chrome_state(&views, 1440.0, 900.0);
        state.find.map(|find| (find.query.to_owned(), find.focused))
    }

    #[test]
    fn find_bar_follows_its_tab() {
        let mut b = browser();
        b.key(KeyPress::Ctrl('f'));
        assert_eq!(find_view(&b), Some((String::new(), true)));
        b.key(KeyPress::Text("lien".into()));
        b.key(KeyPress::Enter);
        b.key(KeyPress::ShiftEnter);
        assert_eq!(find_view(&b), Some(("lien".into(), true)));
        assert!(b.is_editing());

        // Un clic dans la page retire la saisie à la barre, sans la fermer ;
        // un clic sur son champ la lui rend, avec son texte.
        b.click(Hit::Reload, None);
        assert_eq!(find_view(&b), Some(("lien".into(), false)));
        assert!(!b.is_editing());
        assert_eq!(b.address, "a.test");
        b.click(Hit::FindInput, None);
        assert_eq!(find_view(&b), Some(("lien".into(), true)));
        b.key(KeyPress::Backspace);
        assert_eq!(find_view(&b), Some(("lie".into(), true)));
        b.click(Hit::FindNext, None);
        b.click(Hit::FindPrevious, None);

        // Ctrl L prend la saisie pour l'adresse ; Ctrl F la reprend.
        b.key(KeyPress::Ctrl('l'));
        assert_eq!(find_view(&b), Some(("lie".into(), false)));
        b.key(KeyPress::Escape);
        b.key(KeyPress::Ctrl('f'));
        assert_eq!(find_view(&b), Some(("lie".into(), true)));

        // La barre se ferme avec Échap, par son bouton, ou quand son onglet
        // n'est plus affiché.
        b.key(KeyPress::Escape);
        assert_eq!(find_view(&b), None);
        assert!(!b.is_editing());
        b.key(KeyPress::Ctrl('f'));
        b.click(Hit::FindClose, None);
        assert_eq!(find_view(&b), None);
        b.key(KeyPress::Ctrl('f'));
        b.key(KeyPress::Ctrl('t'));
        b.tick(std::time::Instant::now());
        assert_eq!(find_view(&b), None);
        // Un nouvel onglet vide n'a rien où chercher.
        b.key(KeyPress::Escape);
        b.key(KeyPress::Ctrl('f'));
        assert_eq!(find_view(&b), None);
    }

    #[test]
    fn onboarding_imports_then_never_returns() {
        let mut b = browser();
        let file = std::path::PathBuf::from("ailleurs/Bookmarks");
        b.start_onboarding(vec![("Chrome".to_owned(), file.clone())]);
        assert!(b.library_open());
        // Rien d'autre ne répond pendant l'accueil.
        assert!(b.click(Hit::Tab(0), None).is_empty());
        b.key(KeyPress::Ctrl('t'));
        assert_eq!(b.tabs.len(), 1);

        // « Ne rien importer », puis retour au navigateur trouvé.
        b.click(Hit::OnboardingSource(1), None);
        b.key(KeyPress::Down);
        let commands = b.click(Hit::OnboardingNext, None);
        assert!(
            matches!(commands.as_slice(), [Command::ImportBookmarks(path)] if *path == file),
            "{commands:?}"
        );
        // L'application lit le fichier et remet ce qu'elle a trouvé.
        b.bookmarks.add("https://deja.test/", "Déjà là", 1);
        b.finish_import(&[
            ("https://deja.test/".to_owned(), "Doublon".to_owned()),
            ("https://neuf.test/".to_owned(), "Neuf".to_owned()),
        ]);
        {
            let views = b.views();
            let state = b.chrome_state(&views, 1440.0, 900.0);
            let view = state.onboarding.expect("accueil");
            assert_eq!((view.step, view.imported), (1, Some(1)));
        }
        assert_eq!(b.bookmarks.items().len(), 2);

        // Couleur d'accent, puis terminé : l'accueil ne reviendra pas.
        b.click(Hit::OnboardingAccent(1), None);
        assert!(b.key(KeyPress::Enter).is_empty());
        assert!(!b.library_open());
        assert!(b.settings.flag(KEY_WELCOME, false));
        assert_eq!(b.settings.get(KEY_ACCENT), Some("ambre"));

        // Passer la première étape n'importe rien.
        b.start_onboarding(vec![("Chrome".to_owned(), file)]);
        assert!(b.click(Hit::OnboardingSkip, None).is_empty());
        assert!(b.key(KeyPress::Escape).is_empty());
        assert!(!b.library_open());
    }
}
