//! Réglages du mode Copie (F-01 à F-06) : valeurs par défaut et bornes.
//!
//! Stockés sous la clé `practice` du JSON `profiles.settings` (ADR 0010) ;
//! une valeur absente prend sa valeur par défaut, une valeur hors bornes
//! est ramenée dans ses bornes.

use serde::{Deserialize, Serialize};

use crate::text::WordSpan;
use crate::typing::{CorrectionMode, Rules, TypingSession};

/// Bornes de la taille de police, en px (F-01).
pub const FONT_SIZE_RANGE: (u32, u32) = (14, 32);
/// Bornes du nombre de lignes visibles (F-03).
pub const VISIBLE_LINES_RANGE: (u32, u32) = (1, 10);
/// Durées proposées pour une session chronométrée (F-06).
pub const DURATION_CHOICES: [u32; 4] = [1, 2, 5, 10];
/// Bornes du nombre de mots d'une session « nombre de mots fixe » (F-06).
pub const WORD_COUNT_RANGE: (u32, u32) = (1, 10_000);

/// Position de la ligne active dans la zone de frappe (F-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ActiveLine {
    /// La ligne en cours reste en haut de la zone.
    #[default]
    Top,
    /// La ligne en cours reste au milieu de la zone.
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
    Duration {
        /// Durée de la session, en minutes.
        minutes: u32,
    },
    /// Nombre de mots fixe.
    Words {
        /// Nombre de mots à taper.
        count: u32,
    },
}

impl SessionGoal {
    /// Position à atteindre : fin du texte, ou fin du N-ième mot.
    pub fn end_position(&self, text_len: usize, words: &[WordSpan]) -> usize {
        match *self {
            // Fin du N-ième mot ; si le texte en a moins, fin du texte.
            SessionGoal::Words { count } => (count as usize)
                .checked_sub(1)
                .and_then(|i| words.get(i))
                .map_or(text_len, |w| w.end as usize),
            // Les autres types de session vont jusqu'au bout du texte.
            _ => text_len,
        }
    }

    /// Durée limite en ms, pour une session chronométrée seulement.
    pub fn duration_ms(&self) -> Option<u32> {
        match *self {
            SessionGoal::Duration { minutes } => Some(minutes * 60_000),
            _ => None,
        }
    }

    /// La session est-elle terminée ? (temps actif en ms)
    pub fn is_reached(&self, session: &TypingSession, active_ms: u32) -> bool {
        // Terminée si la position cible est atteinte ou si le temps est écoulé.
        session.position() >= self.end_position(session.text().len(), session.words())
            || self.duration_ms().is_some_and(|limit| active_ms >= limit)
    }

