//! Tentatives de mots d'une session : forme tapée, justesse et durée.
//!
//! Elles alimentent le suivi des mots difficiles (F-40 à F-42, ADR 0015). La
//! lenteur n'est pas jugée ici : le seuil est un réglage du module Révision, qui
//! compare la durée de chaque mot à la moyenne de la session (`is_slow`).

use serde::{Deserialize, Serialize};

use super::{ErrorType, TypingSession};
use crate::text::WordSpan;

/// Seuil de lenteur par défaut : un mot tapé 1,5 fois plus lentement que la
/// moyenne de la session est jugé lent (F-40).
pub const DEFAULT_SLOW_FACTOR: f64 = 1.5;

/// Nombre minimal de graphèmes d'un mot suivi : « a », « l », « y » ne sont pas des
/// mots difficiles, seulement du bruit.
const MIN_WORD_LEN: usize = 2;

/// Un mot tapé pendant une session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WordAttempt {
    /// Clé du mot : minuscules, NFC (« Le » et « le » sont le même mot).
    pub word: String,
    /// Mot tel qu'il figure dans le texte.
    pub expected: String,
    /// Forme tapée au premier passage.
    pub typed: String,
    /// Juste du premier coup ?
    pub correct: bool,
    /// Type de faute dominant, si le mot est faux.
    pub error_type: Option<ErrorType>,
    /// Index du premier graphème du mot dans le texte.
    pub position: u32,
    /// Temps passé sur le mot, en ms, depuis la frappe qui le précède.
    pub duration_ms: u32,
    /// Caractères couverts par cette durée (le premier mot du texte en a un de moins).
    pub timed_chars: u32,
}

impl WordAttempt {
    /// Le mot a-t-il été tapé plus de `factor` fois plus lentement que la moyenne
    /// `ms_per_char` de la session ? Sans durée mesurable, il n'est jamais lent.
    pub fn is_slow(&self, ms_per_char: f64, factor: f64) -> bool {
        self.timed_chars > 0
            && ms_per_char > 0.0
            && f64::from(self.duration_ms) / f64::from(self.timed_chars) > factor * ms_per_char
    }
}

/// Mots d'une session et vitesse de référence pour juger leur lenteur.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SessionWords {
    /// Mots suivis entièrement tapés, dans l'ordre du texte.
    pub attempts: Vec<WordAttempt>,
    /// Durée moyenne par caractère de tous les mots tapés (courts compris), en ms.
    pub ms_per_char: f64,
}

/// Mot entièrement tapé, avant le filtrage des mots suivis.
struct Timed<'a> {
    /// Position du mot dans le texte.
    span: &'a WordSpan,
    /// Durée du mot, en ms.
    duration_ms: u32,
    /// Caractères couverts par la durée.
    timed_chars: u32,
}

impl TypingSession {
    /// Mots entièrement tapés et durée moyenne par caractère de la session.
    ///
    /// Les mots d'une seule lettre et ceux sans lettre (nombres) ne sont pas
    /// renvoyés, mais comptent dans la moyenne.
    pub fn word_attempts(&self) -> SessionWords {
        // Première passe : durée de chaque mot entièrement tapé.
        let timed: Vec<Timed> = self
            .words
            .iter()
            .filter_map(|span| self.time_word(span))
            .collect();
        // Durée moyenne par caractère, référence de la lenteur.
        let (total_ms, total_chars) = timed.iter().fold((0u64, 0u64), |(ms, n), t| {
            (ms + u64::from(t.duration_ms), n + u64::from(t.timed_chars))
        });
        let ms_per_char = if total_chars > 0 {
            total_ms as f64 / total_chars as f64
        } else {
            0.0
        };

        // Seconde passe : les mots suivis, avec leur justesse.
        let attempts = timed
            .into_iter()
            .filter(|t| self.is_trackable(t.span))
            .map(|t| {
                let (start, end) = (t.span.start as usize, t.span.end as usize);
                let expected = self.text[start..end].concat();
                let error_type = self.word_error_type(t.span);
                WordAttempt {
                    word: expected.to_lowercase(),
                    typed: self.first_typed(t.span),
                    expected,
                    correct: error_type.is_none(),
                    error_type,
                    position: t.span.start,
                    duration_ms: t.duration_ms,
                    timed_chars: t.timed_chars,
                }
            })
            .collect();
        SessionWords {
            attempts,
            ms_per_char,
        }
    }

