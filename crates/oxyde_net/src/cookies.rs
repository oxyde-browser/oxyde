//! Cookies HTTP (RFC 6265 et sa révision « 6265bis ») : analyse de
//! `Set-Cookie`, pot à cookies, choix des cookies à joindre à une requête.
//!
//! Règles de protection appliquées :
//! - un cookie `Secure` n'est accepté et renvoyé qu'en `https` ;
//! - un cookie `HttpOnly` est invisible des scripts, qui ne peuvent ni le
//!   lire ni l'écraser ;
//! - `SameSite` vaut `Lax` par défaut ; une requête émise par un document
//!   d'un autre site ne reçoit que les cookies `SameSite=None; Secure`, et
//!   sa réponse ne peut poser que des cookies de cette sorte (pas de cookies
//!   tiers implicites) ;
//! - les préfixes `__Secure-` et `__Host-` sont vérifiés.
//!
//! LIMITES CONNUES : sans la liste des suffixes publics, le « site » d'un
//! hôte est approché (deux derniers libellés, trois pour les formes du type
//! `exemple.co.uk`) ; le pot vit en mémoire, il n'est pas encore écrit sur
//! disque ; pas de partitionnement (`Partitioned`).

use std::time::{SystemTime, UNIX_EPOCH};

use url::Url;

/// Taille maximale d'un cookie (nom et valeur), en octets.
const MAX_SIZE: usize = 4096;
/// Nombre maximal de cookies par domaine.
const MAX_PER_DOMAIN: usize = 180;
/// Nombre maximal de cookies dans le pot.
const MAX_COOKIES: usize = 3000;

/// Attribut `SameSite` d'un cookie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SameSite {
    Strict,
    Lax,
    None,
}

#[derive(Clone, Debug)]
struct Cookie {
    name: String,
    value: String,
    /// Domaine, en minuscules, sans point initial.
    domain: String,
    /// Vrai si le cookie n'est rendu qu'à l'hôte exact qui l'a posé.
    host_only: bool,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    /// Échéance en secondes depuis l'époque Unix ; `None` : cookie de session.
    expires: Option<u64>,
    /// Rang de création, pour l'ordre d'envoi.
    order: u64,
}

/// Qui lit ou écrit le pot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// La couche réseau, pour une requête ou une réponse HTTP.
    Http,
    /// Un script (`document.cookie`) : les cookies `HttpOnly` lui échappent.
    Script,
}

/// Pot à cookies.
#[derive(Debug, Default)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
    next: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Hôte de `url`, en minuscules, sans crochets.
fn host_of(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    Some(
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase(),
    )
}

fn is_ip(host: &str) -> bool {
    host.contains(':') || host.parse::<std::net::Ipv4Addr>().is_ok()
}

/// Domaine enregistrable approché d'un hôte : ses deux derniers libellés,
/// ou trois quand l'avant-dernier est un libellé générique sous un domaine
/// national (`exemple.co.uk`). Une adresse IP est son propre site.
fn registrable(host: &str) -> &str {
    if is_ip(host) {
        return host;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let count = labels.len();
    if count <= 2 {
        return host;
    }
    const GENERIC: &[&str] = &[
        "co", "com", "org", "net", "ac", "gov", "edu", "or", "ne", "go", "gouv", "asso", "nom",
        "sch", "ltd", "plc", "me", "mil", "gob", "nic",
    ];
    let national = labels[count - 1].len() == 2;
    let keep = if national && GENERIC.contains(&labels[count - 2]) {
        3
    } else {
        2
    };
    let skipped: usize = labels[..count - keep].iter().map(|l| l.len() + 1).sum();
    host.get(skipped..).unwrap_or(host)
}

/// Vrai si `a` et `b` sont du même site (même domaine enregistrable).
#[must_use]
pub fn same_site(a: &Url, b: &Url) -> bool {
    match (host_of(a), host_of(b)) {
        (Some(x), Some(y)) => registrable(&x) == registrable(&y),
        // Deux adresses sans hôte (fichiers locaux) : même site si même schéma.
        (None, None) => a.scheme() == b.scheme(),
        _ => false,
    }
}

/// Vrai si `host` est `domain` ou l'un de ses sous-domaines.
fn domain_matches(host: &str, domain: &str) -> bool {
    host == domain
        || (!is_ip(host)
            && host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes().get(host.len() - domain.len() - 1) == Some(&b'.'))
}

/// Vrai si le chemin `request` de la requête est couvert par `cookie`.
fn path_matches(request: &str, cookie: &str) -> bool {
    request == cookie
        || (request.starts_with(cookie)
            && (cookie.ends_with('/') || request.as_bytes().get(cookie.len()) == Some(&b'/')))
}

/// Chemin par défaut d'un cookie : le répertoire de l'adresse.
fn default_path(url: &Url) -> String {
    let path = url.path();
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(last) => path.get(..last).unwrap_or("/").to_owned(),
    }
}