    /// Ramène l'objectif dans les valeurs autorisées.
    fn sanitized(self) -> Self {
        match self {
            // Une durée libre est remplacée par la durée proposée la plus proche.
            SessionGoal::Duration { minutes } => SessionGoal::Duration {
                minutes: *DURATION_CHOICES
                    .iter()
                    .min_by_key(|&&d| d.abs_diff(minutes))
                    .unwrap_or(&1),
            },
            // Un nombre de mots est borné.
            SessionGoal::Words { count } => SessionGoal::Words {
                count: count.clamp(WORD_COUNT_RANGE.0, WORD_COUNT_RANGE.1),
            },
            // Rien à corriger pour le texte entier.
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
    /// Position de la ligne active, en haut ou au centre (F-03).
    pub active_line: ActiveLine,
    /// Mode de correction, bloquant ou libre (F-04).
    pub correction: CorrectionMode,
    /// Retour arrière autorisé ou non (F-04).
    pub backspace_allowed: bool,
    /// Bandeau MPM / précision / temps / progression (F-05).
    pub show_banner: bool,
    /// Tolère les variantes typographiques (apostrophe courbe, espace insécable…).
    pub lenient_punctuation: bool,
    /// Type de session (F-06).
    pub goal: SessionGoal,
}

impl Default for PracticeSettings {
    /// Valeurs par défaut du cahier des charges.
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
        // Seules les valeurs numériques et l'objectif ont des bornes ; le reste est repris tel quel.
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
        // Un JSON illisible donne les réglages par défaut plutôt qu'une erreur.
        value
            .and_then(|v| serde_json::from_value::<Self>(v.clone()).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Règles de comparaison de frappe qui découlent de ces réglages.
    pub fn rules(&self) -> Rules {
        Rules {
            correction: self.correction,
            backspace_allowed: self.backspace_allowed,
            lenient_punctuation: self.lenient_punctuation,
        }
    }
}

/// Tests des réglages et des objectifs de session.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::text::{graphemes, word_spans};
    use crate::typing::{KeyKind, Keystroke};

    /// Les valeurs par défaut sont celles du cahier des charges.
    #[test]
    fn defaults_follow_the_specification() {
        let s = PracticeSettings::default();
        assert_eq!(s.visible_lines, 3);
        assert_eq!(s.goal, SessionGoal::WholeText);
        assert_eq!(s.rules(), Rules::default());
    }

    /// Un champ absent du JSON prend sa valeur par défaut.
    #[test]
    fn missing_fields_take_defaults() {
        // JSON partiel : seul le nombre de lignes est fourni.
        let s = PracticeSettings::from_json(Some(&json!({ "visibleLines": 5 })));
        assert_eq!(s.visible_lines, 5);
        assert_eq!(s.font_size, 20);
        // Aucun JSON : tous les réglages par défaut.
        assert_eq!(
            PracticeSettings::from_json(None),
            PracticeSettings::default()
        );
    }

    /// Un JSON mal typé donne les réglages par défaut.
    #[test]
    fn invalid_json_falls_back_to_defaults() {
        let s = PracticeSettings::from_json(Some(&json!({ "visibleLines": "beaucoup" })));
        assert_eq!(s, PracticeSettings::default());
    }

    /// Les valeurs hors bornes sont ramenées dans leurs bornes.
    #[test]
    fn out_of_range_values_are_clamped() {
        // Police trop grande, zéro ligne et durée non proposée.
        let s = PracticeSettings::from_json(Some(&json!({
            "fontSize": 99,
            "visibleLines": 0,
            "goal": { "type": "duration", "minutes": 4 }
        })));
        assert_eq!(s.font_size, 32);
        assert_eq!(s.visible_lines, 1);
        assert_eq!(s.goal, SessionGoal::Duration { minutes: 5 });

        // Zéro mot devient un mot.
        let words = SessionGoal::Words { count: 0 }.sanitized();
        assert_eq!(words, SessionGoal::Words { count: 1 });
    }

    /// L'objectif est sérialisé avec une étiquette `type`.
    #[test]
    fn goal_serializes_with_type_tag() {
        let json = serde_json::to_value(SessionGoal::Words { count: 50 }).unwrap();
        assert_eq!(json, json!({ "type": "words", "count": 50 }));
        let json = serde_json::to_value(SessionGoal::WholeText).unwrap();
        assert_eq!(json, json!({ "type": "wholeText" }));
    }

    /// Position de fin pour chaque type de session.
    #[test]
    fn end_position_for_each_goal() {
        let text = graphemes("le petit chat");
        let words = word_spans(&text);
        // Texte entier : fin du texte.
        assert_eq!(SessionGoal::WholeText.end_position(13, &words), 13);
        // Deux mots : fin de « petit ».
        assert_eq!(SessionGoal::Words { count: 2 }.end_position(13, &words), 8);
        // Plus de mots que le texte n'en a : fin du texte.
        assert_eq!(SessionGoal::Words { count: 9 }.end_position(13, &words), 13);
        // Session chronométrée : fin du texte.
        assert_eq!(
            SessionGoal::Duration { minutes: 1 }.end_position(13, &words),
            13
        );
    }

    /// L'objectif est atteint par la position ou par le temps écoulé.
    #[test]
    fn goal_is_reached_by_position_or_time() {
        // On tape le premier mot « ab ».
        let mut session = TypingSession::new(graphemes("ab cd"), Rules::default());
        for (i, c) in ["a", "b"].iter().enumerate() {
            session.apply(&Keystroke {
                seq: i as u32,
                kind: KeyKind::Insert,
                typed: Some((*c).into()),
                t_ms: 0,
            });
        }
        // Un mot suffit pour l'objectif « 1 mot », pas pour le texte entier.
        assert!(SessionGoal::Words { count: 1 }.is_reached(&session, 0));
        assert!(!SessionGoal::WholeText.is_reached(&session, 0));
        // La session d'une minute se termine exactement à 60 000 ms.
        let timed = SessionGoal::Duration { minutes: 1 };
        assert!(!timed.is_reached(&session, 59_999));
        assert!(timed.is_reached(&session, 60_000));
        assert_eq!(timed.duration_ms(), Some(60_000));
    }
}
