//! Fenêtre du navigateur : boucle d'événements winit, clavier, souris,
//! chargements en arrière-plan.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use oxyde_content::{Page, Url};
use oxyde_gpu::{Gpu, GpuScene, Surface};
use oxyde_text::TextContext;
use oxyde_ui::{Chrome, Hit, theme};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use crate::browser::{Browser, Command, KeyPress};
use crate::{frame, loader};

/// Pixels CSS défilés par cran de molette.
const WHEEL_STEP: f32 = 100.0;
/// Épaisseur de la zone de redimensionnement au bord de la fenêtre.
const RESIZE_BORDER: f32 = 5.0;
/// Intervalle maximal entre deux clics d'un double-clic.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// Événement envoyé par un fil de chargement.
#[derive(Debug)]
pub(crate) enum UserEvent {
    Loaded {
        tab: u64,
        page: Box<Page>,
        push_history: bool,
    },
}

struct App {
    gpu: Gpu,
    text: TextContext,
    browser: Browser,
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    surface: Option<Surface>,
    scene: GpuScene,
    /// Interface de la dernière image, pour le test de clic.
    chrome: Chrome,
    /// Position du pointeur, en pixels physiques.
    cursor: PhysicalPosition<f64>,
    modifiers: ModifiersState,
    /// Élément sur lequel le bouton gauche a été enfoncé.
    pressed: Option<Hit>,
    last_title_click: Option<Instant>,
    /// Première erreur fatale rencontrée dans la boucle d'événements.
    error: Option<anyhow::Error>,
}

impl App {
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
        let scroll = self.browser.tab(tab)?.scroll;
        Some((x - pane.rect.x, y - pane.rect.y + scroll))
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
                } => self.spawn_load(tab, url, push_history),
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
            }
        }
        if let Some(window) = &self.window {
            window.set_title(&self.browser.window_title());
        }
        self.request_redraw();
    }

    /// Charge `url` sur un fil dédié : l'interface reste réactive.
    fn spawn_load(&self, tab: u64, url: Url, push_history: bool) {
        let proxy = self.proxy.clone();
        let spawned = std::thread::Builder::new()
            .name("oxyde-chargement".to_owned())
            .spawn(move || {
                let page = Box::new(loader::load_page(url));
                // La boucle d'événements peut s'être arrêtée entre-temps : rien à faire alors.
                let _ = proxy.send_event(UserEvent::Loaded {
                    tab,
                    page,
                    push_history,
                });
            });
        if let Err(e) = spawned {
            // Sans fil disponible, on signale l'échec dans l'onglet plutôt que de bloquer.
            let error = loader::LoadError {
                title: "Chargement impossible",
                detail: e.to_string(),
            };
            let _ = self.proxy.send_event(UserEvent::Loaded {
                tab,
                page: Box::new(loader::error_page_for(&error)),
                push_history: false,
            });
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
        self.chrome = frame::compose(
            &mut self.browser,
            &mut self.text,
            &mut self.scene,
            size.width,
            size.height,
            scale,
        );
        self.gpu.render(surface, &self.scene, theme::WINDOW)?;
        Ok(())
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
                    let over_link = self
                        .page_point(tab)
                        .and_then(|(x, y)| self.browser.tab(tab)?.page.link_at(x, y))
                        .is_some();
                    if over_link {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    }
                }
                Some(Hit::Address) if self.browser.is_editing() => CursorIcon::Text,
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

    fn mouse_released(&mut self, event_loop: &ActiveEventLoop) {
        let hit = self
            .cursor_logical()
            .and_then(|(x, y)| self.chrome.hit(x, y));
        // Un clic compte si le bouton est relâché sur l'élément où il a été enfoncé.
        let (Some(hit), true) = (hit, hit == self.pressed.take()) else {
            return;
        };
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
        let press = match key {
            Key::Named(NamedKey::Enter) => Some(KeyPress::Enter),
            Key::Named(NamedKey::Escape) => Some(KeyPress::Escape),
            Key::Named(NamedKey::Backspace) => Some(KeyPress::Backspace),
            Key::Named(NamedKey::F5) => Some(KeyPress::F5),
            Key::Named(NamedKey::Tab) if ctrl => Some(KeyPress::CtrlTab),
            Key::Named(NamedKey::ArrowLeft) if alt => Some(KeyPress::AltLeft),
            Key::Named(NamedKey::ArrowRight) if alt => Some(KeyPress::AltRight),
            Key::Character(c) if ctrl && !alt => c
                .chars()
                .next()
                .map(|ch| KeyPress::Ctrl(ch.to_ascii_lowercase())),
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
        // Barre de titre dessinée par Oxyde (maquette) : pas de décorations du système.
        let attributes = Window::default_attributes()
            .with_title(self.browser.window_title())
            .with_decorations(false)
            .with_min_inner_size(LogicalSize::new(640.0, 400.0))
            .with_inner_size(LogicalSize::new(1440.0, 900.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => return self.fail(event_loop, e.into()),
        };
        let size = window.inner_size();
        match self
            .gpu
            .create_surface(Arc::clone(&window), size.width, size.height)
        {
            Ok(surface) => self.surface = Some(surface),
            Err(e) => return self.fail(event_loop, e.into()),
        }
        self.window = Some(window);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Loaded {
                tab,
                page,
                push_history,
            } => {
                self.browser.finish_load(tab, *page, push_history);
                self.execute(event_loop, Vec::new());
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.key_pressed(event_loop, &event.logical_key, event.text.as_deref());
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
                if let Some(Hit::Pane(tab)) = self.chrome.hit(x, y)
                    && let Some(pane) = self.chrome.panes.iter().find(|p| p.tab == tab)
                    && self.browser.scroll_by(tab, -dy, pane.rect.h)
                {
                    self.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = position;
                self.update_hover();
            }
            WindowEvent::CursorLeft { .. } => {
                if self.browser.hover.take().is_some() {
                    self.request_redraw();
                }
            }
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
pub(crate) fn run(browser: Browser) -> Result<()> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let mut app = App {
        gpu: Gpu::new(),
        text: TextContext::new(),
        browser,
        proxy: event_loop.create_proxy(),
        window: None,
        surface: None,
        scene: GpuScene::new(),
        chrome: Chrome::default(),
        cursor: PhysicalPosition::new(0.0, 0.0),
        modifiers: ModifiersState::empty(),
        pressed: None,
        last_title_click: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    app.error.map_or(Ok(()), Err)
}
