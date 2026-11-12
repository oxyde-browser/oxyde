//! Abstraction du système d'exploitation.
//!
//! Tout ce qui dépend du système passera par ce crate, afin que les portages
//! Linux, macOS, Android et iOS/iPadOS (jalon 10) n'aient pas à toucher au
//! reste du navigateur. Pour l'instant il ne porte que l'heure locale ; la
//! fenêtre est gérée directement par `winit` dans le crate `oxyde`.
//!
//! LIMITES CONNUES : hors de Windows, l'heure locale est l'heure UTC.

mod time;
#[cfg(windows)]
mod windows;

pub use time::{CivilTime, local_time, utc_time};