    /// Durée d'un mot, ou `None` s'il n'est pas entièrement tapé.
    fn time_word<'a>(&self, span: &'a WordSpan) -> Option<Timed<'a>> {
        let (start, end) = (span.start as usize, span.end as usize);
        // Le mot est fini quand chacune de ses positions a reçu une insertion.
        let times: Option<Vec<u32>> = self.insert_t[start..end].iter().copied().collect();
        let last = times?.into_iter().max()?;
        // Départ : la frappe qui précède le mot (souvent l'espace). Pour le premier
        // mot du texte, sa première lettre, qui n'a alors pas de durée propre.
        let len = (end - start) as u32;
        let (begin, timed_chars) = match start.checked_sub(1).and_then(|p| self.insert_t[p]) {
            Some(t) => (t, len),
            None => (self.insert_t[start]?, len - 1),
        };
        Some(Timed {
            span,
            duration_ms: last.saturating_sub(begin),
            timed_chars,
        })
    }

    /// Mot assez long et contenant au moins une lettre pour être suivi.
    fn is_trackable(&self, span: &WordSpan) -> bool {
        let graphemes = &self.text[span.start as usize..span.end as usize];
        graphemes.len() >= MIN_WORD_LEN
            && graphemes
                .iter()
                .any(|g| g.chars().next().is_some_and(char::is_alphabetic))
    }
}

