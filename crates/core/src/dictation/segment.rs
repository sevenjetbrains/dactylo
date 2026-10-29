//! Découpage du texte en segments lus l'un après l'autre (F-11).

use serde::{Deserialize, Serialize};

use super::settings::SegmentMode;
use crate::text::{WordSpan, word_spans};

/// Un segment de la dictée : une portion du texte et son nombre de mots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// Premier graphème du segment.
    pub start: u32,
    /// Graphème qui suit le dernier (borne exclue).
    pub end: u32,
    /// Mots du segment.
    pub words: u32,
    /// Texte du segment, espaces de bord retirées.
    pub text: String,
}

/// Fin de phrase entre deux mots : ponctuation finale ou retour à la ligne.
fn ends_sentence(gap: &str) -> bool {
    gap.contains(['.', '!', '?', '…', '\n'])
}

/// Coupe le texte (graphèmes NFC) en segments selon le mode. Chaque segment commence
/// à un mot et garde la ponctuation qui suit ses mots ; le premier inclut ce qui
/// précède le premier mot (guillemet ouvrant…). Un texte sans mot n'a aucun segment.
pub fn segments(graphemes: &[String], mode: SegmentMode, group_size: u32) -> Vec<Segment> {
    let spans = word_spans(graphemes);
    if spans.is_empty() {
        return Vec::new();
    }
    // Texte entre la fin d'un mot et le début du suivant.
    let gap = |a: &WordSpan, b: &WordSpan| graphemes[a.end as usize..b.start as usize].concat();
    // Paragraphes : séparés par une ligne vide, ou à défaut par chaque retour à la ligne.
    let blank_lines = graphemes.concat().contains("\n\n");
    let mut starts = vec![0usize];
    for k in 1..spans.len() {
        let between = gap(&spans[k - 1], &spans[k]);
        // Le mot `k` ouvre-t-il un nouveau segment ?
        let cut = match mode {
            SegmentMode::Word => true,
            SegmentMode::Group => {
                (k - starts.last().copied().unwrap_or(0)) >= group_size.max(1) as usize
            }
            SegmentMode::Sentence => ends_sentence(&between),
            SegmentMode::Paragraph if blank_lines => between.contains("\n\n"),
            SegmentMode::Paragraph => between.contains('\n'),
        };
        if cut {
            starts.push(k);
        }
    }
    // Chaque segment va du début de son premier mot au début du segment suivant.
    starts
        .iter()
        .enumerate()
        .map(|(n, &first)| {
            let next = starts.get(n + 1).copied();
            let start = if n == 0 {
                0
            } else {
                spans[first].start as usize
            };
            let end = next.map_or(graphemes.len(), |k| spans[k].start as usize);
            let words = next.unwrap_or(spans.len()) - first;
            Segment {
                start: start as u32,
                end: end as u32,
                words: words as u32,
                text: graphemes[start..end].concat().trim().to_owned(),
            }
        })
        .collect()
}

/// Tests du découpage en segments.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{graphemes, normalize};

    /// Textes des segments d'un texte.
    fn texts(text: &str, mode: SegmentMode, size: u32) -> Vec<String> {
        segments(&graphemes(&normalize(text)), mode, size)
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    /// Par mot : la ponctuation reste avec le mot qui la précède.
    #[test]
    fn by_word() {
        assert_eq!(
            texts("« Oui, dit-il. »", SegmentMode::Word, 4),
            ["« Oui,", "dit-il. »"]
        );
    }

    /// Par groupe de N mots ; le dernier groupe peut être plus court.
    #[test]
    fn by_group() {
        let text = "un deux trois quatre cinq six sept huit neuf";
        assert_eq!(
            texts(text, SegmentMode::Group, 4),
            ["un deux trois quatre", "cinq six sept huit", "neuf"]
        );
        let s = segments(&graphemes(text), SegmentMode::Group, 4);
        assert_eq!(s.iter().map(|s| s.words).collect::<Vec<_>>(), [4, 4, 1]);
        // Les segments couvrent tout le texte, bout à bout.
        assert_eq!(s[0].start, 0);
        assert_eq!(s[1].start, s[0].end);
        assert_eq!(s[2].end as usize, graphemes(text).len());
    }

    /// Par phrase : ponctuation finale ou retour à la ligne.
    #[test]
    fn by_sentence() {
        assert_eq!(
            texts("Il pleut. Tu viens ? Oui !\nFin", SegmentMode::Sentence, 4),
            ["Il pleut.", "Tu viens ?", "Oui !", "Fin"]
        );
    }

    /// Par paragraphe : lignes vides, ou retours à la ligne s'il n'y en a pas.
    #[test]
    fn by_paragraph() {
        assert_eq!(
            texts("Un. Deux.\nSuite.\n\nTrois.", SegmentMode::Paragraph, 4),
            ["Un. Deux.\nSuite.", "Trois."]
        );
        assert_eq!(
            texts("Un.\nDeux.", SegmentMode::Paragraph, 4),
            ["Un.", "Deux."]
        );
    }

    /// Un texte sans mot n'a pas de segment.
    #[test]
    fn no_words_no_segment() {
        assert!(segments(&graphemes("… !"), SegmentMode::Group, 4).is_empty());
    }
}
