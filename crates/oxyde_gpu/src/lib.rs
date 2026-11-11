//! Rastérisation GPU d'Oxyde.
//!
//! Convertit une [`DisplayList`] en scène Vello et la rend sur une surface
//! wgpu. C'est le pendant de la chaîne Skia → Dawn/ANGLE de Chromium ; sous
//! Windows, wgpu s'appuie sur Direct3D 12.

use oxyde_css::Color;
use oxyde_paint::{DisplayItem, DisplayList};
use vello::kurbo::{Affine, Cap, Join, Rect, RoundedRect, Stroke};
use vello::peniko::{self, Fill};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu::{self, CurrentSurfaceTexture, SurfaceTarget};
use vello::{AaConfig, Glyph, RenderParams, Renderer, RendererOptions, Scene};

/// Erreur du rendu GPU.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// Aucune carte graphique compatible, ou échec de création de la surface.
    #[error("initialisation du rendu GPU impossible : {0}")]
    Init(String),
    /// Échec du rendu d'une image.
    #[error("échec du rendu : {0}")]
    Render(String),
}

/// Surface de rendu associée à une fenêtre.
pub struct Surface {
    inner: RenderSurface<'static>,
}

impl Surface {
    /// Taille actuelle en pixels physiques.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.inner.config.width, self.inner.config.height)
    }
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Surface")
            .field("size", &self.size())
            .finish()
    }
}

/// Contexte GPU : instance wgpu, périphériques et moteur de rendu Vello.
pub struct Gpu {
    context: RenderContext,
    /// Un moteur de rendu par périphérique (même index que `context.devices`).
    renderers: Vec<Option<Renderer>>,
}

impl std::fmt::Debug for Gpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gpu")
            .field("devices", &self.context.devices.len())
            .finish()
    }
}

