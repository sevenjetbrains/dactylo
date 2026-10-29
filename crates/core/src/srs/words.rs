//! Suivi d'un mot : rencontres, erreurs, série de réussites, maîtrise et
//! prochaine révision (F-40 à F-42, F-45 ; ADR 0016).
//!
//! Chaque mot rencontré est compté. Il n'entre en révision (`next_review_at`)
//! que s'il est raté ou lent ; il en sort, maîtrisé, après `MASTERY_STREAK`
//! sessions réussies de suite à vitesse normale.

use serde::{Deserialize, Serialize};

use super::{Grade, Scheduler, SrsState};
use crate::days::DAY_MS;
use crate::typing::WordAttempt;

/// Sessions réussies d'affilée, sans faute et à vitesse normale, pour qu'un mot
/// soit maîtrisé (F-42).
pub const MASTERY_STREAK: u32 = 3;

/// Niveau de maîtrise ; valeurs identiques à la contrainte `words.mastery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Mastery {
    /// En cours d'apprentissage (ou jamais difficile, tant qu'il n'est pas planifié).
    #[default]
    Learning,
    /// Maîtrisé après `MASTERY_STREAK` réussites.
    Mastered,
    /// Marqué maîtrisé à la main (F-45) : plus jamais planifié.
    ManualMastered,
}

impl Mastery {
    /// Valeur stockée dans la colonne `words.mastery`.
    pub fn as_str(self) -> &'static str {
        match self {
            Mastery::Learning => "learning",
            Mastery::Mastered => "mastered",
            Mastery::ManualMastered => "manual_mastered",
        }
    }

    /// Lit la valeur de la colonne ; toute valeur inconnue vaut `Learning`.
    pub fn parse(value: &str) -> Self {
        match value {
            "mastered" => Mastery::Mastered,
            "manual_mastered" => Mastery::ManualMastered,
            _ => Mastery::Learning,
        }
    }
}

/// Contexte d'une session pour noter ses mots.
#[derive(Clone, Copy)]
pub struct SessionReview<'a> {
    /// Planificateur des révisions.
    pub scheduler: &'a dyn Scheduler,
    /// Durée moyenne par caractère de la session, en ms (référence de la lenteur).
    pub ms_per_char: f64,
    /// Seuil de lenteur (F-40), par exemple 1,5.
    pub slow_factor: f64,
    /// Instant de la fin de session, en ms Unix.
    pub now_ms: i64,
}

/// État d'un mot pour un profil, tel que rangé dans la table `words`.
#[derive(Debug, Clone, PartialEq)]
pub struct WordRecord {
    /// Clé du mot (minuscules, NFC).
    pub word: String,
    /// Nombre de fois où le mot a été tapé.
    pub encounters: u32,
    /// Nombre de fois où il a été mal écrit.
    pub errors: u32,
    /// Durée moyenne d'une frappe du mot, en ms.
    pub avg_ms: Option<f64>,
    /// Niveau de maîtrise.
    pub mastery: Mastery,
    /// Sessions réussies d'affilée depuis la dernière faute ou lenteur.
    pub streak: u32,
    /// Dernière faute, en ms Unix.
    pub last_error_at: Option<i64>,
    /// Prochaine révision, en ms Unix ; `None` = hors révision.
    pub next_review_at: Option<i64>,
    /// État du planificateur.
    pub srs: SrsState,
}

impl WordRecord {
    /// Mot jamais rencontré, hors révision.
    pub fn new(word: impl Into<String>) -> Self {
        Self {
            word: word.into(),
            encounters: 0,
            errors: 0,
            avg_ms: None,
            mastery: Mastery::Learning,
            streak: 0,
            last_error_at: None,
            next_review_at: None,
            srs: SrsState::default(),
        }
    }

    /// Mot ajouté à la main (F-45) : à réviser tout de suite.
    pub fn added(word: impl Into<String>, now_ms: i64) -> Self {
        Self {
            next_review_at: Some(now_ms),
            ..Self::new(word)
        }
    }

    /// Le mot est-il en révision (difficile, ni maîtrisé, ni hors suivi) ?
    pub fn is_tracked(&self) -> bool {
        self.mastery == Mastery::Learning && self.next_review_at.is_some()
    }

    /// Le mot est-il à réviser avant `before_ms` (borne exclue) ?
    pub fn is_due(&self, before_ms: i64) -> bool {
        self.is_tracked() && self.next_review_at.is_some_and(|at| at < before_ms)
    }

    /// Marque le mot maîtrisé à la main (F-45) : il sort de la révision.
    pub fn mark_mastered(&mut self) {
        self.mastery = Mastery::ManualMastered;
        self.next_review_at = None;
    }