/// Jours écoulés depuis l'époque Unix jusqu'à la date civile donnée.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y.rem_euclid(400);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Analyse une date de cookie (RFC 6265 §5.1.1), tolérante sur la forme.
/// Renvoie l'instant en secondes depuis l'époque Unix (0 pour une date
/// antérieure).
fn parse_date(text: &str) -> Option<u64> {
    let (mut time, mut day, mut month, mut year) = (None, None, None, None);
    let delimiter = |c: char| !(c.is_ascii_alphanumeric() || c == ':');
    for token in text.split(delimiter).filter(|t| !t.is_empty()) {
        if time.is_none() && token.contains(':') {
            let parts: Vec<&str> = token.split(':').collect();
            if let [h, m, s] = parts.as_slice()
                && let (Ok(h), Ok(m), Ok(s)) =
                    (h.parse::<i64>(), m.parse::<i64>(), s.parse::<i64>())
            {
                time = Some((h, m, s));
                continue;
            }
        }
        let digits = token.chars().take_while(char::is_ascii_digit).count();
        if day.is_none() && (1..=2).contains(&digits) {
            day = token.get(..digits).and_then(|d| d.parse::<i64>().ok());
            continue;
        }
        if month.is_none() && token.len() >= 3 {
            const MONTHS: [&str; 12] = [
                "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
            ];
            let prefix = token.get(..3)?.to_ascii_lowercase();
            if let Some(index) = MONTHS.iter().position(|m| *m == prefix) {
                month = i64::try_from(index + 1).ok();
                continue;
            }
        }
        if year.is_none() && (2..=4).contains(&digits) {
            year = token.get(..digits).and_then(|y| y.parse::<i64>().ok());
        }
    }
    let ((hour, minute, second), day, month, mut year) = (time?, day?, month?, year?);
    if (70..=99).contains(&year) {
        year += 1900;
    } else if (0..=69).contains(&year) {
        year += 2000;
    }
    if !(1..=31).contains(&day) || year < 1601 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    Some(u64::try_from(seconds).unwrap_or(0))
}

