//! Classification des fautes de frappe.

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Type de faute ; valeurs identiques à la contrainte `session_errors.error_type`.
///
/// Le mode Copie, caractère par caractère, produit `Accent`, `Case`,
/// `Punctuation` et `Substitution` ; les autres viennent de l'alignement
/// mot à mot de la dictée (F-16, Phase 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    Accent,
    MissingLetter,
    ExtraLetter,
    Transposition,
    Substitution,
    MissingWord,
    ExtraWord,
    Punctuation,
    Case,
}

impl ErrorType {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorType::Accent => "accent",
            ErrorType::MissingLetter => "missing_letter",
            ErrorType::ExtraLetter => "extra_letter",
            ErrorType::Transposition => "transposition",
            ErrorType::Substitution => "substitution",
            ErrorType::MissingWord => "missing_word",
            ErrorType::ExtraWord => "extra_word",
            ErrorType::Punctuation => "punctuation",
            ErrorType::Case => "case",
        }
    }
}

/// Faute relevée dans une session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TypingError {
    /// Mot attendu, ou caractère attendu hors mot.
    pub expected: String,
    /// Forme tapée au premier passage.
    pub typed: String,
    pub error_type: ErrorType,
    /// Index du premier graphème concerné.
    pub position: u32,
    /// `true` pour un mot, `false` pour une espace ou une ponctuation isolée.
    pub word: bool,
}

/// Classe une faute sur un caractère : attendu `expected`, tapé `typed`.
pub fn classify(expected: &str, typed: &str) -> ErrorType {
    if expected.to_lowercase() == typed.to_lowercase() {
        ErrorType::Case
    } else if is_letter(expected) && base_letters(expected) == base_letters(typed) {
        ErrorType::Accent
    } else if is_punctuation(expected) || is_punctuation(typed) {
        ErrorType::Punctuation
    } else {
        ErrorType::Substitution
    }
}

/// Lettres sans diacritiques, en minuscules : « Ê » → « e ».
fn base_letters(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_letter(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_alphabetic)
}

fn is_punctuation(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| !c.is_alphanumeric() && !c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_case_accent_punctuation_substitution() {
        assert_eq!(classify("L", "l"), ErrorType::Case);
        assert_eq!(classify("ê", "e"), ErrorType::Accent);
        assert_eq!(classify("e", "é"), ErrorType::Accent);
        assert_eq!(classify("É", "e"), ErrorType::Accent);
        assert_eq!(classify(",", "."), ErrorType::Punctuation);
        assert_eq!(classify("a", ";"), ErrorType::Punctuation);
        assert_eq!(classify("a", "z"), ErrorType::Substitution);
        assert_eq!(classify(" ", "a"), ErrorType::Substitution);
        assert_eq!(classify("1", "2"), ErrorType::Substitution);
    }

    #[test]
    fn error_types_match_database_values() {
        let all = [
            ErrorType::Accent,
            ErrorType::MissingLetter,
            ErrorType::ExtraLetter,
            ErrorType::Transposition,
            ErrorType::Substitution,
            ErrorType::MissingWord,
            ErrorType::ExtraWord,
            ErrorType::Punctuation,
            ErrorType::Case,
        ];
        for t in all {
            let json = serde_json::to_value(t).unwrap();
            assert_eq!(json, t.as_str());
        }
    }
}
