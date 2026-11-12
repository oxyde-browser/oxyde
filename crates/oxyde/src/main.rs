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
//! oxyde --profile DOSSIER …
//!         range la session, l'historique et les favoris dans DOSSIER plutôt
//!         que dans le profil de l'utilisateur
//! oxyde --no-profile …
//!         ne lit ni n'écrit aucun profil
//! oxyde --screen historique|favoris …
//!         ouvre la bibliothèque sur l'historique ou les favoris
//! oxyde --single-process …
//!         garde toutes les pages dans le processus du navigateur
//! oxyde --no-sandbox …
//!         lance les processus de rendu hors du bac à sable (diagnostic)
//! oxyde --screenshot SORTIE.png --page-only --multi-process ADRESSE
//!         rend la page dans un processus de rendu, comme le fait la fenêtre
//! ```
//!
//! `oxyde --type=renderer` et `oxyde --type=network` sont réservés au
//! navigateur : c'est ainsi qu'il lance les processus de rendu de ses
//! onglets et son processus réseau.

mod a11y;
mod app;
mod browser;
mod frame;
mod loader;
mod network;
mod renderer;

use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use oxyde_content::Url;
use oxyde_css::Viewport;
use oxyde_gpu::{Gpu, GpuScene, Transform, append_scrolled};
use oxyde_storage::Profile;
use oxyde_text::TextContext;
use oxyde_ui::{LibrarySection, theme};

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
    /// Dossier du profil, s'il est imposé.
    profile: Option<PathBuf>,
    /// Aucun profil : rien n'est lu ni conservé.
    no_profile: bool,
    /// Rubrique de la bibliothèque à ouvrir au lancement.
    screen: Option<LibrarySection>,
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
        profile: None,
        no_profile: false,
        screen: None,
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
            "--profile" => {
                let dir = it.next().context("--profile attend un dossier")?;
                args.profile = Some(PathBuf::from(dir));
            }
            "--no-profile" => args.no_profile = true,
            "--screen" => {
                args.screen = Some(match it.next().as_deref() {
                    Some("historique") => LibrarySection::History,
                    Some("favoris") => LibrarySection::Bookmarks,
                    _ => bail!("--screen attend « historique » ou « favoris »"),
                });
            }
            "--page-only" => args.page_only = true,
            "--dump-dom" => args.dump_dom = true,
            "--dom-coverage" => args.dom_coverage = true,
            "--single-process" => args.single_process = true,
            "--multi-process" => args.multi_process = true,
            "--no-sandbox" => renderer::host::disable_sandbox(),
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
        network::use_process();
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
        // Avec `--profile`, la capture montre l'historique et les favoris du
        // profil ; elle n'y écrit rien.
        if let Some(dir) = &args.profile
            && let Ok(profile) = Profile::open(dir)
        {
            browser.history = profile.load_history();
            browser.bookmarks = profile.load_bookmarks();
        }
        if let Some(section) = args.screen {
            browser.open_screen(section);
        }
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
    if !remote.is_sandboxed() {
        eprintln!("processus de rendu hors bac à sable");
    }
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

/// Profil de la fenêtre : celui de `--profile`, sinon celui de
/// l'utilisateur ; aucun avec `--no-profile` ou si son dossier ne peut pas
/// être créé.
fn open_profile(args: &Args) -> Option<Profile> {
    if args.no_profile {
        return None;
    }
    let dir = args.profile.clone().or_else(Profile::default_dir)?;
    match Profile::open(&dir) {
        Ok(profile) => Some(profile),
        Err(error) => {
            eprintln!("profil indisponible ({}) : {error}", dir.display());
            None
        }
    }
}

fn main() -> Result<ExitCode> {
    // Processus de rendu ou processus réseau : lancé par le navigateur, il
    // ne lit rien d'autre de sa ligne de commande.
    match std::env::args().nth(1).as_deref() {
        Some("--type=renderer") => return Ok(renderer::child::run()),
        Some("--type=network") => return Ok(network::run()),
        _ => {}
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
    if multi_process {
        network::use_process();
    }
    let profile = open_profile(&args);
    let mut browser = match profile
        .as_ref()
        .and_then(|p| Browser::from_session(&p.load_session()))
    {
        // Une session attend : elle revient, et les adresses de la ligne de
        // commande s'y ajoutent.
        Some(mut browser) => {
            let open = |input: &str| {
                if multi_process {
                    loader::placeholder(input)
                } else {
                    loader::load_input(input)
                }
            };
            for address in &args.pinned {
                browser.open_pinned(open(address));
            }
            if let Some(target) = &args.target {
                browser.open_active(open(target));
            }
            if let Some(second) = &args.split {
                browser.open_in_split(open(second));
            }
            browser
        }
        None => open_browser(&args, multi_process),
    };
    if let Some(profile) = &profile {
        browser.history = profile.load_history();
        browser.bookmarks = profile.load_bookmarks();
    }
    if let Some(section) = args.screen {
        browser.open_screen(section);
    }
    app::run(browser, multi_process, profile)
}
