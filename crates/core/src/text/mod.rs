//! Découpage, nettoyage et normalisation (NFC) du texte.
//!
//! L'unité de position est le **graphème** : `ê` saisi par touche morte
//! compte pour un seul caractère, qu'il soit composé ou non dans la source.

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Normalise un texte avant stockage : NFC, fins de ligne `\n`,
/// espaces de fin de ligne et lignes vides de début/fin supprimés.
pub fn normalize(text: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed: Vec<&str> = unified.lines().map(str::trim_end).collect();
    trimmed.join("\n").trim_matches('\n').nfc().collect()
}

/// Découpe un texte (supposé NFC) en graphèmes étendus.
pub fn graphemes(text: &str) -> Vec<String> {
    text.graphemes(true).map(str::to_owned).collect()
}

/// Espace, tabulation, retour à la ligne ou espace insécable.
pub fn is_whitespace(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace) && !grapheme.is_empty()
}

fn is_word_char(grapheme: &str) -> bool {
    grapheme.chars().next().is_some_and(char::is_alphanumeric)
}

/// Bornes d'un mot dans le texte : `[start, end[` en indices de graphèmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WordSpan {
    pub start: u32,
    pub end: u32,
}

/// Repère les mots : suites de lettres ou chiffres, reliées par un trait d'union
/// interne (« peut-être »). L'apostrophe sépare (« l'école » → « l », « école »).
pub fn word_spans(graphemes: &[String]) -> Vec<WordSpan> {
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (i, g) in graphemes.iter().enumerate() {
        let joins = g == "-"
            && start.is_some()
            && graphemes.get(i + 1).is_some_and(|next| is_word_char(next));
        if is_word_char(g) || joins {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            spans.push(span(s, i));
        }
    }
    if let Some(s) = start {
        spans.push(span(s, graphemes.len()));
    }
    spans
}

fn span(start: usize, end: usize) -> WordSpan {
    WordSpan {
        start: start as u32,
        end: end as u32,
    }
}

/// Coupe le texte en lignes d'au plus `columns` graphèmes, sans couper les mots
/// (police monospace, F-03). Renvoie l'indice du premier graphème de chaque ligne.
///
/// Les espaces ne provoquent jamais de retour : ils restent en fin de ligne.
/// Un mot plus long qu'une ligne est coupé net ; `\n` force un retour.
pub fn wrap(graphemes: &[String], columns: u32) -> Vec<u32> {
    let columns = columns.max(1) as usize;
    let mut starts = vec![0u32];
    let mut line_start = 0usize;
    let mut last_break: Option<usize> = None;
    for (i, g) in graphemes.iter().enumerate() {
        if g == "\n" {
            line_start = i + 1;
            last_break = None;
            if line_start < graphemes.len() {
                starts.push(line_start as u32);
            }
        } else if is_whitespace(g) {
            last_break = Some(i + 1);
        } else if i - line_start >= columns {
            let new_start = last_break.filter(|&b| b > line_start).unwrap_or(i);
            starts.push(new_start as u32);
            line_start = new_start;
            last_break = None;
        }
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(text: &str) -> Vec<String> {
        graphemes(text)
    }

    #[test]
    fn normalize_composes_to_nfc() {
        // « e » + accent circonflexe combinant → « ê » précomposé.
        let decomposed = "for\u{0065}\u{0302}t";
        assert_eq!(normalize(decomposed), "for\u{00ea}t");
    }

    #[test]
    fn normalize_unifies_line_endings_and_trims() {
        assert_eq!(normalize("\r\nab  \r\ncd\t\rde\n\n"), "ab\ncd\nde");
    }

    #[test]
    fn decomposed_accent_is_one_grapheme() {
        assert_eq!(g("e\u{0302}t"), vec!["e\u{0302}", "t"]);
        assert_eq!(g(&normalize("e\u{0302}t")), vec!["ê", "t"]);
    }

    #[test]
    fn whitespace_detection() {
        for s in [" ", "\n", "\t", "\u{a0}", "\u{202f}"] {
            assert!(is_whitespace(s), "{s:?}");
        }
        for s in ["a", "", ",", "'"] {
            assert!(!is_whitespace(s), "{s:?}");
        }
    }

    #[test]
    fn word_spans_split_on_apostrophe_and_punctuation() {
        let text = g("L'école, peut-être !");
        let words: Vec<String> = word_spans(&text)
            .iter()
            .map(|s| text[s.start as usize..s.end as usize].concat())
            .collect();
        assert_eq!(words, ["L", "école", "peut-être"]);
    }

    #[test]
    fn word_spans_handle_edges() {
        assert!(word_spans(&g("")).is_empty());
        assert!(word_spans(&g(" - ! ")).is_empty());
        assert_eq!(word_spans(&g("a-")), vec![span(0, 1)]);
        assert_eq!(word_spans(&g("42")), vec![span(0, 2)]);
    }

    #[test]
    fn wrap_breaks_after_spaces() {
        // « le chat dort » sur 8 colonnes → « le chat » / « dort ».
        assert_eq!(wrap(&g("le chat dort"), 8), vec![0, 8]);
        assert_eq!(wrap(&g("le chat dort"), 7), vec![0, 8]);
        assert_eq!(wrap(&g("le chat dort"), 100), vec![0]);
    }

    #[test]
    fn wrap_cuts_words_longer_than_a_line() {
        assert_eq!(wrap(&g("abcdefgh"), 3), vec![0, 3, 6]);
    }

    #[test]
    fn wrap_honours_newlines() {
        assert_eq!(wrap(&g("ab\ncd\n"), 10), vec![0, 3]);
        assert_eq!(wrap(&g("ab\n\ncd"), 10), vec![0, 3, 4]);
    }

    #[test]
    fn wrap_handles_empty_and_zero_columns() {
        assert_eq!(wrap(&[], 10), vec![0]);
        assert_eq!(wrap(&g("abc"), 0), vec![0, 1, 2]);
    }
}
