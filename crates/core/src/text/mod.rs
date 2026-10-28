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
    // Fins de ligne Windows (`\r\n`) et Mac anciennes (`\r`) ramenées à `\n`.
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    // Espaces de fin de chaque ligne retirés.
    let trimmed: Vec<&str> = unified.lines().map(str::trim_end).collect();
    // Lignes vides aux extrémités retirées, puis composition NFC.
    trimmed.join("\n").trim_matches('\n').nfc().collect()
}

/// Découpe un texte (supposé NFC) en graphèmes étendus.
pub fn graphemes(text: &str) -> Vec<String> {
    text.graphemes(true).map(str::to_owned).collect()
}

/// Espace, tabulation, retour à la ligne ou espace insécable.
pub fn is_whitespace(grapheme: &str) -> bool {
    // `all` est vrai sur une chaîne vide : on l'exclut explicitement.
    grapheme.chars().all(char::is_whitespace) && !grapheme.is_empty()
}

/// Le graphème commence-t-il par une lettre ou un chiffre ?
fn is_word_char(grapheme: &str) -> bool {
    grapheme.chars().next().is_some_and(char::is_alphanumeric)
}

/// Bornes d'un mot dans le texte : `[start, end[` en indices de graphèmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WordSpan {
    /// Indice du premier graphème du mot.
    pub start: u32,
    /// Indice qui suit le dernier graphème du mot (borne exclue).
    pub end: u32,
}

/// Repère les mots : suites de lettres ou chiffres, reliées par un trait d'union
/// interne (« peut-être »). L'apostrophe sépare (« l'école » → « l », « école »).
pub fn word_spans(graphemes: &[String]) -> Vec<WordSpan> {
    let mut spans = Vec::new();
    // Début du mot en cours, s'il y en a un.
    let mut start: Option<usize> = None;
    for (i, g) in graphemes.iter().enumerate() {
        // Un trait d'union ne relie que s'il est entre deux caractères de mot.
        let joins = g == "-"
            && start.is_some()
            && graphemes.get(i + 1).is_some_and(|next| is_word_char(next));
        if is_word_char(g) || joins {
            // Dans un mot : on ouvre le mot s'il ne l'est pas déjà.
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            // Hors d'un mot : on ferme le mot en cours.
            spans.push(span(s, i));
        }
    }
    // Un mot qui va jusqu'à la fin du texte n'a pas encore été fermé.
    if let Some(s) = start {
        spans.push(span(s, graphemes.len()));
    }
    spans
}

/// Construit un `WordSpan` à partir d'indices `usize`.
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
    // Au moins une colonne, pour toujours avancer.
    let columns = columns.max(1) as usize;
    let mut starts = vec![0u32];
    let mut line_start = 0usize;
    // Position juste après le dernier espace de la ligne : meilleur point de coupure.
    let mut last_break: Option<usize> = None;
    for (i, g) in graphemes.iter().enumerate() {
        if g == "\n" {
            // Retour forcé : la ligne suivante commence après le `\n`,
            // sauf si le texte s'arrête là.
            line_start = i + 1;
            last_break = None;
            if line_start < graphemes.len() {
                starts.push(line_start as u32);
            }
        } else if is_whitespace(g) {
            // Un espace reste sur la ligne mais marque un point de coupure.
            last_break = Some(i + 1);
        } else if i - line_start >= columns {
            // Ligne pleine : on coupe après le dernier espace, sinon en plein mot.
            let new_start = last_break.filter(|&b| b > line_start).unwrap_or(i);
            starts.push(new_start as u32);
            line_start = new_start;
            last_break = None;
        }
    }
    starts
}

/// Tests du découpage et de la normalisation.
#[cfg(test)]
mod tests {
    use super::*;

    /// Raccourci pour `graphemes`.
    fn g(text: &str) -> Vec<String> {
        graphemes(text)
    }

