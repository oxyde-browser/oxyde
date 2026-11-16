//! Tests de référence : la page du test est dessinée, puis comparée point
//! par point à la page que le test désigne par `<link rel="match">` (elles
//! doivent être identiques) ou `<link rel="mismatch">` (elles doivent
//! différer).
//!
//! Le dessin passe par le même chemin que les captures d'écran du
//! navigateur : liste d'affichage, puis rendu hors écran par `oxyde_gpu`.
//!
//! LIMITES :
//! - une seule marche de la chaîne : la référence d'une référence n'est pas
//!   suivie ;
//! - `<meta name="fuzzy">` : un nombre seul vaut « de zéro à ce nombre », et
//!   deux images identiques passent toujours ;
//! - pas de tests d'impression (`-print`), ni de `reftest-wait` levé par un
//!   geste de `testdriver.js`.

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use oxyde_content::{Page, Url};
use oxyde_gpu::{Gpu, GpuScene, Transform, append_scrolled};
use oxyde_text::TextContext;

use crate::{CHILD_BUDGET, MAX_TICKS, ORIGIN, VIEWPORT, open, serve, serve_resources};

/// Largeur de l'image comparée, en points : celle de la fenêtre des tests.
const WIDTH: u32 = 800;
/// Hauteur de l'image comparée.
const HEIGHT: u32 = 600;
/// Message d'une réussite où la page du test est d'une seule couleur : la
/// comparaison ne prouve alors presque rien.
pub(crate) const UNIFORM: &str = "page unie";
/// Fin du message d'un échec dû à une page absente de la suite.
pub(crate) const UNREADABLE: &str = "fichier illisible";

/// Vrai si le fichier `name`, de contenu `bytes`, déclare une référence.
pub(crate) fn is_reftest(name: &str, bytes: &[u8]) -> bool {
    if ["-manual.", "-print."].iter().any(|m| name.contains(m)) {
        return false;
    }
    if ![".html", ".htm", ".xhtml", ".xht", ".svg"]
        .iter()
        .any(|extension| name.ends_with(extension))
    {
        return false;
    }
    let text = String::from_utf8_lossy(bytes);
    text.match_indices("rel=").any(|(at, _)| {
        let rest = text.get(at + 4..).unwrap_or("");
        let rest = rest.trim_start_matches(['"', '\'']);
        rest.starts_with("match") || rest.starts_with("mismatch")
    })
}

/// Ce qui dessine les pages : un seul contexte par processus.
struct Renderer {
    text: TextContext,
    gpu: Gpu,
    started: Instant,
}

/// Une page dessinée.
struct Shot {
    /// Points RGBA, ligne par ligne.
    pixels: Vec<u8>,
    /// Références déclarées : `true` pour `match`, et l'adresse.
    links: Vec<(bool, Url)>,
    /// Contenus des `<meta name="fuzzy">`.
    fuzzy: Vec<String>,
}

/// Écart entre deux images.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Diff {
    /// Nombre de points qui diffèrent.
    count: u64,
    /// Plus grand écart sur un canal.
    max: u32,
}

/// Tolérance déclarée par `<meta name="fuzzy">` : bornes incluses de l'écart
/// maximal par canal et du nombre de points différents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Fuzzy {
    max: (u32, u32),
    total: (u64, u64),
}

impl Fuzzy {
    /// Vrai si `diff` est toléré.
    fn allows(self, diff: Diff) -> bool {
        diff.count == 0
            || ((self.max.0..=self.max.1).contains(&diff.max)
                && (self.total.0..=self.total.1).contains(&diff.count))
    }
}

/// Bornes d'une plage `a-b`, ou `0-a` pour un nombre seul.
fn range(text: &str) -> Option<(u64, u64)> {
    let text = text.trim();
    match text.split_once('-') {
        Some((low, high)) => Some((low.trim().parse().ok()?, high.trim().parse().ok()?)),
        None => Some((0, text.parse().ok()?)),
    }
}

/// Tolérance que `content` accorde à la comparaison de la page `test` avec
/// `reference` ; `None` si le contenu est mal formé ou vise une autre
/// référence.
fn parse_fuzzy(content: &str, test: &Url, reference: &Url) -> Option<Fuzzy> {
    // Forme `référence:valeurs` ou `test==référence:valeurs` ; une adresse
    // absolue porte elle-même des deux-points, d'où la coupe au dernier.
    let values = match content.rsplit_once(':') {
        Some((target, values)) => {
            let target = target.rsplit("==").next().unwrap_or(target).trim();
            if test.join(target).ok().as_ref() != Some(reference) {
                return None;
            }
            values
        }
        None => content,
    };
    let (first, second) = values.split_once(';')?;
    let mut max = None;
    let mut total = None;
    for (index, part) in [first, second].into_iter().enumerate() {
        let (name, value) = match part.split_once('=') {
            Some((name, value)) => (name.trim(), value),
            None => (
                if index == 0 {
                    "maxDifference"
                } else {
                    "totalPixels"
                },
                part,
            ),
        };
        match name {
            "maxDifference" => max = range(value),
            "totalPixels" => total = range(value),
            _ => return None,
        }
    }
    let (max, total) = (max?, total?);
    Some(Fuzzy {
        max: (
            u32::try_from(max.0).unwrap_or(u32::MAX),
            u32::try_from(max.1).unwrap_or(u32::MAX),
        ),
        total,
    })
}

