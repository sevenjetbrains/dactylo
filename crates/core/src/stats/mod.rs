//! Statistiques de session : MPM, CPM, précision (formules : ADR 0009).

use serde::{Deserialize, Serialize};

use crate::typing::Counters;

/// Un « mot » vaut 5 caractères frappés, espaces compris (glossaire).
pub const CHARS_PER_WORD: f64 = 5.0;

/// Statistiques d'une session terminée (F-30).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    /// Temps actif, pauses exclues.
    pub duration_ms: u32,
    /// Caractères frappés (insertions, justes ou non).
    pub typed: u32,
    /// Frappes fausses, corrigées ou non.
    pub errors: u32,
    /// Caractères encore faux à la fin.
    pub uncorrected: u32,
    /// Mots par minute bruts : toutes les frappes, justes ou non.
    pub wpm_raw: f64,
    /// Mots par minute nets : brut moins les erreurs non corrigées.
    pub wpm_net: f64,
    /// Caractères justes par minute.
    pub cpm: f64,
    /// En %, de 0 à 100.
    pub accuracy: f64,
}

/// Calcule les statistiques à partir des compteurs et du temps actif.
pub fn compute(counters: Counters, duration_ms: u32) -> SessionStats {
    // Ramène un compte à une vitesse par minute ; une durée nulle donne 0.
    let minutes = f64::from(duration_ms) / 60_000.0;
    let per_minute = |n: f64| if minutes > 0.0 { n / minutes } else { 0.0 };

    // Vitesse brute, puis nette : chaque erreur restante retire un mot par minute.
    let wpm_raw = per_minute(f64::from(counters.inserts) / CHARS_PER_WORD);
    let wpm_net = (wpm_raw - per_minute(f64::from(counters.uncorrected))).max(0.0);
    // Sans frappe, la précision est parfaite par convention.
    let accuracy = if counters.inserts == 0 {
        100.0
    } else {
        100.0 * f64::from(counters.correct_inserts) / f64::from(counters.inserts)
    };
    SessionStats {
        duration_ms,
        typed: counters.inserts,
        errors: counters.inserts - counters.correct_inserts,
        uncorrected: counters.uncorrected,
        wpm_raw,
        wpm_net,
        cpm: per_minute(f64::from(counters.correct_cells)),
        accuracy,
    }
}

/// Moyenne de sessions passées, pour la comparaison de l'écran de fin (F-32).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AverageStats {
    /// Nombre de sessions prises en compte.
    pub sessions: u32,
    /// MPM net moyen.
    pub wpm_net: f64,
    /// CPM moyen.
    pub cpm: f64,
    /// Précision moyenne, en %.
    pub accuracy: f64,
}

/// Moyenne arithmétique ; `None` s'il n'y a aucune session.
pub fn average(sessions: &[SessionStats]) -> Option<AverageStats> {
    // Pas de moyenne possible sans session.
    if sessions.is_empty() {
        return None;
    }
    // Moyenne d'un champ choisi par une fonction d'accès.
    let n = sessions.len() as f64;
    let mean = |f: fn(&SessionStats) -> f64| sessions.iter().map(f).sum::<f64>() / n;
    Some(AverageStats {
        sessions: sessions.len() as u32,
        wpm_net: mean(|s| s.wpm_net),
        cpm: mean(|s| s.cpm),
        accuracy: mean(|s| s.accuracy),
    })
}

/// Tests des formules de statistiques.
#[cfg(test)]
mod tests {
    use super::*;

    /// Construit des compteurs à partir de leurs quatre valeurs.
    fn counters(
        inserts: u32,
        correct_inserts: u32,
        correct_cells: u32,
        uncorrected: u32,
    ) -> Counters {
        Counters {
            inserts,
            correct_inserts,
            correct_cells,
            uncorrected,
        }
    }

    /// Une minute de frappe parfaite.
    #[test]
    fn one_minute_of_perfect_typing() {
        // 250 caractères justes en 1 min → 50 MPM, 250 CPM, 100 %.
        let s = compute(counters(250, 250, 250, 0), 60_000);
        assert_eq!(s.wpm_raw, 50.0);
        assert_eq!(s.wpm_net, 50.0);
        assert_eq!(s.cpm, 250.0);
        assert_eq!(s.accuracy, 100.0);
        assert_eq!(s.errors, 0);
    }

    /// Les erreurs non corrigées font baisser la vitesse nette.
    #[test]
    fn uncorrected_errors_lower_net_speed() {
        // 30 s, 100 frappes dont 10 fausses, 4 restées fausses.
        let s = compute(counters(100, 90, 86, 4), 30_000);
        assert_eq!(s.wpm_raw, 40.0);
        assert_eq!(s.wpm_net, 32.0);
        assert_eq!(s.cpm, 172.0);
        assert_eq!(s.accuracy, 90.0);
        assert_eq!(s.errors, 10);
        assert_eq!(s.uncorrected, 4);
        assert_eq!(s.typed, 100);
        assert_eq!(s.duration_ms, 30_000);
    }

    /// La vitesse nette ne descend jamais sous zéro.
    #[test]
    fn net_speed_never_goes_negative() {
        let s = compute(counters(5, 0, 0, 5), 6_000);
        assert_eq!(s.wpm_net, 0.0);
        assert_eq!(s.accuracy, 0.0);
    }

    /// Une session vide a une vitesse nulle et une précision de 100 %.
    #[test]
    fn empty_session_has_zero_speed_and_full_accuracy() {
        let s = compute(Counters::default(), 0);
        assert_eq!(s.wpm_raw, 0.0);
        assert_eq!(s.cpm, 0.0);
        assert_eq!(s.accuracy, 100.0);
    }

    /// Moyenne de deux sessions, et absence de moyenne sans session.
    #[test]
    fn average_of_sessions() {
        assert_eq!(average(&[]), None);
        // Les deux sessions des tests précédents.
        let a = compute(counters(250, 250, 250, 0), 60_000);
        let b = compute(counters(100, 90, 86, 4), 30_000);
        let avg = average(&[a, b]).unwrap();
        assert_eq!(avg.sessions, 2);
        assert_eq!(avg.wpm_net, 41.0);
        assert_eq!(avg.cpm, 211.0);
        assert_eq!(avg.accuracy, 95.0);
    }

    /// Les champs sont sérialisés en camelCase pour l'interface.
    #[test]
    fn stats_serialize_in_camel_case() {
        let json = serde_json::to_value(compute(Counters::default(), 0)).unwrap();
        assert!(json.get("wpmNet").is_some());
        assert!(json.get("durationMs").is_some());
    }
}
