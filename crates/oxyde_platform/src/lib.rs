//! Abstraction du système d'exploitation (fenêtres, presse-papiers, magasin de secrets).
//!
//! Tout ce qui dépend du système passera par les traits de ce crate, afin que les portages Linux, macOS, Android et iOS/iPadOS (jalon 10) n'aient pas à toucher au reste du navigateur. Pour l'instant, la fenêtre est gérée directement par `winit` dans le crate `oxyde`.
//!
//! Ce crate est un emplacement réservé : il ne contient encore aucun code.
