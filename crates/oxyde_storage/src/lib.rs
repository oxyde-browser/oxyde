//! Profil d'Oxyde sur disque : session, historique, favoris et réglages.
//!
//! Un [`Profile`] est un dossier ; chaque sorte de donnée y a son fichier
//! texte (voir le module `record` pour le format). Les modèles
//! ([`Session`], [`History`], [`Bookmarks`], [`Settings`]) vivent en mémoire
//! et ne touchent pas au disque : c'est le profil qui les lit et les écrit.
//!
//! Un fichier est remplacé d'un coup (écriture à côté, puis renommage) : une
//! coupure de courant laisse l'ancien ou le nouveau, jamais un mélange. Seul
//! l'historique est allongé sur place ; une dernière ligne tronquée y est
//! ignorée à la lecture.
//!
//! LIMITES : rien n'empêche deux navigateurs d'ouvrir le même profil, le
//! dernier qui écrit l'emporte ; les fichiers ne sont pas chiffrés.

mod bookmarks;
mod history;
mod import;
mod record;
mod session;
mod settings;

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use bookmarks::{Bookmark, Bookmarks};
pub use history::{History, Visit};
pub use import::chromium_bookmarks;
pub use session::{Session, SessionTab};
pub use settings::Settings;

/// Taille maximale d'un fichier du profil accepté à la lecture, en octets.
const MAX_FILE: u64 = 32 * 1024 * 1024;

const SESSION_FILE: &str = "session";
const HISTORY_FILE: &str = "historique";
const BOOKMARKS_FILE: &str = "favoris";
const SETTINGS_FILE: &str = "reglages";

/// Dossier d'un profil.
#[derive(Clone, Debug)]
pub struct Profile {
    dir: PathBuf,
}

