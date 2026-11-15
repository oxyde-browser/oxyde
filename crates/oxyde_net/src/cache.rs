//! Cache HTTP en mémoire (RFC 9111, pour un cache privé de navigateur).
//!
//! Il garde les réponses `200` aux requêtes `GET` que leur serveur permet de
//! garder, les ressert tant qu'elles sont fraîches, et, périmées, les
//! revalide par une requête conditionnelle (`If-None-Match`,
//! `If-Modified-Since`) : une réponse `304` ressert le corps gardé.
//!
//! Le cache est cloisonné par site du document demandeur : un site ne peut
//! pas savoir, au temps de réponse, ce qu'un autre a déjà chargé.
//!
//! LIMITES CONNUES : rien n'est écrit sur disque (le cache se vide avec le
//! processus) ; seules les réponses `200` sont gardées (ni redirections, ni
//! `206`) ; les dates ne sont lues qu'au format courant (`Sun, 06 Nov 1994
//! 08:49:37 GMT`) ; pas de `stale-while-revalidate` ; une réponse gardée
//! n'est pas invalidée par un `POST` vers la même adresse.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use url::Url;

use crate::cookies::site_of;
use crate::{HttpRequest, Response};

/// Taille totale des corps gardés (64 Mio), et taille maximale d'un corps
/// gardé (8 Mio).
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 8 * 1024 * 1024;
/// Durée de fraîcheur maximale accordée sans consigne du serveur, d'après
/// l'ancienneté de la ressource.
const MAX_HEURISTIC: Duration = Duration::from_secs(24 * 3600);

/// Réponse gardée.
struct Entry {
    response: Response,
    /// Instant où la réponse a été reçue ou revalidée.
    stored: Instant,
    /// Durée pendant laquelle elle se ressert sans rien demander.
    lifetime: Duration,
    /// En-têtes de la requête dont la réponse dépend (`Vary`), avec la
    /// valeur qu'ils avaient.
    vary: Vec<(String, Option<String>)>,
    /// Rang du dernier usage, pour évincer la moins récemment servie.
    used: u64,
}

/// Ce que le cache sait d'une requête.
pub(crate) enum Lookup {
    /// Réponse fraîche, à resservir telle quelle.
    Fresh(Response),
    /// Réponse périmée : en-têtes à joindre pour la revalider.
    Stale(Vec<(String, String)>),
    /// Rien d'utilisable.
    Miss,
}

/// Cache HTTP.
#[derive(Default)]
pub(crate) struct Cache {
    entries: HashMap<String, Entry>,
    bytes: usize,
    clock: u64,
}

/// Valeur de l'en-tête `name` (nom en minuscules) parmi `headers`.
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// Consignes d'un en-tête `Cache-Control` : (nom en minuscules, valeur).
fn directives(value: Option<&str>) -> Vec<(String, Option<String>)> {
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            Some(match part.split_once('=') {
                Some((name, value)) => (
                    name.trim().to_ascii_lowercase(),
                    Some(value.trim().trim_matches('"').to_owned()),
                ),
                None => (part.to_ascii_lowercase(), None),
            })
        })
        .collect()
}

