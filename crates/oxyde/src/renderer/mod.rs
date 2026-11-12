//! Processus de rendu : une page par processus, à l'écart du navigateur.
//!
//! - [`protocol`] : les messages échangés ;
//! - [`child`] : le processus de rendu lui-même (`oxyde --type=renderer`) ;
//! - [`host`] : côté navigateur, la page d'un onglet et son processus.
//!
//! Le processus de rendu est lancé dans le bac à sable d'`oxyde_sandbox`
//! (`--no-sandbox` y renonce, pour le diagnostic).
//!
//! LIMITES CONNUES : un processus par onglet, pas encore par site (une
//! navigation vers un autre site garde le même processus) ; les cadres
//! vivent dans le processus de leur page ; la lecture de l'arbre
//! d'accessibilité d'une page distante n'est pas faite.

pub(crate) mod child;
pub(crate) mod host;
pub(crate) mod protocol;