/// Compare deux images RGBA de même taille ; des tailles différentes
/// comptent comme un écart total.
fn compare(a: &[u8], b: &[u8]) -> Diff {
    if a.len() != b.len() {
        return Diff {
            count: u64::MAX,
            max: 255,
        };
    }
    let mut diff = Diff { count: 0, max: 0 };
    for (p, q) in a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0) {
        let worst = p
            .iter()
            .zip(q)
            .map(|(x, y)| u32::from(x.abs_diff(*y)))
            .max()
            .unwrap_or(0);
        if worst > 0 {
            diff.count += 1;
            diff.max = diff.max.max(worst);
        }
    }
    diff
}

/// Vrai si tous les points de l'image ont la même couleur.
fn uniform(pixels: &[u8]) -> bool {
    let mut points = pixels.as_chunks::<4>().0.iter();
    let Some(first) = points.next() else {
        return true;
    };
    points.all(|point| point == first)
}

/// Vrai si l'élément racine porte la classe `reftest-wait` : le test demande
/// d'attendre qu'il la retire.
fn waiting(page: &Page) -> bool {
    let doc = page.document();
    doc.children(doc.root())
        .find_map(|node| doc.element(node))
        .and_then(|element| element.attr("class"))
        .is_some_and(|class| class.split_ascii_whitespace().any(|c| c == "reftest-wait"))
}

/// Charge et dessine la page `rel` de la suite.
fn shoot(suite: &Path, rel: &str, renderer: &mut Renderer) -> Result<Shot, String> {
    let (mut page, encoding) = open(suite, rel).ok_or_else(|| UNREADABLE.to_owned())?;
    let url = Url::parse(&format!("{ORIGIN}/{rel}")).map_err(|e| e.to_string())?;
    for _ in 0..MAX_TICKS {
        let _ = page.take_console();
        let loaded = serve(suite, &mut page, encoding);
        let served = serve_resources(suite, &mut page);
        let spent = oxyde_platform::thread_cpu_time().unwrap_or_else(|| renderer.started.elapsed());
        if spent > CHILD_BUDGET {
            return Err("temps d'exécution dépassé".to_owned());
        }
        if loaded
            || served
            || !page.pending_frames().is_empty()
            || !page.pending_scripts().is_empty()
        {
            continue;
        }
        // L'image se prend à la fin du chargement ; le temps n'avance que
        // pour un test qui demande d'attendre.
        if !waiting(&page) {
            break;
        }
        match page.next_wake() {
            Some(due) => {
                page.tick(due);
            }
            None => return Err("`reftest-wait` jamais retiré".to_owned()),
        }
    }
    if waiting(&page) {
        return Err("`reftest-wait` jamais retiré".to_owned());
    }
    // Le calcul des styles peut demander des images de fond et des polices.
    for _ in 0..4 {
        page.update(&mut renderer.text, VIEWPORT);
        if !serve_resources(suite, &mut page) {
            break;
        }
    }
    page.update(&mut renderer.text, VIEWPORT);

    let doc = page.document();
    let mut links = Vec::new();
    let mut fuzzy = Vec::new();
    for node in doc.descendants(doc.root()) {
        let Some(element) = doc.element(node) else {
            continue;
        };
        if element.is_html("link") {
            let rel = element.attr("rel").unwrap_or("");
            let is = |word: &str| {
                rel.split_ascii_whitespace()
                    .any(|token| token.eq_ignore_ascii_case(word))
            };
            let kind = if is("match") {
                Some(true)
            } else if is("mismatch") {
                Some(false)
            } else {
                None
            };
            if let (Some(kind), Some(href)) = (kind, element.attr("href"))
                && let Ok(target) = url.join(href.trim())
            {
                links.push((kind, target));
            }
        } else if element.is_html("meta")
            && element
                .attr("name")
                .is_some_and(|n| n.eq_ignore_ascii_case("fuzzy"))
            && let Some(content) = element.attr("content")
        {
            fuzzy.push(content.to_owned());
        }
    }

    let mut scene = GpuScene::new();
    let (scroll_x, scroll_y) = page.scroll_offset();
    append_scrolled(
        &mut scene,
        page.display_list(),
        Transform::translate((-f64::from(scroll_x), -f64::from(scroll_y))),
        &|id| page.scroll_of(id),
    );
    let pixels = renderer
        .gpu
        .render_to_rgba(&scene, WIDTH, HEIGHT, page.background())
        .map_err(|e| format!("rendu impossible : {e}"))?;
    Ok(Shot {
        pixels,
        links,
        fuzzy,
    })
}

/// Chemin dans la suite de la page `url`, si elle est servie par l'origine
/// des tests.
fn rel_of(url: &Url) -> Option<String> {
    url.as_str()
        .starts_with(ORIGIN)
        .then(|| url.path().trim_start_matches('/').to_owned())
}