    /// Un accent combinant est composé en caractère précomposé.
    #[test]
    fn normalize_composes_to_nfc() {
        // « e » + accent circonflexe combinant → « ê » précomposé.
        let decomposed = "for\u{0065}\u{0302}t";
        assert_eq!(normalize(decomposed), "for\u{00ea}t");
    }

    /// Les fins de ligne sont unifiées et les espaces superflus retirés.
    #[test]
    fn normalize_unifies_line_endings_and_trims() {
        assert_eq!(normalize("\r\nab  \r\ncd\t\rde\n\n"), "ab\ncd\nde");
    }

    /// Un accent décomposé forme un seul graphème, avant comme après normalisation.
    #[test]
    fn decomposed_accent_is_one_grapheme() {
        assert_eq!(g("e\u{0302}t"), vec!["e\u{0302}", "t"]);
        assert_eq!(g(&normalize("e\u{0302}t")), vec!["ê", "t"]);
    }

    /// Les espaces, y compris insécables, sont reconnus ; le reste non.
    #[test]
    fn whitespace_detection() {
        // Espace, retour, tabulation, insécable, insécable fine.
        for s in [" ", "\n", "\t", "\u{a0}", "\u{202f}"] {
            assert!(is_whitespace(s), "{s:?}");
        }
        // Lettre, chaîne vide et ponctuation.
        for s in ["a", "", ",", "'"] {
            assert!(!is_whitespace(s), "{s:?}");
        }
    }

    /// L'apostrophe et la ponctuation séparent les mots, pas le trait d'union.
    #[test]
    fn word_spans_split_on_apostrophe_and_punctuation() {
        let text = g("L'école, peut-être !");
        // Texte de chaque mot repéré.
        let words: Vec<String> = word_spans(&text)
            .iter()
            .map(|s| text[s.start as usize..s.end as usize].concat())
            .collect();
        assert_eq!(words, ["L", "école", "peut-être"]);
    }

    /// Cas limites : texte vide, sans mot, trait d'union final, chiffres.
    #[test]
    fn word_spans_handle_edges() {
        assert!(word_spans(&g("")).is_empty());
        assert!(word_spans(&g(" - ! ")).is_empty());
        assert_eq!(word_spans(&g("a-")), vec![span(0, 1)]);
        assert_eq!(word_spans(&g("42")), vec![span(0, 2)]);
    }

    /// La coupure se fait après les espaces, jamais au milieu d'un mot court.
    #[test]
    fn wrap_breaks_after_spaces() {
        // « le chat dort » sur 8 colonnes → « le chat » / « dort ».
        assert_eq!(wrap(&g("le chat dort"), 8), vec![0, 8]);
        // Sur 7 colonnes, l'espace final déborde sans provoquer de retour.
        assert_eq!(wrap(&g("le chat dort"), 7), vec![0, 8]);
        // Assez de place : une seule ligne.
        assert_eq!(wrap(&g("le chat dort"), 100), vec![0]);
    }

    /// Un mot plus long qu'une ligne est coupé net.
    #[test]
    fn wrap_cuts_words_longer_than_a_line() {
        assert_eq!(wrap(&g("abcdefgh"), 3), vec![0, 3, 6]);
    }

    /// Les `\n` forcent un retour ; un `\n` final n'ouvre pas de ligne vide.
    #[test]
    fn wrap_honours_newlines() {
        assert_eq!(wrap(&g("ab\ncd\n"), 10), vec![0, 3]);
        assert_eq!(wrap(&g("ab\n\ncd"), 10), vec![0, 3, 4]);
    }

    /// Texte vide et largeur nulle ne bouclent pas et donnent un résultat sûr.
    #[test]
    fn wrap_handles_empty_and_zero_columns() {
        assert_eq!(wrap(&[], 10), vec![0]);
        assert_eq!(wrap(&g("abc"), 0), vec![0, 1, 2]);
    }
}
