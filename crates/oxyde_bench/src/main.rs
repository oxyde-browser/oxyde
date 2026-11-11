//! Bancs d'essai d'Oxyde.
//!
//! Quatre familles de mesures, chacune reproductible depuis le dépôt :
//!
//! - **performances** : temps des étapes de la chaîne de rendu (analyse HTML,
//!   analyse CSS, style + mise en page + liste d'affichage) sur des pages
//!   synthétiques de taille fixe et sur `tests/pages/demo.html` ;
//! - **sécurité** : comportement de la pile TLS face aux certificats et
//!   protocoles de badssl.com (accès réseau requis) ;
//! - **vie privée** : en-têtes réellement envoyés par `oxyde_net`, capturés par
//!   un serveur local ;
//! - **compatibilité** : état des suites et bancs publics (BrowserBench,
//!   Web Platform Tests, test262…), avec ce qui bloque leur exécution.
//!
//! Le rapport est écrit dans `docs/BANCS.md` et les mesures brutes dans
//! `bench/resultats/AAAA-MM-JJ.json`. Aucun score n'est inventé : un banc
//! qui ne peut pas tourner est marqué comme tel, avec sa raison.
//!
//! Usage : `cargo run -p oxyde_bench --release -- [perf|securite|vie-privee|tout] [--hors-ligne] [--publier]`

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use oxyde_content::{Page, Url};
use oxyde_css::Viewport;
use oxyde_net::{Client, NetError};
use oxyde_text::TextContext;

type Resultat<T> = Result<T, Box<dyn std::error::Error>>;

/// Une mesure de performance.
struct Mesure {
    nom: String,
    iterations: usize,
    mediane: Duration,
    minimum: Duration,
}

/// Un contrôle qui réussit ou échoue (sécurité, vie privée).
struct Controle {
    nom: String,
    attendu: String,
    obtenu: String,
    conforme: bool,
}

#[derive(Default)]
struct Rapport {
    perf: Vec<Mesure>,
    securite: Vec<Controle>,
    vie_privee: Vec<Controle>,
    notes: Vec<String>,
}

fn main() -> Resultat<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a = |s: &str| args.iter().any(|x| x == s);
    let tout = a("tout") || !(a("perf") || a("securite") || a("vie-privee"));
    let hors_ligne = a("--hors-ligne");
    let racine = racine_depot();

    if cfg!(debug_assertions) {
        eprintln!("attention : compilation sans --release, les temps ne sont pas représentatifs");
    }

    let mut rapport = Rapport::default();
    if tout || a("perf") {
        eprintln!("== performances");
        rapport.perf = performances(&racine)?;
    }
    if tout || a("vie-privee") {
        eprintln!("== vie privée");
        rapport.vie_privee = vie_privee()?;
    }
    if tout || a("securite") {
        if hors_ligne {
            rapport
                .notes
                .push("Sécurité TLS non mesurée (exécution hors ligne).".into());
        } else {
            eprintln!("== sécurité (réseau : badssl.com)");
            rapport.securite = securite()?;
        }
    }

    let date = date_du_jour();
    let markdown = rapport_markdown(&rapport, &date);
    print!("{markdown}");
    if a("--publier") {
        std::fs::write(racine.join("docs/BANCS.md"), &markdown)?;
        let dossier = racine.join("bench/resultats");
        std::fs::create_dir_all(&dossier)?;
        std::fs::write(
            dossier.join(format!("{date}.json")),
            rapport_json(&rapport, &date),
        )?;
        eprintln!("publié : docs/BANCS.md et bench/resultats/{date}.json");
    }
    Ok(())
}

fn racine_depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from("."))
}

// ───────────────────────────── performances ─────────────────────────────

