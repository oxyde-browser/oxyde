//! Liste des suffixes publics (<https://publicsuffix.org/>) : les domaines
//! sous lesquels chacun peut enregistrer le sien (`com`, `co.uk`,
//! `github.io`…). Elle dit où passe la frontière d'un site : `a.exemple.co.uk`
//! et `b.exemple.co.uk` sont du même site, `exemple.co.uk` et `autre.co.uk`
//! non, et aucun des deux ne peut poser un cookie pour `co.uk`.
//!
//! La liste est embarquée telle que publiée (fichier
//! `data/public_suffix_list.dat`, sous licence MPL 2.0), sections ICANN et
//! privée comprises.
//!
//! LIMITES CONNUES : la liste est celle du jour de la compilation, elle n'est
//! pas mise à jour en service ; un hôte est attendu en minuscules et, s'il
//! porte des caractères hors de l'ASCII, sous sa forme `xn--` (celle que
//! rend `url::Url`).

use std::collections::HashSet;
use std::sync::OnceLock;

/// La liste publiée.
const LIST: &str = include_str!("../data/public_suffix_list.dat");

/// Règles de la liste, noms écrits en ASCII.
struct Rules {
    /// Règles ordinaires : `com`, `co.uk`.
    exact: HashSet<String>,
    /// Règles à joker, sans leur `*.` : `ck` pour `*.ck`.
    wildcard: HashSet<String>,
    /// Exceptions aux jokers, sans leur `!` : `www.ck`.
    exception: HashSet<String>,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut rules = Rules {
            exact: HashSet::new(),
            wildcard: HashSet::new(),
            exception: HashSet::new(),
        };
        for line in LIST.lines() {
            // Une règle tient avant le premier blanc de sa ligne.
            let rule = line.split_whitespace().next().unwrap_or_default();
            if rule.is_empty() || rule.starts_with("//") {
                continue;
            }
            let (set, name) = if let Some(name) = rule.strip_prefix('!') {
                (&mut rules.exception, name)
            } else if let Some(name) = rule.strip_prefix("*.") {
                (&mut rules.wildcard, name)
            } else {
                (&mut rules.exact, rule)
            };
            // Les noms hors de l'ASCII sont ramenés à leur forme `xn--`.
            if let Ok(ascii) = idna::domain_to_ascii(name) {
                set.insert(ascii);
            }
        }
        rules
    })
}

/// Position, dans `host`, du début de son suffixe public.
fn suffix_start(host: &str) -> usize {
    let rules = rules();
    // Débuts des suffixes de `host`, du plus long au plus court.
    let starts = std::iter::once(0).chain(host.match_indices('.').map(|(at, _)| at + 1));
    for start in starts {
        let candidate = host.get(start..).unwrap_or_default();
        let parent = candidate.split_once('.').map(|(_, parent)| parent);
        if rules.exception.contains(candidate) {
            // Une exception : le suffixe est celui du dessus.
            return parent.map_or(start, |parent| host.len() - parent.len());
        }
        if rules.exact.contains(candidate) || parent.is_some_and(|p| rules.wildcard.contains(p)) {
            return start;
        }
    }
    // Aucune règle : le dernier libellé est le suffixe.
    host.rfind('.').map_or(0, |at| at + 1)
}

/// Suffixe public de `host` (`co.uk` pour `www.exemple.co.uk`).
#[must_use]
pub fn public_suffix(host: &str) -> &str {
    host.get(suffix_start(host)..).unwrap_or(host)
}

/// Domaine enregistrable de `host` : son suffixe public précédé d'un libellé
/// (`exemple.co.uk` pour `www.exemple.co.uk`). Un hôte qui est lui-même un
/// suffixe public est rendu tel quel.
#[must_use]
pub fn registrable_domain(host: &str) -> &str {
    let suffix = suffix_start(host);
    // Le libellé qui précède le suffixe, s'il y en a un.
    let Some(before) = suffix.checked_sub(1).and_then(|end| host.get(..end)) else {
        return host;
    };
    let start = before.rfind('.').map_or(0, |at| at + 1);
    host.get(start..).unwrap_or(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffixes_follow_the_published_rules() {
        for (host, suffix, registrable) in [
            ("exemple.com", "com", "exemple.com"),
            ("a.b.exemple.com", "com", "exemple.com"),
            ("www.exemple.co.uk", "co.uk", "exemple.co.uk"),
            ("exemple.gouv.fr", "gouv.fr", "exemple.gouv.fr"),
            // Section privée : chaque utilisateur de github.io a son site.
            ("moi.github.io", "github.io", "moi.github.io"),
            ("a.moi.github.io", "github.io", "moi.github.io"),
            // Joker : `*.ck`, et son exception `!www.ck`.
            ("a.b.c.ck", "c.ck", "b.c.ck"),
            ("www.ck", "ck", "www.ck"),
            ("a.www.ck", "ck", "www.ck"),
            // Un suffixe public est son propre « domaine ».
            ("co.uk", "co.uk", "co.uk"),
            ("com", "com", "com"),
            // Nom inconnu de la liste : le dernier libellé fait suffixe.
            (
                "serveur.inconnu-de-la-liste",
                "inconnu-de-la-liste",
                "serveur.inconnu-de-la-liste",
            ),
            ("localhost", "localhost", "localhost"),
            // Règle écrite hors de l'ASCII dans la liste (`公司.cn`).
            (
                "exemple.xn--55qx5d.cn",
                "xn--55qx5d.cn",
                "exemple.xn--55qx5d.cn",
            ),
        ] {
            assert_eq!(public_suffix(host), suffix, "{host}");
            assert_eq!(registrable_domain(host), registrable, "{host}");
        }
        // La liste embarquée est bien la vraie.
        assert!(rules().exact.len() > 8000 && rules().wildcard.contains("ck"));
    }
}
