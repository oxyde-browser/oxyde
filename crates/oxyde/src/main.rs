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
//! oxyde --single-process …
//!         garde toutes les pages dans le processus du navigateur
//! oxyde --screenshot SORTIE.png --page-only --multi-process ADRESSE
//!         rend la page dans un processus de rendu, comme le fait la fenêtre
//! ```
//!
//! `oxyde --type=renderer` est réservé au navigateur : c'est ainsi qu'il
//! lance les processus de rendu de ses onglets.

mod a11y;
mod app;
mod browser;
mod frame;
mod loader;
mod renderer;

use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use oxyde_content::Url;
use oxyde_css::Viewport;
use oxyde_gpu::{Gpu, GpuScene, Transform, append_scrolled};
use oxyde_text::TextContext;
use oxyde_ui::theme;

use crate::browser::Browser;
use crate::renderer::host::{PageHost, RemotePage};

struct Args {
    target: Option<String>,
    split: Option<String>,
    pinned: Vec<String>,
    screenshot: Option<PathBuf>,
    page_only: bool,
    dump_dom: bool,
    /// Affiche le rapport de couverture du DOM et quitte.
    dom_coverage: bool,
    /// Garde toutes les pages dans le processus du navigateur.
    single_process: bool,
    /// Capture : rend la page dans un processus de rendu.
    multi_process: bool,
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
        single_process: false,
        multi_process: false,
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
            "--single-process" => args.single_process = true,
            "--multi-process" => args.multi_process = true,
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

/// Navigateur dans l'état demandé par la ligne de commande. `deferred` :
/// les pages du web ne sont pas chargées ici ; leurs onglets attendent que
/// la boucle d'événements les confie à des processus de rendu.
fn open_browser(args: &Args, deferred: bool) -> Browser {
    let open = |input: &str| {
        if deferred {
            loader::placeholder(input)
        } else {
            loader::load_input(input)
        }
    };
    let target = args.target.as_deref().unwrap_or("about:newtab");
    let mut browser = Browser::new(open(target));
    for address in &args.pinned {
        browser.open_pinned(open(address));
    }
    if let Some(second) = &args.split {
        browser.open_in_split(open(second));
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
    let clear = if args.page_only && args.multi_process {
        let input = args.target.as_deref().unwrap_or("about:newtab");
        let url = loader::resolve_input(input)
            .map_err(|error| anyhow::anyhow!("{} : {}", error.title, error.detail))?;
        let page = remote_page(url, &mut text, (w, h))?;
        let (scroll_x, scroll_y) = page.scroll_offset();
        append_scrolled(
            &mut scene,
            page.display_list(),
            Transform::translate((-f64::from(scroll_x), -f64::from(scroll_y))),
            &|id| page.scroll_of(id),
        );
        page.background()
    } else if args.page_only {
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
        let mut browser = open_browser(args, false);
        frame::compose(&mut browser, &mut text, &mut scene, w, h, 1.0);
        theme::WINDOW
    };
    let pixels = Gpu::new().render_to_rgba(&scene, width, height, clear)?;
    write_png(output, args.size, &pixels)
}

/// Fait charger `url` par un processus de rendu et attend que la page soit
/// stable : chargée, puis sans nouvelle image pendant un court instant.
fn remote_page(url: Url, text: &mut TextContext, viewport: (f32, f32)) -> Result<PageHost> {
    /// Durée sans nouvelle image après laquelle la page est tenue pour stable.
    const QUIET: Duration = Duration::from_millis(300);
    /// Attente maximale.
    const LIMIT: Duration = Duration::from_secs(60);
    let (wake, woken) = std::sync::mpsc::channel::<()>();
    let remote = RemotePage::spawn(url, None, viewport, move || {
        // Le récepteur disparaît avec la capture : rien à réveiller alors.
        let _ = wake.send(());
    })
    .context("lancement du processus de rendu impossible")?;
    let mut page = PageHost::Remote(Box::new(remote));
    let size = Viewport {
        width: viewport.0,
        height: viewport.1,
    };
    let started = Instant::now();
    let mut committed = false;
    let mut painted = false;
    loop {
        let timed_out = woken.recv_timeout(QUIET).is_err();
        page.tick(Instant::now());
        painted |= page.update(text, size);
        if let Some(remote) = page.remote_mut() {
            if remote.is_crashed() {
                bail!("le processus de rendu s'est arrêté");
            }
            for commit in remote.take_commits() {
                if let Some(error) = commit.error {
                    bail!("{} : {}", error.title, error.detail);
                }
                committed = true;
                painted = false;
            }
        }
        if committed && painted && timed_out {
            return Ok(page);
        }
        if started.elapsed() > LIMIT {
            bail!(
                "le processus de rendu n'a rien affiché en {} s",
                LIMIT.as_secs()
            );
        }
    }
}

fn main() -> Result<ExitCode> {
    // Processus de rendu : lancé par le navigateur, il ne lit rien d'autre
    // de sa ligne de commande.
    if std::env::args().nth(1).as_deref() == Some("--type=renderer") {
        return Ok(renderer::child::run());
    }
    run().map(|()| ExitCode::SUCCESS)
}

fn run() -> Result<()> {
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
    let multi_process = !args.single_process;
    app::run(open_browser(&args, multi_process), multi_process)
}