impl Default for Gpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Gpu {
    /// Crée l'instance wgpu (aucun périphérique n'est encore ouvert).
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
        }
    }

    /// Crée la surface de rendu d'une fenêtre de `width` × `height` pixels physiques.
    ///
    /// # Errors
    /// [`GpuError::Init`] si aucun périphérique compatible n'existe ou si la
    /// surface ne peut pas être créée.
    pub fn create_surface(
        &mut self,
        target: impl Into<SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Surface, GpuError> {
        let inner = pollster::block_on(self.context.create_surface(
            target,
            width.max(1),
            height.max(1),
            wgpu::PresentMode::AutoVsync,
        ))
        .map_err(|e| GpuError::Init(e.to_string()))?;
        self.ensure_renderer(inner.dev_id)?;
        Ok(Surface { inner })
    }

    /// Ouvre (ou retrouve) le périphérique `dev_id` et son moteur de rendu.
    fn ensure_renderer(&mut self, dev_id: usize) -> Result<(), GpuError> {
        if self.renderers.len() < self.context.devices.len() {
            self.renderers
                .resize_with(self.context.devices.len(), || None);
        }
        let unknown = || GpuError::Init("périphérique inconnu".to_owned());
        let slot = self.renderers.get_mut(dev_id).ok_or_else(unknown)?;
        if slot.is_none() {
            let device = &self.context.devices.get(dev_id).ok_or_else(unknown)?.device;
            *slot = Some(
                Renderer::new(device, RendererOptions::default())
                    .map_err(|e| GpuError::Init(e.to_string()))?,
            );
        }
        Ok(())
    }

    /// Rend `scene` hors écran et renvoie les pixels RGBA8 (non prémultipliés
    /// tels que produits par Vello), ligne par ligne, sans marge.
    ///
    /// Sert aux captures d'écran et aux tests de référence.
    ///
    /// # Errors
    /// [`GpuError::Init`] sans carte graphique compatible ou si la taille est
    /// nulle ou supérieure à 8192 ; [`GpuError::Render`] si le rendu ou la
    /// relecture échoue.
    pub fn render_to_rgba(
        &mut self,
        scene: &Scene,
        width: u32,
        height: u32,
        clear: Color,
    ) -> Result<Vec<u8>, GpuError> {
        const MAX_SIDE: u32 = 8192;
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(GpuError::Init(format!(
                "taille de rendu invalide : {width}×{height}"
            )));
        }
        let dev_id = pollster::block_on(self.context.device(None))
            .ok_or_else(|| GpuError::Init("aucune carte graphique compatible".to_owned()))?;
        self.ensure_renderer(dev_id)?;
        let handle = self
            .context
            .devices
            .get(dev_id)
            .ok_or_else(|| GpuError::Init("périphérique inconnu".to_owned()))?;
        let renderer = self
            .renderers
            .get_mut(dev_id)
            .and_then(Option::as_mut)
            .ok_or_else(|| GpuError::Render("moteur de rendu non initialisé".to_owned()))?;

        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = handle.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("oxyde capture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        renderer
            .render_to_texture(
                &handle.device,
                &handle.queue,
                scene,
                &view,
                &RenderParams {
                    base_color: to_peniko(clear),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(|e| GpuError::Render(e.to_string()))?;

        // wgpu impose un pas de ligne multiple de 256 octets pour les copies.
        // Pas de débordement : width <= 8192, donc row_bytes <= 32768.
        let row_bytes = width * 4;
        let padded = row_bytes.div_ceil(256) * 256;
        let buffer = handle.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oxyde capture"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = handle
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oxyde capture"),
            });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            extent,
        );
        handle.queue.submit([encoder.finish()]);

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                // Le récepteur vit jusqu'à la fin de cette fonction ; un échec d'envoi est sans effet.
                let _ = sender.send(result);
            });
        handle
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| GpuError::Render(e.to_string()))?;
        receiver
            .recv()
            .map_err(|e| GpuError::Render(e.to_string()))?
            .map_err(|e| GpuError::Render(e.to_string()))?;

        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| GpuError::Render(e.to_string()))?;
        let mut pixels = Vec::with_capacity(row_bytes as usize * height as usize);
        for row in mapped.chunks(padded as usize) {
            pixels.extend_from_slice(row.get(..row_bytes as usize).unwrap_or(row));
        }
        drop(mapped);
        buffer.unmap();
        Ok(pixels)
    }

    /// Redimensionne la surface (sans effet si une dimension est nulle).
    pub fn resize(&self, surface: &mut Surface, width: u32, height: u32) {
        if width > 0 && height > 0 && surface.size() != (width, height) {
            self.context
                .resize_surface(&mut surface.inner, width, height);
        }
    }

    /// Rend `scene` sur la surface et présente l'image.
    ///
    /// # Errors
    /// [`GpuError::Render`] si Vello échoue. Une surface momentanément
    /// indisponible (fenêtre masquée, redimensionnement) n'est pas une erreur :
    /// l'image est simplement sautée.
    pub fn render(
        &mut self,
        surface: &mut Surface,
        scene: &Scene,
        clear: Color,
    ) -> Result<(), GpuError> {
        let (width, height) = surface.size();
        let handle = self
            .context
            .devices
            .get(surface.inner.dev_id)
            .ok_or_else(|| GpuError::Render("périphérique inconnu".to_owned()))?;
        let renderer = self
            .renderers
            .get_mut(surface.inner.dev_id)
            .and_then(Option::as_mut)
            .ok_or_else(|| GpuError::Render("moteur de rendu non initialisé".to_owned()))?;
        renderer
            .render_to_texture(
                &handle.device,
                &handle.queue,
                scene,
                &surface.inner.target_view,
                &RenderParams {
                    base_color: to_peniko(clear),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(|e| GpuError::Render(e.to_string()))?;

        let frame = match surface.inner.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.context.configure_surface(&surface.inner);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let mut encoder = handle
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oxyde blit"),
            });
        surface.inner.blitter.copy(
            &handle.device,
            &mut encoder,
            &surface.inner.target_view,
            &frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        handle.queue.submit([encoder.finish()]);
        handle.queue.present(frame);
        Ok(())
    }
}

