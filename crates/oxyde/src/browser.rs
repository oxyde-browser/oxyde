//! Modèle du navigateur : onglets, historique, vue scindée, barre d'adresse.
//!
//! Ce module ne touche ni à la fenêtre ni au réseau : il transforme les
//! actions de l'utilisateur en [`Command`] que l'application exécute.

use oxyde_content::{Page, Url};
use oxyde_ui::{ChromeState, Hit, TabView};

use crate::loader;

/// Action à exécuter par l'application.
#[derive(Debug)]
pub(crate) enum Command {
    /// Charger `url` dans l'onglet `tab` (en arrière-plan).
    Load {
        tab: u64,
        url: Url,
        push_history: bool,
    },
    /// Réduire la fenêtre.
    Minimize,
    /// Agrandir ou restaurer la fenêtre.
    ToggleMaximize,
    /// Fermer la fenêtre.
    CloseWindow,
}

/// Touche reconnue par le navigateur, indépendante de la bibliothèque de fenêtrage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum KeyPress {
    /// Caractère(s) saisi(s).
    Text(String),
    Enter,
    Escape,
    Backspace,
    /// Lettre accompagnée de Ctrl (en minuscule).
    Ctrl(char),
    CtrlTab,
    AltLeft,
    AltRight,
    F5,
}

pub(crate) struct Tab {
    id: u64,
    pub(crate) page: Page,
    /// Défilement vertical, en pixels CSS.
    pub(crate) scroll: f32,
    history: Vec<Url>,
    /// Index de la page courante dans `history`.
    position: usize,
    loading: bool,
    title: String,
    host: String,
}

impl Tab {
    fn new(id: u64, page: Page) -> Self {
        let mut tab = Self {
            id,
            history: vec![page.url().clone()],
            page,
            scroll: 0.0,
            position: 0,
            loading: false,
            title: String::new(),
            host: String::new(),
        };
        tab.refresh_labels();
        tab
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
}

/// Adresse telle qu'affichée dans la barre : sans schéma ni barre finale.
fn display_url(url: &Url) -> String {
    match url.scheme() {
        "about" => String::new(),
        "http" | "https" => {
            let rest = &url[url::Position::BeforeHost..];
            rest.trim_end_matches('/').to_owned()
        }
        _ => url.to_string(),
    }
}

pub(crate) struct Browser {
    tabs: Vec<Tab>,
    active: usize,
    /// Index de l'onglet du second volet.
    split: Option<usize>,
    /// Texte en cours de saisie dans la barre d'adresse.
    editing: Option<String>,
    select_all: bool,
    pub(crate) hover: Option<Hit>,
    next_id: u64,
    /// Texte affiché dans la barre d'adresse hors saisie (cache).
    address: String,
}

fn new_tab_page() -> Page {
    loader::load_page(Url::parse("about:newtab").expect("URL constante valide"))
}

impl Browser {
    pub(crate) fn new(first: Page) -> Self {
        let mut browser = Self {
            tabs: vec![Tab::new(1, first)],
            active: 0,
            split: None,
            editing: None,
            select_all: false,
            hover: None,
            next_id: 2,
            address: String::new(),
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

    /// Ouvre `page` dans un nouvel onglet affiché dans le second volet.
    pub(crate) fn open_in_split(&mut self, page: Page) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab::new(id, page));
        self.split = Some(self.tabs.len() - 1);
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

    /// Vues d'onglets pour l'interface.
    pub(crate) fn tab_views(&self) -> Vec<TabView<'_>> {
        self.tabs
            .iter()
            .map(|t| TabView {
                title: &t.title,
                host: &t.host,
                loading: t.loading,
            })
            .collect()
    }

