//! Historique de navigation.
//!
//! Le fichier est un journal : une visite s'y ajoute en une ligne, sans
//! récrire le reste. Les corrections (titre connu plus tard, visite
//! supprimée) sont elles aussi des lignes, rejouées à la lecture ; le fichier
//! est récrit en entier quand elles s'accumulent.

use crate::record;

const MAGIC: &str = "oxyde historique 1";

/// Nombre maximal de visites conservées ; les plus anciennes sont oubliées.
const MAX_VISITS: usize = 20_000;
/// Nombre de visites récentes parmi lesquelles un titre tardif cherche sa page.
const TITLE_WINDOW: usize = 200;

/// Page visitée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    /// Adresse de la page.
    pub url: String,
    /// Titre de la page (vide s'il n'est pas connu).
    pub title: String,
    /// Moment de la visite, en secondes depuis l'époque Unix.
    pub time: u64,
}

/// Ce qu'il faut écrire sur disque pour enregistrer l'historique.
pub(crate) enum Changes {
    /// Rien.
    None,
    /// Ces lignes, à la suite du fichier.
    Append(String),
    /// Le fichier entier.
    Rewrite(String),
}

/// Historique de navigation, de la visite la plus ancienne à la plus récente.
#[derive(Debug, Default)]
pub struct History {
    visits: Vec<Visit>,
    /// Lignes pas encore écrites sur disque.
    pending: Vec<String>,
    /// Le fichier doit être récrit en entier.
    rewrite: bool,
}

fn visit_line(visit: &Visit) -> String {
    record::encode(&["v", &visit.time.to_string(), &visit.url, &visit.title])
}

impl History {
    /// Historique lu dans le texte d'un fichier. Les lignes illisibles sont
    /// ignorées.
    #[must_use]
    pub fn decode(text: &str) -> Self {
        let mut history = Self::default();
        let mut corrections = 0_usize;
        for line in record::body(text, MAGIC) {
            let Some(fields) = record::decode(line) else {
                continue;
            };
            match fields.as_slice() {
                [kind, time, url, title] if kind == "v" => {
                    if let Ok(time) = time.parse() {
                        history.push(url, title, time);
                    }
                }
                [kind, url, title] if kind == "t" => {
                    corrections += 1;
                    history.retitle(url, title);
                }
                [kind, time, url] if kind == "d" => {
                    corrections += 1;
                    if let Ok(time) = time.parse() {
                        history.delete(url, time);
                    }
                }
                _ => {}
            }
        }
        history.pending.clear();
        // Trop de corrections ou de visites oubliées : le journal a grossi
        // pour rien, autant le récrire.
        history.rewrite |= corrections > 64 + history.visits.len() / 4;
        history
    }

    /// Texte complet du fichier.
    fn encode(&self) -> String {
        let mut text = format!("{MAGIC}\n");
        for visit in &self.visits {
            text.push_str(&visit_line(visit));
            text.push('\n');
        }
        text
    }

    /// Ajoute une visite sans rien noter pour le disque. Une visite de la
    /// même page que la précédente la remplace (rechargement).
    fn push(&mut self, url: &str, title: &str, time: u64) {
        if url.is_empty() || url.len() > record::MAX_FIELD {
            return;
        }
        let visit = Visit {
            url: url.to_owned(),
            title: record::clip(title, record::MAX_FIELD).to_owned(),
            time,
        };
        match self.visits.last_mut() {
            Some(last) if last.url == url => *last = visit,
            _ => self.visits.push(visit),
        }
        if self.visits.len() > MAX_VISITS {
            let excess = self.visits.len() - MAX_VISITS;
            self.visits.drain(..excess);
            self.rewrite = true;
        }
    }

    fn retitle(&mut self, url: &str, title: &str) -> bool {
        let recent = self.visits.iter_mut().rev().take(TITLE_WINDOW);
        for visit in recent {
            if visit.url == url {
                if visit.title == title {
                    return false;
                }
                record::clip(title, record::MAX_FIELD).clone_into(&mut visit.title);
                return true;
            }
        }
        false
    }

    fn delete(&mut self, url: &str, time: u64) -> bool {
        let found = self
            .visits
            .iter()
            .rposition(|visit| visit.time == time && visit.url == url);
        match found {
            Some(index) => {
                self.visits.remove(index);
                true
            }
            None => false,
        }
    }

    /// Enregistre la visite de `url` à l'instant `time` (secondes depuis
    /// l'époque Unix). Une adresse démesurée est ignorée.
    pub fn record(&mut self, url: &str, title: &str, time: u64) {
        if url.is_empty() || url.len() > record::MAX_FIELD {
            return;
        }
        self.push(url, title, time);
        if let Some(last) = self.visits.last() {
            self.pending.push(visit_line(last));
        }
    }

    /// Donne son titre à la visite la plus récente de `url`, quand il n'est
    /// connu qu'après le chargement.
    pub fn set_title(&mut self, url: &str, title: &str) {
        if self.retitle(url, title) {
            self.pending.push(record::encode(&["t", url, title]));
        }
    }