impl Profile {
    /// Ouvre le profil rangé dans `dir`, en créant le dossier au besoin.
    ///
    /// # Errors
    /// Le dossier n'a pas pu être créé.
    pub fn open(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Dossier du profil par défaut de l'utilisateur, si le système en
    /// désigne un : `%LOCALAPPDATA%\Oxyde\Profil` sous Windows,
    /// `~/Library/Application Support/Oxyde/Profil` sous macOS,
    /// `$XDG_DATA_HOME/oxyde/profil` (ou `~/.local/share/oxyde/profil`)
    /// ailleurs.
    #[must_use]
    pub fn default_dir() -> Option<PathBuf> {
        let var = |name: &str| {
            std::env::var_os(name)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        };
        if cfg!(windows) {
            Some(var("LOCALAPPDATA")?.join("Oxyde").join("Profil"))
        } else if cfg!(target_os = "macos") {
            Some(var("HOME")?.join("Library/Application Support/Oxyde/Profil"))
        } else {
            let data = var("XDG_DATA_HOME").or_else(|| Some(var("HOME")?.join(".local/share")))?;
            Some(data.join("oxyde").join("profil"))
        }
    }

    /// Dossier du profil.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Texte du fichier `name` ; vide s'il manque, s'il est démesuré ou
    /// illisible. Les octets qui ne sont pas de l'UTF-8 sont remplacés.
    fn read(&self, name: &str) -> String {
        let path = self.dir.join(name);
        match fs::metadata(&path) {
            Ok(metadata) if metadata.len() <= MAX_FILE => fs::read(&path)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Remplace le fichier `name` d'un coup.
    fn write(&self, name: &str, text: &str) -> io::Result<()> {
        let path = self.dir.join(name);
        let temporary = self.dir.join(format!("{name}.tmp"));
        let mut file = File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &path)
    }

    /// Session du dernier lancement ; vide s'il n'y en a pas.
    #[must_use]
    pub fn load_session(&self) -> Session {
        Session::decode(&self.read(SESSION_FILE))
    }

    /// Enregistre la session.
    ///
    /// # Errors
    /// Le fichier n'a pas pu être écrit.
    pub fn save_session(&self, session: &Session) -> io::Result<()> {
        self.write(SESSION_FILE, &session.encode())
    }

    /// Historique de navigation ; vide s'il n'y en a pas.
    #[must_use]
    pub fn load_history(&self) -> History {
        History::decode(&self.read(HISTORY_FILE))
    }

    /// Écrit les changements de l'historique : les nouvelles lignes à la
    /// suite du fichier, ou le fichier entier s'il doit être récrit.
    ///
    /// # Errors
    /// Le fichier n'a pas pu être écrit ; les changements sont alors perdus
    /// pour le disque, pas pour la mémoire.
    pub fn save_history(&self, history: &mut History) -> io::Result<()> {
        match history.take_changes() {
            history::Changes::None => Ok(()),
            history::Changes::Rewrite(text) => self.write(HISTORY_FILE, &text),
            history::Changes::Append(lines) => {
                let path = self.dir.join(HISTORY_FILE);
                let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
                if file.metadata()?.len() == 0 {
                    file.write_all(History::header().as_bytes())?;
                }
                file.write_all(lines.as_bytes())
            }
        }
    }

    /// Favoris ; vides s'il n'y en a pas.
    #[must_use]
    pub fn load_bookmarks(&self) -> Bookmarks {
        Bookmarks::decode(&self.read(BOOKMARKS_FILE))
    }

    /// Enregistre les favoris s'ils ont changé.
    ///
    /// # Errors
    /// Le fichier n'a pas pu être écrit.
    pub fn save_bookmarks(&self, bookmarks: &mut Bookmarks) -> io::Result<()> {
        if bookmarks.is_dirty() {
            self.write(BOOKMARKS_FILE, &bookmarks.encode())?;
            bookmarks.mark_saved();
        }
        Ok(())
    }

    /// Ajoute aux favoris du disque ceux de `bookmarks` qui n'y sont pas,
    /// sans rien en retirer : pour une fenêtre qui n'est pas seule à écrire
    /// ce profil (navigation privée).
    ///
    /// # Errors
    /// Le fichier n'a pas pu être écrit.
    pub fn merge_bookmarks(&self, bookmarks: &mut Bookmarks) -> io::Result<()> {
        if !bookmarks.is_dirty() {
            return Ok(());
        }
        let mut stored = self.load_bookmarks();
        for item in bookmarks.items() {
            stored.add(&item.url, &item.title, item.added);
        }
        self.save_bookmarks(&mut stored)?;
        bookmarks.mark_saved();
        Ok(())
    }

    /// Réglages ; vides s'il n'y en a pas.
    #[must_use]
    pub fn load_settings(&self) -> Settings {
        Settings::decode(&self.read(SETTINGS_FILE))
    }

    /// Enregistre les réglages s'ils ont changé.
    ///
    /// # Errors
    /// Le fichier n'a pas pu être écrit.
    pub fn save_settings(&self, settings: &mut Settings) -> io::Result<()> {
        if settings.is_dirty() {
            self.write(SETTINGS_FILE, &settings.encode())?;
            settings.mark_saved();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dossier temporaire propre à un test, supprimé à la fin.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("oxyde-storage-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_new_profile_is_empty() -> io::Result<()> {
        let scratch = Scratch::new("vide");
        let profile = Profile::open(scratch.0.join("a/b"))?;
        assert_eq!(profile.load_session(), Session::default());
        assert!(profile.load_history().visits().is_empty());
        assert!(profile.load_bookmarks().items().is_empty());
        assert_eq!(profile.load_settings().get("x"), None);
        Ok(())
    }

    #[test]
    fn everything_survives_a_reopen() -> io::Result<()> {
        let scratch = Scratch::new("retour");
        let profile = Profile::open(&scratch.0)?;
        let session = Session {
            tabs: vec![SessionTab {
                url: "https://a.example/".to_owned(),
                title: "A".to_owned(),
                pinned: true,
                space: 1,
            }],
            ..Session::default()
        };
        profile.save_session(&session)?;

        let mut history = profile.load_history();
        history.record("https://a.example/", "A", 1);
        profile.save_history(&mut history)?;
        // Deuxième écriture : ajoutée à la suite, sans second en-tête.
        history.record("https://b.example/", "", 2);
        history.set_title("https://b.example/", "B");
        profile.save_history(&mut history)?;

        let mut bookmarks = profile.load_bookmarks();
        bookmarks.add("https://a.example/", "A", 3);
        profile.save_bookmarks(&mut bookmarks)?;
        let mut settings = profile.load_settings();
        settings.set_flag("essai", true);
        profile.save_settings(&mut settings)?;

        let reopened = Profile::open(&scratch.0)?;
        assert_eq!(reopened.load_session(), session);
        assert_eq!(reopened.load_history().visits(), history.visits());
        assert_eq!(reopened.load_history().visits().len(), 2);
        assert_eq!(reopened.load_bookmarks().items(), bookmarks.items());
        assert!(reopened.load_settings().flag("essai", false));
        // Aucun fichier temporaire ne reste.
        let leftovers = fs::read_dir(&scratch.0)?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        Ok(())
    }

    #[test]
    fn merging_bookmarks_never_drops_any() -> io::Result<()> {
        let scratch = Scratch::new("fusion");
        let profile = Profile::open(&scratch.0)?;
        // Deux fenêtres partent du même fichier.
        let mut main = profile.load_bookmarks();
        main.add("https://a.example/", "A", 1);
        profile.save_bookmarks(&mut main)?;
        let mut private = profile.load_bookmarks();
        // La fenêtre principale ajoute un favori, la fenêtre privée un autre.
        main.add("https://b.example/", "B", 2);
        profile.save_bookmarks(&mut main)?;
        private.add("https://c.example/", "C", 3);
        profile.merge_bookmarks(&mut private)?;
        assert!(!private.is_dirty());
        let urls: Vec<String> = profile
            .load_bookmarks()
            .items()
            .iter()
            .map(|b| b.url.clone())
            .collect();
        assert_eq!(
            urls,
            [
                "https://a.example/",
                "https://b.example/",
                "https://c.example/"
            ]
        );
        Ok(())
    }

    #[test]
    fn a_truncated_history_line_is_ignored() -> io::Result<()> {
        let scratch = Scratch::new("coupure");
        let profile = Profile::open(&scratch.0)?;
        let mut history = History::default();
        history.record("https://a.example/", "A", 1);
        history.record("https://b.example/", "B", 2);
        profile.save_history(&mut history)?;
        // Coupure au milieu de la dernière ligne.
        let path = scratch.0.join(HISTORY_FILE);
        let text = fs::read_to_string(&path)?;
        let cut = text.len() - "https://b.example/\tB\n".len();
        fs::write(&path, text.get(..cut).unwrap_or(""))?;
        let reread = profile.load_history();
        assert_eq!(
            reread.visits().iter().map(|v| v.time).collect::<Vec<_>>(),
            [1]
        );
        Ok(())
    }
}