    /// État complet de l'interface pour une fenêtre de `width` × `height`.
    pub(crate) fn chrome_state<'a>(
        &'a self,
        views: &'a [TabView<'a>],
        width: f32,
        height: f32,
    ) -> ChromeState<'a> {
        let active = self.tabs.get(self.active);
        ChromeState {
            width,
            height,
            tabs: views,
            active: self.active,
            split: self.split,
            address: self.editing.as_deref().unwrap_or(&self.address),
            editing: self.editing.is_some(),
            select_all: self.select_all,
            secure: active.is_some_and(|t| t.page.url().scheme() == "https"),
            can_back: active.is_some_and(|t| t.position > 0),
            can_forward: active.is_some_and(|t| t.position + 1 < t.history.len()),
            hover: self.hover,
            space: "Travail",
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
        self.editing = Some(current.unwrap_or_default());
        self.select_all = true;
    }

    fn load(&mut self, index: usize, url: Url, push_history: bool) -> Vec<Command> {
        let Some(tab) = self.tabs.get_mut(index) else {
            return Vec::new();
        };
        tab.loading = true;
        vec![Command::Load {
            tab: tab.id,
            url,
            push_history,
        }]
    }

    fn go(&mut self, delta: isize) -> Vec<Command> {
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Vec::new();
        };
        let Some(target) = tab.position.checked_add_signed(delta) else {
            return Vec::new();
        };
        let Some(url) = tab.history.get(target).cloned() else {
            return Vec::new();
        };
        tab.position = target;
        self.load(self.active, url, false)
    }

    fn reload(&mut self) -> Vec<Command> {
        let Some(url) = self.tabs.get(self.active).map(|t| t.page.url().clone()) else {
            return Vec::new();
        };
        self.load(self.active, url, false)
    }

    fn new_tab(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab::new(id, new_tab_page()));
        self.active = self.tabs.len() - 1;
        self.refresh_address();
        self.start_editing();
    }

    fn close_tab(&mut self, index: usize) -> Vec<Command> {
        if index >= self.tabs.len() {
            return Vec::new();
        }
        self.tabs.remove(index);
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
            self.active = self
                .split
                .take()
                .unwrap_or_else(|| index.min(self.tabs.len() - 1));
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

    fn toggle_split(&mut self) {
        if self.split.take().is_some() {
            return;
        }
        // Second volet : l'onglet voisin, ou un nouvel onglet s'il n'y en a qu'un.
        if self.tabs.len() < 2 {
            let id = self.next_id;
            self.next_id += 1;
            self.tabs.push(Tab::new(id, new_tab_page()));
        }
        let neighbour = if self.active + 1 < self.tabs.len() {
            self.active + 1
        } else {
            self.active - 1
        };
        self.split = Some(neighbour);
    }

    fn activate(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active {
            return;
        }
        // Cliquer sur l'autre volet échange les rôles sans défaire la vue scindée.
        if self.split == Some(index) {
            self.split = Some(self.active);
        }
        self.active = index;
        self.editing = None;
        self.refresh_address();
    }

    /// Clic sur un élément de l'interface. `page_point` : position dans la
    /// page (pixels CSS, défilement compris) quand le clic vise un volet.
    pub(crate) fn click(&mut self, hit: Hit, page_point: Option<(f32, f32)>) -> Vec<Command> {
        if hit != Hit::Address {
            self.editing = None;
        }
        match hit {
            Hit::Back => self.go(-1),
            Hit::Forward => self.go(1),
            Hit::Reload => self.reload(),
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
                let link = page_point
                    .and_then(|(x, y)| self.tabs.get(i).and_then(|t| t.page.link_at(x, y)));
                match link {
                    Some(url) => self.load(i, url, true),
                    None => Vec::new(),
                }
            }
            Hit::Minimize => vec![Command::Minimize],
            Hit::Maximize => vec![Command::ToggleMaximize],
            Hit::CloseWindow => vec![Command::CloseWindow],
            // Paramètres : écran prévu au jalon 7. Barre de titre : gérée par la fenêtre.
            Hit::Settings | Hit::TitleBar => Vec::new(),
        }
    }

    /// Touche du clavier.
    pub(crate) fn key(&mut self, key: KeyPress) -> Vec<Command> {
        match key {
            KeyPress::Ctrl('l') => self.start_editing(),
            KeyPress::Ctrl('t') => self.new_tab(),
            KeyPress::Ctrl('w') => return self.close_tab(self.active),
            KeyPress::Ctrl('r') | KeyPress::F5 => return self.reload(),
            KeyPress::CtrlTab => {
                if !self.tabs.is_empty() {
                    self.activate((self.active + 1) % self.tabs.len());
                }
            }
            KeyPress::AltLeft => return self.go(-1),
            KeyPress::AltRight => return self.go(1),
            KeyPress::Escape => self.editing = None,
            KeyPress::Backspace => {
                if let Some(text) = &mut self.editing {
                    if self.select_all {
                        text.clear();
                    } else {
                        text.pop();
                    }
                    self.select_all = false;
                }
            }
            KeyPress::Text(typed) => {
                if let Some(text) = &mut self.editing {
                    if self.select_all {
                        text.clear();
                    }
                    self.select_all = false;
                    text.extend(typed.chars().filter(|c| !c.is_control()));
                }
            }
            KeyPress::Enter => {
                if let Some(input) = self.editing.take()
                    && !input.trim().is_empty()
                {
                    return match loader::resolve_input(&input) {
                        Ok(url) => self.load(self.active, url, true),
                        Err(error) => {
                            self.finish_load_at(self.active, loader::error_page_for(&error), true);
                            Vec::new()
                        }
                    };
                }
            }
            KeyPress::Ctrl(_) => {}
        }
        Vec::new()
    }

    /// Molette au-dessus du volet de l'onglet `index` ; renvoie `true` si le
    /// défilement a changé.
    pub(crate) fn scroll_by(&mut self, index: usize, dy: f32, viewport_height: f32) -> bool {
        let Some(tab) = self.tabs.get_mut(index) else {
            return false;
        };
        let max = (tab.page.content_height() - viewport_height).max(0.0);
        let target = (tab.scroll + dy).clamp(0.0, max);
        let changed = (target - tab.scroll).abs() > f32::EPSILON;
        tab.scroll = target;
        changed
    }

    fn finish_load_at(&mut self, index: usize, page: Page, push_history: bool) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if push_history {
            tab.history.truncate(tab.position + 1);
            tab.history.push(page.url().clone());
            tab.position = tab.history.len() - 1;
        } else if let Some(entry) = tab.history.get_mut(tab.position) {
            // Une redirection peut avoir changé l'adresse finale.
            entry.clone_from(page.url());
        }
        tab.page = page;
        tab.scroll = 0.0;
        tab.loading = false;
        tab.refresh_labels();
        if index == self.active && self.editing.is_none() {
            self.refresh_address();
        }
    }

    /// Fin du chargement demandé par [`Command::Load`]. Sans effet si l'onglet
    /// a été fermé entre-temps.
    pub(crate) fn finish_load(&mut self, tab_id: u64, page: Page, push_history: bool) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) {
            self.finish_load_at(index, page, push_history);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str, html: &str) -> Page {
        Page::from_html(Url::parse(url).expect("url"), html)
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
    fn stale_load_results_are_ignored() {
        let mut b = browser();
        b.finish_load(99, page("https://x.test/", ""), true);
        assert_eq!(b.address, "a.test");
    }
}