/// Exécute `f` après un échauffement et renvoie médiane et minimum.
fn chronometrer(nom: &str, iterations: usize, mut f: impl FnMut()) -> Mesure {
    for _ in 0..iterations.div_ceil(5) {
        f();
    }
    let mut temps: Vec<Duration> = (0..iterations)
        .map(|_| {
            let debut = Instant::now();
            f();
            debut.elapsed()
        })
        .collect();
    temps.sort();
    let m = Mesure {
        nom: nom.to_owned(),
        iterations,
        mediane: temps[temps.len() / 2],
        minimum: temps[0],
    };
    eprintln!("  {:<55} {:>10.3} ms", m.nom, ms(m.mediane));
    m
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn page_paragraphes(n: usize) -> String {
    let mut s = String::from("<!DOCTYPE html><html><head><title>banc</title></head><body>");
    for i in 0..n {
        let _ = write!(
            s,
            "<p class='c{}'>Paragraphe {i} avec <a href='#{i}'>un lien</a>, du <b>gras</b> \
             et des &eacute;l&eacute;ments en ligne.</p>",
            i % 10
        );
    }
    s.push_str("</body></html>");
    s
}

fn page_imbriquee(profondeur: usize, largeur: usize) -> String {
    let mut s = String::from("<!DOCTYPE html><body>");
    for _ in 0..largeur {
        for d in 0..profondeur {
            let _ = write!(s, "<div class='n{d}' style='padding:1px'>");
        }
        s.push_str("feuille de texte");
        for _ in 0..profondeur {
            s.push_str("</div>");
        }
    }
    s
}

fn feuille_css(regles: usize) -> String {
    let mut s = String::new();
    for i in 0..regles {
        let _ = writeln!(
            s,
            "body .c{} > a:not(.x), div.n{} p {{ color: #{:06x}; margin: {}px 2px; font-size: {}px }}",
            i % 10,
            i % 30,
            (i * 2_654_435) % 0xFF_FFFF,
            i % 8,
            12 + i % 6
        );
    }
    s
}

fn performances(racine: &Path) -> Resultat<Vec<Mesure>> {
    let url = Url::parse("https://banc.oxyde.test/")?;
    let vue = Viewport {
        width: 1280.0,
        height: 800.0,
    };
    let mut texte = TextContext::new();
    let mut mesures = Vec::new();

    let gros = page_paragraphes(10_000);
    mesures.push(chronometrer(
        "Analyse HTML — 10 000 paragraphes (≈1,3 Mo)",
        20,
        || {
            std::hint::black_box(oxyde_html::parse_document(&gros));
        },
    ));

    let css = feuille_css(2_000);
    mesures.push(chronometrer("Analyse CSS — 2 000 règles", 30, || {
        std::hint::black_box(oxyde_css::parse_stylesheet(&css));
    }));

    let moyen = page_paragraphes(1_000);
    mesures.push(chronometrer(
        "Rendu complet — 1 000 paragraphes, CSS par défaut",
        20,
        || {
            let mut p = Page::from_html(url.clone(), &moyen);
            p.update(&mut texte, vue);
            std::hint::black_box(p.content_height());
        },
    ));

    let style = format!(
        "<style>{}</style>{}",
        feuille_css(500),
        page_paragraphes(1_000)
    );
    mesures.push(chronometrer(
        "Rendu complet — 1 000 paragraphes × 500 règles CSS",
        10,
        || {
            let mut p = Page::from_html(url.clone(), &style);
            p.update(&mut texte, vue);
            std::hint::black_box(p.content_height());
        },
    ));

    let profond = page_imbriquee(30, 200);
    mesures.push(chronometrer(
        "Rendu complet — 200 arbres de 30 div imbriqués",
        20,
        || {
            let mut p = Page::from_html(url.clone(), &profond);
            p.update(&mut texte, vue);
            std::hint::black_box(p.content_height());
        },
    ));

    let demo_dir = racine.join("tests/pages");
    if let (Ok(html), Ok(base)) = (
        std::fs::read_to_string(demo_dir.join("demo.html")),
        std::fs::read_to_string(demo_dir.join("base.css")),
    ) {
        let demo_url = Url::parse("https://banc.oxyde.test/demo.html")?;
        mesures.push(chronometrer(
            "Rendu complet — tests/pages/demo.html (sans images)",
            50,
            || {
                let mut p = Page::from_html(demo_url.clone(), &html);
                for feuille in p.pending_stylesheets() {
                    p.set_stylesheet(&feuille, &base);
                }
                p.update(&mut texte, vue);
                std::hint::black_box(p.content_height());
            },
        ));
    }
    Ok(mesures)
}

// ─────────────────────────────── sécurité ───────────────────────────────

/// Sites de badssl.com et comportement attendu d'un navigateur sûr
/// (celui de Chromium sauf mention).
const BADSSL: &[(&str, bool, &str)] = &[
    ("sha256.badssl.com", true, "certificat valide"),
    ("ecc256.badssl.com", true, "clé ECDSA P-256"),
    ("rsa2048.badssl.com", true, "clé RSA 2048"),
    ("tls-v1-2.badssl.com:1012", true, "TLS 1.2"),
    ("expired.badssl.com", false, "certificat expiré"),
    ("wrong.host.badssl.com", false, "nom d'hôte incorrect"),
    ("self-signed.badssl.com", false, "certificat auto-signé"),
    ("untrusted-root.badssl.com", false, "racine non reconnue"),
    ("revoked.badssl.com", false, "certificat révoqué"),
    ("no-common-name.badssl.com", false, "certificat sans nom"),
    (
        "incomplete-chain.badssl.com",
        false,
        "chaîne incomplète (plus strict que Chromium, qui la complète par AIA)",
    ),
    ("tls-v1-0.badssl.com:1010", false, "TLS 1.0 obsolète"),
    ("tls-v1-1.badssl.com:1011", false, "TLS 1.1 obsolète"),
    ("rc4.badssl.com", false, "chiffrement RC4"),
    ("3des.badssl.com", false, "chiffrement 3DES"),
    ("null.badssl.com", false, "chiffrement nul"),
    ("dh480.badssl.com", false, "Diffie-Hellman 480 bits"),
    ("dh1024.badssl.com", false, "Diffie-Hellman 1024 bits"),
];

fn securite() -> Resultat<Vec<Controle>> {
    let client = Client::new()?;
    let mut controles = Vec::new();
    for &(hote, accepter, libelle) in BADSSL {
        let url = Url::parse(&format!("https://{hote}/"))?;
        let resultat = client.get(&url);
        let (accepte, obtenu) = match &resultat {
            Ok(r) => (true, format!("accepté (HTTP {})", r.status)),
            Err(e @ (NetError::Connect { .. } | NetError::Timeout)) => {
                // Le serveur n'a pas pu être joint : on ne conclut rien.
                controles.push(Controle {
                    nom: format!("{hote} — {libelle}"),
                    attendu: attendu(accepter),
                    obtenu: format!("injoignable ({})", e.code()),
                    conforme: false,
                });
                continue;
            }
            Err(e) => (false, format!("refusé ({})", e.code())),
        };
        let c = Controle {
            nom: format!("{hote} — {libelle}"),
            attendu: attendu(accepter),
            obtenu,
            conforme: accepte == accepter,
        };
        eprintln!("  {} {}", if c.conforme { "ok " } else { "ÉCHEC" }, c.nom);
        controles.push(c);
    }

    // Une redirection https → http doit être refusée.
    let downgrade = Url::parse("https://http.badssl.com/")?;
    let r = client.get(&downgrade);
    controles.push(Controle {
        nom: "http.badssl.com — accès https à un site servi en http".into(),
        attendu: "refusé ou erreur".into(),
        obtenu: match &r {
            Ok(r) => format!("accepté (HTTP {})", r.status),
            Err(e) => format!("refusé ({})", e.code()),
        },
        conforme: r.is_err(),
    });
    Ok(controles)
}

fn attendu(accepter: bool) -> String {
    if accepter { "accepté" } else { "refusé" }.into()
}

// ────────────────────────────── vie privée ──────────────────────────────

/// Capture les en-têtes d'une requête d'`oxyde_net` avec un serveur local.
fn capturer_entetes() -> Resultat<Vec<(String, String)>> {
    let ecoute = TcpListener::bind("127.0.0.1:0")?;
    let port = ecoute.local_addr()?.port();
    let serveur = std::thread::spawn(move || -> std::io::Result<String> {
        let (mut flux, _) = ecoute.accept()?;
        flux.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut recu = Vec::new();
        let mut tampon = [0u8; 4096];
        while !recu.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = flux.read(&mut tampon)?;
            if n == 0 {
                break;
            }
            recu.extend_from_slice(&tampon[..n]);
        }
        flux.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
        )?;
        Ok(String::from_utf8_lossy(&recu).into_owned())
    });
    let client = Client::new()?;
    client.get(&Url::parse(&format!("http://127.0.0.1:{port}/page?x=1"))?)?;
    let brut = serveur.join().map_err(|_| "le serveur local a paniqué")??;
    Ok(brut
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect())
}

