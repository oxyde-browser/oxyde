//! Sonde : affiche l'état, les en-têtes et la taille du corps d'une réponse.
fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage : sonde ADRESSE");
        return;
    };
    let url = match oxyde_net::Url::parse(&address) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("adresse invalide : {e}");
            return;
        }
    };
    // Deuxième argument facultatif : un en-tête `User-Agent` à essayer.
    let mut request = oxyde_net::HttpRequest::get(&url);
    if let Some(agent) = std::env::args().nth(2) {
        request.headers.push(("User-Agent".to_owned(), agent));
    }
    match oxyde_net::Client::new().and_then(|client| client.send(&request)) {
        Ok(response) => {
            println!("{} {}", response.status, response.url);
            for (name, value) in &response.headers {
                println!("{name}: {value}");
            }
            println!("corps : {} octets", response.body.len());
        }
        Err(e) => eprintln!("échec : {e} — {e:?}"),
    }
}