impl CookieJar {
    /// Pot vide.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Nombre de cookies gardés.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    /// Vrai si le pot est vide.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    fn purge(&mut self) {
        let now = now();
        self.cookies.retain(|c| c.expires.is_none_or(|at| at > now));
    }

    /// Enregistre le cookie décrit par `header` (la valeur d'un en-tête
    /// `Set-Cookie`, ou le texte affecté à `document.cookie`), reçu pour
    /// `url`. `cross_site` : la requête venait d'un document d'un autre
    /// site. Renvoie `false` si le cookie est refusé.
    pub fn store(&mut self, url: &Url, header: &str, access: Access, cross_site: bool) -> bool {
        let Some(host) = host_of(url) else {
            return false;
        };
        let https = url.scheme() == "https";
        let mut parts = header.split(';');
        let pair = parts.next().unwrap_or("");
        let (name, value) = match pair.split_once('=') {
            Some((name, value)) => (name.trim(), value.trim()),
            None => ("", pair.trim()),
        };
        if (name.is_empty() && value.is_empty())
            || name.len() + value.len() > MAX_SIZE
            || name.chars().chain(value.chars()).any(|c| c.is_control())
        {
            return false;
        }
        let now = now();
        let mut cookie = Cookie {
            name: name.to_owned(),
            value: value.to_owned(),
            domain: host.clone(),
            host_only: true,
            path: default_path(url),
            secure: false,
            http_only: false,
            same_site: SameSite::Lax,
            expires: None,
            order: self.next,
        };
        let (mut max_age, mut expires, mut domain_given) = (None, None, false);
        for part in parts {
            let (key, attribute) = match part.split_once('=') {
                Some((key, attribute)) => (key.trim(), attribute.trim()),
                None => (part.trim(), ""),
            };
            match key.to_ascii_lowercase().as_str() {
                "expires" => expires = parse_date(attribute).or(expires),
                "max-age" => {
                    if let Ok(seconds) = attribute.parse::<i64>() {
                        max_age = Some(match u64::try_from(seconds) {
                            Ok(positive) if positive > 0 => now.saturating_add(positive),
                            _ => 0,
                        });
                    }
                }
                "domain" => {
                    let domain = attribute.trim_start_matches('.').to_ascii_lowercase();
                    if !domain.is_empty() {
                        domain_given = true;
                        if domain == host {
                            // Le domaine est l'hôte : le cookie couvre ses sous-domaines,
                            // sauf si l'hôte est une adresse IP.
                            cookie.host_only = is_ip(&host);
                        } else if domain_matches(&host, &domain)
                            && domain.len() >= registrable(&host).len()
                        {
                            cookie.domain = domain;
                            cookie.host_only = false;
                        } else {
                            // Domaine étranger à l'hôte, ou suffixe public.
                            return false;
                        }
                    }
                }
                "path" if attribute.starts_with('/') => cookie.path = attribute.to_owned(),
                "secure" => cookie.secure = true,
                "httponly" => cookie.http_only = true,
                "samesite" => {
                    cookie.same_site = match attribute.to_ascii_lowercase().as_str() {
                        "strict" => SameSite::Strict,
                        "none" => SameSite::None,
                        _ => SameSite::Lax,
                    };
                }
                _ => {}
            }
        }
        cookie.expires = max_age.or(expires);
        let refused = (cookie.secure && !https)
            || (cookie.http_only && access == Access::Script)
            || (cookie.same_site == SameSite::None && !cookie.secure)
            || (cross_site && cookie.same_site != SameSite::None)
            || (cookie.name.starts_with("__Secure-") && !cookie.secure)
            || (cookie.name.starts_with("__Host-")
                && (!cookie.secure || domain_given || cookie.path != "/"));
        if refused {
            return false;
        }
        let same = |c: &Cookie| {
            c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path
        };
        if let Some(index) = self.cookies.iter().position(same) {
            let existing = &self.cookies[index];
            // Un script n'écrase pas un cookie `HttpOnly`, ni une page non
            // sécurisée un cookie `Secure`.
            if (existing.http_only && access == Access::Script) || (existing.secure && !https) {
                return false;
            }
            cookie.order = existing.order;
            self.cookies.remove(index);
        }
        if cookie.expires.is_some_and(|at| at <= now) {
            // Échéance passée : c'était une suppression.
            return true;
        }
        self.next += 1;
        // Limites : le plus ancien cookie du domaine, puis du pot, cède la place.
        let site = registrable(&cookie.domain).to_owned();
        let in_site = |c: &Cookie| registrable(&c.domain) == site;
        if self.cookies.iter().filter(|c| in_site(c)).count() >= MAX_PER_DOMAIN
            && let Some(oldest) = self.cookies.iter().position(in_site)
        {
            self.cookies.remove(oldest);
        }
        if self.cookies.len() >= MAX_COOKIES {
            self.cookies.remove(0);
        }
        self.cookies.push(cookie);
        true
    }

    /// Valeur de l'en-tête `Cookie` pour une requête vers `url`, ou — pour
    /// un script — le texte de `document.cookie`. `cross_site` : la requête
    /// est émise par un document d'un autre site. `None` s'il n'y a rien à
    /// envoyer.
    pub fn header_for(&mut self, url: &Url, access: Access, cross_site: bool) -> Option<String> {
        self.purge();
        let host = host_of(url)?;
        let https = url.scheme() == "https";
        let path = url.path();
        let mut matching: Vec<&Cookie> = self
            .cookies
            .iter()
            .filter(|c| {
                (if c.host_only {
                    host == c.domain
                } else {
                    domain_matches(&host, &c.domain)
                }) && path_matches(path, &c.path)
                    && (!c.secure || https)
                    && (!c.http_only || access == Access::Http)
                    && (!cross_site || c.same_site == SameSite::None)
            })
            .collect();
        if matching.is_empty() {
            return None;
        }
        // Chemins les plus précis d'abord, puis les plus anciens.
        matching.sort_by(|a, b| b.path.len().cmp(&a.path.len()).then(a.order.cmp(&b.order)));
        Some(
            matching
                .iter()
                .map(|c| {
                    if c.name.is_empty() {
                        c.value.clone()
                    } else {
                        format!("{}={}", c.name, c.value)
                    }
                })
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).expect("adresse de test")
    }

    fn http(jar: &mut CookieJar, at: &str) -> Option<String> {
        jar.header_for(&url(at), Access::Http, false)
    }

    #[test]
    fn stores_and_returns_by_domain_and_path() {
        let mut jar = CookieJar::new();
        let page = url("https://www.exemple.test/compte/accueil");
        assert!(jar.store(&page, "a=1", Access::Http, false));
        assert!(jar.store(
            &page,
            "b=2; Domain=exemple.test; Path=/",
            Access::Http,
            false
        ));
        assert!(jar.store(&page, "c=3; Path=/compte/prive", Access::Http, false));
        assert_eq!(
            http(&mut jar, "https://www.exemple.test/compte/prive/x").as_deref(),
            Some("c=3; a=1; b=2")
        );
        // `a` est lié à l'hôte et au répertoire `/compte`.
        assert_eq!(
            http(&mut jar, "https://www.exemple.test/ailleurs").as_deref(),
            Some("b=2")
        );
        assert_eq!(
            http(&mut jar, "https://api.exemple.test/").as_deref(),
            Some("b=2")
        );
        assert_eq!(http(&mut jar, "https://autre.test/"), None);
    }

    #[test]
    fn refuses_foreign_and_public_domains() {
        let mut jar = CookieJar::new();
        let page = url("https://www.exemple.co.uk/");
        assert!(!jar.store(&page, "a=1; Domain=autre.test", Access::Http, false));
        assert!(!jar.store(&page, "a=1; Domain=co.uk", Access::Http, false));
        assert!(!jar.store(&page, "a=1; Domain=uk", Access::Http, false));
        assert!(jar.store(&page, "a=1; Domain=exemple.co.uk", Access::Http, false));
        assert!(jar.is_empty() || jar.len() == 1);
    }

    #[test]
    fn secure_http_only_and_prefixes() {
        let mut jar = CookieJar::new();
        let plain = url("http://exemple.test/");
        let safe = url("https://exemple.test/");
        assert!(!jar.store(&plain, "s=1; Secure", Access::Http, false));
        assert!(jar.store(&safe, "s=1; Secure", Access::Http, false));
        assert!(jar.store(&safe, "h=1; HttpOnly", Access::Http, false));
        assert!(!jar.store(&safe, "h=2", Access::Script, false));
        assert!(!jar.store(&safe, "x=1; HttpOnly", Access::Script, false));
        assert!(!jar.store(&safe, "__Host-k=1; Secure; Path=/sous", Access::Http, false));
        assert!(jar.store(&safe, "__Host-k=1; Secure; Path=/", Access::Http, false));
        assert_eq!(
            http(&mut jar, "http://exemple.test/").as_deref(),
            Some("h=1")
        );
        assert_eq!(
            jar.header_for(&safe, Access::Script, false).as_deref(),
            Some("s=1; __Host-k=1")
        );
    }

    #[test]
    fn same_site_and_expiry() {
        let mut jar = CookieJar::new();
        let safe = url("https://exemple.test/");
        assert!(jar.store(&safe, "lax=1", Access::Http, false));
        assert!(!jar.store(&safe, "none=1; SameSite=None", Access::Http, false));
        assert!(jar.store(&safe, "none=1; SameSite=None; Secure", Access::Http, false));
        // Une réponse à une requête venue d'un autre site ne pose pas de cookie ordinaire.
        assert!(!jar.store(&safe, "tiers=1", Access::Http, true));
        assert_eq!(
            jar.header_for(&safe, Access::Http, true).as_deref(),
            Some("none=1")
        );
        assert!(jar.store(&safe, "lax=1; Max-Age=0", Access::Http, false));
        assert!(jar.store(
            &safe,
            "vieux=1; Expires=Wed, 21 Oct 2015 07:28:00 GMT",
            Access::Http,
            false
        ));
        assert_eq!(
            http(&mut jar, "https://exemple.test/").as_deref(),
            Some("none=1")
        );
        assert_eq!(parse_date("Thu, 01 Jan 1970 00:01:00 GMT"), Some(60));
        assert_eq!(
            parse_date("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(784_111_777)
        );
        assert!(same_site(
            &url("https://a.exemple.test/"),
            &url("http://b.exemple.test/x")
        ));
        assert!(!same_site(
            &url("https://a.co.uk/"),
            &url("https://b.co.uk/")
        ));
    }
}
