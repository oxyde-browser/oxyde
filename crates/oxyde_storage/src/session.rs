//! Session : les onglets ouverts, à rouvrir au prochain lancement.

use crate::record;

const MAGIC: &str = "oxyde session 1";

/// Nombre maximal d'onglets conservés.
const MAX_TABS: usize = 500;
/// Nombre maximal d'onglets fermés conservés.
const MAX_CLOSED: usize = 25;

/// Onglet d'une session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionTab {
    /// Adresse de la page.
    pub url: String,
    /// Titre de la page, affiché en attendant son chargement.
    pub title: String,
    /// Onglet épinglé.
    pub pinned: bool,
    /// Index de l'espace auquel l'onglet appartient.
    pub space: usize,
}

/// État des onglets d'une fenêtre.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    /// Onglets ouverts, dans l'ordre de la fenêtre.
    pub tabs: Vec<SessionTab>,
    /// Index de l'onglet actif. INVARIANT après [`Session::decode`] : nul ou
    /// inférieur à `tabs.len()`.
    pub active: usize,
    /// Index de l'onglet du second volet, si la vue est scindée.
    pub split: Option<usize>,
    /// Index de l'espace courant.
    pub space: usize,
    /// Onglets fermés récemment, du plus récent au plus ancien.
    pub closed: Vec<SessionTab>,
}

fn tab_line(kind: &str, tab: &SessionTab) -> String {
    record::encode(&[
        kind,
        &tab.url,
        &tab.title,
        if tab.pinned { "1" } else { "0" },
        &tab.space.to_string(),
    ])
}

fn parse_tab(fields: &[String]) -> Option<SessionTab> {
    let [_, url, title, pinned, space] = fields else {
        return None;
    };
    Some(SessionTab {
        url: url.clone(),
        title: title.clone(),
        pinned: pinned == "1",
        space: space.parse().ok()?,
    })
}

impl Session {
    /// Texte du fichier de session.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut text = format!("{MAGIC}\n");
        let mut push = |line: String| {
            text.push_str(&line);
            text.push('\n');
        };
        for tab in self.tabs.iter().take(MAX_TABS) {
            push(tab_line("tab", tab));
        }
        for tab in self.closed.iter().take(MAX_CLOSED) {
            push(tab_line("closed", tab));
        }
        push(record::encode(&["active", &self.active.to_string()]));
        if let Some(split) = self.split {
            push(record::encode(&["split", &split.to_string()]));
        }
        push(record::encode(&["space", &self.space.to_string()]));
        text
    }

    /// Session lue dans `text`. Les lignes illisibles sont ignorées ; un
    /// fichier d'un autre format donne une session vide.
    #[must_use]
    pub fn decode(text: &str) -> Self {
        let mut session = Self::default();
        for line in record::body(text, MAGIC) {
            let Some(fields) = record::decode(line) else {
                continue;
            };
            let number = || fields.get(1).and_then(|n| n.parse::<usize>().ok());
            match fields.first().map(String::as_str) {
                Some("tab") if session.tabs.len() < MAX_TABS => {
                    session.tabs.extend(parse_tab(&fields));
                }
                Some("closed") if session.closed.len() < MAX_CLOSED => {
                    session.closed.extend(parse_tab(&fields));
                }
                Some("active") => session.active = number().unwrap_or(0),
                Some("split") => session.split = number(),
                Some("space") => session.space = number().unwrap_or(0),
                _ => {}
            }
        }
        if session.active >= session.tabs.len() {
            session.active = 0;
        }
        session.split = session
            .split
            .filter(|&split| split < session.tabs.len() && split != session.active);
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(url: &str, pinned: bool, space: usize) -> SessionTab {
        SessionTab {
            url: url.to_owned(),
            title: format!("Titre de {url}"),
            pinned,
            space,
        }
    }

    #[test]
    fn round_trip() {
        let session = Session {
            tabs: vec![
                tab("https://a.example/", true, 0),
                tab("https://b.example/x\ty", false, 2),
            ],
            active: 1,
            split: Some(0),
            space: 2,
            closed: vec![tab("https://c.example/", false, 1)],
        };
        assert_eq!(Session::decode(&session.encode()), session);
    }

    #[test]
    fn damaged_files_give_a_consistent_session() {
        // Index hors bornes, lignes tronquées, nombres invalides.
        let text = format!(
            "{MAGIC}\ntab\thttps://a.example/\tA\t0\t0\ntab\tincomplet\nactive\t7\nsplit\t0\nspace\tx\nbruit\n"
        );
        let session = Session::decode(&text);
        assert_eq!(session.tabs.len(), 1);
        assert_eq!((session.active, session.split, session.space), (0, None, 0));
        assert_eq!(
            Session::decode("autre format\ntab\tu\tt\t0\t0\n"),
            Session::default()
        );
        assert_eq!(Session::decode(""), Session::default());
    }

    #[test]
    fn tab_count_is_bounded() {
        let session = Session {
            tabs: (0..MAX_TABS + 10)
                .map(|i| tab(&format!("https://e.example/{i}"), false, 0))
                .collect(),
            ..Session::default()
        };
        assert_eq!(Session::decode(&session.encode()).tabs.len(), MAX_TABS);
    }
}