/// Secondes depuis l'époque Unix d'une date HTTP au format courant
/// (`Sun, 06 Nov 1994 08:49:37 GMT`).
fn http_date(text: &str) -> Option<i64> {
    let mut parts = text.trim().split_ascii_whitespace();
    let _weekday = parts.next()?;
    let day: i64 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut clock = parts.next()?.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next()?.parse().ok()?;
    let second: i64 = clock.next()?.parse().ok()?;
    if parts.next()? != "GMT"
        || !(1..=31).contains(&day)
        || !(1970..=9999).contains(&year)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // Jours depuis l'époque (algorithme de Howard Hinnant, calendrier civil).
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted.rem_euclid(400);
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

/// Vrai si la requête peut être servie par le cache et sa réponse gardée.
fn cacheable(request: &HttpRequest) -> bool {
    let bypass = [
        "authorization",
        "range",
        "if-none-match",
        "if-modified-since",
    ]
    .iter()
    .any(|name| header(&request.headers, name).is_some());
    let refuses = directives(header(&request.headers, "cache-control"))
        .iter()
        .any(|(name, _)| name == "no-cache" || name == "no-store")
        || header(&request.headers, "pragma").is_some_and(|v| v.contains("no-cache"));
    request.method == "GET" && !bypass && !refuses
}

/// Clé d'une requête : le site du document demandeur, puis l'adresse sans
/// son fragment.
fn key(request: &HttpRequest) -> String {
    let mut url: Url = request.url.clone();
    url.set_fragment(None);
    let site = site_of(request.initiator.as_ref().unwrap_or(&request.url));
    format!("{site} {url}")
}

/// Durée de fraîcheur de `response`, ou `None` s'il ne faut pas la garder.
fn lifetime(response: &Response) -> Option<Duration> {
    let control = directives(header(&response.headers, "cache-control"));
    let has = |wanted: &str| control.iter().any(|(name, _)| name == wanted);
    if has("no-store") || header(&response.headers, "vary").is_some_and(|v| v.trim() == "*") {
        return None;
    }
    let validator = header(&response.headers, "etag").is_some()
        || header(&response.headers, "last-modified").is_some();
    let age = header(&response.headers, "age")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let date = header(&response.headers, "date").and_then(http_date);
    let seconds = |span: i64| u64::try_from(span.max(0)).unwrap_or(0);
    let fresh = if has("no-cache") {
        // À revalider avant chaque usage.
        Some(0)
    } else if let Some(max_age) = control
        .iter()
        .find(|(name, _)| name == "max-age")
        .and_then(|(_, value)| value.as_deref()?.parse::<u64>().ok())
    {
        Some(max_age)
    } else if let Some(expires) = header(&response.headers, "expires") {
        // Une date illisible vaut « déjà périmé ».
        Some(match (http_date(expires), date) {
            (Some(expires), Some(date)) => seconds(expires - date),
            _ => 0,
        })
    } else {
        // Sans consigne : un dixième de l'ancienneté de la ressource.
        header(&response.headers, "last-modified")
            .and_then(http_date)
            .zip(date)
            .map(|(modified, date)| (seconds(date - modified) / 10).min(MAX_HEURISTIC.as_secs()))
    };
    match fresh {
        Some(fresh) if fresh > age => Some(Duration::from_secs(fresh - age)),
        // Périmée d'emblée : utile seulement si elle peut être revalidée.
        Some(_) | None if validator => Some(Duration::ZERO),
        _ => None,
    }
}

impl Cache {
    /// Ce que le cache peut faire pour `request`.
    pub(crate) fn lookup(&mut self, request: &HttpRequest) -> Lookup {
        if !cacheable(request) {
            return Lookup::Miss;
        }
        self.clock += 1;
        let clock = self.clock;
        let Some(entry) = self.entries.get_mut(&key(request)) else {
            return Lookup::Miss;
        };
        // La réponse gardée répond-elle à cette requête-ci ?
        let matches = entry
            .vary
            .iter()
            .all(|(name, value)| header(&request.headers, name) == value.as_deref());
        if !matches {
            return Lookup::Miss;
        }
        entry.used = clock;
        if entry.stored.elapsed() < entry.lifetime {
            return Lookup::Fresh(entry.response.clone());
        }
        let mut validators = Vec::new();
        if let Some(etag) = header(&entry.response.headers, "etag") {
            validators.push(("if-none-match".to_owned(), etag.to_owned()));
        }
        if let Some(modified) = header(&entry.response.headers, "last-modified") {
            validators.push(("if-modified-since".to_owned(), modified.to_owned()));
        }
        if validators.is_empty() {
            Lookup::Miss
        } else {
            Lookup::Stale(validators)
        }
    }

    /// Garde `response`, réponse à `request`, si son serveur le permet.
    pub(crate) fn store(&mut self, request: &HttpRequest, response: &Response) {
        if !cacheable(request) || response.status != 200 || response.body.len() > MAX_ENTRY_BYTES {
            return;
        }
        let key = key(request);
        self.remove(&key);
        let Some(lifetime) = lifetime(response) else {
            return;
        };
        let vary = header(&response.headers, "vary")
            .unwrap_or_default()
            .split(',')
            .map(|name| name.trim().to_ascii_lowercase())
            .filter(|name| !name.is_empty())
            .map(|name| {
                let value = header(&request.headers, &name).map(str::to_owned);
                (name, value)
            })
            .collect();
        // Place faite en évinçant les réponses les moins récemment servies.
        while self.bytes.saturating_add(response.body.len()) > MAX_BYTES {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.remove(&oldest);
        }
        self.clock += 1;
        self.bytes = self.bytes.saturating_add(response.body.len());
        self.entries.insert(
            key,
            Entry {
                response: response.clone(),
                stored: Instant::now(),
                lifetime,
                vary,
                used: self.clock,
            },
        );
    }

    /// Le serveur a répondu `304` (`not_modified`) à la revalidation de
    /// `request` : la réponse gardée redevient fraîche, avec les en-têtes
    /// que le `304` met à jour, et est rendue. `None` si elle n'y est plus.
    pub(crate) fn refresh(
        &mut self,
        request: &HttpRequest,
        not_modified: &Response,
    ) -> Option<Response> {
        let entry = self.entries.get_mut(&key(request))?;
        for (name, value) in &not_modified.headers {
            // Ces en-têtes décrivent le `304` lui-même, pas la ressource.
            if matches!(
                name.as_str(),
                "content-length" | "content-encoding" | "transfer-encoding"
            ) {
                continue;
            }
            entry.response.headers.retain(|(known, _)| known != name);
            entry.response.headers.push((name.clone(), value.clone()));
        }
        entry.stored = Instant::now();
        entry.lifetime = lifetime(&entry.response).unwrap_or(Duration::ZERO);
        Some(entry.response.clone())
    }

    /// Oublie tout ce qui est gardé pour les documents du site de
    /// `document` (l'utilisateur recharge la page).
    pub(crate) fn forget_site(&mut self, document: &Url) {
        let prefix = format!("{} ", site_of(document));
        let doomed: Vec<String> = self
            .entries
            .keys()
            .filter(|key| key.starts_with(&prefix))
            .cloned()
            .collect();
        for key in doomed {
            self.remove(&key);
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes = self.bytes.saturating_sub(entry.response.body.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(url: &str, headers: &[(&str, &str)]) -> HttpRequest {
        HttpRequest {
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            ..HttpRequest::get(&Url::parse(url).expect("url"))
        }
    }

    fn response(headers: &[(&str, &str)], body: &str) -> Response {
        Response {
            url: Url::parse("https://exemple.test/a").expect("url"),
            status: 200,
            content_type: None,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn dates_and_lifetimes() {
        assert_eq!(http_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(
            http_date("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(784_111_777)
        );
        assert_eq!(
            http_date("Tue, 29 Feb 2000 12:00:00 GMT"),
            Some(951_825_600)
        );
        assert_eq!(http_date("hier"), None);
        let life = |headers: &[(&str, &str)]| lifetime(&response(headers, "x"));
        assert_eq!(
            life(&[("cache-control", "public, max-age=60"), ("age", "10")]),
            Some(Duration::from_secs(50))
        );
        assert_eq!(life(&[("cache-control", "no-store, max-age=60")]), None);
        assert_eq!(
            life(&[("cache-control", "max-age=60"), ("vary", "*")]),
            None
        );
        // `no-cache` : gardée si elle peut être revalidée, jamais fraîche.
        assert_eq!(
            life(&[("cache-control", "no-cache"), ("etag", "\"v1\"")]),
            Some(Duration::ZERO)
        );
        assert_eq!(life(&[("cache-control", "no-cache")]), None);
        assert_eq!(
            life(&[
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("expires", "Sun, 06 Nov 1994 08:59:37 GMT")
            ]),
            Some(Duration::from_secs(600))
        );
        // Sans consigne : un dixième de l'ancienneté, un jour au plus.
        assert_eq!(
            life(&[
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("last-modified", "Sun, 06 Nov 1994 08:39:37 GMT")
            ]),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            life(&[
                ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
                ("last-modified", "Sun, 06 Nov 1984 08:39:37 GMT")
            ]),
            Some(MAX_HEURISTIC)
        );
        // Ni consigne ni moyen de revalider : pas gardée.
        assert_eq!(life(&[("content-type", "text/css")]), None);
    }

    #[test]
    fn fresh_stale_and_revalidated() {
        let mut cache = Cache::default();
        let get = request("https://exemple.test/a#ancre", &[]);
        assert!(matches!(cache.lookup(&get), Lookup::Miss));
        cache.store(
            &get,
            &response(
                &[("cache-control", "max-age=3600"), ("etag", "\"v1\"")],
                "frais",
            ),
        );
        assert!(matches!(cache.lookup(&get), Lookup::Fresh(r) if r.body == b"frais"));
        // Le fragment ne compte pas ; un autre site demandeur a son propre cache.
        assert!(matches!(
            cache.lookup(&request("https://exemple.test/a", &[])),
            Lookup::Fresh(_)
        ));
        let mut other = request("https://exemple.test/a", &[]);
        other.initiator = Some(Url::parse("https://ailleurs.test/").expect("url"));
        assert!(matches!(cache.lookup(&other), Lookup::Miss));
        // Une requête qui refuse le cache, ou n'est pas un GET, passe outre.
        assert!(matches!(
            cache.lookup(&request(
                "https://exemple.test/a",
                &[("cache-control", "no-cache")]
            )),
            Lookup::Miss
        ));
        let mut post = request("https://exemple.test/a", &[]);
        post.method = "POST".to_owned();
        assert!(matches!(cache.lookup(&post), Lookup::Miss));

        // Périmée d'emblée : revalidée, puis resservie avec ses en-têtes à jour.
        let stale = request("https://exemple.test/b", &[]);
        cache.store(
            &stale,
            &response(
                &[
                    ("cache-control", "no-cache"),
                    ("etag", "\"v1\""),
                    ("last-modified", "hier"),
                ],
                "gardé",
            ),
        );
        let Lookup::Stale(validators) = cache.lookup(&stale) else {
            panic!("réponse périmée attendue");
        };
        assert_eq!(
            validators,
            [
                ("if-none-match".to_owned(), "\"v1\"".to_owned()),
                ("if-modified-since".to_owned(), "hier".to_owned())
            ]
        );
        let mut not_modified = response(
            &[("cache-control", "max-age=60"), ("content-length", "0")],
            "",
        );
        not_modified.status = 304;
        let refreshed = cache
            .refresh(&stale, &not_modified)
            .expect("réponse gardée");
        assert_eq!(refreshed.body, "gardé".as_bytes());
        assert_eq!(
            header(&refreshed.headers, "cache-control"),
            Some("max-age=60")
        );
        assert_eq!(header(&refreshed.headers, "content-length"), None);
        assert!(matches!(cache.lookup(&stale), Lookup::Fresh(_)));
        // Recharger la page oublie ce qui est gardé pour son site, et lui seul.
        let mut elsewhere = request("https://exemple.test/c", &[]);
        elsewhere.initiator = Some(Url::parse("https://ailleurs.test/").expect("url"));
        cache.store(
            &elsewhere,
            &response(&[("cache-control", "max-age=60")], "autre"),
        );
        cache.forget_site(&Url::parse("https://www.exemple.test/page").expect("url"));
        assert!(matches!(cache.lookup(&get), Lookup::Miss));
        assert!(matches!(cache.lookup(&stale), Lookup::Miss));
        assert!(matches!(cache.lookup(&elsewhere), Lookup::Fresh(_)));
    }

    #[test]
    fn vary_and_eviction() {
        let mut cache = Cache::default();
        let french = request("https://exemple.test/v", &[("accept-language", "fr")]);
        cache.store(
            &french,
            &response(
                &[("cache-control", "max-age=60"), ("vary", "Accept-Language")],
                "bonjour",
            ),
        );
        assert!(matches!(cache.lookup(&french), Lookup::Fresh(_)));
        assert!(matches!(
            cache.lookup(&request(
                "https://exemple.test/v",
                &[("accept-language", "en")]
            )),
            Lookup::Miss
        ));
        assert!(matches!(
            cache.lookup(&request("https://exemple.test/v", &[])),
            Lookup::Miss
        ));
        // Un corps trop gros n'est pas gardé ; le cache évince le moins servi.
        let big = "x".repeat(MAX_ENTRY_BYTES + 1);
        let huge = request("https://exemple.test/gros", &[]);
        cache.store(&huge, &response(&[("cache-control", "max-age=60")], &big));
        assert!(matches!(cache.lookup(&huge), Lookup::Miss));
        let chunk = "y".repeat(MAX_ENTRY_BYTES);
        for rank in 0..9 {
            let get = request(&format!("https://exemple.test/{rank}"), &[]);
            cache.store(&get, &response(&[("cache-control", "max-age=60")], &chunk));
        }
        assert!(cache.bytes <= MAX_BYTES);
        assert!(matches!(cache.lookup(&french), Lookup::Miss));
        assert!(matches!(
            cache.lookup(&request("https://exemple.test/8", &[])),
            Lookup::Fresh(_)
        ));
    }
}
