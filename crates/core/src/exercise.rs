//! Découpage d'un texte long en exercices (F-24) et progression (ADR 0012).
//!
//! Les exercices ne sont pas stockés : ils sont recalculés à partir du texte et
//! de son découpage. La progression d'un profil retient l'exercice suivant.

use serde::{Deserialize, Serialize};

/// Nombre maximal de paragraphes ou de lignes par exercice.
pub const MAX_PER_EXERCISE: u32 = 100;

/// Découpage d'un texte en exercices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Segmentation {
    /// Un seul exercice : le texte entier.
    #[default]
    Whole,
    /// `count` paragraphes par exercice.
    Paragraphs {
        /// Paragraphes par exercice.
        count: u32,
    },
    /// `count` lignes par exercice.
    Lines {
        /// Lignes par exercice.
        count: u32,
    },
}

impl Segmentation {
    /// Ramène le nombre par exercice entre 1 et `MAX_PER_EXERCISE`.
    pub fn sanitized(self) -> Self {
        let clamp = |count: u32| count.clamp(1, MAX_PER_EXERCISE);
        match self {
            Segmentation::Whole => Segmentation::Whole,
            Segmentation::Paragraphs { count } => Segmentation::Paragraphs {
                count: clamp(count),
            },
            Segmentation::Lines { count } => Segmentation::Lines {
                count: clamp(count),
            },
        }
    }

    /// Lit la colonne JSON `texts.segmentation` ; illisible → texte entier.
    pub fn from_json(raw: &str) -> Self {
        serde_json::from_str::<Self>(raw)
            .unwrap_or_default()
            .sanitized()
    }

    /// Forme JSON stockée dans `texts.segmentation`.
    pub fn to_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| r#"{"type":"whole"}"#.into())
    }
}

/// Portion du texte d'un exercice : `[start, end[` en indices de graphèmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ExerciseSpan {
    /// Premier graphème.
    pub start: u32,
    /// Graphème qui suit le dernier (borne exclue).
    pub end: u32,
}

/// Découpe un texte (en graphèmes) en exercices ; toujours au moins un exercice.
pub fn exercises(text: &[String], segmentation: Segmentation) -> Vec<ExerciseSpan> {
    let whole = vec![ExerciseSpan {
        start: 0,
        end: text.len() as u32,
    }];
    // Lignes du texte (séparées par `\n`), puis blocs selon le découpage choisi.
    let lines = lines(text);
    let (blocks, count) = match segmentation.sanitized() {
        Segmentation::Whole => return whole,
        Segmentation::Lines { count } => (non_empty(text, &lines), count),
        Segmentation::Paragraphs { count } => (paragraphs(text, &lines), count),
    };
    // Texte sans contenu : un seul exercice, vide.
    if blocks.is_empty() {
        return whole;
    }
    // Regroupement de `count` blocs : du début du premier à la fin du dernier.
    blocks
        .chunks(count as usize)
        .map(|chunk| ExerciseSpan {
            start: chunk[0].0 as u32,
            end: chunk[chunk.len() - 1].1 as u32,
        })
        .collect()
}

/// Bornes `[début, fin[` de chaque ligne, `\n` exclu.
fn lines(text: &[String]) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let mut start = 0;
    for (i, g) in text.iter().enumerate() {
        if g == "\n" {
            result.push((start, i));
            start = i + 1;
        }
    }
    // Dernière ligne, sans `\n` final.
    result.push((start, text.len()));
    result
}

/// Une ligne est-elle vide (ou faite seulement d'espaces) ?
fn is_blank(text: &[String], (start, end): (usize, usize)) -> bool {
    text[start..end].iter().all(|g| g.trim().is_empty())
}

/// Lignes non vides.
fn non_empty(text: &[String], lines: &[(usize, usize)]) -> Vec<(usize, usize)> {
    lines
        .iter()
        .copied()
        .filter(|&l| !is_blank(text, l))
        .collect()
}

