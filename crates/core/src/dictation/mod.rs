//! Mode Dictée (F-10 à F-17) : correction mot à mot, découpage de la lecture,
//! ponctuation lue, avancement et réglages.

/// Avancement d'un segment au suivant (F-15).
pub mod advance;
/// Alignement mot à mot et classement des fautes (F-16).
pub mod align;
/// Texte lu, ponctuation dite en toutes lettres (F-13).
pub mod punctuation;
/// Découpage en segments (F-11).
pub mod segment;
/// Réglages de la dictée et des profils (F-12 à F-15, F-17).
pub mod settings;

use crate::typing::{SessionWords, WordAttempt};

pub use align::{AlignedToken, Correction, TypeCount, correct};

/// Mots d'une dictée pour le suivi des mots difficiles (ADR 0015) : un par mot
/// attendu aligné, juste ou faux. Sans mesure du temps par mot, aucun n'est « lent ».
/// Les mots collés ou scindés (« parce que » tapé « parceque ») ne sont pas suivis.
pub fn session_words(correction: &Correction) -> SessionWords {
    let attempts = correction
        .tokens
        .iter()
        .filter(|t| t.word)
        .filter_map(|t| {
            // Seuls les mots attendus d'un seul tenant ont une clé de mot.
            let expected = t.expected.as_ref().filter(|e| !e.contains(' '))?;
            Some(WordAttempt {
                word: expected.to_lowercase(),
                expected: expected.clone(),
                typed: t.typed.clone().unwrap_or_default(),
                correct: t.error.is_none(),
                error_type: t.error,
                position: t.position.unwrap_or(0),
                duration_ms: 0,
                timed_chars: 0,
            })
        })
        .collect();
    SessionWords {
        attempts,
        ms_per_char: 0.0,
    }
}

/// Tests du passage d'une dictée aux mots suivis.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::typing::ErrorType;

    /// Mots justes et faux suivis, oubliés compris ; mots en trop et collés ignorés.
    #[test]
    fn session_words_follow_expected_words() {
        let c = correct("Le chat parce que dort", "le gros chat parceque", true);
        let words = session_words(&c);
        let got: Vec<(&str, &str, bool)> = words
            .attempts
            .iter()
            .map(|a| (a.word.as_str(), a.typed.as_str(), a.correct))
            .collect();
        assert_eq!(
            got,
            [
                ("le", "le", false),
                ("chat", "chat", true),
                ("dort", "", false)
            ]
        );
        assert_eq!(words.attempts[0].error_type, Some(ErrorType::Case));
        assert_eq!(words.attempts[2].error_type, Some(ErrorType::MissingWord));
        // Jamais lent : aucune durée mesurée.
        assert!(
            words
                .attempts
                .iter()
                .all(|a| !a.is_slow(words.ms_per_char, 1.5))
        );
    }
}
