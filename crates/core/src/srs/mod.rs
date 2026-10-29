//! Répétition espacée derrière le trait `Scheduler` (F-42, ADR 0016).
//!
//! `Sm2` est l'implémentation actuelle ; FSRS pourra la remplacer sans toucher
//! au suivi des mots (`words`).

/// Exercices générés à partir des mots à réviser (F-44).
pub mod exercises;
/// Suivi des mots difficiles : rencontres, erreurs, maîtrise, prochaine révision.
pub mod words;

use serde::{Deserialize, Serialize};

/// Facteur de facilité initial de SM-2.
pub const INITIAL_EASE: f64 = 2.5;

/// Facteur de facilité minimal de SM-2 : en dessous, les intervalles stagnent.
pub const MIN_EASE: f64 = 1.3;

/// Résultat d'une révision, du point de vue du planificateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Grade {
    /// Mot mal écrit : à revoir dès demain.
    Again,
    /// Juste mais lent.
    Hard,
    /// Juste, à vitesse normale.
    Good,
}

impl Grade {
    /// Note de qualité SM-2, de 0 à 5.
    pub fn quality(self) -> u8 {
        match self {
            Grade::Again => 1,
            Grade::Hard => 3,
            Grade::Good => 5,
        }
    }
}

/// État de planification d'un mot, stocké dans `words.srs_state` (JSON).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SrsState {
    /// Révisions réussies d'affilée, au sens du planificateur.
    pub repetitions: u32,
    /// Dernier intervalle, en jours.
    pub interval_days: f64,
    /// Facteur de facilité.
    pub ease: f64,
}

impl Default for SrsState {
    /// Mot jamais révisé.
    fn default() -> Self {
        Self {
            repetitions: 0,
            interval_days: 0.0,
            ease: INITIAL_EASE,
        }
    }
}

/// Planificateur de révisions : calcule l'état suivant d'un mot après une révision.
pub trait Scheduler: Send + Sync {
    /// État après une révision notée `grade` ; `interval_days` donne l'écart
    /// jusqu'à la prochaine révision.
    fn review(&self, state: &SrsState, grade: Grade) -> SrsState;
}

/// Algorithme SM-2 (SuperMemo 2).
#[derive(Debug, Clone, Copy, Default)]
pub struct Sm2;

impl Scheduler for Sm2 {
    /// 1 jour, puis 6, puis l'intervalle précédent × facteur de facilité.
    /// Un échec remet le compteur à zéro et redonne 1 jour.
    fn review(&self, state: &SrsState, grade: Grade) -> SrsState {
        // Le facteur de facilité baisse d'autant plus que la note est faible.
        let gap = 5.0 - f64::from(grade.quality());
        let ease = (state.ease + 0.1 - gap * (0.08 + gap * 0.02)).max(MIN_EASE);
        // Un échec fait tout recommencer.
        if grade == Grade::Again {
            return SrsState {
                repetitions: 0,
                interval_days: 1.0,
                ease,
            };
        }
        // Réussite : l'intervalle grandit avec le nombre de réussites.
        let interval_days = match state.repetitions {
            0 => 1.0,
            1 => 6.0,
            _ => (state.interval_days * state.ease).round(),
        };
        SrsState {
            repetitions: state.repetitions + 1,
            interval_days,
            ease,
        }
    }
}

/// Tests du planificateur SM-2.
#[cfg(test)]
mod tests {
    use super::*;

    /// Trois bonnes réponses : 1 j, 6 j, puis 6 × 2,7 ≈ 16 j (facilité montée deux fois).
    #[test]
    fn good_answers_grow_intervals() {
        let s1 = Sm2.review(&SrsState::default(), Grade::Good);
        let s2 = Sm2.review(&s1, Grade::Good);
        let s3 = Sm2.review(&s2, Grade::Good);
        assert_eq!(
            [s1.interval_days, s2.interval_days, s3.interval_days],
            [1.0, 6.0, 16.0]
        );
        assert_eq!(s3.repetitions, 3);
        // Une note de 5 fait monter la facilité de 0,1 à chaque fois.
        assert!((s3.ease - 2.8).abs() < 1e-9);
    }

    /// Un échec remet à zéro et redonne 1 jour.
    #[test]
    fn failure_resets_repetitions() {
        let good = Sm2.review(&Sm2.review(&SrsState::default(), Grade::Good), Grade::Good);
        let failed = Sm2.review(&good, Grade::Again);
        assert_eq!(failed.repetitions, 0);
        assert_eq!(failed.interval_days, 1.0);
        assert!(failed.ease < good.ease);
    }

    /// Une note « difficile » fait avancer, mais baisse un peu la facilité.
    #[test]
    fn hard_advances_and_lowers_ease() {
        let s = Sm2.review(&SrsState::default(), Grade::Hard);
        assert_eq!(s.repetitions, 1);
        assert!((s.ease - 2.36).abs() < 1e-9);
    }

    /// La facilité ne descend jamais sous 1,3.
    #[test]
    fn ease_has_a_floor() {
        let mut s = SrsState::default();
        for _ in 0..20 {
            s = Sm2.review(&s, Grade::Again);
        }
        assert_eq!(s.ease, MIN_EASE);
    }

    /// Un état JSON incomplet ou vide (`{}` en base) prend les valeurs par défaut.
    #[test]
    fn empty_json_state_is_default() {
        let s: SrsState = serde_json::from_str("{}").unwrap();
        assert_eq!(s, SrsState::default());
        let back: SrsState = serde_json::from_value(serde_json::to_value(s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    /// Les notes SM-2 associées aux trois résultats.
    #[test]
    fn grade_qualities() {
        assert_eq!(
            [Grade::Again, Grade::Hard, Grade::Good].map(Grade::quality),
            [1, 3, 5]
        );
    }
}
