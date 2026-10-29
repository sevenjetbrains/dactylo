//! Jours locaux à partir d'horodatages Unix en millisecondes (ADR 0003).
//!
//! Le cœur ne connaît pas le fuseau horaire : l'interface fournit son décalage
//! par rapport à UTC, en minutes (positif à l'est : +60 pour Paris en hiver).

/// Une journée, en millisecondes.
pub const DAY_MS: i64 = 86_400_000;

/// Une minute, en millisecondes.
const MINUTE_MS: i64 = 60_000;

/// Numéro du jour local qui contient l'instant `ms` (0 = 1ᵉʳ janvier 1970).
pub fn local_day(ms: i64, tz_offset_min: i32) -> i64 {
    (ms + i64::from(tz_offset_min) * MINUTE_MS).div_euclid(DAY_MS)
}

/// Instant du début (minuit local) du jour local numéro `day`.
pub fn day_start(day: i64, tz_offset_min: i32) -> i64 {
    day * DAY_MS - i64::from(tz_offset_min) * MINUTE_MS
}

/// Instant où commence le lendemain local de `ms` : borne exclue de « aujourd'hui ».
pub fn end_of_day(ms: i64, tz_offset_min: i32) -> i64 {
    day_start(local_day(ms, tz_offset_min) + 1, tz_offset_min)
}

/// Tests des jours locaux.
#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-29 00:00 UTC, en ms.
    const OCT_29_UTC: i64 = 1_793_232_000_000;

    /// À Paris (+60), 23 h 30 UTC appartient déjà au lendemain local.
    #[test]
    fn offset_moves_late_evening_to_next_day() {
        let late = OCT_29_UTC - 30 * MINUTE_MS;
        assert_eq!(local_day(late, 0) + 1, local_day(late, 60));
        assert_eq!(local_day(OCT_29_UTC, 0), local_day(late, 60));
    }

    /// Un décalage négatif (Amérique) recule le jour.
    #[test]
    fn negative_offset_moves_back() {
        assert_eq!(local_day(OCT_29_UTC, -300), local_day(OCT_29_UTC, 0) - 1);
    }

    /// Début et fin de journée encadrent l'instant.
    #[test]
    fn day_bounds_surround_instant() {
        let now = OCT_29_UTC + 15 * 3_600_000;
        let start = day_start(local_day(now, 60), 60);
        let end = end_of_day(now, 60);
        assert!(start <= now && now < end);
        assert_eq!(end - start, DAY_MS);
        // Minuit local à Paris = 23 h UTC la veille.
        assert_eq!(start, OCT_29_UTC - 60 * MINUTE_MS);
    }

    /// Avant 1970, le jour reste juste (division euclidienne).
    #[test]
    fn handles_instants_before_epoch() {
        assert_eq!(local_day(-1, 0), -1);
    }
}