    /// Applique les tentatives d'une session sur ce mot. Le planificateur n'est
    /// appelé qu'une fois par session, même si le mot y figure plusieurs fois.
    pub fn apply_session(&mut self, attempts: &[&WordAttempt], review: &SessionReview) {
        // Rien à faire sans tentative.
        if attempts.is_empty() {
            return;
        }
        // Compteurs bruts : rencontres, erreurs et moyenne des durées.
        let wrong = attempts.iter().filter(|a| !a.correct).count() as u32;
        let total_ms: f64 = attempts.iter().map(|a| f64::from(a.duration_ms)).sum();
        let previous = self.avg_ms.unwrap_or(0.0) * f64::from(self.encounters);
        self.encounters += attempts.len() as u32;
        self.avg_ms = Some((previous + total_ms) / f64::from(self.encounters));
        self.errors += wrong;
        if wrong > 0 {
            self.last_error_at = Some(review.now_ms);
        }

        // Note de la session : une faute l'emporte sur la lenteur.
        let grade = if wrong > 0 {
            Grade::Again
        } else if attempts
            .iter()
            .any(|a| a.is_slow(review.ms_per_char, review.slow_factor))
        {
            Grade::Hard
        } else {
            Grade::Good
        };

        // Selon la maîtrise, la session planifie le mot ou le laisse tel quel.
        match self.mastery {
            // Maîtrisé à la main : on compte, sans jamais replanifier.
            Mastery::ManualMastered => return,
            // Maîtrisé : une réussite ne change rien ; une rechute le remet en révision.
            Mastery::Mastered if grade == Grade::Good => return,
            Mastery::Mastered => {
                self.mastery = Mastery::Learning;
                self.srs = SrsState::default();
            }
            // Jamais difficile et réussi : il reste hors révision.
            Mastery::Learning if !self.is_tracked() && grade == Grade::Good => return,
            Mastery::Learning => {}
        }
        self.schedule(grade, review.scheduler, review.now_ms);
    }

    /// Planifie la prochaine révision et met à jour la série et la maîtrise.
    fn schedule(&mut self, grade: Grade, scheduler: &dyn Scheduler, now_ms: i64) {
        self.srs = scheduler.review(&self.srs, grade);
        // Seule une réussite à vitesse normale allonge la série.
        self.streak = if grade == Grade::Good {
            self.streak + 1
        } else {
            0
        };
        // Série complète : maîtrisé, hors révision. Sinon, prochaine date.
        if self.streak >= MASTERY_STREAK {
            self.mastery = Mastery::Mastered;
            self.next_review_at = None;
        } else {
            let delay = (self.srs.interval_days * DAY_MS as f64).round() as i64;
            self.next_review_at = Some(now_ms + delay);
        }
    }
}