fn to_peniko(c: Color) -> peniko::Color {
    peniko::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn shape_of(rect: oxyde_paint::Rect, radius: f32) -> RoundedRect {
    let (x, y, w, h) = (
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.w),
        f64::from(rect.h),
    );
    let max_radius = (w.min(h) / 2.0).max(0.0);
    RoundedRect::from_rect(
        Rect::new(x, y, x + w, y + h),
        f64::from(radius).clamp(0.0, max_radius),
    )
}

/// Ouvre une zone de découpe dans `scene` ; à refermer par [`pop_clip`].
pub fn push_clip(scene: &mut Scene, rect: oxyde_paint::Rect, radius: f32, transform: Affine) {
    scene.push_clip_layer(Fill::NonZero, transform, &shape_of(rect, radius));
}

/// Referme la zone de découpe ouverte par le dernier [`push_clip`].
pub fn pop_clip(scene: &mut Scene) {
    scene.pop_layer();
}

/// Ajoute `list` à `scene`, transformée par `transform` (défilement, échelle
/// de l'écran, position dans la fenêtre).
pub fn append_display_list(scene: &mut Scene, list: &DisplayList, transform: Affine) {
    let mut clip_depth = 0usize;
    for item in &list.items {
        match item {
            DisplayItem::Rect {
                rect,
                color,
                radius,
            } => {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    to_peniko(*color),
                    None,
                    &shape_of(*rect, *radius),
                );
            }
            DisplayItem::StrokeRect {
                rect,
                color,
                width,
                radius,
            } => {
                // Trait centré sur un rectangle rentré d'une demi-épaisseur.
                let half = width / 2.0;
                let inset = oxyde_paint::Rect {
                    x: rect.x + half,
                    y: rect.y + half,
                    w: (rect.w - width).max(0.0),
                    h: (rect.h - width).max(0.0),
                };
                scene.stroke(
                    &Stroke::new(f64::from(*width)),
                    transform,
                    to_peniko(*color),
                    None,
                    &shape_of(inset, (radius - half).max(0.0)),
                );
            }
            DisplayItem::Glyphs(run) => {
                let glyph_transform = run
                    .skew
                    .map(|angle| Affine::skew(f64::from(angle.to_radians().tan()), 0.0));
                scene
                    .draw_glyphs(&run.font)
                    .brush(to_peniko(run.color))
                    .hint(false)
                    .transform(transform)
                    .glyph_transform(glyph_transform)
                    .font_size(run.size)
                    .normalized_coords(&run.coords)
                    .draw(
                        Fill::NonZero,
                        run.glyphs.iter().map(|g| Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        }),
                    );
            }
            DisplayItem::Path {
                path,
                x,
                y,
                color,
                stroke,
            } => {
                let local = transform * Affine::translate((f64::from(*x), f64::from(*y)));
                match stroke {
                    Some(width) => scene.stroke(
                        &Stroke::new(f64::from(*width))
                            .with_caps(Cap::Round)
                            .with_join(Join::Round),
                        local,
                        to_peniko(*color),
                        None,
                        path,
                    ),
                    None => scene.fill(Fill::NonZero, local, to_peniko(*color), None, path),
                }
            }
            DisplayItem::PushClip { rect, radius } => {
                scene.push_clip_layer(Fill::NonZero, transform, &shape_of(*rect, *radius));
                clip_depth += 1;
            }
            DisplayItem::Image { rect, image } => {
                let (w, h) = (f64::from(image.width()), f64::from(image.height()));
                if w > 0.0 && h > 0.0 {
                    let local = transform
                        * Affine::translate((f64::from(rect.x), f64::from(rect.y)))
                        * Affine::scale_non_uniform(f64::from(rect.w) / w, f64::from(rect.h) / h);
                    scene.draw_image(image.data(), local);
                }
            }
            DisplayItem::PopClip => {
                // Une liste mal formée ne doit pas dépiler un calque qu'elle n'a pas ouvert.
                if clip_depth > 0 {
                    scene.pop_layer();
                    clip_depth -= 1;
                }
            }
        }
    }
    for _ in 0..clip_depth {
        scene.pop_layer();
    }
}

pub use vello::Scene as GpuScene;
pub use vello::kurbo::Affine as Transform;