    /// Supprime la visite de `url` à l'instant `time` ; renvoie `true` si
    /// elle existait.
    pub fn remove(&mut self, url: &str, time: u64) -> bool {
        let removed = self.delete(url, time);
        if removed {
            self.pending
                .push(record::encode(&["d", &time.to_string(), url]));
        }
        removed
    }

    /// Oublie tout l'historique.
    pub fn clear(&mut self) {
        self.visits.clear();
        self.pending.clear();
        self.rewrite = true;
    }

    /// Visites, de la plus ancienne à la plus récente.
    #[must_use]
    pub fn visits(&self) -> &[Visit] {
        &self.visits
    }

    /// Visites dont le titre ou l'adresse contient chaque mot de `query`
    /// (sans tenir compte de la casse), de la plus récente à la plus ancienne.
    /// Une recherche vide les rend toutes.
    pub fn search<'a>(&'a self, query: &str) -> impl Iterator<Item = &'a Visit> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.visits.iter().rev().filter(move |visit| {
            if words.is_empty() {
                return true;
            }
            let title = visit.title.to_lowercase();
            let url = visit.url.to_lowercase();
            words
                .iter()
                .all(|word| title.contains(word) || url.contains(word))
        })
    }

    /// Vrai s'il reste des changements à écrire sur disque.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.rewrite || !self.pending.is_empty()
    }

    /// Ce qu'il faut écrire pour que le disque rejoigne la mémoire ; les
    /// changements sont alors considérés comme écrits.
    pub(crate) fn take_changes(&mut self) -> Changes {
        if std::mem::take(&mut self.rewrite) {
            self.pending.clear();
            return Changes::Rewrite(self.encode());
        }
        if self.pending.is_empty() {
            return Changes::None;
        }
        let mut text = String::new();
        for line in self.pending.drain(..) {
            text.push_str(&line);
            text.push('\n');
        }
        Changes::Append(text)
    }

    /// Texte de l'en-tête, pour créer le fichier avant d'y ajouter des lignes.
    pub(crate) fn header() -> String {
        format!("{MAGIC}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Écrit les changements dans `file` comme le ferait le profil.
    fn flush(history: &mut History, file: &mut String) {
        match history.take_changes() {
            Changes::None => {}
            Changes::Append(lines) => {
                if file.is_empty() {
                    file.push_str(&History::header());
                }
                file.push_str(&lines);
            }
            Changes::Rewrite(text) => *file = text,
        }
    }

    #[test]
    fn journal_replays_to_the_same_history() {
        let mut file = String::new();
        let mut history = History::default();
        history.record("https://a.example/", "", 10);
        history.record("https://b.example/", "B", 20);
        flush(&mut history, &mut file);
        history.set_title("https://a.example/", "A");
        // Rechargement : la visite est remplacée, pas doublée.
        history.record("https://b.example/", "B bis", 30);
        history.record("https://c.example/", "C\tavec tabulation", 40);
        assert!(history.remove("https://c.example/", 40));
        assert!(!history.remove("https://c.example/", 41));
        assert!(history.is_dirty());
        flush(&mut history, &mut file);
        assert!(!history.is_dirty());

        let reread = History::decode(&file);
        assert_eq!(reread.visits(), history.visits());
        assert_eq!(
            reread
                .visits()
                .iter()
                .map(|v| (v.url.as_str(), v.title.as_str(), v.time))
                .collect::<Vec<_>>(),
            [
                ("https://a.example/", "A", 10),
                ("https://b.example/", "B bis", 30)
            ]
        );
        assert!(!reread.is_dirty());
    }

    #[test]
    fn search_matches_every_word_most_recent_first() {
        let mut history = History::default();
        history.record("https://docs.example/rust", "Le langage Rust", 1);
        history.record("https://news.example/", "Actualités", 2);
        history.record("https://docs.example/css", "Feuilles de style", 3);
        let found = |query: &str| history.search(query).map(|v| v.time).collect::<Vec<_>>();
        assert_eq!(found(""), [3, 2, 1]);
        assert_eq!(found("DOCS"), [3, 1]);
        assert_eq!(found("docs rust"), [1]);
        assert_eq!(found("absent"), Vec::<u64>::new());
    }

    #[test]
    fn old_visits_are_forgotten_and_damage_is_ignored() {
        let mut history = History::default();
        for time in 0..(MAX_VISITS as u64 + 5) {
            history.record(&format!("https://e.example/{time}"), "", time);
        }
        assert_eq!(history.visits().len(), MAX_VISITS);
        assert_eq!(history.visits().first().map(|v| v.time), Some(5));
        let Changes::Rewrite(text) = history.take_changes() else {
            panic!("le dépassement doit récrire le fichier");
        };
        assert_eq!(History::decode(&text).visits().len(), MAX_VISITS);

        let damaged =
            format!("{MAGIC}\nv\tpas un nombre\tu\tt\nv\t5\nx\ty\nv\t7\thttps://ok.example/\tOK\n");
        assert_eq!(History::decode(&damaged).visits().len(), 1);
        assert!(
            History::decode("autre format\nv\t1\tu\tt\n")
                .visits()
                .is_empty()
        );

        history.clear();
        assert!(history.visits().is_empty());
        assert!(matches!(history.take_changes(), Changes::Rewrite(_)));
    }
}
