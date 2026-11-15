//! Fenêtre du navigateur : boucle d'événements winit, clavier, souris,
//! chargements en arrière-plan, accessibilité.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use oxyde_content::Url;
use oxyde_gpu::{Gpu, GpuScene, Surface};
use oxyde_storage::{Profile, Session};
use oxyde_text::TextContext;
use oxyde_ui::{Chrome, Hit, theme};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use crate::browser::{Browser, Command, KeyPress};
use crate::loader::{self, Loaded};
use crate::renderer::protocol::{MouseKind, SelectGesture};
use crate::{a11y, frame};

/// Pixels CSS défilés par cran de molette.
const WHEEL_STEP: f32 = 100.0;
/// Épaisseur de la zone de redimensionnement au bord de la fenêtre.
const RESIZE_BORDER: f32 = 5.0;
/// Intervalle maximal entre deux clics d'un double-clic.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// Durée minimale de sommeil entre deux passes de minuteurs.
const MIN_TIMER_SLEEP: Duration = Duration::from_millis(4);
/// Longueur maximale d'un texte collé dans la barre d'adresse.
const MAX_PASTE_CHARS: usize = 8192;

/// Événement reçu hors de la fenêtre : fin de chargement ou demande d'une
/// technologie d'assistance.
#[derive(Debug)]
pub(crate) enum UserEvent {
    Loaded {
        tab: u64,
        loaded: Box<Loaded>,
        push_history: bool,
    },
    /// Un processus de rendu a quelque chose à dire : la boucle se réveille
    /// et le lit avant de se rendormir.
    Renderer,
    /// Nouvelle de la connexion WebSocket `id` d'un onglet servi sur place.
    Socket {
        tab: u64,
        id: u64,
        event: oxyde_content::SocketEvent,
    },
    /// Réponse à une requête `fetch` d'un script.
    Fetched {
        tab: u64,
        id: u64,
        result: Box<Result<oxyde_content::FetchResponse, String>>,
    },
    Accessibility(accesskit_winit::Event),
}

impl From<accesskit_winit::Event> for UserEvent {
    fn from(event: accesskit_winit::Event) -> Self {
        Self::Accessibility(event)
    }
}

struct App {
    gpu: Gpu,
    text: TextContext,
    browser: Browser,
    proxy: EventLoopProxy<UserEvent>,
    /// Connexions WebSocket des onglets servis sur place.
    sockets: loader::PageSockets,
    /// Vrai si les pages du web vont chacune dans un processus de rendu.
    multi_process: bool,
    window: Option<Arc<Window>>,
    surface: Option<Surface>,
    accessibility: Option<accesskit_winit::Adapter>,
    scene: GpuScene,
    /// Interface de la dernière image, pour le test de clic.
    chrome: Chrome,
    /// Position du pointeur, en pixels physiques.
    cursor: PhysicalPosition<f64>,
    modifiers: ModifiersState,
    /// Élément sur lequel le bouton gauche a été enfoncé.
    pressed: Option<Hit>,
    last_title_click: Option<Instant>,
    /// Sélection de texte en cours à la souris : onglet, et point de la
    /// fenêtre où le bouton a été enfoncé.
    selecting: Option<(usize, (f32, f32))>,
    /// Vrai si le pointeur s'est éloigné de ce point : c'est un glissement,
    /// pas un clic.
    dragged: bool,
    /// Dernier clic dans une page : instant et point de la fenêtre.
    last_pane_click: Option<(Instant, (f32, f32))>,
    /// Rang du clic en cours dans une série rapprochée : 1, ou 2 pour le
    /// second clic d'un double clic.
    click_detail: u8,
    /// Onglet dont la page a reçu le bouton droit enfoncé.
    right_pressed: Option<usize>,
    /// Première erreur fatale rencontrée dans la boucle d'événements.
    error: Option<anyhow::Error>,
    /// Profil où le navigateur range sa session, son historique et ses
    /// favoris ; `None` : rien n'est conservé.
    profile: Option<Profile>,
    /// Dernière session écrite sur disque.
    saved_session: Session,
    /// Dernier passage de [`App::persist`].
    last_persist: Instant,
}

/// Intervalle minimal entre deux écritures du profil.
const PERSIST_INTERVAL: Duration = Duration::from_secs(1);

