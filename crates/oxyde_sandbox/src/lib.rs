//! Bac à sable des processus de rendu (pendant de `//sandbox`).
//!
//! Un processus de rendu traite du contenu venu du web : il faut supposer
//! qu'il peut être pris. Le bac à sable borne alors ce que l'attaquant y
//! gagne. Sous Windows, [`spawn`] lance le processus avec :
//!
//! - un **jeton restreint** : privilèges retirés, accès limité aux objets
//!   ouverts à tous, aux utilisateurs ou à la session — ni les fichiers du
//!   profil de l'utilisateur, ni son registre ;
//! - un **niveau d'intégrité bas** : aucune écriture vers ce que
//!   l'utilisateur peut modifier ;
//! - un **objet de travail** : il meurt avec le navigateur, ne lance aucun
//!   processus, n'a accès ni au presse-papiers ni aux réglages du système ;
//! - seulement les **poignées** qu'on lui destine (ses tubes).
//!
//! Le processus appelle ensuite [`harden`] sur lui-même, avant tout contenu.
//! Un processus qui doit parler au réseau reçoit un bac à sable moins
//! serré : voir [`Policy`].
//!
//! LIMITES CONNUES : ce n'est pas un `AppContainer` — le processus peut
//! encore lire ce qui est lisible par tous (fichiers du système, dossiers
//! partagés) et ouvrir des connexions réseau ; le verrouillage de `win32k`
//! n'est pas appliqué. Hors de Windows, le processus est lancé sans bac à
//! sable, et [`Process::is_sandboxed`] le dit.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
mod windows;

/// Rigueur du bac à sable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Pour un processus qui n'a besoin que de ses tubes (rendu) : jeton
    /// restreint par identifiants, intégrité basse, objet de travail. Il ne
    /// lit ni n'écrit les fichiers de l'utilisateur.
    Locked,
    /// Pour un processus qui parle au réseau : privilèges retirés, intégrité
    /// basse, objet de travail, mais pas de restriction par identifiants —
    /// la résolution de noms et les connexions la refusent. Il ne peut rien
    /// écrire, mais peut encore lire ce que l'utilisateur peut lire.
    Networked,
}

/// Processus enfant, dans le bac à sable ou non.
#[derive(Debug)]
pub struct Process {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    Plain(Child),
    #[cfg(windows)]
    Restricted(windows::Restricted),
}

impl Process {
    /// Vrai si le processus tourne dans le bac à sable.
    #[must_use]
    pub fn is_sandboxed(&self) -> bool {
        match &self.inner {
            Inner::Plain(_) => false,
            #[cfg(windows)]
            Inner::Restricted(_) => true,
        }
    }

    /// Arrête le processus. Sans effet s'il est déjà terminé.
    pub fn kill(&mut self) {
        match &mut self.inner {
            Inner::Plain(child) => {
                // Un échec signifie qu'il est déjà terminé.
                let _ = child.kill();
            }
            #[cfg(windows)]
            Inner::Restricted(child) => child.kill(),
        }
    }

    /// Attend la fin du processus.
    pub fn wait(&mut self) {
        match &mut self.inner {
            Inner::Plain(child) => {
                let _ = child.wait();
            }
            #[cfg(windows)]
            Inner::Restricted(child) => child.wait(),
        }
    }
}

/// Processus enfant et ses tubes.
pub struct Spawned {
    /// Le processus.
    pub process: Process,
    /// Vers son entrée standard.
    pub stdin: Box<dyn Write + Send>,
    /// Depuis sa sortie standard.
    pub stdout: Box<dyn Read + Send>,
}

impl std::fmt::Debug for Spawned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spawned")
            .field("process", &self.process)
            .finish_non_exhaustive()
    }
}

/// Lance `executable` sans bac à sable, relié par son entrée et sa sortie
/// standard.
///
/// # Errors
///
/// L'erreur de lancement du processus.
pub fn spawn_unsandboxed(executable: &Path, arguments: &[&str]) -> io::Result<Spawned> {
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::other("tube du processus enfant indisponible"));
    };
    Ok(Spawned {
        process: Process {
            inner: Inner::Plain(child),
        },
        stdin: Box::new(stdin),
        stdout: Box::new(stdout),
    })
}

/// Lance `executable` dans le bac à sable, relié par son entrée et sa sortie
/// standard. Hors de Windows, le lance sans bac à sable.
///
/// # Errors
///
/// L'erreur du système si le bac à sable ou le processus n'a pas pu être
/// créé ; rien n'est alors lancé.
pub fn spawn(executable: &Path, arguments: &[&str], policy: Policy) -> io::Result<Spawned> {
    #[cfg(windows)]
    {
        let (child, stdin, stdout) = windows::spawn(executable, arguments, policy)?;
        Ok(Spawned {
            process: Process {
                inner: Inner::Restricted(child),
            },
            stdin: Box::new(stdin),
            stdout: Box::new(stdout),
        })
    }
    #[cfg(not(windows))]
    {
        let _ = policy;
        spawn_unsandboxed(executable, arguments)
    }
}

