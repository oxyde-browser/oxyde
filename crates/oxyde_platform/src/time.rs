//! Date et heure civiles d'un instant.

/// Date et heure civiles (calendrier grégorien).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilTime {
    /// Année.
    pub year: i32,
    /// Mois, de 1 à 12.
    pub month: u8,
    /// Jour du mois, de 1 à 31.
    pub day: u8,
    /// Heure, de 0 à 23.
    pub hour: u8,
    /// Minute, de 0 à 59.
    pub minute: u8,
    /// Seconde, de 0 à 59.
    pub second: u8,
}

/// Date et heure UTC de l'instant `epoch_seconds` (secondes depuis le
/// 1er janvier 1970).
#[must_use]
pub fn utc_time(epoch_seconds: u64) -> CivilTime {
    let days = epoch_seconds / 86_400;
    let rest = epoch_seconds % 86_400;
    // Date civile d'un nombre de jours (algorithme de Howard Hinnant),
    // décalée pour que l'ère commence un 1er mars.
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    // Chaque valeur tient dans son type par construction ; un instant
    // démesuré sature l'année.
    let small = |value: u64| u8::try_from(value).unwrap_or(u8::MAX);
    CivilTime {
        year: i32::try_from(year).unwrap_or(i32::MAX),
        month: small(month),
        day: small(day),
        hour: small(rest / 3_600),
        minute: small(rest / 60 % 60),
        second: small(rest % 60),
    }
}

/// Date et heure de l'instant `epoch_seconds` dans le fuseau horaire du
/// système. À défaut de pouvoir l'interroger, rend l'heure UTC.
#[must_use]
pub fn local_time(epoch_seconds: u64) -> CivilTime {
    #[cfg(windows)]
    if let Some(local) = crate::windows::local_time(epoch_seconds) {
        return local;
    }
    utc_time(epoch_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civil(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> CivilTime {
        CivilTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    #[test]
    fn utc_dates() {
        assert_eq!(utc_time(0), civil(1970, 1, 1, 0, 0, 0));
        assert_eq!(utc_time(86_399), civil(1970, 1, 1, 23, 59, 59));
        // Année bissextile, fin de février, changement de siècle.
        assert_eq!(utc_time(951_782_400), civil(2000, 2, 29, 0, 0, 0));
        assert_eq!(utc_time(1_794_508_334), civil(2026, 11, 12, 18, 32, 14));
        assert_eq!(utc_time(4_102_444_799), civil(2099, 12, 31, 23, 59, 59));
        // Un instant démesuré ne fait rien paniquer.
        let far = utc_time(u64::MAX);
        assert!(far.month >= 1 && far.month <= 12);
    }

    #[test]
    fn local_time_is_within_a_day_of_utc() {
        let instant = 1_794_508_334;
        let (local, utc) = (local_time(instant), utc_time(instant));
        assert_eq!(
            (local.minute % 15, local.second),
            (utc.minute % 15, utc.second)
        );
        let days =
            |t: CivilTime| i64::from(t.year) * 372 + i64::from(t.month) * 31 + i64::from(t.day);
        assert!((days(local) - days(utc)).abs() <= 1);
    }
}
