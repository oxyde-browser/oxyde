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

| | Départ | 14 novembre 2026 | 15 novembre 2026 |
|---|---|---|---|
| lisible | 14 | 20 | 19 |
| partiel | 8 | 12 | 13 |
| vide | 13 | 9 | 9 |
| erreur | 3 | 1 | 2 |
| mur | 7 | 8 | 7 |
| délai | 5 | 0 | 0 |

« Départ » : état avant tout travail sur l'étape H (branche principale au 14 novembre 2026).

Le critère de sortie de l'étape H (40 pages lisibles sur 50) n'est pas atteint : 19.
Entre les deux mesures, eBay s'est ouvert ; Vimeo et X ne s'affichent plus. Le temps de
chargement a baissé : 45 secondes au plus (eBay), 33 pour Le Figaro, moins de 26 pour
tous les autres.

La colonne du 15 novembre tient compte des corrections faites le jour même après la
mesure (Mastodon, MDN, GitHub, BBC rechargés) : Mastodon passe d'« erreur » à « partiel ».

| N° | Site | Famille | Départ | 14 novembre | 15 novembre | Remarque |
|---|---|---|---|---|---|---|
| 1 | google.com | recherche | vide | lisible | lisible | HTTP/2 était refusé par les serveurs de Google |
| 2 | bing.com | recherche | partiel | partiel | partiel | champ de recherche sur fond gris, sans logo |
| 3 | duckduckgo.com | recherche | vide | partiel | partiel | au premier essai le serveur a fermé la connexion (curl aussi) ; jugé sur un second chargement |
| 4 | qwant.com | recherche | erreur | partiel | partiel | titre et champ sans mise en forme, en 8 s ; `attachEvent` absent |
| 5 | fr.wikipedia.org | encyclopédie | lisible | lisible | lisible | en-tête mal aligné |
| 6 | en.wikipedia.org | encyclopédie | lisible | lisible | lisible | en-tête mal aligné |
| 7 | fr.wiktionary.org | encyclopédie | lisible | lisible | lisible |  |
| 8 | lemonde.fr | presse | partiel | lisible | lisible | la fenêtre de consentement s'affiche |
| 9 | lefigaro.fr | presse | délai | lisible | lisible | en 33 s, sous le voile de la fenêtre de consentement ; le site dit ne pas reconnaître le navigateur |
| 10 | francetvinfo.fr | presse | partiel | partiel | partiel | fenêtre de consentement lisible ; la page derrière est noire |
| 11 | bbc.com | presse | partiel | partiel | partiel | menu et bandeau de cookies ; une erreur de syntaxe dans un script ajouté par la page, non localisée |
| 12 | theguardian.com | presse | délai | partiel | partiel | bandeau et titre ; le contenu est masqué par un bloc bleu |
| 13 | nytimes.com | presse | mur | mur | mur | 403 (DataDome) |
| 14 | mail.google.com | messagerie | vide | vide | vide |  |
| 15 | outlook.live.com | messagerie | erreur | vide | vide | le script d'un worker ne se charge pas (`importScripts`) |
| 16 | proton.me | messagerie | erreur | lisible | lisible | texte lisible sur fond uni, sans les illustrations |
| 17 | youtube.com | vidéo | vide | partiel | partiel | squelette de la page, sans vidéos, en 18 s |
| 18 | twitch.tv | vidéo | vide | vide | erreur | l'application démarre en 25 s puis affiche sa page d'erreur |
| 19 | dailymotion.com | vidéo | vide | vide | vide |  |
| 20 | vimeo.com | vidéo | lisible | lisible | erreur | **régression** : l'épreuve Cloudflare dépasse le temps d'exécution accordé, puis l'application affiche son erreur ; déjà le cas le 14 novembre en fin de journée |
| 21 | amazon.fr | commerce | mur | mur | mur | épreuve anti-robot |
| 22 | leboncoin.fr | commerce | mur | mur | mur | 403 (DataDome) |
| 23 | cdiscount.com | commerce | mur | mur | mur | épreuve anti-robot |
| 24 | fnac.com | commerce | mur | mur | mur | 403 |
| 25 | ebay.fr | commerce | mur | mur | **lisible** | en 45 s ; la page est maintenant servie |
| 26 | credit-agricole.fr | banque | partiel | partiel | partiel | contenu lisible, recouvert par l'annonce d'une configuration non prise en charge |
| 27 | boursobank.com | banque | lisible | lisible | lisible |  |
| 28 | paypal.com | banque | vide | mur | mur | 403 |
| 29 | service-public.fr | administration | lisible | lisible | lisible | bandeau de cookies mal placé |
| 30 | impots.gouv.fr | administration | partiel | partiel | partiel | fond noir, texte peu lisible |
| 31 | ameli.fr | administration | lisible | lisible | lisible | bandeau de cookies mal placé |
| 32 | doctolib.fr | administration | vide | vide | vide | l'application ne s'affiche pas |
| 33 | developer.mozilla.org | documentation | lisible | lisible | lisible | en 2 s ; ses composants se chargent, mais la feuille de style principale ne s'applique pas |
| 34 | docs.rs | documentation | lisible | lisible | lisible |  |
| 35 | doc.rust-lang.org | documentation | lisible | lisible | lisible |  |
| 36 | stackoverflow.com | documentation | mur | mur | mur | épreuve Cloudflare |
| 37 | github.com | documentation | délai | lisible | lisible | en 20 s, sans erreur de script ; mise en forme encore grossière |
| 38 | crates.io | documentation | vide | vide | vide |  |
| 39 | x.com | réseaux sociaux | lisible | lisible | vide | **régression** : le site sert une nouvelle application (un module unique) qui ne produit aucun contenu ; cause non trouvée |
| 40 | facebook.com | réseaux sociaux | vide | vide | vide |  |
| 41 | instagram.com | réseaux sociaux | délai | lisible | lisible | page de connexion en 11 s ; la fenêtre de cookies chevauche le formulaire |
| 42 | linkedin.com | réseaux sociaux | lisible | lisible | lisible | avec son bandeau de cookies |
| 43 | reddit.com | réseaux sociaux | vide | vide | vide |  |
| 44 | mastodon.social | réseaux sociaux | vide | erreur | **partiel** | l'application s'affiche (menu, recherche) ; la colonne des messages reste vide ; `indexedDB` absent |
| 45 | openstreetmap.org | outils | délai | partiel | partiel | en-tête ; la carte reste vide |
| 46 | deepl.com | outils | partiel | partiel | partiel | texte sans mise en forme |
| 47 | sncf-connect.com | outils | vide | vide | vide |  |
| 48 | meteofrance.com | outils | partiel | partiel | partiel | fenêtre de consentement lisible ; la carte manque |
| 49 | allocine.fr | outils | lisible | lisible | lisible | fenêtre de consentement |
| 50 | news.ycombinator.com | outils | lisible | lisible | lisible |  |

