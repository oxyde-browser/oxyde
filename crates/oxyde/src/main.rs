//! Oxyde, navigateur web écrit en Rust.
//!
//! ```text
//! oxyde [ADRESSE] [--split ADRESSE]        ouvre la fenêtre du navigateur
//! oxyde --screenshot SORTIE.png [--size LxH] [--page-only] [ADRESSE] [--split ADRESSE]
//!                                          rend la fenêtre (ou la page seule) dans un fichier
//! oxyde --dump-dom ADRESSE                 affiche l'arbre DOM (format html5lib)
//! ```

mod app;
mod browser;
mod frame;
mod loader;

use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oxyde_content::Page;
use oxyde_css::{Color, Viewport};
use oxyde_gpu::{Gpu, GpuScene, Transform, append_display_list};
use oxyde_text::TextContext;
use oxyde_ui::theme;

use crate::browser::Browser;

struct Args {
    target: Option<String>,
    split: Option<String>,
    screenshot: Option<PathBuf>,
    page_only: bool,
    dump_dom: bool,
    size: (u32, u32),
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        target: None,
        split: None,
        screenshot: None,
        page_only: false,
        dump_dom: false,
        size: (1440, 900),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--screenshot" => {
                args.screenshot = Some(PathBuf::from(
                    it.next()
                        .context("--screenshot attend un chemin de fichier")?,
                ));
            }
            "--split" => args.split = Some(it.next().context("--split attend une adresse")?),
            "--page-only" => args.page_only = true,
            "--dump-dom" => args.dump_dom = true,
            "--size" => {
                let value = it.next().context("--size attend LARGEURxHAUTEUR")?;
                let (w, h) = value
                    .split_once('x')
                    .context("--size attend LARGEURxHAUTEUR")?;
                args.size = (
                    w.parse().context("largeur invalide")?,
                    h.parse().context("hauteur invalide")?,
                );
            }
            other if other.starts_with("--") => bail!("option inconnue : {other}"),
            _ => args.target = Some(arg),
        }
    }
    Ok(args)
}

fn load(target: &str) -> Page {
    match loader::resolve_input(target) {
        Ok(url) => loader::load_page(url),
        Err(error) => loader::error_page_for(&error),
    }
}

fn write_png(output: &Path, (width, height): (u32, u32), pixels: &[u8]) -> Result<()> {
    let file = std::fs::File::create(output)
        .with_context(|| format!("création de {} impossible", output.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(pixels)?;
    Ok(())
}

/// Rend hors écran la fenêtre du navigateur (ou la page seule) et l'écrit en PNG.
fn screenshot(args: &Args, output: &Path) -> Result<()> {
    let (width, height) = args.size;
    #[expect(
        clippy::cast_precision_loss,
        reason = "tailles d'écran petites devant 2^24"
    )]
    let (w, h) = (width as f32, height as f32);
    let mut text = TextContext::new();
    let mut scene = GpuScene::new();
    let mut page = load(args.target.as_deref().unwrap_or("about:newtab"));
    let clear: Color = if args.page_only {
        page.update(
            &mut text,
            Viewport {
                width: w,
                height: h,
            },
        );
        append_display_list(&mut scene, page.display_list(), Transform::IDENTITY);
        page.background()
    } else {
        let mut browser = Browser::new(page);
        if let Some(second) = &args.split {
            browser.open_in_split(load(second));
        }
        frame::compose(&mut browser, &mut text, &mut scene, w, h, 1.0);
        theme::WINDOW
    };
    let pixels = Gpu::new().render_to_rgba(&scene, width, height, clear)?;
    write_png(output, args.size, &pixels)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let target = args.target.as_deref().unwrap_or("about:newtab");
    if args.dump_dom {
        #[expect(clippy::print_stdout, reason = "sortie demandée par --dump-dom")]
        {
            print!("{}", load(target).document().dump());
        }
        return Ok(());
    }
    if let Some(output) = &args.screenshot {
        return screenshot(&args, output);
    }
    let mut browser = Browser::new(load(target));
    if let Some(second) = &args.split {
        browser.open_in_split(load(second));
    }
    app::run(browser)
}
