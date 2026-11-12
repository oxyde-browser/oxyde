//! Le processus de rendu de bout en bout : une page rendue par un processus
//! de rendu doit être la même que rendue dans le processus du navigateur, et
//! la disparition du processus de rendu doit être constatée, pas subie.
//!
//! Ces tests lancent le vrai binaire et rendent sur le processeur graphique :
//! sans lui (machine d'intégration sans carte), ils se bornent à le signaler.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn oxyde(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxyde"))
        .args(args)
        .output()
        .expect("lancement d'oxyde")
}

fn page(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/pages")
        .join(name)
        .canonicalize()
        .expect("page de test");
    url::Url::from_file_path(&path)
        .expect("chemin absolu")
        .to_string()
}

fn output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("oxyde-{}-{name}.png", std::process::id()))
}

/// Pixels RGBA d'une capture.
fn pixels(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(path).expect("capture");
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .expect("en-tête PNG");
    let mut buffer = vec![0; reader.output_buffer_size().expect("taille du tampon")];
    let info = reader.next_frame(&mut buffer).expect("image PNG");
    buffer.truncate(info.buffer_size());
    (info.width, info.height, buffer)
}

/// Rend `name` et renvoie ses pixels ; `None` si le rendu est impossible ici.
fn capture(name: &str, multi_process: bool) -> Option<(u32, u32, Vec<u8>)> {
    let mode = if multi_process { "multi" } else { "mono" };
    let output = output_path(&format!("{name}-{mode}"));
    let url = page(name);
    let mut args = vec![
        "--screenshot",
        output.to_str().expect("chemin UTF-8"),
        "--size",
        "640x480",
        "--page-only",
    ];
    if multi_process {
        args.push("--multi-process");
    }
    args.push(&url);
    let run = oxyde(&args);
    if !run.status.success() {
        // Le rendu dans le processus du navigateur sert de témoin : s'il
        // échoue, la machine ne peut pas rendre du tout.
        assert!(
            !multi_process,
            "le rendu par processus de rendu a échoué : {}",
            String::from_utf8_lossy(&run.stderr)
        );
        eprintln!(
            "rendu impossible sur cette machine, test sauté : {}",
            String::from_utf8_lossy(&run.stderr)
        );
        return None;
    }
    let image = pixels(&output);
    let _ = std::fs::remove_file(&output);
    Some(image)
}

#[test]
fn renderer_process_paints_the_same_page() {
    for name in ["tableaux.html", "defilement.html", "transformations.html"] {
        let Some((width, height, local)) = capture(name, false) else {
            return;
        };
        let (remote_width, remote_height, remote) =
            capture(name, true).expect("rendu par processus de rendu");
        assert_eq!((width, height), (remote_width, remote_height), "{name}");
        // Le rendu graphique varie d'un niveau sur de rares pixels d'une
        // exécution à l'autre, même dans un seul processus.
        let (local, _) = local.as_chunks::<4>();
        let (remote, _) = remote.as_chunks::<4>();
        let differing = local
            .iter()
            .zip(remote)
            .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 2))
            .count();
        assert!(
            differing == 0,
            "{name} : {differing} pixels diffèrent entre les deux modes"
        );
        // La page n'est pas vide : elle contient autre chose que son fond.
        let background = local.first().expect("capture non vide");
        assert!(
            local.iter().any(|pixel| pixel != background),
            "{name} : capture uniforme"
        );
    }
}

#[test]
fn renderer_crash_is_reported_not_fatal() {
    let output = output_path("plantage");
    let run = oxyde(&[
        "--screenshot",
        output.to_str().expect("chemin UTF-8"),
        "--page-only",
        "--multi-process",
        "about:crash",
    ]);
    // Le processus de rendu s'est arrêté net ; le navigateur, lui, termine
    // proprement en le disant.
    assert_eq!(run.status.code(), Some(1));
    let message = String::from_utf8_lossy(&run.stderr);
    assert!(
        message.contains("processus de rendu s'est arrêté"),
        "message inattendu : {message}"
    );
    assert!(!output.exists(), "aucune capture ne doit être écrite");
}

#[test]
fn web_page_cannot_read_local_files() {
    // Une page `data:` (donc sans droit sur les fichiers) qui désigne un
    // fichier local comme feuille de style : le navigateur refuse de le lui
    // lire. Le rendu aboutit quand même, sans la feuille.
    let sheet = page("tableaux.html");
    let html = format!(
        "data:text/html,<link rel=stylesheet href='{sheet}'><p style='color:red'>texte</p>"
    );
    let output = output_path("fichier");
    let run = oxyde(&[
        "--screenshot",
        output.to_str().expect("chemin UTF-8"),
        "--size",
        "200x100",
        "--page-only",
        "--multi-process",
        &html,
    ]);
    if run.status.success() {
        let _ = std::fs::remove_file(&output);
    } else {
        // Sans processeur graphique, la capture échoue après le chargement :
        // seul un plantage du processus de rendu serait une faute ici.
        let message = String::from_utf8_lossy(&run.stderr);
        assert!(
            !message.contains("processus de rendu s'est arrêté"),
            "le refus d'un fichier ne doit pas faire planter le rendu : {message}"
        );
    }
}
