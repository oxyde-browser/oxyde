//! Favoris.

use crate::record;

const MAGIC: &str = "oxyde favoris 1";

/// Nombre maximal de favoris conservés.
const MAX_BOOKMARKS: usize = 10_000;

/// Page mise en favori.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    /// Adresse de la page.
    pub url: String,
    /// Titre donné au favori.
    pub title: String,
    /// Moment de l'ajout, en secondes depuis l'époque Unix.
    pub added: u64,
}

/// Favoris, dans l'ordre de leur ajout. INVARIANT : une adresse y figure au
/// plus une fois.
#[derive(Debug, Default)]
pub struct Bookmarks {
    items: Vec<Bookmark>,
    dirty: bool,
}

impl Bookmarks {
    /// Favoris lus dans le texte d'un fichier. Les lignes illisibles et les
    /// doublons sont ignorés.
    #[must_use]
    pub fn decode(text: &str) -> Self {
        let mut bookmarks = Self::default();
        for line in record::body(text, MAGIC) {
            let Some(fields) = record::decode(line) else {
                continue;
            };
            if let [added, url, title] = fields.as_slice()
                && let Ok(added) = added.parse()
            {
                bookmarks.add(url, title, added);
            }
        }
        bookmarks.dirty = false;
        bookmarks
    }

    /// Texte du fichier des favoris.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut text = format!("{MAGIC}\n");
        for item in &self.items {
            text.push_str(&record::encode(&[
                &item.added.to_string(),
                &item.url,
                &item.title,
            ]));
            text.push('\n');
        }
        text
    }

    /// Ajoute `url` aux favoris ; renvoie `false` si elle y est déjà, si elle
    /// est vide ou démesurée, ou si la liste est pleine.
    pub fn add(&mut self, url: &str, title: &str, added: u64) -> bool {
        if url.is_empty()
            || url.len() > record::MAX_FIELD
            || self.items.len() >= MAX_BOOKMARKS
            || self.contains(url)
        {
            return false;
        }
        self.items.push(Bookmark {
            url: url.to_owned(),
            title: record::clip(title, record::MAX_FIELD).to_owned(),
            added,
        });
        self.dirty = true;
        true
    }

    /// Retire `url` des favoris ; renvoie `true` si elle y était.
    pub fn remove(&mut self, url: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.url != url);
        let removed = self.items.len() != before;
        self.dirty |= removed;
        removed
    }

    /// Ajoute `url` si elle n'est pas en favori, la retire sinon ; renvoie
    /// `true` si elle est en favori après l'appel.
    pub fn toggle(&mut self, url: &str, title: &str, added: u64) -> bool {
        if self.remove(url) {
            return false;
        }
        self.add(url, title, added)
    }

    /// Vrai si `url` est en favori.
    #[must_use]
    pub fn contains(&self, url: &str) -> bool {
        self.items.iter().any(|item| item.url == url)
    }

    /// Favoris, dans l'ordre de leur ajout.
    #[must_use]
    pub fn items(&self) -> &[Bookmark] {
        &self.items
    }

    /// Vrai s'il reste des changements à écrire sur disque.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(crate) fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_adds_then_removes() {
        let mut bookmarks = Bookmarks::default();
        assert!(bookmarks.toggle("https://a.example/", "A", 1));
        assert!(bookmarks.toggle("https://b.example/", "B", 2));
        assert!(!bookmarks.add("https://a.example/", "doublon", 3));
        assert!(bookmarks.contains("https://a.example/"));
        assert!(!bookmarks.toggle("https://a.example/", "A", 4));
        assert_eq!(
            bookmarks
                .items()
                .iter()
                .map(|b| b.url.as_str())
                .collect::<Vec<_>>(),
            ["https://b.example/"]
        );
        assert!(bookmarks.is_dirty());
        assert!(!bookmarks.add("", "vide", 5));
    }

    #[test]
    fn round_trip_ignores_damage_and_duplicates() {
        let mut bookmarks = Bookmarks::default();
        bookmarks.add("https://a.example/", "Titre\tA", 1);
        bookmarks.add("https://b.example/", "", 2);
        let mut text = bookmarks.encode();
        text.push_str("pas un nombre\tu\tt\n1\thttps://a.example/\tdoublon\nincomplet\n");
        let reread = Bookmarks::decode(&text);
        assert_eq!(reread.items(), bookmarks.items());
        assert!(!reread.is_dirty());
        assert!(
            Bookmarks::decode("autre format\n1\tu\tt\n")
                .items()
                .is_empty()
        );
    }
}
