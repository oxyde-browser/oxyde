//! Enregistrements d'une ligne : des champs séparés par des tabulations.
//!
//! Les fichiers du profil sont du texte UTF-8, un enregistrement par ligne.
//! Dans un champ, la barre oblique inverse, la tabulation et les fins de
//! ligne sont échappées (`\\`, `\t`, `\n`, `\r`) : une ligne ne contient
//! donc jamais d'autre tabulation que ses séparateurs.
//!
//! Un fichier du profil peut avoir été tronqué ou modifié : la lecture ne
//! panique jamais et borne ce qu'elle accepte.

/// Taille maximale d'un champ, en octets. Au-delà, le champ est coupé à
/// l'écriture et la ligne ignorée à la lecture.
pub(crate) const MAX_FIELD: usize = 4096;

/// Coupe `text` à `max` octets au plus, sur une frontière de caractère.
pub(crate) fn clip(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.get(..end).unwrap_or("")
}

/// Ligne (sans fin de ligne) portant `fields`.
pub(crate) fn encode(fields: &[&str]) -> String {
    let mut line = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            line.push('\t');
        }
        for c in clip(field, MAX_FIELD).chars() {
            match c {
                '\\' => line.push_str("\\\\"),
                '\t' => line.push_str("\\t"),
                '\n' => line.push_str("\\n"),
                '\r' => line.push_str("\\r"),
                _ => line.push(c),
            }
        }
    }
    line
}

/// Champs de `line`, ou `None` si un champ dépasse [`MAX_FIELD`].
pub(crate) fn decode(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    for raw in line.split('\t') {
        if raw.len() > 2 * MAX_FIELD {
            return None;
        }
        let mut field = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                field.push(c);
                continue;
            }
            match chars.next() {
                Some('t') => field.push('\t'),
                Some('n') => field.push('\n'),
                Some('r') => field.push('\r'),
                // Échappement inconnu ou barre finale : gardé tel quel.
                Some(other) => field.push(other),
                None => field.push('\\'),
            }
        }
        if field.len() > MAX_FIELD {
            return None;
        }
        fields.push(field);
    }
    Some(fields)
}

/// Lignes utiles d'un fichier dont la première doit être `magic` ; aucune
/// si l'en-tête ne correspond pas (fichier d'une autre version ou abîmé).
pub(crate) fn body<'a>(text: &'a str, magic: &str) -> impl Iterator<Item = &'a str> {
    let mut lines = text.lines();
    let known = lines.next() == Some(magic);
    lines.filter(move |line| known && !line.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_escapes_separators() {
        let fields = ["a\tb", "ligne 1\nligne 2\r", "barre \\ oblique", "", "é→✓"];
        let line = encode(&fields);
        assert_eq!(line.matches('\t').count(), fields.len() - 1);
        assert!(!line.contains('\n'));
        assert_eq!(
            decode(&line).as_deref(),
            Some(fields.map(String::from).as_slice())
        );
    }

    #[test]
    fn long_fields_are_clipped_on_a_character_boundary() {
        let long = "é".repeat(MAX_FIELD);
        let line = encode(&[&long]);
        let decoded = decode(&line).unwrap_or_default();
        assert_eq!(decoded.len(), 1);
        assert!(
            decoded
                .first()
                .is_some_and(|f| f.len() <= MAX_FIELD && f.chars().all(|c| c == 'é'))
        );
        // Une ligne démesurée venue du disque est refusée.
        assert_eq!(decode(&"x".repeat(3 * MAX_FIELD)), None);
    }

    #[test]
    fn unknown_escapes_and_headers_are_tolerated() {
        assert_eq!(decode("a\\qb\\"), Some(vec!["aqb\\".to_owned()]));
        assert_eq!(
            body("oxyde test 1\nun\n\ndeux\n", "oxyde test 1").collect::<Vec<_>>(),
            ["un", "deux"]
        );
        assert_eq!(body("autre chose\nun\n", "oxyde test 1").count(), 0);
        assert_eq!(body("", "oxyde test 1").count(), 0);
    }
}
