//! Abstraction du système d'exploitation.
//!
//! Tout ce qui dépend du système passera par ce crate, afin que les portages
//! Linux, macOS, Android et iOS/iPadOS (jalon 10) n'aient pas à toucher au
//! reste du navigateur. Pour l'instant il ne porte que l'heure locale et le
//! temps processeur consommé ; la fenêtre est gérée directement par `winit`
//! dans le crate `oxyde`.
//!
//! LIMITES CONNUES : hors de Windows, l'heure locale est l'heure UTC et le
//! temps processeur n'est pas connu.

mod time;
#[cfg(windows)]
mod windows;

pub use time::{CivilTime, local_time, utc_time};

/// Temps processeur (noyau et utilisateur) que le fil d'exécution appelant a
/// consommé depuis son lancement ; `None` si le système ne le dit pas.
/// Contrairement au temps d'horloge, il ne dépend pas de la charge de la
/// machine ; il ne compte pas le travail confié à d'autres fils.
#[must_use]
pub fn thread_cpu_time() -> Option<std::time::Duration> {
    #[cfg(windows)]
    {
        crate::windows::thread_cpu_time()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn cpu_time_grows_with_work() {
        let before = super::thread_cpu_time().expect("temps processeur");
        let mut sum = 0u64;
        while super::thread_cpu_time().expect("temps processeur") == before {
            for i in 0..1_000_000u64 {
                sum = sum.wrapping_add(std::hint::black_box(i));
            }
        }
        assert!(sum > 0);
    }
}