/// Verdict du test `rel` : `Ok(note)` s'il réussit, `Err(cause)` sinon.
fn verdict(suite: &Path, rel: &str, renderer: &mut Renderer) -> Result<String, String> {
    let test = shoot(suite, rel, renderer)?;
    if test.links.is_empty() {
        return Err("aucune référence déclarée".to_owned());
    }
    let url = Url::parse(&format!("{ORIGIN}/{rel}")).map_err(|e| e.to_string())?;
    let mut matched: Option<bool> = None;
    let mut cause = String::new();
    for (must_match, target) in &test.links {
        let name = target.path().trim_start_matches('/');
        let reference = rel_of(target)
            .ok_or_else(|| "hors de la suite".to_owned())
            .and_then(|path| shoot(suite, &path, renderer))
            .map_err(|e| format!("référence `{name}` : {e}"))?;
        let diff = compare(&test.pixels, &reference.pixels);
        if *must_match {
            let fuzzy = test
                .fuzzy
                .iter()
                .find_map(|content| parse_fuzzy(content, &url, target))
                .unwrap_or_default();
            if fuzzy.allows(diff) {
                matched = Some(true);
            } else {
                matched.get_or_insert(false);
                if cause.is_empty() {
                    cause = format!(
                        "{} points diffèrent de `{name}` (écart maximal {})",
                        diff.count, diff.max
                    );
                }
            }
        } else if diff.count == 0 {
            return Err(format!("identique à `{name}`, dont elle devait différer"));
        }
    }
    // Plusieurs références `match` sont des variantes : une seule suffit.
    if matched == Some(false) {
        return Err(cause);
    }
    Ok(if uniform(&test.pixels) {
        UNIFORM.to_owned()
    } else {
        String::new()
    })
}

/// Exécute le test de référence `rel` et écrit son résultat dans la forme
/// que le processus père lit pour les tests `testharness.js` : un sous-test
/// unique, puis l'état du fichier.
pub(crate) fn run_child(suite: &Path, rel: &str) -> ExitCode {
    let mut renderer = Renderer {
        text: TextContext::new(),
        gpu: Gpu::reproducible(),
        started: Instant::now(),
    };
    match verdict(suite, rel, &mut renderer) {
        Ok(note) => println!("WPT-T\t0\tréférence\t{note}"),
        Err(cause) => println!("WPT-T\t1\tréférence\t{}", cause.replace(['\t', '\n'], " ")),
    }
    println!("WPT-H\t0\t");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(path: &str) -> Url {
        Url::parse(&format!("{ORIGIN}/{path}")).expect("adresse de test valide")
    }

    #[test]
    fn reftests_are_recognised_by_their_links() {
        assert!(is_reftest(
            "a.html",
            b"<link rel=\"match\" href=\"a-ref.html\">"
        ));
        assert!(is_reftest("a.xht", b"<link rel='mismatch' href='b.xht'/>"));
        assert!(is_reftest("a.html", b"<link rel=match href=b.html>"));
        assert!(!is_reftest("a.html", b"<link rel=\"help\" href=\"x\">"));
        assert!(!is_reftest("a-print.html", b"<link rel=match href=b.html>"));
        assert!(!is_reftest("a.js", b"<link rel=match href=b.html>"));
    }

    #[test]
    fn images_are_compared_point_by_point() {
        let a = [0, 0, 0, 255, 10, 20, 30, 255];
        let b = [0, 0, 0, 255, 10, 25, 28, 255];
        assert_eq!(compare(&a, &a), Diff { count: 0, max: 0 });
        assert_eq!(compare(&a, &b), Diff { count: 1, max: 5 });
        assert_eq!(compare(&a, &a[..4]).count, u64::MAX);
        assert!(uniform(&[1, 2, 3, 4, 1, 2, 3, 4]));
        assert!(!uniform(&a));
    }

    #[test]
    fn fuzzy_tolerance_is_read_in_its_three_forms() {
        let test = url("css/a.html");
        let reference = url("css/a-ref.html");
        let named = parse_fuzzy("maxDifference=0-2;totalPixels=10-100", &test, &reference);
        assert_eq!(
            named,
            Some(Fuzzy {
                max: (0, 2),
                total: (10, 100)
            })
        );
        let bare = parse_fuzzy("5;300", &test, &reference).expect("forme courte");
        assert!(bare.allows(Diff { count: 300, max: 5 }));
        assert!(!bare.allows(Diff { count: 301, max: 5 }));
        assert!(!bare.allows(Diff { count: 1, max: 6 }));
        assert!(parse_fuzzy("a-ref.html:5;300", &test, &reference).is_some());
        assert!(parse_fuzzy("a.html==a-ref.html:5;300", &test, &reference).is_some());
        assert!(parse_fuzzy("other.html:5;300", &test, &reference).is_none());
        assert!(parse_fuzzy("n'importe quoi", &test, &reference).is_none());
        // Sans tolérance, seule l'identité passe.
        assert!(Fuzzy::default().allows(Diff { count: 0, max: 0 }));
        assert!(!Fuzzy::default().allows(Diff { count: 1, max: 1 }));
    }
}
