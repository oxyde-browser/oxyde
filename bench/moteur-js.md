# Vitesse du moteur JavaScript

Quatorze boucles élémentaires, les mêmes pour chaque moteur
([moteur-js/essais.js](moteur-js/essais.js)). Elles ne disent pas la vitesse d'un site, mais
où le moteur perd son temps.

```
oxyde --dump-dom bench/moteur-js/oxyde.html      # Oxyde (lire la sortie d'erreur)
node bench/moteur-js/node.js                     # V8 complet
node --jitless bench/moteur-js/node.js           # interpréteur de V8 seul
```

Mesures du 14 novembre 2026, en millisecondes, sur la même machine (Windows 11, compilation
`--release`). Pour Oxyde, le plus petit de trois passages ; pour Node, un passage. Chaque
essai d'Oxyde tourne dans sa propre tâche de script.

| Essai | Oxyde, avant | Oxyde | V8 sans JIT | V8 |
|---|---|---|---|---|
| boucle arithmétique (5 M) | 1000 | 291 | 57 | 16 |
| appels de fonction (2 M) | 692 | 384 | 40 | 2 |
| lecture de propriété (3 M) | 560 | 168 | 32 | 1 |
| appel de méthode (2 M) | 815 | 484 | 51 | 1 |
| allocation d'objets (500 k) | 351 | 295 | 29 | 3 |
| fermetures (1 M) | 424 | 311 | 68 | 11 |
| tableau : `push` puis somme (1 M) | 678 | 505 | 48 | 17 |
| `map` / `filter` / `reduce` (300 k) | 212 | 184 | 21 | 15 |
| concaténation de chaînes (200 k) | 3056 | 2733 | 4 | 2 |
| `Map` : `set` puis `get` (300 k) | 480 | 430 | 144 | 130 |
| variables globales (2 M) | 534 | 266 | 22 | 2 |
| `try` / `catch` sans erreur (2 M) | 310 | 130 | 26 | 4 |
| expression rationnelle (100 k) | 100 | 86 | 12 | 2 |
| JSON aller-retour (20 k) | 96 | 83 | 8 | 7 |
| **Total** | **9308** | **6350** | **562** | **213** |

« Oxyde, avant » : état de la branche principale avant le travail du 14 novembre (un seul
passage).

## Lecture

- Hors concaténation de chaînes, Oxyde est passé de 6,3 s à 3,6 s ; il reste **6 à 7 fois
  plus lent que l'interpréteur de V8** et 17 fois plus lent que V8 complet.
- **Concaténation de chaînes** : `s += "ab"` recopie toute la chaîne à chaque tour (coût
  quadratique). C'est le premier défaut à corriger ; il demande de changer la représentation
  des chaînes (cordes, ou tampon partagé).
- **Appels** : chaque appel alloue la liste de ses arguments et un cadre complet.
- **Objets** : chaque objet porte sa propre table de hachage ; il n'y a ni formes partagées
  ni cache d'accès aux propriétés.
- **Compilateur** : le code produit est naïf (une boucle `for` vide exécute huit
  instructions par tour).
- Pas de compilation à la volée (jalon 8).

Deux défauts trouvés le même jour ne se voient pas dans ce tableau, parce qu'ils rendaient
des programmes entiers inutilisables plutôt que lents ; ils sont corrigés :

- lire une chaîne caractère par caractère (`length`, `charCodeAt`, `s[i]`, `slice`,
  `indexOf`) coûtait le carré de sa longueur : 200 000 caractères ne finissaient pas en deux
  minutes, 63 ms à présent ;
- chaque retour de fonction parcourait toutes les variables capturées encore ouvertes.
