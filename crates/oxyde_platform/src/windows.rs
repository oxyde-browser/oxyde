//! Appels à l'API Win32.
#![allow(unsafe_code, reason = "appels à l'API Win32, sans équivalent sûr")]

use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};
use windows_sys::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

use crate::time::CivilTime;

/// Secondes entre le 1er janvier 1601 (origine de `FILETIME`) et l'époque Unix.
const EPOCH_GAP: u64 = 11_644_473_600;

const EMPTY: SYSTEMTIME = SYSTEMTIME {
    wYear: 0,
    wMonth: 0,
    wDayOfWeek: 0,
    wDay: 0,
    wHour: 0,
    wMinute: 0,
    wSecond: 0,
    wMilliseconds: 0,
};

/// Temps processeur, noyau et utilisateur, consommé par le fil d'exécution
/// appelant.
pub(crate) fn thread_cpu_time() -> Option<std::time::Duration> {
    const ZERO: FILETIME = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let (mut created, mut exited, mut kernel, mut user) = (ZERO, ZERO, ZERO, ZERO);
    // SAFETY: `GetCurrentThread` ne prend aucun argument et rend un
    // pseudo-descripteur toujours valide, qu'il ne faut pas fermer.
    let thread = unsafe { GetCurrentThread() };
    // SAFETY: `thread` désigne le fil appelant ; les quatre pointeurs désignent
    // des variables locales distinctes, valides, alignées et initialisées,
    // qui vivent pendant l'appel et dans lesquelles la fonction écrit.
    let read = unsafe {
        GetThreadTimes(
            thread,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        )
    };
    if read == 0 {
        return None;
    }
    // Durées en pas de 100 ns, en deux moitiés de 32 bits.
    let ticks =
        |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    let total = ticks(kernel).checked_add(ticks(user))?;
    Some(std::time::Duration::from_nanos(total.checked_mul(100)?))
}

/// Date et heure de `epoch_seconds` dans le fuseau du système, avec les
/// règles d'heure d'été en vigueur ; `None` si l'instant sort de ce que
/// Windows sait convertir.
pub(crate) fn local_time(epoch_seconds: u64) -> Option<CivilTime> {
    // `FILETIME` compte des pas de 100 ns en deux moitiés de 32 bits.
    let ticks = epoch_seconds
        .checked_add(EPOCH_GAP)?
        .checked_mul(10_000_000)?;
    let file_time = FILETIME {
        dwLowDateTime: u32::try_from(ticks & 0xFFFF_FFFF).ok()?,
        dwHighDateTime: u32::try_from(ticks >> 32).ok()?,
    };
    let mut utc = EMPTY;
    // SAFETY: les deux pointeurs désignent des variables locales valides,
    // alignées et initialisées, qui vivent pendant l'appel ; la fonction lit
    // `file_time` et n'écrit que dans `utc`.
    let converted = unsafe { FileTimeToSystemTime(&raw const file_time, &raw mut utc) };
    if converted == 0 {
        return None;
    }
    let mut local = EMPTY;
    // SAFETY: `utc` et `local` sont des variables locales valides, alignées
    // et initialisées, distinctes l'une de l'autre ; un fuseau nul désigne,
    // d'après la documentation, le fuseau courant du système ; la fonction
    // lit `utc` et n'écrit que dans `local`.
    let shifted = unsafe {
        SystemTimeToTzSpecificLocalTime(std::ptr::null(), &raw const utc, &raw mut local)
    };
    if shifted == 0 {
        return None;
    }
    Some(CivilTime {
        year: i32::from(local.wYear),
        month: u8::try_from(local.wMonth).ok()?,
        day: u8::try_from(local.wDay).ok()?,
        hour: u8::try_from(local.wHour).ok()?,
        minute: u8::try_from(local.wMinute).ok()?,
        second: u8::try_from(local.wSecond).ok()?,
    })
}
