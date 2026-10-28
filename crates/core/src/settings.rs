//! Réglages du mode Copie (F-01 à F-06) : valeurs par défaut et bornes.
//!
//! Stockés sous la clé `practice` du JSON `profiles.settings` (ADR 0010) ;
//! une valeur absente prend sa valeur par défaut, une valeur hors bornes
//! est ramenée dans ses bornes.

use serde::{Deserialize, Serialize};

use crate::text::WordSpan;
use crate::typing::{CorrectionMode, Rules, TypingSession};

pub const FONT_SIZE_RANGE: (u32, u32) = (14, 32);
pub const VISIBLE_LINES_RANGE: (u32, u32) = (1, 10);
/// Durées proposées pour une session chronométrée (F-06).
pub const DURATION_CHOICES: [u32; 4] = [1, 2, 5, 10];
pub const WORD_COUNT_RANGE: (u32, u32) = (1, 10_000);

/// Position de la ligne active dans la zone de frappe (F-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ActiveLine {
    #[default]
    Top,
    Center,
}

/// Type de session (F-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionGoal {
    /// Jusqu'au dernier caractère du texte.
    #[default]
    WholeText,
    /// Durée fixe, en minutes.
    Duration { minutes: u32 },
    /// Nombre de mots fixe.
    Words { count: u32 },
}

impl SessionGoal {
    /// Position à atteindre : fin du texte, ou fin du N-ième mot.
    pub fn end_position(&self, text_len: usize, words: &[WordSpan]) -> usize {
        match *self {
            SessionGoal::Words { count } => (count as usize)
                .checked_sub(1)
                .and_then(|i| words.get(i))
                .map_or(text_len, |w| w.end as usize),
            _ => text_len,
        }
    }

    pub fn duration_ms(&self) -> Option<u32> {
        match *self {
            SessionGoal::Duration { minutes } => Some(minutes * 60_000),
            _ => None,
        }
    }

    /// La session est-elle terminée ? (temps actif en ms)
    pub fn is_reached(&self, session: &TypingSession, active_ms: u32) -> bool {
        session.position() >= self.end_position(session.text().len(), session.words())
            || self.duration_ms().is_some_and(|limit| active_ms >= limit)
    }

    fn sanitized(self) -> Self {
        match self {
            SessionGoal::Duration { minutes } => SessionGoal::Duration {
                minutes: *DURATION_CHOICES
                    .iter()
                    .min_by_key(|&&d| d.abs_diff(minutes))
                    .unwrap_or(&1),
            },
            SessionGoal::Words { count } => SessionGoal::Words {
                count: count.clamp(WORD_COUNT_RANGE.0, WORD_COUNT_RANGE.1),
            },
            SessionGoal::WholeText => SessionGoal::WholeText,
        }
    }
}

/// Réglages d'entraînement d'un profil.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct PracticeSettings {
    /// Taille de police en px (F-01).
    pub font_size: u32,
    /// Lignes visibles (F-03).
    pub visible_lines: u32,
    pub active_line: ActiveLine,
    pub correction: CorrectionMode,
    pub backspace_allowed: bool,
    /// Bandeau MPM / précision / temps / progression (F-05).
    pub show_banner: bool,
    pub lenient_punctuation: bool,
    pub goal: SessionGoal,
}

impl Default for PracticeSettings {
    fn default() -> Self {
        Self {
            font_size: 20,
            visible_lines: 3,
            active_line: ActiveLine::Top,
            correction: CorrectionMode::Free,
            backspace_allowed: true,
            show_banner: true,
            lenient_punctuation: true,
            goal: SessionGoal::WholeText,
        }
    }
}

impl PracticeSettings {
    /// Ramène chaque valeur dans ses bornes.
    pub fn sanitized(self) -> Self {
        Self {
            font_size: self.font_size.clamp(FONT_SIZE_RANGE.0, FONT_SIZE_RANGE.1),
            visible_lines: self
                .visible_lines
                .clamp(VISIBLE_LINES_RANGE.0, VISIBLE_LINES_RANGE.1),
            goal: self.goal.sanitized(),
            ..self
        }
    }

    /// Lit des réglages JSON ; tout champ absent ou invalide prend sa valeur par défaut.
    pub fn from_json(value: Option<&serde_json::Value>) -> Self {
        value
            .and_then(|v| serde_json::from_value::<Self>(v.clone()).ok())
            .unwrap_or_default()
            .sanitized()
    }

    pub fn rules(&self) -> Rules {
        Rules {
            correction: self.correction,
            backspace_allowed: self.backspace_allowed,
            lenient_punctuation: self.lenient_punctuation,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::text::{graphemes, word_spans};
    use crate::typing::{KeyKind, Keystroke};

    #[test]
    fn defaults_follow_the_specification() {
        let s = PracticeSettings::default();
        assert_eq!(s.visible_lines, 3);
        assert_eq!(s.goal, SessionGoal::WholeText);
        assert_eq!(s.rules(), Rules::default());
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s = PracticeSettings::from_json(Some(&json!({ "visibleLines": 5 })));
        assert_eq!(s.visible_lines, 5);
        assert_eq!(s.font_size, 20);
        assert_eq!(
            PracticeSettings::from_json(None),
            PracticeSettings::default()
        );
    }

    #[test]
    fn invalid_json_falls_back_to_defaults() {
        let s = PracticeSettings::from_json(Some(&json!({ "visibleLines": "beaucoup" })));
        assert_eq!(s, PracticeSettings::default());
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let s = PracticeSettings::from_json(Some(&json!({
            "fontSize": 99,
            "visibleLines": 0,
            "goal": { "type": "duration", "minutes": 4 }
        })));
        assert_eq!(s.font_size, 32);
        assert_eq!(s.visible_lines, 1);
        assert_eq!(s.goal, SessionGoal::Duration { minutes: 5 });

        let words = SessionGoal::Words { count: 0 }.sanitized();
        assert_eq!(words, SessionGoal::Words { count: 1 });
    }

    #[test]
    fn goal_serializes_with_type_tag() {
        let json = serde_json::to_value(SessionGoal::Words { count: 50 }).unwrap();
        assert_eq!(json, json!({ "type": "words", "count": 50 }));
        let json = serde_json::to_value(SessionGoal::WholeText).unwrap();
        assert_eq!(json, json!({ "type": "wholeText" }));
    }

    #[test]
    fn end_position_for_each_goal() {
        let text = graphemes("le petit chat");
        let words = word_spans(&text);
        assert_eq!(SessionGoal::WholeText.end_position(13, &words), 13);
        assert_eq!(SessionGoal::Words { count: 2 }.end_position(13, &words), 8);
        assert_eq!(SessionGoal::Words { count: 9 }.end_position(13, &words), 13);
        assert_eq!(
            SessionGoal::Duration { minutes: 1 }.end_position(13, &words),
            13
        );
    }

    #[test]
    fn goal_is_reached_by_position_or_time() {
        let mut session = TypingSession::new(graphemes("ab cd"), Rules::default());
        for (i, c) in ["a", "b"].iter().enumerate() {
            session.apply(&Keystroke {
                seq: i as u32,
                kind: KeyKind::Insert,
                typed: Some((*c).into()),
                t_ms: 0,
            });
        }
        assert!(SessionGoal::Words { count: 1 }.is_reached(&session, 0));
        assert!(!SessionGoal::WholeText.is_reached(&session, 0));
        let timed = SessionGoal::Duration { minutes: 1 };
        assert!(!timed.is_reached(&session, 59_999));
        assert!(timed.is_reached(&session, 60_000));
        assert_eq!(timed.duration_ms(), Some(60_000));
    }
}
