//! Avancement d'un segment au suivant (F-15) : adaptatif (selon les mots tapés)
//! ou automatique (après la pause).

use super::settings::{Advance, DictationSettings};
use crate::text::{graphemes, normalize, word_spans};

/// Nombre de mots dans le texte tapé.
pub fn typed_words(typed: &str) -> u32 {
    word_spans(&graphemes(&normalize(typed))).len() as u32
}

/// Avec l'avancement adaptatif, le segment qui suit `current` est-il dû ? Oui quand
/// l'élève a tapé au moins autant de mots que les segments lus jusqu'ici.
pub fn adaptive_ready(typed_words: u32, segment_words: &[u32], current: usize) -> bool {
    let heard: u32 = segment_words.iter().take(current + 1).sum();
    typed_words >= heard
}

/// Délai entre la fin de lecture d'un segment et la lecture du suivant : la pause
/// pour l'avancement automatique ; `None` pour l'adaptatif, qui attend la frappe.
pub fn automatic_delay_ms(settings: &DictationSettings) -> Option<u32> {
    (settings.advance == Advance::Automatic).then_some(settings.pause_ms)
}

/// Tests de l'avancement.
#[cfg(test)]
mod tests {
    use super::*;

    /// Les mots tapés sont comptés comme dans le texte (apostrophe et trait d'union).
    #[test]
    fn counts_typed_words() {
        assert_eq!(typed_words("l'été, peut-être"), 3);
        assert_eq!(typed_words("  "), 0);
    }

    /// Adaptatif : le segment suivant attend les mots de tous les segments lus.
    #[test]
    fn adaptive_waits_for_heard_words() {
        let words = [4, 4, 2];
        assert!(!adaptive_ready(3, &words, 0));
        assert!(adaptive_ready(4, &words, 0));
        assert!(!adaptive_ready(7, &words, 1));
        assert!(adaptive_ready(9, &words, 1));
    }

    /// Automatique : la pause réglée ; adaptatif : pas de délai fixe.
    #[test]
    fn automatic_delay() {
        let auto = DictationSettings {
            advance: Advance::Automatic,
            pause_ms: 2_000,
            ..DictationSettings::default()
        };
        assert_eq!(automatic_delay_ms(&auto), Some(2_000));
        assert_eq!(automatic_delay_ms(&DictationSettings::default()), None);
    }
}
