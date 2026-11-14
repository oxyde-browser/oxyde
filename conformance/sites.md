# Sites de référence

Cinquante sites courants, chargés par Oxyde à chaque palier de l'étape H de la
[feuille de route](../docs/FEUILLE-DE-ROUTE.md). C'est la mesure de ce qui compte pour
l'utilisateur : la page s'affiche-t-elle ?

## Méthode

Chaque adresse est chargée hors écran, sans profil, dans une fenêtre de 1280 × 800 :

```
oxyde --screenshot capture.png --page-only --size 1280x800 --no-profile ADRESSE
```

La page a 60 secondes pour se charger ; ses scripts, ses requêtes et ses minuteurs tournent
pendant deux secondes de temps virtuel au plus. La capture est ensuite jugée à l'œil :

| État | Sens |
|---|---|
| **lisible** | le contenu principal est là et se lit, même avec des défauts de mise en page |
| **partiel** | une partie de la page s'affiche (en-tête, squelette), le contenu principal manque ou est cassé |
| **vide** | page blanche ou presque |
| **erreur** | la page affiche elle-même un message d'erreur |
| **mur** | le site refuse le navigateur avant d'envoyer la page (réponse 403 ou épreuve anti-robot) |
| **délai** | le chargement dépasse 60 secondes |

LIMITES DE LA MESURE : elle ne dit que si la page d'accueil s'affiche. La connexion à un
compte et l'action principale de chaque site (chercher, lire un message, payer) ne sont pas
encore essayées : elles demandent la fenêtre réelle et la saisie au clavier. Le jugement est
fait à la main sur une capture du haut de la page. Les temps dépendent de la charge de la
machine et du réseau au moment de la mesure ; un site proche de 60 secondes peut passer d'un
état à « délai » d'une mesure à l'autre.

## Résultat

| | Départ | 14 novembre 2026 |
|---|---|---|
| lisible | 14 | 17 |
| partiel | 8 | 9 |
| vide | 13 | 10 |
| erreur | 3 | 1 |
| mur | 7 | 8 |
| délai | 5 | 5 |

« Départ » : état avant tout travail sur l'étape H (branche principale au 14 novembre 2026).