/// Écart, en pixels, à partir duquel un bouton enfoncé puis déplacé est un
/// glissement et non un clic.
const DRAG_SLOP: f32 = 3.0;

/// Vrai si `a` et `b` sont à moins de [`DRAG_SLOP`] l'un de l'autre.
fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() <= DRAG_SLOP && (a.1 - b.1).abs() <= DRAG_SLOP
}

/// Place `text` dans le presse-papiers ; un refus du système est signalé.
fn set_clipboard(text: String) {
    if let Err(error) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
        eprintln!("presse-papiers non écrit : {error}");
    }
}

/// Texte du presse-papiers, borné ; `None` s'il est vide ou inaccessible.
fn clipboard_text() -> Option<String> {
    let text = arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .ok()?;
    Some(text.chars().take(MAX_PASTE_CHARS).collect())
}

impl App {
    /// Lance un second navigateur en navigation privée. C'est un autre
    /// processus, avec son propre processus réseau : ses cookies, gardés en
    /// mémoire, ne se mêlent pas à ceux de cette fenêtre et disparaissent
    /// avec lui.
    fn spawn_private_window(&self) -> std::io::Result<()> {
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.arg("--private");
        match &self.profile {
            Some(profile) => command.arg("--profile").arg(profile.dir()),
            None => command.arg("--no-profile"),
        };
        if !self.multi_process {
            command.arg("--single-process");
        }
        command.spawn().map(drop)
    }