/// Durcit le processus courant ; à appeler par un processus de rendu avant
/// de traiter le moindre contenu. Renvoie le nom des protections que le
/// système a refusées (aucune hors de Windows, où il n'y en a pas encore).
#[must_use]
pub fn harden() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        windows::harden()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Le binaire de test sert lui-même d'enfant : relancé sur l'un des tests
    // « sonde » ci-dessous (ignorés en temps normal), il rapporte sur sa
    // sortie ce que le bac à sable lui laisse faire.

    fn secret() -> PathBuf {
        std::env::temp_dir().join("oxyde-bac-a-sable-secret.txt")
    }

    fn intruder() -> PathBuf {
        std::env::temp_dir().join("oxyde-bac-a-sable-intrus.txt")
    }

    /// Sonde : tente ce qu'un processus de rendu pris voudrait faire.
    #[test]
    #[ignore = "sonde lancée par `sandboxed_child_is_confined`"]
    #[expect(clippy::print_stdout, reason = "la sonde rapporte sur sa sortie")]
    fn probe() {
        println!("SONDE vivante");
        let read = std::fs::read_to_string(secret()).is_ok_and(|text| text == "confidentiel");
        println!("SONDE lecture {read}");
        let written = std::fs::write(intruder(), "intrus").is_ok();
        println!("SONDE ecriture {written}");
        let launched = std::env::current_exe()
            .and_then(|exe| Command::new(exe).arg("--list").output())
            .is_ok();
        println!("SONDE lancement {launched}");
        println!("SONDE durcissement {:?}", harden());
    }

    /// Sonde : attend la fin de son entrée standard.
    #[test]
    #[ignore = "sonde lancée par `killing_a_sandboxed_child_closes_its_pipes`"]
    fn wait_for_input() {
        let mut sink = Vec::new();
        let _ = io::stdin().read_to_end(&mut sink);
    }

    fn probe_arguments(name: &str) -> Vec<String> {
        vec![
            "--ignored".to_owned(),
            "--exact".to_owned(),
            format!("tests::{name}"),
            "--nocapture".to_owned(),
        ]
    }

    /// Lance la sonde `probe` et rend ses constats : (nom, valeur).
    fn run_probe(policy: Option<Policy>) -> Vec<(String, String)> {
        let sandboxed = policy.is_some();
        let exe = std::env::current_exe().expect("binaire de test");
        let arguments = probe_arguments("probe");
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let mut spawned = match policy {
            Some(policy) => spawn(&exe, &arguments, policy),
            None => spawn_unsandboxed(&exe, &arguments),
        }
        .expect("lancement de la sonde");
        assert_eq!(spawned.process.is_sandboxed(), sandboxed);
        drop(spawned.stdin);
        let mut output = String::new();
        spawned
            .stdout
            .read_to_string(&mut output)
            .expect("sortie de la sonde");
        spawned.process.wait();
        output
            .lines()
            .filter_map(|line| line.strip_prefix("SONDE "))
            .filter_map(|line| line.split_once(' '))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    }

    fn finding<'a>(findings: &'a [(String, String)], name: &str) -> &'a str {
        findings
            .iter()
            .find(|(known, _)| known == name)
            .map_or("absent", |(_, value)| value.as_str())
    }

    #[test]
    fn sandboxed_child_is_confined() {
        std::fs::write(secret(), "confidentiel").expect("fichier témoin");
        let _ = std::fs::remove_file(intruder());

        let inside = run_probe(Some(Policy::Locked));
        let wrote = intruder().exists();
        let _ = std::fs::remove_file(intruder());
        // La sonde a bien tourné, par ses tubes, dans le bac à sable.
        assert!(
            inside
                .iter()
                .any(|(name, _)| name == "vivante" || name == "lecture"),
            "la sonde n'a rien rapporté : {inside:?}"
        );
        assert_eq!(
            finding(&inside, "lecture"),
            "false",
            "fichier de l'utilisateur lu"
        );
        assert_eq!(
            finding(&inside, "ecriture"),
            "false",
            "fichier écrit dans le profil"
        );
        assert!(!wrote, "le fichier de l'intrus existe");
        assert_eq!(finding(&inside, "lancement"), "false", "processus lancé");
        assert_eq!(
            finding(&inside, "durcissement"),
            "[]",
            "protections refusées"
        );

        // Témoin : hors bac à sable, tout cela aboutit — les refus viennent
        // donc bien du bac à sable.
        let outside = run_probe(None);
        let _ = std::fs::remove_file(intruder());
        assert_eq!(finding(&outside, "lecture"), "true");
        assert_eq!(finding(&outside, "ecriture"), "true");

        // Bac à sable d'un processus réseau : il ne peut toujours rien
        // écrire ni lancer, mais la lecture ne lui est pas retirée.
        let networked = run_probe(Some(Policy::Networked));
        let wrote = intruder().exists();
        let _ = std::fs::remove_file(intruder());
        let _ = std::fs::remove_file(secret());
        assert_eq!(finding(&networked, "ecriture"), "false");
        assert!(!wrote);
        assert_eq!(finding(&networked, "lancement"), "false");
        assert_eq!(finding(&networked, "durcissement"), "[]");
    }

    #[test]
    fn killing_a_sandboxed_child_closes_its_pipes() {
        let exe = std::env::current_exe().expect("binaire de test");
        let arguments = probe_arguments("wait_for_input");
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let mut spawned = spawn(&exe, &arguments, Policy::Locked).expect("lancement");
        // L'entrée reste ouverte : la sonde attend. On la tue.
        spawned.process.kill();
        spawned.process.wait();
        let mut rest = Vec::new();
        // La lecture se termine : le tube est fermé, le processus est mort.
        let _ = spawned.stdout.read_to_end(&mut rest);
    }

    #[test]
    fn command_line_quoting_survives_spaces_and_quotes() {
        // Le binaire de test reçoit ses arguments intacts : un filtre qui
        // contient des espaces et des guillemets ne correspond à aucun test.
        let exe = std::env::current_exe().expect("binaire de test");
        let mut spawned =
            spawn(&exe, &["--list", r#"a b\\ "c" \"#], Policy::Locked).expect("lancement");
        drop(spawned.stdin);
        let mut output = String::new();
        spawned
            .stdout
            .read_to_string(&mut output)
            .expect("sortie de l'enfant");
        spawned.process.wait();
        assert!(output.contains("0 tests"), "sortie inattendue : {output}");
    }
}