| N° | Site | Famille | Départ | Aujourd'hui | Remarque |
|---|---|---|---|---|---|
| 1 | google.com | recherche | vide | **lisible** | HTTP/2 était refusé par les serveurs de Google |
| 2 | bing.com | recherche | partiel | partiel | champ de recherche sans logo ni fond |
| 3 | duckduckgo.com | recherche | vide | erreur | l'application signale une exception côté client |
| 4 | qwant.com | recherche | erreur | délai | affichait « Maintenance » ; se charge en 50 s environ quand la machine est libre |
| 5 | fr.wikipedia.org | encyclopédie | lisible | lisible | en-tête mal aligné |
| 6 | en.wikipedia.org | encyclopédie | lisible | lisible | en-tête mal aligné |
| 7 | fr.wiktionary.org | encyclopédie | lisible | lisible | |
| 8 | lemonde.fr | presse | partiel | **lisible** | la fenêtre de consentement s'affiche |
| 9 | lefigaro.fr | presse | délai | **lisible** | sous le voile de la fenêtre de consentement |
| 10 | francetvinfo.fr | presse | partiel | partiel | en-tête seul |
| 11 | bbc.com | presse | partiel | partiel | menu seul |
| 12 | theguardian.com | presse | délai | délai | |
| 13 | nytimes.com | presse | mur | mur | 403 (DataDome) |
| 14 | mail.google.com | messagerie | vide | vide | |
| 15 | outlook.live.com | messagerie | erreur | vide | l'application ne démarre pas |
| 16 | proton.me | messagerie | erreur | **lisible** | réponse compressée, affichée en caractères illisibles |
| 17 | youtube.com | vidéo | vide | partiel | squelette de la page, sans vidéos |
| 18 | twitch.tv | vidéo | vide | vide | |
| 19 | dailymotion.com | vidéo | vide | vide | |
| 20 | vimeo.com | vidéo | lisible | délai | RÉGRESSION : l'épreuve Cloudflare s'exécute désormais et ne se termine pas |
| 21 | amazon.fr | commerce | mur | mur | épreuve anti-robot |
| 22 | leboncoin.fr | commerce | mur | mur | 403 (DataDome) |
| 23 | cdiscount.com | commerce | mur | mur | épreuve anti-robot |
| 24 | fnac.com | commerce | mur | mur | 403 |
| 25 | ebay.fr | commerce | mur | mur | 403 (Akamai) |
| 26 | credit-agricole.fr | banque | partiel | partiel | le site annonce une configuration non prise en charge |
| 27 | boursobank.com | banque | lisible | lisible | |
| 28 | paypal.com | banque | vide | mur | 403 |
| 29 | service-public.fr | administration | lisible | lisible | bandeau de cookies mal placé |
| 30 | impots.gouv.fr | administration | partiel | partiel | fond noir, texte peu lisible |
| 31 | ameli.fr | administration | lisible | lisible | bandeau de cookies mal placé |
| 32 | doctolib.fr | administration | vide | vide | se charge en 6 s (l'analyse d'un script ne finissait pas) ; l'application ne s'affiche pas |
| 33 | developer.mozilla.org | documentation | lisible | lisible | |
| 34 | docs.rs | documentation | lisible | lisible | |
| 35 | doc.rust-lang.org | documentation | lisible | lisible | |
| 36 | stackoverflow.com | documentation | mur | mur | épreuve Cloudflare |
| 37 | github.com | documentation | délai | délai | les scripts finissent ; la mise en page ne se termine pas |
| 38 | crates.io | documentation | vide | vide | |
| 39 | x.com | réseaux sociaux | lisible | lisible | page de connexion |
| 40 | facebook.com | réseaux sociaux | vide | vide | |
| 41 | instagram.com | réseaux sociaux | délai | délai | |
| 42 | linkedin.com | réseaux sociaux | lisible | lisible | |
| 43 | reddit.com | réseaux sociaux | vide | vide | |
| 44 | mastodon.social | réseaux sociaux | vide | vide | |
| 45 | openstreetmap.org | outils | délai | partiel | en-tête ; la carte reste vide |
| 46 | deepl.com | outils | partiel | partiel | texte sans mise en forme |
| 47 | sncf-connect.com | outils | vide | vide | |
| 48 | meteofrance.com | outils | partiel | partiel | |
| 49 | allocine.fr | outils | lisible | lisible | |
| 50 | news.ycombinator.com | outils | lisible | lisible | |

Au départ, trois sites affichaient le contenu de leur `<noscript>` (« activez JavaScript ») :
ils sont comptés « mur » (Amazon, Cdiscount, Stack Overflow) ou « vide » (Mastodon).

## Ce qui bloque, par ordre de fréquence

1. **Applications qui ne démarrent pas** (Gmail, Outlook, Twitch, Dailymotion, Doctolib,
   crates.io, Facebook, Reddit, Mastodon, SNCF Connect) : il manque `Intl`, `customElements`
   et le Shadow DOM, les cartes d'import, les modules insérés par script, `DOMParser`, les
   workers. Les erreurs de script n'indiquent ni fichier ni ligne, ce qui ralentit le
   diagnostic.
2. **Murs anti-robot** (NYTimes, Leboncoin, Fnac, eBay, PayPal : réponse 403 ; Amazon,
   Cdiscount, Stack Overflow, Vimeo : épreuve en JavaScript). Oxyde s'annonce sous son nom et
   ne cherche pas à se faire passer pour un autre navigateur ; ces sites s'ouvriront quand
   leurs épreuves s'exécuteront correctement.
3. **Lenteur** : GitHub (mise en page), The Guardian, Instagram, Qwant.
4. **Mise en page** : en-têtes mal alignés, bandeaux mal placés, fonds manquants.