    /// Écrit dans le profil ce qui a changé depuis la dernière fois.
    /// `force` : sans attendre l'intervalle (à la fermeture).
    fn persist(&mut self, force: bool) {
        let Some(profile) = &self.profile else {
            return;
        };
        let now = Instant::now();
        if !force && now.duration_since(self.last_persist) < PERSIST_INTERVAL {
            return;
        }
        self.last_persist = now;
        // Une fenêtre privée ne laisse au profil que ses favoris, ajoutés à
        // ceux qu'une autre fenêtre a pu écrire entre-temps.
        if self.browser.is_private() {
            if let Err(error) = profile.merge_bookmarks(&mut self.browser.bookmarks) {
                eprintln!(
                    "favoris non enregistrés ({}) : {error}",
                    profile.dir().display()
                );
            }
            return;
        }
        let session = self.browser.session();
        let mut result = Ok(());
        if session != self.saved_session {
            result = profile.save_session(&session);
            self.saved_session = session;
        }
        let result = result
            .and_then(|()| profile.save_history(&mut self.browser.history))
            .and_then(|()| profile.save_bookmarks(&mut self.browser.bookmarks))
            .and_then(|()| profile.save_settings(&mut self.browser.settings));
        if let Err(error) = result {
            eprintln!(
                "profil non enregistré ({}) : {error}",
                profile.dir().display()
            );
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Position du pointeur en pixels logiques.
    fn cursor_logical(&self) -> Option<(f32, f32)> {
        let window = self.window.as_ref()?;
        let p = self.cursor.to_logical::<f32>(window.scale_factor());
        Some((p.x, p.y))
    }

    /// Position dans la page de l'onglet `tab` (pixels CSS, défilement compris).
    fn page_point(&self, tab: usize) -> Option<(f32, f32)> {
        let (x, y) = self.cursor_logical()?;
        let pane = self.chrome.panes.iter().find(|p| p.tab == tab)?;
        let (scroll_x, scroll_y) = self.browser.tab(tab)?.page.scroll_offset();
        Some((x - pane.rect.x + scroll_x, y - pane.rect.y + scroll_y))
    }

    /// Pointeur dans la page de l'onglet `tab`, pour un événement de
    /// souris du bouton `button` : sa position et les touches tenues.
    fn pointer(&self, tab: usize, button: u8, detail: u8) -> Option<oxyde_content::MouseInput> {
        let (x, y) = self.page_point(tab)?;
        Some(oxyde_content::MouseInput {
            x,
            y,
            button,
            buttons: 0,
            detail,
            ctrl: self.modifiers.control_key(),
            shift: self.modifiers.shift_key(),
            alt: self.modifiers.alt_key(),
            meta: self.modifiers.super_key(),
        })
    }

    /// Transmet un événement de souris à la page de l'onglet `tab`.
    fn page_mouse(&mut self, tab: usize, kind: MouseKind, button: u8, detail: u8) {
        if let Some(input) = self.pointer(tab, button, detail)
            && self.browser.page_mouse(tab, kind, &input)
        {
            self.request_redraw();
        }
    }

    /// Pointeur déplacé : la page qui est dessous, ou celle où un bouton
    /// est resté enfoncé, reçoit le déplacement.
    fn pointer_moved(&mut self) {
        let under = self
            .cursor_logical()
            .and_then(|(x, y)| self.chrome.hit(x, y));
        let tab = match (self.selecting, under) {
            (Some((tab, _)), _) | (None, Some(Hit::Pane(tab))) => tab,
            _ => return,
        };
        self.page_mouse(tab, MouseKind::Move, 0, 0);
    }

    /// Bord de redimensionnement sous le pointeur (fenêtre non agrandie).
    fn resize_direction(&self) -> Option<ResizeDirection> {
        let window = self.window.as_ref()?;
        if window.is_maximized() {
            return None;
        }
        let (x, y) = self.cursor_logical()?;
        let size = window.inner_size().to_logical::<f32>(window.scale_factor());
        let (left, right) = (x < RESIZE_BORDER, x >= size.width - RESIZE_BORDER);
        let (top, bottom) = (y < RESIZE_BORDER, y >= size.height - RESIZE_BORDER);
        Some(match (left, right, top, bottom) {
            (true, _, true, _) => ResizeDirection::NorthWest,
            (true, _, _, true) => ResizeDirection::SouthWest,
            (_, true, true, _) => ResizeDirection::NorthEast,
            (_, true, _, true) => ResizeDirection::SouthEast,
            (true, ..) => ResizeDirection::West,
            (_, true, ..) => ResizeDirection::East,
            (_, _, true, _) => ResizeDirection::North,
            (_, _, _, true) => ResizeDirection::South,
            _ => return None,
        })
    }

    fn execute(&mut self, event_loop: &ActiveEventLoop, commands: Vec<Command>) {
        for command in commands {
            match command {
                Command::Load {
                    tab,
                    url,
                    push_history,
                    post,
                } => self.spawn_load(tab, url, push_history, post),
                Command::Minimize => {
                    if let Some(window) = &self.window {
                        window.set_minimized(true);
                    }
                }
                Command::ToggleMaximize => {
                    if let Some(window) = &self.window {
                        window.set_maximized(!window.is_maximized());
                    }
                }
                Command::CloseWindow => event_loop.exit(),
                Command::PrivateWindow => {
                    if let Err(error) = self.spawn_private_window() {
                        eprintln!("fenêtre privée non ouverte : {error}");
                    }
                }
                Command::ImportBookmarks(file) => {
                    let found = crate::import_bookmarks(&file);
                    self.browser.finish_import(&found);
                }
                Command::CopyText(text) => set_clipboard(text),
                Command::PastePage => {
                    if let Some(text) = clipboard_text() {
                        self.browser.page_paste(&text);
                    }
                }
                Command::NetworkDiagnostics => {
                    // Ouvre le dépannage réseau des Paramètres de Windows. Un
                    // échec de lancement est sans conséquence pour le navigateur.
                    let _ = std::process::Command::new("explorer.exe")
                        .arg("ms-settings:network-troubleshoot")
                        .spawn();
                }
            }
        }
        if let Some(window) = &self.window {
            window.set_title(&self.browser.window_title());
        }
        self.request_redraw();
    }

    /// Charge `url` : dans un processus de rendu pour une page du web, sinon
    /// sur un fil dédié. Dans les deux cas l'interface reste réactive.
    fn spawn_load(
        &mut self,
        tab: u64,
        url: Url,
        push_history: bool,
        post: Option<oxyde_content::FormPost>,
    ) {
        if self.multi_process && crate::renderer::host::is_remote(&url) {
            let proxy = self.proxy.clone();
            let wake = move || {
                // La boucle d'événements peut s'être arrêtée entre-temps.
                let _ = proxy.send_event(UserEvent::Renderer);
            };
            if self
                .browser
                .load_remote(tab, &url, post.as_ref(), push_history, wake)
            {
                return;
            }
        }
        let proxy = self.proxy.clone();
        let failed_url = url.clone();
        let spawned = std::thread::Builder::new()
            .name("oxyde-chargement".to_owned())
            .spawn(move || {
                let loaded = Box::new(loader::load_with(url, post));
                // La boucle d'événements peut s'être arrêtée entre-temps : rien à faire alors.
                let _ = proxy.send_event(UserEvent::Loaded {
                    tab,
                    loaded,
                    push_history,
                });
            });
        if let Err(e) = spawned {
            // Sans fil disponible, on signale l'échec dans l'onglet plutôt que de bloquer.
            let error =
                loader::LoadError::other("Chargement impossible", e.to_string(), "ERR_FAILED");
            let loaded = Box::new(Loaded {
                page: oxyde_content::Page::from_html(failed_url, ""),
                error: Some(error),
            });
            let _ = self.proxy.send_event(UserEvent::Loaded {
                tab,
                loaded,
                push_history: false,
            });
        }
    }

    /// Exécute les ordres que les scripts viennent de donner à leurs
    /// connexions WebSocket ; chaque nouvelle revient par un événement
    /// `Socket`.
    fn run_socket_commands(&mut self) {
        for (tab, command) in self.browser.take_socket_commands() {
            let proxy = self.proxy.clone();
            self.sockets.apply(command, move |id| {
                Box::new(move |event| {
                    // La boucle d'événements peut s'être arrêtée entre-temps.
                    let _ = proxy.send_event(UserEvent::Socket { tab, id, event });
                })
            });
        }
    }

    /// Lance sur des fils dédiés les requêtes `fetch` que les scripts viennent
    /// d'émettre ; chaque réponse revient par un événement `Fetched`.
    fn spawn_fetches(&mut self) {
        for (tab, request) in self.browser.take_fetches() {
            let proxy = self.proxy.clone();
            let id = request.id;
            let spawned = std::thread::Builder::new()
                .name("oxyde-fetch".to_owned())
                .spawn(move || {
                    let result = Box::new(loader::fetch_for_script(&request));
                    // La boucle d'événements peut s'être arrêtée entre-temps.
                    let _ = proxy.send_event(UserEvent::Fetched { tab, id, result });
                });
            if let Err(e) = spawned {
                let result = Box::new(Err(e.to_string()));
                let _ = self
                    .proxy
                    .send_event(UserEvent::Fetched { tab, id, result });
            }
        }
    }

    fn redraw(&mut self) -> Result<()> {
        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut()) else {
            return Ok(());
        };
        let scale = window.scale_factor();
        let size = window.inner_size().to_logical::<f32>(scale);
        if size.width <= 0.0 || size.height <= 0.0 {
            return Ok(());
        }
        let found = self.browser.find_status();
        self.chrome = frame::compose(
            &mut self.browser,
            &mut self.text,
            &mut self.scene,
            size.width,
            size.height,
            scale,
        );
        // La mise en page de cette image a changé le compte des occurrences
        // cherchées : la barre, dessinée avant, le dira à l'image suivante.
        if self.browser.find_status() != found {
            window.request_redraw();
        }
        // Les paramètres viennent de sauter à une rubrique : le défilement
        // retenu par l'interface devient celui du navigateur.
        if let Some(scroll) = self.chrome.settings_scroll {
            self.browser.settle_settings_scroll(scroll);
        }
        // Une liste raccourcie (ligne retirée, fenêtre agrandie) peut laisser le
        // défilement trop loin : il revient dans ses bornes à l'image suivante.
        if self
            .browser
            .scroll_library(0.0, self.chrome.library_scroll_max)
        {
            window.request_redraw();
        }
        self.gpu.render(surface, &self.scene, theme::WINDOW)?;
        // La liste a pu raccourcir (onglet fermé, fenêtre agrandie) : le défilement suit.
        self.browser.tab_scroll = self
            .browser
            .tab_scroll
            .clamp(0.0, self.chrome.tab_scroll_max);
        self.update_accessibility();
        Ok(())
    }

    /// Publie l'arbre d'accessibilité si une technologie d'assistance écoute.
    fn update_accessibility(&mut self) {
        let (Some(adapter), Some(window)) = (self.accessibility.as_mut(), self.window.as_ref())
        else {
            return;
        };
        let (chrome, browser) = (&self.chrome, &self.browser);
        let scale = window.scale_factor();
        adapter.update_if_active(|| {
            a11y::tree_update(chrome, scale, &browser.window_title(), browser.is_editing())
        });
    }

    fn update_hover(&mut self) {
        let hit = self
            .cursor_logical()
            .and_then(|(x, y)| self.chrome.hit(x, y));
        let icon = if let Some(direction) = self.resize_direction() {
            CursorIcon::from(direction)
        } else {
            match hit {
                Some(Hit::Pane(tab)) => {
                    let over_link = self.page_point(tab).is_some_and(|(x, y)| {
                        self.browser
                            .tab(tab)
                            .is_some_and(|tab| tab.page.over_link(x, y))
                    });
                    if over_link {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    }
                }
                Some(Hit::Address) => CursorIcon::Text,
                _ => CursorIcon::Default,
            }
        };
        if let Some(window) = &self.window {
            window.set_cursor(icon);
        }
        // Le survol d'un volet ne change rien au dessin de l'interface.
        let hover = hit.filter(|h| !matches!(h, Hit::Pane(_) | Hit::TitleBar));
        if hover != self.browser.hover {
            self.browser.hover = hover;
            self.request_redraw();
        }
    }

    fn mouse_pressed(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if let Some(direction) = self.resize_direction() {
            // Un refus du système (plateforme sans redimensionnement) est sans conséquence.
            let _ = window.drag_resize_window(direction);
            return;
        }
        let hit = self
            .cursor_logical()
            .and_then(|(x, y)| self.chrome.hit(x, y));
        self.pressed = hit;
        self.selecting = None;
        self.dragged = false;
        if let Some(Hit::Pane(tab)) = hit
            && let (Some(at), Some((x, y))) = (self.cursor_logical(), self.page_point(tab))
        {
            // Bouton enfoncé dans une page : une sélection commence ; au
            // second clic d'un double clic, c'est le mot qui est pris.
            let now = Instant::now();
            let double = self
                .last_pane_click
                .is_some_and(|(t, p)| now.duration_since(t) < DOUBLE_CLICK && near(p, at));
            self.last_pane_click = if double { None } else { Some((now, at)) };
            // Les scripts voient le bouton enfoncé avant tout le reste.
            self.click_detail = if double { 2 } else { 1 };
            self.page_mouse(tab, MouseKind::Down, 0, self.click_detail);
            let gesture = if double {
                SelectGesture::Word
            } else {
                self.selecting = Some((tab, at));
                SelectGesture::Start
            };
            if self.browser.page_select(tab, gesture, x, y) {
                self.request_redraw();
            }
        }
        if hit == Some(Hit::TitleBar) {
            let now = Instant::now();
            let double = self
                .last_title_click
                .is_some_and(|t| now.duration_since(t) < DOUBLE_CLICK);
            self.last_title_click = if double { None } else { Some(now) };
            if double {
                window.set_maximized(!window.is_maximized());
            } else {
                let _ = window.drag_window();
            }
        }
    }

    /// Clic droit : dans une page, ouvre son menu contextuel ; ailleurs,
    /// ferme celui qui était ouvert.
    fn context_menu(&mut self) {
        let Some(at) = self.cursor_logical() else {
            return;
        };
        let changed = match self.chrome.hit(at.0, at.1) {
            Some(Hit::Pane(tab)) => {
                // Les scripts voient le bouton droit enfoncé, puis
                // `contextmenu`, qu'ils peuvent annuler.
                self.right_pressed = Some(tab);
                self.page_mouse(tab, MouseKind::Down, 2, 1);
                self.page_point(tab)
                    .is_some_and(|point| self.browser.open_context_menu(tab, at, point))
            }
            _ => self.browser.close_menu(),
        };
        if changed {
            self.request_redraw();
        }
    }

    /// Pointeur déplacé, bouton enfoncé dans une page : la sélection de
    /// texte suit le pointeur, même sorti du volet.
    fn drag_selection(&mut self) {
        let (Some((tab, from)), Some(at)) = (self.selecting, self.cursor_logical()) else {
            return;
        };
        if !self.dragged && near(from, at) {
            return;
        }
        self.dragged = true;
        if let Some((x, y)) = self.page_point(tab)
            && self.browser.page_select(tab, SelectGesture::Extend, x, y)
        {
            self.request_redraw();
        }
    }

    fn mouse_released(&mut self, event_loop: &ActiveEventLoop) {
        let hit = self
            .cursor_logical()
            .and_then(|(x, y)| self.chrome.hit(x, y));
        // Un clic compte si le bouton est relâché sur l'élément où il a été enfoncé.
        // Un glissement a sélectionné du texte : ce n'est pas un clic.
        let dragged = self.selecting.take().is_some() && std::mem::take(&mut self.dragged);
        let pressed = self.pressed.take();
        // La page où le bouton a été enfoncé le voit relâché, où que soit
        // le pointeur.
        if let Some(Hit::Pane(tab)) = pressed {
            self.page_mouse(tab, MouseKind::Up, 0, self.click_detail);
        }
        let (Some(hit), true) = (hit, hit == pressed) else {
            return;
        };
        if dragged {
            return;
        }
        let page_point = match hit {
            Hit::Pane(tab) => self.page_point(tab),
            _ => None,
        };
        let commands = self.browser.click(hit, page_point);
        self.execute(event_loop, commands);
    }

    fn key_pressed(&mut self, event_loop: &ActiveEventLoop, key: &Key, text: Option<&str>) {
        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();
        let press = match key {
            Key::Named(NamedKey::Enter) if shift => Some(KeyPress::ShiftEnter),
            Key::Named(NamedKey::Enter) => Some(KeyPress::Enter),
            Key::Named(NamedKey::Escape) => Some(KeyPress::Escape),
            Key::Named(NamedKey::Backspace) => Some(KeyPress::Backspace),
            Key::Named(NamedKey::F5) => Some(KeyPress::F5),
            Key::Named(NamedKey::Tab) if ctrl => Some(KeyPress::CtrlTab),
            Key::Named(NamedKey::Tab) => Some(KeyPress::Tab),
            Key::Named(NamedKey::ArrowUp) => Some(KeyPress::Up),
            Key::Named(NamedKey::ArrowDown) => Some(KeyPress::Down),
            Key::Named(NamedKey::ArrowLeft) if alt => Some(KeyPress::AltLeft),
            Key::Named(NamedKey::ArrowRight) if alt => Some(KeyPress::AltRight),
            Key::Named(NamedKey::ArrowLeft) => Some(KeyPress::Left),
            Key::Named(NamedKey::ArrowRight) => Some(KeyPress::Right),
            Key::Named(NamedKey::Home) => Some(KeyPress::Home),
            Key::Named(NamedKey::End) => Some(KeyPress::End),
            Key::Named(NamedKey::Delete) => Some(KeyPress::Delete),
            Key::Character(c) if ctrl && !alt => {
                match c.chars().next().map(|ch| ch.to_ascii_lowercase()) {
                    // Le presse-papiers n'est lu que sur ce geste : pour la
                    // saisie d'une adresse, sinon pour le champ de la page
                    // qui a le focus.
                    Some('v') if !shift => {
                        if self.browser.is_editing() {
                            clipboard_text().map(KeyPress::Paste)
                        } else {
                            if let Some(text) = clipboard_text()
                                && self.browser.page_paste(&text)
                            {
                                self.request_redraw();
                            }
                            None
                        }
                    }
                    Some(ch) if shift => Some(KeyPress::CtrlShift(ch)),
                    Some(ch) => Some(KeyPress::Ctrl(ch)),
                    None => None,
                }
            }
            _ => text
                .filter(|t| !ctrl && !t.is_empty())
                .map(|t| KeyPress::Text(t.to_owned())),
        };
        if let Some(press) = press {
            let commands = self.browser.key(press);
            self.execute(event_loop, commands);
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        self.error.get_or_insert(error);
        event_loop.exit();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // Barre de titre dessinée par Oxyde (maquette) : pas de décorations du
        // système. La fenêtre naît masquée : AccessKit doit s'y attacher avant
        // son premier affichage.
        let attributes = Window::default_attributes()
            .with_title(self.browser.window_title())
            .with_decorations(false)
            .with_visible(false)
            .with_min_inner_size(LogicalSize::new(640.0, 400.0))
            .with_inner_size(LogicalSize::new(1440.0, 900.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => return self.fail(event_loop, e.into()),
        };
        self.accessibility = Some(accesskit_winit::Adapter::with_event_loop_proxy(
            event_loop,
            &window,
            self.proxy.clone(),
        ));
        window.set_visible(true);
        let size = window.inner_size();
        match self
            .gpu
            .create_surface(Arc::clone(&window), size.width, size.height)
        {
            Ok(surface) => self.surface = Some(surface),
            Err(e) => return self.fail(event_loop, e.into()),
        }
        self.window = Some(window);
        // Les onglets ouverts au démarrage attendent leur page.
        let commands = self.browser.initial_loads();
        if !commands.is_empty() {
            self.execute(event_loop, commands);
        }
    }

    /// Avant de s'endormir : exécute les minuteurs des pages et programme le
    /// réveil pour le prochain.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self.browser.tick(now) {
            if let Some(window) = &self.window {
                window.set_title(&self.browser.window_title());
            }
            self.request_redraw();
        }
        self.spawn_fetches();
        self.run_socket_commands();
        // Texte copié dans une page (Ctrl C).
        if let Some(text) = self.browser.take_copied() {
            set_clipboard(text);
        }
        // Navigations demandées par les scripts depuis le dernier passage, et
        // onglets d'une session reprise qui viennent d'apparaître.
        let mut navigations = self.browser.drain_navigations();
        navigations.extend(self.browser.wake_visible());
        if !navigations.is_empty() {
            self.execute(event_loop, navigations);
        }
        self.persist(false);
        event_loop.set_control_flow(match self.browser.next_wake() {
            // Jamais plus tôt que dans 4 ms : un minuteur en boucle ne monopolise pas le fil.
            Some(wake) => ControlFlow::WaitUntil(wake.max(now + MIN_TIMER_SLEEP)),
            None => ControlFlow::Wait,
        });
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.persist(true);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Loaded {
                tab,
                loaded,
                push_history,
            } => {
                self.browser.finish_load(tab, *loaded, push_history);
                self.execute(event_loop, Vec::new());
            }
            // Rien à faire ici : `about_to_wait` lit les messages des
            // processus de rendu avant que la boucle ne se rendorme.
            UserEvent::Renderer => {}
            UserEvent::Socket { tab, id, event } => {
                let last = matches!(event, oxyde_content::SocketEvent::Close { .. });
                match self.browser.socket_event(tab, id, event) {
                    Some(redraw) => {
                        if last {
                            self.sockets.forget(id);
                        }
                        if redraw {
                            self.request_redraw();
                        }
                    }
                    // Onglet fermé ou page quittée : la connexion est coupée.
                    None => self.sockets.forget(id),
                }
            }
            UserEvent::Fetched { tab, id, result } => {
                if self.browser.complete_fetch(tab, id, *result) {
                    if let Some(window) = &self.window {
                        window.set_title(&self.browser.window_title());
                    }
                    self.request_redraw();
                }
            }
            UserEvent::Accessibility(event) => match event.window_event {
                accesskit_winit::WindowEvent::InitialTreeRequested => self.update_accessibility(),
                accesskit_winit::WindowEvent::ActionRequested(request) => {
                    if matches!(
                        request.action,
                        accesskit::Action::Click | accesskit::Action::Focus
                    ) && let Some(hit) = a11y::hit_for(&self.chrome, request.target_node)
                    {
                        let commands = self.browser.click(hit, None);
                        self.execute(event_loop, commands);
                    }
                }
                accesskit_winit::WindowEvent::AccessibilityDeactivated => {}
            },
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let (Some(adapter), Some(window)) = (self.accessibility.as_mut(), self.window.as_ref()) {
            adapter.process_event(window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(surface) = self.surface.as_mut() {
                    self.gpu.resize(surface, size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => self.request_redraw(),
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, e);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                let down = event.state == ElementState::Pressed;
                // La page reçoit d'abord la touche (`keydown`, `keyup`).
                let key = match &event.logical_key {
                    Key::Character(text) => text.to_string(),
                    Key::Named(NamedKey::Space) => " ".to_owned(),
                    Key::Named(named) => format!("{named:?}"),
                    _ => "Unidentified".to_owned(),
                };
                let code = match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(code) => format!("{code:?}"),
                    winit::keyboard::PhysicalKey::Unidentified(_) => String::new(),
                };
                let input = oxyde_content::KeyInput {
                    key: &key,
                    code: &code,
                    ctrl: self.modifiers.control_key(),
                    shift: self.modifiers.shift_key(),
                    alt: self.modifiers.alt_key(),
                    meta: self.modifiers.super_key(),
                    repeat: event.repeat,
                };
                let (_prevented, redraw) = self.browser.page_key(down, &input);
                if redraw {
                    self.request_redraw();
                }
                if down {
                    self.key_pressed(event_loop, &event.logical_key, event.text.as_deref());
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let Some(window) = self.window.as_ref() else {
                    return;
                };
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines * WHEEL_STEP,
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "delta de défilement borné"
                    )]
                    MouseScrollDelta::PixelDelta(p) => (p.y / window.scale_factor()) as f32,
                };
                let Some((x, y)) = self.cursor_logical() else {
                    return;
                };
                // La palette est posée sur les pages : la molette ne les atteint pas.
                if self.browser.palette_open() {
                    return;
                }
                // La bibliothèque occupe la fenêtre : la molette fait défiler sa liste.
                if self.browser.library_open() {
                    if self
                        .browser
                        .scroll_library(-dy, self.chrome.library_scroll_max)
                    {
                        self.request_redraw();
                    }
                    return;
                }
                // Au-dessus de la barre latérale, la molette fait défiler la liste d'onglets.
                if !self.browser.horizontal() && x < theme::SIDEBAR + 8.0 && y > theme::TITLE_BAR {
                    let max = self.chrome.tab_scroll_max;
                    let target = (self.browser.tab_scroll - dy).clamp(0.0, max);
                    if (target - self.browser.tab_scroll).abs() > f32::EPSILON {
                        self.browser.tab_scroll = target;
                        self.request_redraw();
                    }
                    return;
                }
                if let Some(Hit::Pane(tab)) = self.chrome.hit(x, y)
                    && let Some(pane) = self.chrome.panes.iter().find(|p| p.tab == tab)
                    && self.browser.scroll_by(
                        tab,
                        &oxyde_content::MouseInput {
                            x: x - pane.rect.x,
                            y: y - pane.rect.y,
                            ctrl: self.modifiers.control_key(),
                            shift: self.modifiers.shift_key(),
                            alt: self.modifiers.alt_key(),
                            meta: self.modifiers.super_key(),
                            ..oxyde_content::MouseInput::default()
                        },
                        -dy,
                    )
                {
                    self.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position;
                self.update_hover();
                self.pointer_moved();
                self.drag_selection();
            }
            WindowEvent::CursorLeft { .. } => {
                if self.browser.hover.take().is_some() {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => match state {
                ElementState::Pressed => self.context_menu(),
                ElementState::Released => {
                    if let Some(tab) = self.right_pressed.take() {
                        self.page_mouse(tab, MouseKind::Up, 2, 1);
                    }
                }
            },
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => self.mouse_pressed(),
                ElementState::Released => self.mouse_released(event_loop),
            },
            _ => {}
        }
    }
}

/// Ouvre la fenêtre sur `browser` et fait tourner la boucle d'événements.
/// `multi_process` : les pages du web sont chargées chacune dans un
/// processus de rendu. `profile` : où conserver la session, l'historique et
/// les favoris.
pub(crate) fn run(browser: Browser, multi_process: bool, profile: Option<Profile>) -> Result<()> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let mut app = App {
        gpu: Gpu::new(),
        text: TextContext::new(),
        browser,
        proxy: event_loop.create_proxy(),
        sockets: loader::PageSockets::default(),
        window: None,
        surface: None,
        accessibility: None,
        scene: GpuScene::new(),
        chrome: Chrome::default(),
        cursor: PhysicalPosition::new(0.0, 0.0),
        modifiers: ModifiersState::empty(),
        pressed: None,
        last_title_click: None,
        selecting: None,
        dragged: false,
        last_pane_click: None,
        click_detail: 1,
        right_pressed: None,
        error: None,
        multi_process,
        // La session du disque est celle dont le navigateur vient de renaître,
        // ou une autre : dans le doute, la première comparaison l'écrira.
        saved_session: Session::default(),
        last_persist: Instant::now(),
        profile,
    };
    event_loop.run_app(&mut app)?;
    app.error.map_or(Ok(()), Err)
}
