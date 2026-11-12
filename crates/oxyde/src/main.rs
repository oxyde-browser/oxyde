//! Oxyde, navigateur web écrit en Rust.
//!
//! ```text
//! oxyde [ADRESSE] [--split ADRESSE] [--pin ADRESSE]...
//!         ouvre la fenêtre du navigateur ; --split ouvre un second volet,
//!         --pin ouvre un onglet épinglé (répétable)
//! oxyde --screenshot SORTIE.png [--size LxH] [--page-only] [ADRESSE] [--split …] [--pin …]
//!         rend la fenêtre (ou la page seule) dans un fichier
//! oxyde --dump-dom ADRESSE
//!         affiche l'arbre DOM (format html5lib)
//! ```

mod a11y;
mod app;
mod browser;
mod frame;
mod loader;

use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oxyde_css::Viewport;
use oxyde_gpu::{Gpu, GpuScene, Transform, append_scrolled};
use oxyde_text::TextContext;
use oxyde_ui::theme;

use crate::browser::Browser;

struct Args {
    target: Option<String>,
    split: Option<String>,
    pinned: Vec<String>,
    screenshot: Option<PathBuf>,
    page_only: bool,
    dump_dom: bool,
    /// Affiche le rapport de couverture du DOM et quitte.
    dom_coverage: bool,
    size: (u32, u32),
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        target: None,
        split: None,
        pinned: Vec::new(),
        screenshot: None,
        page_only: false,
        dump_dom: false,
        dom_coverage: false,
        size: (1440, 900),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--screenshot" => {
                let path = it
                    .next()
                    .context("--screenshot attend un chemin de fichier")?;
                args.screenshot = Some(PathBuf::from(path));
            }
            "--split" => args.split = Some(it.next().context("--split attend une adresse")?),
            "--pin" => args
                .pinned
                .push(it.next().context("--pin attend une adresse")?),
            "--page-only" => args.page_only = true,
            "--dump-dom" => args.dump_dom = true,
            "--dom-coverage" => args.dom_coverage = true,
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

/// Navigateur dans l'état demandé par la ligne de commande.
fn open_browser(args: &Args) -> Browser {
    let target = args.target.as_deref().unwrap_or("about:newtab");
    let mut browser = Browser::new(loader::load_input(target));
    for address in &args.pinned {
        browser.open_pinned(loader::load_input(address));
    }
    if let Some(second) = &args.split {
        browser.open_in_split(loader::load_input(second));
    }
    browser
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
    // Les scripts de la page voient la taille de la fenêtre dès le chargement.
    loader::set_viewport_hint(w, h);
    let clear = if args.page_only {
        let mut page = loader::load_input(args.target.as_deref().unwrap_or("about:newtab")).page;
        loader::settle_fetches(&mut page);
        page.update(
            &mut text,
            Viewport {
                width: w,
                height: h,
            },
        );
        // Une capture montre la page telle que ses scripts l'ont fait défiler.
        let (scroll_x, scroll_y) = page.scroll_offset();
        append_scrolled(
            &mut scene,
            page.display_list(),
            Transform::translate((-f64::from(scroll_x), -f64::from(scroll_y))),
            &|id| page.scroll_of(id),
        );
        page.background()
    } else {
        let mut browser = open_browser(args);
        frame::compose(&mut browser, &mut text, &mut scene, w, h, 1.0);
        theme::WINDOW
    };
    let pixels = Gpu::new().render_to_rgba(&scene, width, height, clear)?;
    write_png(output, args.size, &pixels)
}

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.dom_coverage {
        #[expect(clippy::print_stdout, reason = "sortie demandée par --dom-coverage")]
        {
            print!("{}", oxyde_content::dom_coverage_report());
        }
        return Ok(());
    }
    if args.dump_dom {
        let mut loaded = loader::load_input(args.target.as_deref().unwrap_or("about:newtab"));
        loader::settle_fetches(&mut loaded.page);
        #[expect(clippy::print_stdout, reason = "sortie demandée par --dump-dom")]
        {
            print!("{}", loaded.page.document().dump());
        }
        return Ok(());
    }
    if let Some(output) = &args.screenshot {
        return screenshot(&args, output);
    }
    app::run(open_browser(&args))
}
