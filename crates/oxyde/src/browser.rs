//! Modèle du navigateur : onglets, espaces, historique, vue scindée, barre d'adresse.
//!
//! Ce module ne touche ni à la fenêtre ni au réseau : il transforme les
//! actions de l'utilisateur en [`Command`] que l'application exécute.
//!
//! LIMITE : rien n'est conservé d'une session à l'autre (onglets épinglés,
//! espaces, onglets fermés) ; la persistance vient avec le jalon 7.

use oxyde_content::{Navigation, Url};
use oxyde_css::Color;
use oxyde_ui::{
    ChromeState, ClosedView, ErrorStyle, ErrorView, Hit, PaneContent, SpaceView, TabView, theme,
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
}

/// Touche reconnue par le navigateur, indépendante de la bibliothèque de fenêtrage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyPress {
    /// Caractère(s) saisi(s).
    Text(String),
    /// Texte collé depuis le presse-papiers.
    Paste(String),
    Enter,
    Escape,
    Backspace,
    /// Lettre accompagnée de Ctrl (en minuscule).
    Ctrl(char),
    /// Lettre accompagnée de Ctrl et Maj (en minuscule).
    CtrlShift(char),
    CtrlTab,
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
        };
        browser.refresh_address();
        browser
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

    /// Titre de la fenêtre pour le système.
    pub(crate) fn window_title(&self) -> String {
        match self.tabs.get(self.active) {
            Some(tab) if !tab.title.is_empty() => format!("{} — Oxyde", tab.title),
            _ => "Oxyde".to_owned(),
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
        }
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
        }
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
        if hit != Hit::Address {
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
            // Paramètres : écran prévu au jalon 7. Barre de titre : gérée par la fenêtre.
            Hit::Settings | Hit::TitleBar => Vec::new(),
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
        match key {
            KeyPress::Ctrl('l' | 'k') => self.start_editing(),
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
                    return match loader::resolve_input(&input) {
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
            KeyPress::Ctrl(_) | KeyPress::CtrlShift(_) => {}
        }
        Vec::new()
    }

    /// Exécute les minuteurs échus de tous les onglets ; renvoie `true` si un
    /// onglet doit être redessiné.
    pub(crate) fn tick(&mut self, now: std::time::Instant) -> bool {
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
        tab.refresh_labels();
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
                (tab.error.is_none() && crate::renderer::host::is_remote(url)).then(|| url.clone())
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
        tab.refresh_labels();
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
        if self.editing.is_some() {
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
}