/// Paragraphes : blocs séparés par une ligne vide si le texte en contient,
/// sinon chaque ligne (texte collé ou PDF reconstitué : un paragraphe par ligne).
fn paragraphs(text: &[String], lines: &[(usize, usize)]) -> Vec<(usize, usize)> {
    // Sans ligne vide entre deux lignes de texte, chaque ligne est un paragraphe.
    let filled = non_empty(text, lines);
    if filled.len() == lines.len() {
        return filled;
    }
    // Sinon, les lignes consécutives non vides forment un bloc.
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut previous_blank = true;
    for &line in lines {
        let blank = is_blank(text, line);
        match blocks.last_mut() {
            Some(block) if !blank && !previous_blank => block.1 = line.1,
            _ if !blank => blocks.push(line),
            _ => {}
        }
        previous_blank = blank;
    }
    blocks
}

/// Exercice à proposer : celui qui suit le dernier réussi, ou le premier une fois
/// le texte terminé.
pub fn next_exercise(progress: u32, count: usize) -> u32 {
    if progress as usize >= count {
        0
    } else {
        progress
    }
}

/// Tests du découpage en exercices.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::graphemes;

    /// Texte de chaque exercice.
    fn split(text: &str, segmentation: Segmentation) -> Vec<String> {
        let g = graphemes(text);
        exercises(&g, segmentation)
            .iter()
            .map(|s| g[s.start as usize..s.end as usize].concat())
            .collect()
    }

    /// Le texte entier est un seul exercice.
    #[test]
    fn whole_text_is_one_exercise() {
        assert_eq!(split("a\nb", Segmentation::Whole), ["a\nb"]);
    }

    /// Par N lignes : les lignes vides sont sautées, le reste est regroupé.
    #[test]
    fn lines_are_grouped_by_count() {
        let text = "un\ndeux\n\ntrois\nquatre\ncinq";
        assert_eq!(
            split(text, Segmentation::Lines { count: 2 }),
            ["un\ndeux", "trois\nquatre", "cinq"]
        );
    }

    /// Par paragraphe : blocs séparés par une ligne vide.
    #[test]
    fn paragraphs_are_separated_by_blank_lines() {
        let text = "Il était\nune fois.\n\nUn loup.\n\n\nFin.";
        assert_eq!(
            split(text, Segmentation::Paragraphs { count: 1 }),
            ["Il était\nune fois.", "Un loup.", "Fin."]
        );
        assert_eq!(
            split(text, Segmentation::Paragraphs { count: 2 }),
            ["Il était\nune fois.\n\nUn loup.", "Fin."]
        );
    }

    /// Sans ligne vide, chaque ligne est un paragraphe (PDF reconstitué).
    #[test]
    fn without_blank_lines_each_line_is_a_paragraph() {
        assert_eq!(
            split("Premier.\nSecond.", Segmentation::Paragraphs { count: 1 }),
            ["Premier.", "Second."]
        );
    }

    /// Texte vide : un exercice vide ; nombre par exercice borné.
    #[test]
    fn edge_cases() {
        assert_eq!(
            exercises(&[], Segmentation::Lines { count: 3 }),
            [ExerciseSpan { start: 0, end: 0 }]
        );
        assert_eq!(
            Segmentation::Lines { count: 0 }.sanitized(),
            Segmentation::Lines { count: 1 }
        );
        assert_eq!(
            Segmentation::Paragraphs { count: 999 }.sanitized(),
            Segmentation::Paragraphs {
                count: MAX_PER_EXERCISE
            }
        );
    }

    /// Lecture et écriture de la colonne JSON.
    #[test]
    fn json_round_trip() {
        let s = Segmentation::Paragraphs { count: 2 };
        assert_eq!(s.to_json(), r#"{"type":"paragraphs","count":2}"#);
        assert_eq!(Segmentation::from_json(&s.to_json()), s);
        assert_eq!(Segmentation::from_json("pas du json"), Segmentation::Whole);
    }

    /// Après le dernier exercice, on revient au premier.
    #[test]
    fn next_exercise_wraps_around() {
        assert_eq!(next_exercise(0, 8), 0);
        assert_eq!(next_exercise(3, 8), 3);
        assert_eq!(next_exercise(8, 8), 0);
        assert_eq!(next_exercise(12, 8), 0);
    }
}
