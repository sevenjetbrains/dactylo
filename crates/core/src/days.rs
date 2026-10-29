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

/// Premier jour (lundi) de la semaine locale qui contient le jour `day`.
pub fn week_start(day: i64) -> i64 {
    // Le 1ᵉʳ janvier 1970 (jour 0) était un jeudi : +3 ramène lundi à 0.
    day - (day + 3).rem_euclid(7)
}

/// Date civile (année, mois de 1 à 12, jour de 1 à 31) du jour numéro `day`.
/// Algorithme « civil_from_days » de Howard Hinnant, calendrier grégorien proleptique.
pub fn civil_date(day: i64) -> (i64, u32, u32) {
    // Décalage au 1ᵉʳ mars de l'an 0 : le jour bissextile tombe en fin d'année.
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    // Janvier et février appartiennent à l'année civile suivante.
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Date et heure locales au format « AAAA-MM-JJ HH:MM » (export CSV).
pub fn format_local(ms: i64, tz_offset_min: i32) -> String {
    let day = local_day(ms, tz_offset_min);
    let (y, m, d) = civil_date(day);
    // Minutes écoulées depuis minuit local.
    let minutes = (ms - day_start(day, tz_offset_min)) / MINUTE_MS;
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        minutes / 60,
        minutes % 60
    )
}

/// Tests des jours locaux.
#[cfg(test)]
mod tests {
    use super::*;

    /// Les semaines commencent le lundi : le 29 octobre 2026 est un jeudi.
    #[test]
    fn weeks_start_on_monday() {
        let thursday = local_day(OCT_29_UTC, 0);
        assert_eq!(thursday - week_start(thursday), 3);
        // Un lundi est son propre début de semaine.
        let monday = week_start(thursday);
        assert_eq!(week_start(monday), monday);
        assert_eq!(week_start(monday + 6), monday);
        assert_eq!(week_start(monday - 1), monday - 7);
    }

    /// Conversion en date civile, y compris années bissextiles et avant 1970.
    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(local_day(OCT_29_UTC, 0)), (2026, 10, 29));
        // 29 février 2024 = jour 19 782.
        assert_eq!(civil_date(19_782), (2024, 2, 29));
        assert_eq!(civil_date(-1), (1969, 12, 31));
    }

    /// Formatage local, avec le décalage horaire.
    #[test]
    fn formats_local_date_time() {
        let t = OCT_29_UTC + 17 * 3_600_000 + 42 * MINUTE_MS;
        assert_eq!(format_local(t, 0), "2026-10-29 17:42");
        assert_eq!(format_local(t, 60), "2026-10-29 18:42");
        assert_eq!(format_local(OCT_29_UTC, -60), "2026-10-28 23:00");
    }

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
