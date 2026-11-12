//! Processus de rendu : une page par processus, à l'écart du navigateur.
//!
//! - [`protocol`] : les messages échangés ;
//! - [`child`] : le processus de rendu lui-même (`oxyde --type=renderer`) ;
//! - [`host`] : côté navigateur, la page d'un onglet et son processus.
//!
//! Le processus de rendu est lancé dans le bac à sable d'`oxyde_sandbox`
//! (`--no-sandbox` y renonce, pour le diagnostic).
//!
//! Un processus de rendu est voué à un site : une navigation vers un autre
//! site passe par un processus neuf, et le navigateur ne sert à un
//! processus que les cookies et les ressources de son site.
//!
//! LIMITES CONNUES : les cadres vivent dans le processus de leur page, quel
//! que soit leur site ; deux onglets du même site ont chacun leur
//! processus ; le site est approché sans liste des suffixes publics ; la
//! lecture de l'arbre d'accessibilité d'une page distante n'est pas faite.

pub(crate) mod child;
pub(crate) mod host;
pub(crate) mod protocol;