fn vie_privee() -> Resultat<Vec<Controle>> {
    let entetes = capturer_entetes()?;
    let valeur = |n: &str| entetes.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
    let mut c = Vec::new();

    for (nom, raison) in [
        ("cookie", "aucun cookie stocké ni envoyé"),
        ("referer", "pas de fuite de la page précédente"),
        ("x-client-data", "en-tête d'expérimentation propre à Chrome"),
        ("x-requested-with", "identifiant d'application"),
        (
            "sec-ch-ua-full-version-list",
            "empreinte détaillée par Client Hints",
        ),
    ] {
        let v = valeur(nom);
        c.push(Controle {
            nom: format!("En-tête `{nom}` — {raison}"),
            attendu: "absent".into(),
            obtenu: v.clone().unwrap_or_else(|| "absent".into()),
            conforme: v.is_none(),
        });
    }

    let ua = valeur("user-agent").unwrap_or_default();
    c.push(Controle {
        nom: "User-Agent sans version détaillée du système ni identifiant unique".into(),
        attendu: "chaîne courte et commune".into(),
        obtenu: format!("`{ua}`"),
        conforme: ua.len() < 80 && !ua.chars().any(|ch| ch == '{'),
    });
    c.push(Controle {
        nom: "Global Privacy Control (`Sec-GPC: 1`)".into(),
        attendu: "envoyé".into(),
        obtenu: valeur("sec-gpc").unwrap_or_else(|| "absent".into()),
        conforme: valeur("sec-gpc").as_deref() == Some("1"),
    });
    c.push(Controle {
        nom: "Nombre total d'en-têtes envoyés (surface d'empreinte)".into(),
        attendu: "≤ 6".into(),
        obtenu: format!(
            "{} ({})",
            entetes.len(),
            entetes
                .iter()
                .map(|(k, _)| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        conforme: entetes.len() <= 6,
    });
    for x in &c {
        eprintln!("  {} {}", if x.conforme { "ok " } else { "ÉCHEC" }, x.nom);
    }
    Ok(c)
}

// ───────────────────────────── compatibilité ─────────────────────────────

/// Bancs et suites publics, avec leur prérequis et leur état.
const EXTERNES: &[(&str, &str, &str, &str)] = &[
    (
        "Speedometer 3.1",
        "https://browserbench.org/Speedometer3.1/",
        "réactivité des applications web",
        "JavaScript + DOM scriptable (jalon 4)",
    ),
    (
        "JetStream 2.2",
        "https://browserbench.org/JetStream2.2/",
        "JavaScript et WebAssembly",
        "JavaScript (jalon 4), WebAssembly (jalon 8)",
    ),
    (
        "MotionMark 1.3",
        "https://browserbench.org/MotionMark1.3/",
        "rendu graphique animé",
        "JavaScript (jalon 4), canvas et compositeur (jalon 6)",
    ),
    (
        "APEXMark",
        "https://apexmark.tech/",
        "banc navigateur",
        "JavaScript (jalon 4)",
    ),
    (
        "test262",
        "https://github.com/tc39/test262",
        "conformité ECMAScript",
        "moteur `oxyde_js` (jalon 4, en cours)",
    ),
    (
        "Web Platform Tests",
        "https://wpt.fyi/",
        "conformité des standards du web",
        "lanceur de tests de référence + testharness.js (jalon 4)",
    ),
    (
        "html5lib-tests",
        "https://github.com/html5lib/html5lib-tests",
        "analyse HTML",
        "à brancher : `--dump-dom` produit déjà le format",
    ),
    (
        "Acid3",
        "http://acid3.acidtests.org/",
        "conformité historique DOM/CSS",
        "JavaScript (jalon 4)",
    ),
    (
        "HTML5test",
        "https://html5test.co/",
        "fonctions HTML5 détectées",
        "JavaScript (jalon 4)",
    ),
    (
        "badssl.com",
        "https://badssl.com/",
        "TLS et certificats",
        "**intégré** (section Sécurité)",
    ),
    (
        "Qualys SSL Labs — client",
        "https://clienttest.ssllabs.com:8443/ssltest/viewMyClient.html",
        "protocoles et suites TLS",
        "JavaScript (jalon 4)",
    ),
    (
        "PrivacyTests.org",
        "https://privacytests.org/",
        "protections de la vie privée",
        "JavaScript, cookies et stockage (jalons 4, 7, 9)",
    ),
    (
        "Cover Your Tracks (EFF)",
        "https://coveryourtracks.eff.org/",
        "pistage et empreinte",
        "JavaScript (jalon 4)",
    ),
    (
        "BrowserLeaks",
        "https://browserleaks.com/",
        "fuites d'empreinte",
        "JavaScript (jalon 4)",
    ),
];

// ─────────────────────────────── rapports ───────────────────────────────

fn coche(ok: bool) -> &'static str {
    if ok { "✅" } else { "❌" }
}

fn tableau_controles(s: &mut String, controles: &[Controle]) {
    let ok = controles.iter().filter(|c| c.conforme).count();
    let _ = writeln!(s, "**{ok} / {} conformes**\n", controles.len());
    s.push_str("| | Contrôle | Attendu | Obtenu |\n|---|---|---|---|\n");
    for c in controles {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} |",
            coche(c.conforme),
            c.nom,
            c.attendu,
            c.obtenu.replace('|', "\\|")
        );
    }
    s.push('\n');
}

