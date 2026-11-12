//! Règles du standard Fetch appliquées aux requêtes des scripts : en-têtes
//! interdits, requêtes « simples » et pré-vérification CORS
//! (<https://fetch.spec.whatwg.org/>).
//!
//! La page décide ; l'embarqueur exécute les requêtes, y compris la
//! pré-vérification (`OPTIONS`) quand [`FetchRequest::needs_preflight`] le
//! demande, et soumet sa réponse à [`preflight_allows`].

use crate::{FetchRequest, FetchResponse};

/// En-têtes qu'un script ne peut jamais poser (Fetch §2.2.2).
const FORBIDDEN: &[&str] = &[
    "accept-charset",
    "accept-encoding",
    "access-control-request-headers",
    "access-control-request-method",
    "connection",
    "content-length",
    "cookie",
    "cookie2",
    "date",
    "dnt",
    "expect",
    "host",
    "keep-alive",
    "origin",
    "referer",
    "set-cookie",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "user-agent",
    "via",
];

/// Vrai si un script n'a pas le droit de poser l'en-tête `name`.
pub(crate) fn is_forbidden_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    FORBIDDEN.contains(&name.as_str()) || name.starts_with("proxy-") || name.starts_with("sec-")
}

/// Vrai si `name` est un nom d'en-tête valide (un « token » HTTP).
pub(crate) fn is_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

/// Vrai si `value` peut être la valeur d'un en-tête.
pub(crate) fn is_header_value(value: &str) -> bool {
    !value.bytes().any(|b| matches!(b, 0 | b'\r' | b'\n'))
}

/// Vrai si `name` est une méthode HTTP valide et permise aux scripts.
pub(crate) fn is_allowed_method(name: &str) -> bool {
    is_header_name(name)
        && !["CONNECT", "TRACE", "TRACK"]
            .iter()
            .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
}

/// Met en majuscules les méthodes usuelles (`post` → `POST`).
pub(crate) fn normalize_method(name: &str) -> String {
    let upper = name.to_ascii_uppercase();
    if ["DELETE", "GET", "HEAD", "OPTIONS", "POST", "PUT"].contains(&upper.as_str()) {
        upper
    } else {
        name.to_owned()
    }
}

/// Vrai si l'en-tête figure sur la liste sûre de CORS : il ne déclenche pas
/// de pré-vérification (Fetch §2.2.2, « CORS-safelisted request-header »).
fn is_safelisted(name: &str, value: &str) -> bool {
    if value.len() > 128 {
        return false;
    }
    match name.to_ascii_lowercase().as_str() {
        "accept" | "accept-language" | "content-language" => true,
        "content-type" => {
            let essence = value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            matches!(
                essence.as_str(),
                "application/x-www-form-urlencoded" | "multipart/form-data" | "text/plain"
            )
        }
        _ => false,
    }
}

/// Noms, en minuscules et triés, des en-têtes de `headers` hors liste sûre.
fn unsafe_header_names(headers: &[(String, String)]) -> Vec<String> {
    let mut names: Vec<String> = headers
        .iter()
        .filter(|(name, value)| !is_safelisted(name, value))
        .map(|(name, _)| name.to_ascii_lowercase())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Vrai si une requête vers une autre origine doit être précédée d'une
/// pré-vérification : méthode ou en-têtes hors des requêtes « simples ».
pub(crate) fn needs_preflight(method: &str, headers: &[(String, String)]) -> bool {
    !matches!(method, "GET" | "HEAD" | "POST") || !unsafe_header_names(headers).is_empty()
}

impl FetchRequest {
    /// En-têtes de la requête de pré-vérification (`OPTIONS`) à envoyer
    /// avant la requête elle-même.
    #[must_use]
    pub fn preflight_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![(
            "Access-Control-Request-Method".to_owned(),
            self.method.clone(),
        )];
        let names = unsafe_header_names(&self.headers);
        if !names.is_empty() {
            headers.push(("Access-Control-Request-Headers".to_owned(), names.join(",")));
        }
        if let Some(origin) = &self.origin {
            headers.push(("Origin".to_owned(), origin.clone()));
        }
        headers
    }
}

