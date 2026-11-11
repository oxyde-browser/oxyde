# Bancs d'essai d'Oxyde

Rapport généré par `oxyde-bench` le 2026-11-11 (Oxyde 0.1.0, 16 cœurs logiques, windows).
Pour le reproduire :

```bash
cargo run -p oxyde_bench --release -- tout --publier
```

Les mesures brutes sont dans [`bench/resultats/`](../bench/resultats/). Aucun score n'est estimé : un banc qui ne peut pas encore tourner est marqué comme tel.

## Performances de la chaîne de rendu

Temps d'une exécution complète, après échauffement ; médiane et minimum. Plus bas est meilleur.

| Banc | Médiane | Minimum | Itérations |
|---|---:|---:|---:|
| Analyse HTML — 10 000 paragraphes (≈1,3 Mo) | 36.63 ms | 35.58 ms | 20 |
| Analyse CSS — 2 000 règles | 9.20 ms | 8.90 ms | 30 |
| Rendu complet — 1 000 paragraphes, CSS par défaut | 27.98 ms | 27.00 ms | 20 |
| Rendu complet — 1 000 paragraphes × 500 règles CSS | 80.26 ms | 78.20 ms | 10 |
| Rendu complet — 200 arbres de 30 div imbriqués | 20.94 ms | 20.26 ms | 20 |
| Rendu complet — tests/pages/demo.html (sans images) | 0.56 ms | 0.55 ms | 50 |

## Sécurité — TLS et certificats

Comportement de `oxyde_net` face aux serveurs de test de badssl.com.

**18 / 19 conformes**

| | Contrôle | Attendu | Obtenu |
|---|---|---|---|
| ✅ | sha256.badssl.com — certificat valide | accepté | accepté (HTTP 200) |
| ✅ | ecc256.badssl.com — clé ECDSA P-256 | accepté | accepté (HTTP 200) |
| ✅ | rsa2048.badssl.com — clé RSA 2048 | accepté | accepté (HTTP 200) |
| ✅ | tls-v1-2.badssl.com:1012 — TLS 1.2 | accepté | accepté (HTTP 200) |
| ✅ | expired.badssl.com — certificat expiré | refusé | refusé (NET::ERR_CERT_DATE_INVALID) |
| ✅ | wrong.host.badssl.com — nom d'hôte incorrect | refusé | refusé (NET::ERR_CERT_COMMON_NAME_INVALID) |
| ✅ | self-signed.badssl.com — certificat auto-signé | refusé | refusé (NET::ERR_CERT_AUTHORITY_INVALID) |
| ✅ | untrusted-root.badssl.com — racine non reconnue | refusé | refusé (NET::ERR_CERT_AUTHORITY_INVALID) |
| ❌ | revoked.badssl.com — certificat révoqué | refusé | accepté (HTTP 200) |
| ✅ | no-common-name.badssl.com — certificat sans nom | refusé | refusé (NET::ERR_CERT_DATE_INVALID) |
| ✅ | incomplete-chain.badssl.com — chaîne incomplète (plus strict que Chromium, qui la complète par AIA) | refusé | refusé (NET::ERR_CERT_AUTHORITY_INVALID) |
| ✅ | tls-v1-0.badssl.com:1010 — TLS 1.0 obsolète | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | tls-v1-1.badssl.com:1011 — TLS 1.1 obsolète | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | rc4.badssl.com — chiffrement RC4 | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | 3des.badssl.com — chiffrement 3DES | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | null.badssl.com — chiffrement nul | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | dh480.badssl.com — Diffie-Hellman 480 bits | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | dh1024.badssl.com — Diffie-Hellman 1024 bits | refusé | refusé (ERR_SSL_PROTOCOL_ERROR) |
| ✅ | http.badssl.com — accès https à un site servi en http | refusé ou erreur | refusé (ERR_INVALID_URL) |

## Vie privée — ce que le navigateur envoie

En-têtes d'une requête réelle, capturés par un serveur local.

**7 / 8 conformes**

| | Contrôle | Attendu | Obtenu |
|---|---|---|---|
| ✅ | En-tête `cookie` — aucun cookie stocké ni envoyé | absent | absent |
| ✅ | En-tête `referer` — pas de fuite de la page précédente | absent | absent |
| ✅ | En-tête `x-client-data` — en-tête d'expérimentation propre à Chrome | absent | absent |
| ✅ | En-tête `x-requested-with` — identifiant d'application | absent | absent |
| ✅ | En-tête `sec-ch-ua-full-version-list` — empreinte détaillée par Client Hints | absent | absent |
| ✅ | User-Agent sans version détaillée du système ni identifiant unique | chaîne courte et commune | `Mozilla/5.0 (Windows NT 10.0; Win64; x64) Oxyde/0.1` |
| ❌ | Global Privacy Control (`Sec-GPC: 1`) | envoyé | absent |
| ✅ | Nombre total d'en-têtes envoyés (surface d'empreinte) | ≤ 6 | 4 (host, user-agent, accept, accept-encoding) |

## Compatibilité et bancs publics

| Banc | Mesure | État |
|---|---|---|
| [Speedometer 3.1](https://browserbench.org/Speedometer3.1/) | réactivité des applications web | bloqué — JavaScript + DOM scriptable (jalon 4) |
| [JetStream 2.2](https://browserbench.org/JetStream2.2/) | JavaScript et WebAssembly | bloqué — JavaScript (jalon 4), WebAssembly (jalon 8) |
| [MotionMark 1.3](https://browserbench.org/MotionMark1.3/) | rendu graphique animé | bloqué — JavaScript (jalon 4), canvas et compositeur (jalon 6) |
| [APEXMark](https://apexmark.tech/) | banc navigateur | bloqué — JavaScript (jalon 4) |
| [test262](https://github.com/tc39/test262) | conformité ECMAScript | bloqué — moteur `oxyde_js` (jalon 4, en cours) |
| [Web Platform Tests](https://wpt.fyi/) | conformité des standards du web | bloqué — lanceur de tests de référence + testharness.js (jalon 4) |
| [html5lib-tests](https://github.com/html5lib/html5lib-tests) | analyse HTML | bloqué — à brancher : `--dump-dom` produit déjà le format |
| [Acid3](http://acid3.acidtests.org/) | conformité historique DOM/CSS | bloqué — JavaScript (jalon 4) |
| [HTML5test](https://html5test.co/) | fonctions HTML5 détectées | bloqué — JavaScript (jalon 4) |
| [badssl.com](https://badssl.com/) | TLS et certificats | **intégré** (section Sécurité) |
| [Qualys SSL Labs — client](https://clienttest.ssllabs.com:8443/ssltest/viewMyClient.html) | protocoles et suites TLS | bloqué — JavaScript (jalon 4) |
| [PrivacyTests.org](https://privacytests.org/) | protections de la vie privée | bloqué — JavaScript, cookies et stockage (jalons 4, 7, 9) |
| [Cover Your Tracks (EFF)](https://coveryourtracks.eff.org/) | pistage et empreinte | bloqué — JavaScript (jalon 4) |
| [BrowserLeaks](https://browserleaks.com/) | fuites d'empreinte | bloqué — JavaScript (jalon 4) |

Les scores de ces bancs seront ajoutés ici dès qu'Oxyde pourra les exécuter, avec la version de Chromium mesurée sur la même machine comme référence.