fn rapport_markdown(r: &Rapport, date: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Bancs d'essai d'Oxyde\n\n\
         Rapport généré par `oxyde-bench` le {date} (Oxyde {}, {} cœurs logiques, {}).\n\
         Pour le reproduire :\n\n\
         ```bash\ncargo run -p oxyde_bench --release -- tout --publier\n```\n\n\
         Les mesures brutes sont dans [`bench/resultats/`](../bench/resultats/). Aucun score n'est \
         estimé : un banc qui ne peut pas encore tourner est marqué comme tel.\n",
        env!("CARGO_PKG_VERSION"),
        std::thread::available_parallelism().map_or(0, std::num::NonZero::get),
        std::env::consts::OS,
    );

    s.push_str("## Performances de la chaîne de rendu\n\n");
    if r.perf.is_empty() {
        s.push_str("Non mesurées lors de cette exécution.\n\n");
    } else {
        s.push_str(
            "Temps d'une exécution complète, après échauffement ; médiane et minimum. \
             Plus bas est meilleur.\n\n| Banc | Médiane | Minimum | Itérations |\n|---|---:|---:|---:|\n",
        );
        for m in &r.perf {
            let _ = writeln!(
                s,
                "| {} | {:.2} ms | {:.2} ms | {} |",
                m.nom,
                ms(m.mediane),
                ms(m.minimum),
                m.iterations
            );
        }
        s.push('\n');
    }

    s.push_str("## Sécurité — TLS et certificats\n\nComportement de `oxyde_net` face aux serveurs de test de badssl.com.\n\n");
    if r.securite.is_empty() {
        s.push_str("Non mesurée lors de cette exécution.\n\n");
    } else {
        tableau_controles(&mut s, &r.securite);
    }

    s.push_str("## Vie privée — ce que le navigateur envoie\n\nEn-têtes d'une requête réelle, capturés par un serveur local.\n\n");
    if r.vie_privee.is_empty() {
        s.push_str("Non mesurée lors de cette exécution.\n\n");
    } else {
        tableau_controles(&mut s, &r.vie_privee);
    }

    s.push_str(
        "## Compatibilité et bancs publics\n\n\
         | Banc | Mesure | État |\n|---|---|---|\n",
    );
    for (nom, url, quoi, etat) in EXTERNES {
        let etat = if etat.starts_with("**") {
            (*etat).to_owned()
        } else {
            format!("bloqué — {etat}")
        };
        let _ = writeln!(s, "| [{nom}]({url}) | {quoi} | {etat} |");
    }
    s.push_str(
        "\nLes scores de ces bancs seront ajoutés ici dès qu'Oxyde pourra les exécuter, \
         avec la version de Chromium mesurée sur la même machine comme référence.\n",
    );
    if !r.notes.is_empty() {
        s.push_str("\n## Notes\n\n");
        for n in &r.notes {
            let _ = writeln!(s, "- {n}");
        }
    }
    s
}