/// Valeurs, séparées par des virgules, de l'en-tête `name` de `response`.
fn listed(response: &FetchResponse, name: &str) -> Vec<String> {
    response
        .headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case(name))
        .flat_map(|(_, value)| value.split(','))
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Vérifie que `response`, la réponse du serveur à la pré-vérification de
/// `request`, autorise la requête (Fetch §4.8, « CORS-preflight fetch »).
///
/// # Errors
/// Un message destiné au script si le serveur n'autorise pas l'origine, la
/// méthode ou l'un des en-têtes.
pub fn preflight_allows(request: &FetchRequest, response: &FetchResponse) -> Result<(), String> {
    let refused = |what: &str| {
        Err(format!(
            "requête vers {} refusée par la pré-vérification CORS : {what}",
            request.url.host_str().unwrap_or("l'hôte distant")
        ))
    };
    if !(200..300).contains(&response.status) {
        return refused("le serveur n'a pas répondu favorablement");
    }
    let origin = request.origin.as_deref().unwrap_or("");
    let origins = listed(response, "access-control-allow-origin");
    if !origins
        .iter()
        .any(|allowed| allowed == "*" || allowed.eq_ignore_ascii_case(origin))
    {
        return refused("l'origine n'est pas autorisée");
    }
    let methods = listed(response, "access-control-allow-methods");
    let method_allowed = matches!(request.method.as_str(), "GET" | "HEAD" | "POST")
        || methods
            .iter()
            .any(|allowed| allowed == "*" || allowed.eq_ignore_ascii_case(&request.method));
    if !method_allowed {
        return refused("la méthode n'est pas autorisée");
    }
    let allowed_headers = listed(response, "access-control-allow-headers");
    let wildcard = allowed_headers.iter().any(|allowed| allowed == "*");
    for name in unsafe_header_names(&request.headers) {
        // `Authorization` doit être nommé : le joker ne le couvre pas.
        let covered = allowed_headers.contains(&name) || (wildcard && name != "authorization");
        if !covered {
            return refused(&format!("l'en-tête « {name} » n'est pas autorisé"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn request(method: &str, headers: &[(&str, &str)]) -> FetchRequest {
        FetchRequest {
            id: 1,
            initiator: Url::parse("https://page.test/").expect("url"),
            url: Url::parse("https://api.test/v1").expect("url"),
            method: method.to_owned(),
            headers: headers
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            body: Vec::new(),
            origin: Some("https://exemple.test".to_owned()),
            needs_preflight: true,
        }
    }

    fn response(status: u16, headers: &[(&str, &str)]) -> FetchResponse {
        FetchResponse {
            url: Url::parse("https://api.test/v1").expect("url"),
            status,
            headers: headers
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
            body: Vec::new(),
        }
    }

    #[test]
    fn simple_requests_skip_the_preflight() {
        assert!(!needs_preflight("GET", &[]));
        let form = [(
            "Content-Type".to_owned(),
            "application/x-www-form-urlencoded".to_owned(),
        )];
        assert!(!needs_preflight("POST", &form));
        let json = [("Content-Type".to_owned(), "application/json".to_owned())];
        assert!(needs_preflight("POST", &json));
        assert!(needs_preflight("PUT", &[]));
        assert!(needs_preflight(
            "GET",
            &[("X-Jeton".to_owned(), "1".to_owned())]
        ));
    }

    #[test]
    fn forbidden_headers_and_methods() {
        for name in ["Cookie", "host", "Sec-Fetch-Mode", "Proxy-Authorization"] {
            assert!(is_forbidden_header(name), "{name}");
        }
        assert!(!is_forbidden_header("X-Jeton"));
        assert!(is_allowed_method("PATCH"));
        assert!(!is_allowed_method("trace"));
        assert!(!is_allowed_method("GE T"));
        assert_eq!(normalize_method("post"), "POST");
        assert_eq!(normalize_method("patch"), "patch");
    }

    #[test]
    fn preflight_checks_origin_method_and_headers() {
        let wanted = request("PUT", &[("X-Jeton", "1"), ("Authorization", "a")]);
        assert_eq!(
            wanted.preflight_headers(),
            vec![
                ("Access-Control-Request-Method".to_owned(), "PUT".to_owned()),
                (
                    "Access-Control-Request-Headers".to_owned(),
                    "authorization,x-jeton".to_owned()
                ),
                ("Origin".to_owned(), "https://exemple.test".to_owned()),
            ]
        );
        let allow = [
            ("access-control-allow-origin", "https://exemple.test"),
            ("access-control-allow-methods", "GET, PUT"),
            ("access-control-allow-headers", "X-Jeton, Authorization"),
        ];
        assert!(preflight_allows(&wanted, &response(204, &allow)).is_ok());
        // Le joker ne couvre pas `Authorization`.
        let wildcard = [
            ("access-control-allow-origin", "*"),
            ("access-control-allow-methods", "*"),
            ("access-control-allow-headers", "*"),
        ];
        assert!(preflight_allows(&wanted, &response(200, &wildcard)).is_err());
        assert!(preflight_allows(&request("DELETE", &[]), &response(200, &wildcard)).is_ok());
        assert!(preflight_allows(&wanted, &response(403, &allow)).is_err());
        let other_origin = [("access-control-allow-origin", "https://ailleurs.test")];
        assert!(preflight_allows(&request("GET", &[]), &response(200, &other_origin)).is_err());
    }
}