Au départ, trois sites affichaient le contenu de leur `<noscript>` (« activez JavaScript ») :
ils sont comptés « mur » (Amazon, Cdiscount, Stack Overflow) ou « vide » (Mastodon).

## Ce qui bloque, par ordre de fréquence

1. **Applications qui ne démarrent pas ou n'affichent rien** (Gmail, Outlook, Dailymotion,
   Doctolib, crates.io, X, Facebook, Reddit, SNCF Connect ; YouTube s'arrête au squelette).
   La plupart ne signalent aucune erreur. Celles qui en signalent : Outlook (`importScripts`
   d'un worker), YouTube (une exception de son injection de dépendances).
2. **Murs anti-robot** (NYTimes, Leboncoin, Fnac, PayPal : réponse 403 ; Amazon, Cdiscount,
   Stack Overflow : épreuve en JavaScript ; Vimeo : l'épreuve dépasse le temps accordé).
   Oxyde s'annonce sous son nom et ne cherche pas à se faire passer pour un autre navigateur ;
   ces sites s'ouvriront quand leurs épreuves s'exécuteront correctement.
3. **Pages d'erreur des applications** : Twitch et Vimeo (lecture de `length` sur
   `undefined`).
4. **Manques relevés dans les journaux** : `indexedDB` (Mastodon), `attachEvent` (Qwant),
   une erreur de syntaxe dans un script ajouté par la page (BBC), la feuille de style
   principale de MDN qui ne s'applique pas.
5. **Mise en page** : en-têtes mal alignés, bandeaux mal placés, fonds manquants, blocs qui
   masquent le contenu (The Guardian, France Info, BBC).