/// Tests du suivi des mots.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::srs::Sm2;
    use crate::typing::{DEFAULT_SLOW_FACTOR, ErrorType};

    /// Instant de départ arbitraire.
    const T0: i64 = 1_793_232_000_000;

    /// Durée moyenne par caractère des sessions de test.
    const MS_PER_CHAR: f64 = 100.0;

    /// Tentative juste ou fausse de 4 caractères : 400 ms, ou 4 000 ms si lente.
    fn attempt(correct: bool, slow: bool) -> WordAttempt {
        WordAttempt {
            word: "fête".into(),
            expected: "fête".into(),
            typed: if correct { "fête" } else { "fete" }.into(),
            correct,
            error_type: (!correct).then_some(ErrorType::Accent),
            position: 0,
            duration_ms: if slow { 4000 } else { 400 },
            timed_chars: 4,
        }
    }

    /// Contexte de session au jour `day`.
    fn review(day: i64) -> SessionReview<'static> {
        SessionReview {
            scheduler: &Sm2,
            ms_per_char: MS_PER_CHAR,
            slow_factor: DEFAULT_SLOW_FACTOR,
            now_ms: T0 + day * DAY_MS,
        }
    }

    /// Applique une session d'une tentative au jour `day`.
    fn play(record: &mut WordRecord, correct: bool, slow: bool, day: i64) {
        let a = attempt(correct, slow);
        record.apply_session(&[&a], &review(day));
    }

    /// Un mot réussi reste hors révision mais est compté.
    #[test]
    fn correct_unknown_word_is_counted_not_tracked() {
        let mut r = WordRecord::new("fête");
        play(&mut r, true, false, 0);
        assert_eq!(r.encounters, 1);
        assert_eq!(r.errors, 0);
        assert_eq!(r.avg_ms, Some(400.0));
        assert!(!r.is_tracked());
        assert_eq!(r.next_review_at, None);
    }

    /// Critère transverse : un mot raté est à réviser le lendemain et disparaît
    /// après 3 réussites.
    #[test]
    fn mistyped_word_is_due_next_day_and_mastered_after_three_successes() {
        let mut r = WordRecord::new("fête");
        play(&mut r, false, false, 0);
        assert!(r.is_tracked());
        assert_eq!(r.last_error_at, Some(T0));
        // À réviser demain, pas aujourd'hui.
        assert!(!r.is_due(T0 + 1));
        assert!(r.is_due(T0 + DAY_MS + 1));
        // Trois sessions réussies : 1 j, 6 j, puis maîtrisé.
        play(&mut r, true, false, 1);
        assert_eq!(r.next_review_at, Some(T0 + 2 * DAY_MS));
        play(&mut r, true, false, 2);
        assert_eq!(r.next_review_at, Some(T0 + 8 * DAY_MS));
        assert_eq!(r.streak, 2);
        play(&mut r, true, false, 8);
        assert_eq!(r.mastery, Mastery::Mastered);
        assert!(!r.is_tracked());
        assert!(!r.is_due(i64::MAX));
        assert_eq!(r.encounters, 4);
        assert_eq!(r.errors, 1);
    }

    /// Un mot lent entre en révision ; la lenteur casse la série.
    #[test]
    fn slow_word_is_tracked_and_breaks_streak() {
        let mut r = WordRecord::new("fête");
        play(&mut r, true, true, 0);
        assert!(r.is_tracked());
        assert_eq!(r.errors, 0);
        assert_eq!(r.last_error_at, None);
        play(&mut r, true, false, 1);
        assert_eq!(r.streak, 1);
        play(&mut r, true, true, 2);
        assert_eq!(r.streak, 0);
        assert!(r.is_tracked());
    }

    /// Une faute après la maîtrise remet le mot en révision, depuis le début.
    #[test]
    fn relapse_after_mastery_tracks_again() {
        let mut r = WordRecord::new("fête");
        play(&mut r, false, false, 0);
        for day in [1, 2, 8] {
            play(&mut r, true, false, day);
        }
        assert_eq!(r.mastery, Mastery::Mastered);
        // Une réussite ne change rien au mot maîtrisé.
        play(&mut r, true, false, 20);
        assert_eq!(r.mastery, Mastery::Mastered);
        // Une faute : retour en révision, à revoir le lendemain.
        play(&mut r, false, false, 30);
        assert_eq!(r.mastery, Mastery::Learning);
        assert_eq!(r.streak, 0);
        assert_eq!(r.next_review_at, Some(T0 + 31 * DAY_MS));
    }

    /// Marqué maîtrisé à la main : compté, mais plus jamais planifié.
    #[test]
    fn manual_mastery_is_never_rescheduled() {
        let mut r = WordRecord::new("fête");
        play(&mut r, false, false, 0);
        r.mark_mastered();
        assert!(!r.is_tracked());
        play(&mut r, false, false, 1);
        assert_eq!(r.mastery, Mastery::ManualMastered);
        assert_eq!(r.next_review_at, None);
        assert_eq!(r.errors, 2);
    }

    /// Un mot ajouté à la main est à réviser tout de suite.
    #[test]
    fn added_word_is_due_now() {
        let r = WordRecord::added("néanmoins", T0);
        assert!(r.is_due(T0 + 1));
        assert_eq!(r.encounters, 0);
    }

    /// Un mot vu plusieurs fois dans une session n'est planifié qu'une fois.
    #[test]
    fn several_attempts_in_one_session_schedule_once() {
        let mut r = WordRecord::new("fête");
        play(&mut r, false, false, 0);
        let ok = attempt(true, false);
        r.apply_session(&[&ok, &ok, &ok], &review(1));
        assert_eq!(r.streak, 1);
        assert_eq!(r.encounters, 4);
        assert_eq!(r.mastery, Mastery::Learning);
        // Une seule faute parmi les tentatives suffit à rater la session.
        let ko = attempt(false, false);
        r.apply_session(&[&ok, &ko], &review(2));
        assert_eq!(r.streak, 0);
        assert_eq!(r.errors, 2);
    }

    /// Une session sans tentative ne change rien.
    #[test]
    fn empty_session_changes_nothing() {
        let mut r = WordRecord::new("fête");
        r.apply_session(&[], &review(0));
        assert_eq!(r, WordRecord::new("fête"));
    }

    /// `as_str` et `parse` sont réciproques ; une valeur inconnue vaut `Learning`.
    #[test]
    fn mastery_round_trips_through_database_values() {
        for m in [
            Mastery::Learning,
            Mastery::Mastered,
            Mastery::ManualMastered,
        ] {
            assert_eq!(Mastery::parse(m.as_str()), m);
            assert_eq!(serde_json::to_value(m).unwrap(), m.as_str());
        }
        assert_eq!(Mastery::parse("???"), Mastery::Learning);
    }
}
