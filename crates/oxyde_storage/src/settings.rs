//! Réglages : des paires clé-valeur.

use std::collections::BTreeMap;

use crate::record;

const MAGIC: &str = "oxyde reglages 1";

/// Nombre maximal de réglages conservés.
const MAX_SETTINGS: usize = 1_000;

/// Réglages du navigateur. Les clés et leur sens appartiennent à
/// l'application ; une clé absente vaut sa valeur par défaut.
#[derive(Debug, Default)]
pub struct Settings {
    values: BTreeMap<String, String>,
    dirty: bool,
}

impl Settings {
    /// Réglages lus dans le texte d'un fichier. Les lignes illisibles sont
    /// ignorées.
    #[must_use]
    pub fn decode(text: &str) -> Self {
        let mut settings = Self::default();
        for line in record::body(text, MAGIC) {
            if let Some(fields) = record::decode(line)
                && let [key, value] = fields.as_slice()
            {
                settings.set(key, value);
            }
        }
        settings.dirty = false;
        settings
    }

    /// Texte du fichier des réglages.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut text = format!("{MAGIC}\n");
        for (key, value) in &self.values {
            text.push_str(&record::encode(&[key, value]));
            text.push('\n');
        }
        text
    }

    /// Valeur du réglage `key`, s'il a été fixé.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Valeur du réglage booléen `key` ; `default` s'il n'a pas été fixé.
    #[must_use]
    pub fn flag(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some("1") => true,
            Some("0") => false,
            _ => default,
        }
    }

    /// Fixe le réglage `key`. Sans effet si la clé est vide ou si la liste
    /// est pleine.
    pub fn set(&mut self, key: &str, value: &str) {
        if key.is_empty() || self.get(key) == Some(value) {
            return;
        }
        if !self.values.contains_key(key) && self.values.len() >= MAX_SETTINGS {
            return;
        }
        self.values.insert(
            record::clip(key, record::MAX_FIELD).to_owned(),
            record::clip(value, record::MAX_FIELD).to_owned(),
        );
        self.dirty = true;
    }

    /// Fixe le réglage booléen `key`.
    pub fn set_flag(&mut self, key: &str, value: bool) {
        self.set(key, if value { "1" } else { "0" });
    }

    /// Rend au réglage `key` sa valeur par défaut.
    pub fn reset(&mut self, key: &str) {
        self.dirty |= self.values.remove(key).is_some();
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
    fn values_flags_and_round_trip() {
        let mut settings = Settings::default();
        assert!(!settings.flag("onglets.horizontaux", false));
        assert!(!settings.is_dirty());
        settings.set("recherche.moteur", "duckduckgo");
        settings.set_flag("onglets.horizontaux", true);
        settings.set("vide", "");
        assert!(settings.is_dirty());
        let reread = Settings::decode(&settings.encode());
        assert_eq!(reread.get("recherche.moteur"), Some("duckduckgo"));
        assert!(reread.flag("onglets.horizontaux", false));
        assert_eq!(reread.get("vide"), Some(""));
        assert_eq!(reread.get("absent"), None);
        assert!(!reread.is_dirty());

        settings.reset("recherche.moteur");
        assert_eq!(settings.get("recherche.moteur"), None);
        assert!(Settings::decode("autre format\na\tb\n").get("a").is_none());
    }
}