/// Tests des tentatives de mots.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{graphemes, normalize};
    use crate::typing::{KeyKind, Keystroke, Rules};

    /// Frappes d'insertion, une par graphème de `typed`, aux temps donnés.
    fn keys(typed: &str, times: &[u32]) -> Vec<Keystroke> {
        let g = graphemes(typed);
        assert_eq!(g.len(), times.len(), "un temps par graphème");
        g.into_iter()
            .zip(times)
            .enumerate()
            .map(|(i, (typed, &t_ms))| Keystroke {
                seq: i as u32,
                kind: KeyKind::Insert,
                typed: Some(typed),
                t_ms,
            })
            .collect()
    }

    /// Frappes à intervalle régulier de `step` ms.
    fn regular(typed: &str, step: u32) -> Vec<Keystroke> {
        let n = graphemes(typed).len() as u32;
        let times: Vec<u32> = (0..n).map(|i| i * step).collect();
        keys(typed, &times)
    }

    /// Session rejouée sur `text` avec les règles par défaut (mode libre).
    fn replay(text: &str, ks: &[Keystroke]) -> TypingSession {
        TypingSession::replay(graphemes(&normalize(text)), Rules::default(), ks)
    }

    /// Une frappe parfaite et régulière : tous les mots justes, aucun lent.
    #[test]
    fn regular_perfect_typing_gives_correct_normal_words() {
        let session = replay("le chat dort", &regular("le chat dort", 100)).word_attempts();
        let attempts = &session.attempts;
        let words: Vec<&str> = attempts.iter().map(|a| a.word.as_str()).collect();
        assert_eq!(words, ["le", "chat", "dort"]);
        assert!(attempts.iter().all(|a| a.correct && a.error_type.is_none()));
        assert!(
            attempts
                .iter()
                .all(|a| !a.is_slow(session.ms_per_char, DEFAULT_SLOW_FACTOR))
        );
        // 11 caractères chronométrés (le « l » initial n'a pas de durée) en 1 100 ms.
        assert_eq!(session.ms_per_char, 100.0);
        // « chat » : de l'espace (t = 200) à « t » (t = 600).
        assert_eq!(attempts[1].duration_ms, 400);
        assert_eq!(attempts[1].timed_chars, 4);
        assert_eq!(attempts[1].position, 3);
    }

    /// Un mot mal tapé garde sa forme fautive et son type de faute.
    #[test]
    fn mistyped_word_keeps_typed_form_and_error_type() {
        let s = replay("une fête", &regular("une fete", 100));
        let a = &s.word_attempts().attempts[1];
        assert_eq!(a.expected, "fête");
        assert_eq!(a.typed, "fete");
        assert!(!a.correct);
        assert_eq!(a.error_type, Some(ErrorType::Accent));
    }

    /// Un mot corrigé après coup reste faux : la faute du premier passage compte.
    #[test]
    fn corrected_word_still_counts_as_mistyped() {
        let mut ks = regular("ax", 100);
        ks.push(Keystroke {
            seq: 2,
            kind: KeyKind::Delete,
            typed: None,
            t_ms: 200,
        });
        ks.push(Keystroke {
            seq: 3,
            kind: KeyKind::Insert,
            typed: Some("b".into()),
            t_ms: 300,
        });
        let s = replay("ab", &ks);
        let a = &s.word_attempts().attempts[0];
        assert!(!a.correct);
        assert_eq!(a.typed, "ax");
        // Premier mot : de « a » (t = 0) au « b » retapé (t = 300).
        assert_eq!(a.duration_ms, 300);
    }

    /// Un mot tapé bien plus lentement que la moyenne est jugé lent.
    #[test]
    fn slow_word_is_detected() {
        // « vite vite lent vite » : 100 ms par caractère, sauf « ␣lent » à 400 ms.
        let text = "vite vite lent vite";
        let mut times = Vec::new();
        let mut t = 0;
        for i in 0..graphemes(text).len() {
            times.push(t);
            // Les caractères 9 à 13 (« ␣lent ») arrivent 400 ms après le précédent.
            t += if (9..14).contains(&(i + 1)) { 400 } else { 100 };
        }
        let session = replay(text, &keys(text, &times)).word_attempts();
        let slow = |factor: f64| -> Vec<bool> {
            session
                .attempts
                .iter()
                .map(|a| a.is_slow(session.ms_per_char, factor))
                .collect()
        };
        assert_eq!(slow(DEFAULT_SLOW_FACTOR), [false, false, true, false]);
        // Avec un seuil très haut, plus rien n'est lent.
        assert_eq!(slow(10.0), [false; 4]);
    }

    /// Les mots pas finis, d'une lettre ou sans lettre ne sont pas renvoyés.
    #[test]
    fn skips_unfinished_short_and_numeric_words() {
        // « l'arbre a 12 ans » tapé jusqu'au milieu de « ans ».
        let s = replay("l'arbre a 12 ans", &regular("l'arbre a 12 a", 100));
        let words: Vec<String> = s
            .word_attempts()
            .attempts
            .into_iter()
            .map(|a| a.word)
            .collect();
        assert_eq!(words, ["arbre"]);
    }

    /// La clé est en minuscules, le mot attendu garde sa casse.
    #[test]
    fn key_is_lowercase_and_expected_keeps_case() {
        let session = replay("Été", &regular("Été", 100)).word_attempts();
        let a = &session.attempts[0];
        assert_eq!(a.word, "été");
        assert_eq!(a.expected, "Été");
        // Seul mot du texte : la référence est lui-même, il n'est pas lent.
        assert!(!a.is_slow(session.ms_per_char, DEFAULT_SLOW_FACTOR));
    }

    /// Une session sans frappe ne donne aucune tentative.
    #[test]
    fn no_keystrokes_no_attempts() {
        let s = replay("bonjour", &[]);
        assert_eq!(s.word_attempts(), SessionWords::default());
    }

    /// Sans caractère chronométré ou sans moyenne, un mot n'est jamais lent.
    #[test]
    fn untimed_word_is_never_slow() {
        // « ab » seul : la première lettre n'a pas de durée ; « b » la porte.
        let s = replay("ab", &keys("ab", &[0, 5000]));
        let a = &s.word_attempts().attempts[0];
        assert_eq!(a.duration_ms, 5000);
        assert_eq!(a.timed_chars, 1);
        let untimed = WordAttempt {
            timed_chars: 0,
            ..a.clone()
        };
        assert!(!untimed.is_slow(1.0, DEFAULT_SLOW_FACTOR));
        assert!(!a.is_slow(0.0, DEFAULT_SLOW_FACTOR));
    }
}