fn json_texte(t: &str) -> String {
    let mut s = String::from("\"");
    for ch in t.chars() {
        match ch {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(s, "\\u{:04x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

fn json_controles(c: &[Controle]) -> String {
    let items: Vec<String> = c
        .iter()
        .map(|c| {
            format!(
                "{{\"nom\":{},\"attendu\":{},\"obtenu\":{},\"conforme\":{}}}",
                json_texte(&c.nom),
                json_texte(&c.attendu),
                json_texte(&c.obtenu),
                c.conforme
            )
        })
        .collect();
    format!("[{}]", items.join(",\n  "))
}

fn rapport_json(r: &Rapport, date: &str) -> String {
    let perf: Vec<String> = r
        .perf
        .iter()
        .map(|m| {
            format!(
                "{{\"nom\":{},\"mediane_ms\":{:.4},\"minimum_ms\":{:.4},\"iterations\":{}}}",
                json_texte(&m.nom),
                ms(m.mediane),
                ms(m.minimum),
                m.iterations
            )
        })
        .collect();
    format!(
        "{{\"date\":{},\"version\":{},\"os\":{},\"perf\":[{}],\n\"securite\":{},\n\"vie_privee\":{}}}\n",
        json_texte(date),
        json_texte(env!("CARGO_PKG_VERSION")),
        json_texte(std::env::consts::OS),
        perf.join(",\n  "),
        json_controles(&r.securite),
        json_controles(&r.vie_privee)
    )
}

/// Date UTC au format AAAA-MM-JJ (algorithme « civil from days »).
fn date_du_jour() -> String {
    let jours = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400);
    let z = i64::try_from(jours).unwrap_or(0) + 719_468;
    let ere = z.div_euclid(146_097);
    let doe = z - ere * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let j = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let a = yoe + ere * 400 + i64::from(m <= 2);
    format!("{a:04}-{m:02}-{j:02}")
}
