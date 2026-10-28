//! Comparaison frappe / texte attendu et classification des fautes.
//!
//! Le client n'envoie que des frappes brutes (insertion ou retour arrière) ;
//! `TypingSession` les rejoue et décide seul de leur position et de leur
//! justesse. L'interface en possède un miroir pour l'affichage immédiat
//! (ADR 0008), vérifié sur les mêmes cas : `tests/fixtures/typing-cases.json`.

mod errors;

pub use errors::{ErrorType, TypingError, classify};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::text::{WordSpan, is_whitespace, word_spans};

/// Comportement face à une erreur (F-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum CorrectionMode {
    /// Impossible d'avancer tant que l'erreur n'est pas corrigée.
    Blocking,
    /// On avance ; l'erreur reste comptée.
    #[default]
    Free,
}

/// Règles de comparaison d'une session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    pub correction: CorrectionMode,
    pub backspace_allowed: bool,
    /// Assimile apostrophes (’ / '), guillemets (« » / ") et tirets (– / -).
    pub lenient_punctuation: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            correction: CorrectionMode::Free,
            backspace_allowed: true,
            lenient_punctuation: true,
        }
    }
}

/// Nature d'une frappe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum KeyKind {
    Insert,
    Delete,
}

impl KeyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            KeyKind::Insert => "insert",
            KeyKind::Delete => "delete",
        }
    }
}

/// Frappe brute envoyée par l'interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Keystroke {
    /// Numéro d'ordre, croissant à partir de 0.
    pub seq: u32,
    pub kind: KeyKind,
    /// Graphème tapé (insertion uniquement).
    pub typed: Option<String>,
    /// Temps actif écoulé depuis la première frappe, pauses exclues.
    pub t_ms: u32,
}

/// Frappe acceptée et rattachée au texte, telle qu'enregistrée (F-31).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKeystroke {
    pub seq: u32,
    pub position: u32,
    pub kind: KeyKind,
    pub expected: Option<String>,
    pub typed: Option<String>,
    pub t_ms: u32,
    pub correct: bool,
}

/// État d'un caractère du texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Pending,
    Correct,
    /// Juste, après au moins une erreur à cette position.
    Corrected,
    Wrong,
}

impl Cell {
    /// Code d'une lettre, utilisé par les cas de test partagés.
    pub fn code(self) -> char {
        match self {
            Cell::Pending => '.',
            Cell::Correct => 'c',
            Cell::Corrected => 'f',
            Cell::Wrong => 'x',
        }
    }
}

/// Compare deux graphèmes selon les règles : NFC, toute espace vaut toute espace,
/// et équivalences typographiques si `lenient`.
pub fn graphemes_match(expected: &str, typed: &str, lenient: bool) -> bool {
    let typed: String = typed.nfc().collect();
    if expected == typed || (is_whitespace(expected) && is_whitespace(&typed)) {
        return true;
    }
    lenient && fold_punctuation(expected) == fold_punctuation(&typed)
}

fn fold_punctuation(grapheme: &str) -> String {
    grapheme
        .chars()
        .map(|c| match c {
            '\u{2019}' | '\u{2018}' | '\u{02bc}' | '\u{201b}' | '\u{2032}' => '\'',
            '\u{ab}' | '\u{bb}' | '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => '"',
            '\u{2013}' | '\u{2014}' | '\u{2010}' | '\u{2011}' | '\u{2212}' => '-',
            other => other,
        })
        .collect()
}

/// Session de frappe : rejoue les frappes une à une.
#[derive(Debug, Clone)]
pub struct TypingSession {
    text: Vec<String>,
    words: Vec<WordSpan>,
    rules: Rules,
    cells: Vec<Cell>,
    /// Premier graphème erroné tapé à chaque position.
    first_wrong: Vec<Option<String>>,
    position: usize,
    inserts: u32,
    correct_inserts: u32,
    last_seq: Option<u32>,
    last_t_ms: u32,
}

impl TypingSession {
    /// `text` : graphèmes d'un texte normalisé (voir `text::normalize`).
    pub fn new(text: Vec<String>, rules: Rules) -> Self {
        let len = text.len();
        Self {
            words: word_spans(&text),
            text,
            rules,
            cells: vec![Cell::Pending; len],
            first_wrong: vec![None; len],
            position: 0,
            inserts: 0,
            correct_inserts: 0,
            last_seq: None,
            last_t_ms: 0,
        }
    }

    /// Rejoue une suite de frappes ; les frappes refusées sont ignorées.
    pub fn replay<'a>(
        text: Vec<String>,
        rules: Rules,
        keystrokes: impl IntoIterator<Item = &'a Keystroke>,
    ) -> Self {
        let mut session = Self::new(text, rules);
        for k in keystrokes {
            session.apply(k);
        }
        session
    }

    /// Applique une frappe. Renvoie `None` si elle est refusée : numéro déjà vu,
    /// texte terminé, retour arrière interdit ou impossible.
    pub fn apply(&mut self, k: &Keystroke) -> Option<ResolvedKeystroke> {
        if self.last_seq.is_some_and(|last| k.seq <= last) {
            return None;
        }
        let resolved = match k.kind {
            KeyKind::Insert => self.insert(k.typed.as_deref()?)?,
            KeyKind::Delete => self.delete()?,
        };
        self.last_seq = Some(k.seq);
        self.last_t_ms = self.last_t_ms.max(k.t_ms);
        Some(ResolvedKeystroke {
            seq: k.seq,
            t_ms: k.t_ms,
            ..resolved
        })
    }

    fn insert(&mut self, typed: &str) -> Option<ResolvedKeystroke> {
        let typed: String = typed.nfc().collect();
        if typed.graphemes(true).count() != 1 {
            return None;
        }
        let pos = self.position;
        let expected = self.text.get(pos)?.clone();
        let correct = graphemes_match(&expected, &typed, self.rules.lenient_punctuation);

        self.inserts += 1;
        if correct {
            self.correct_inserts += 1;
            self.cells[pos] = if self.first_wrong[pos].is_some() {
                Cell::Corrected
            } else {
                Cell::Correct
            };
            self.position += 1;
        } else {
            self.cells[pos] = Cell::Wrong;
            self.first_wrong[pos].get_or_insert_with(|| typed.clone());
            if self.rules.correction == CorrectionMode::Free {
                self.position += 1;
            }
        }
        Some(resolved(
            pos,
            KeyKind::Insert,
            expected,
            Some(typed),
            correct,
        ))
    }

    fn delete(&mut self) -> Option<ResolvedKeystroke> {
        if !self.rules.backspace_allowed {
            return None;
        }
        let pos = self.position;
        // En mode bloquant, l'erreur se trouve sous le curseur.
        let target = if self.cells.get(pos) == Some(&Cell::Wrong) {
            pos
        } else {
            self.position = pos.checked_sub(1)?;
            self.position
        };
        self.cells[target] = Cell::Pending;
        let expected = self.text[target].clone();
        Some(resolved(target, KeyKind::Delete, expected, None, true))
    }

    pub fn text(&self) -> &[String] {
        &self.text
    }

    pub fn words(&self) -> &[WordSpan] {
        &self.words
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Index du prochain graphème à taper.
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn last_seq(&self) -> Option<u32> {
        self.last_seq
    }

    pub fn last_t_ms(&self) -> u32 {
        self.last_t_ms
    }

    pub fn counters(&self) -> Counters {
        let count = |wanted: &[Cell]| self.cells.iter().filter(|c| wanted.contains(c)).count();
        Counters {
            inserts: self.inserts,
            correct_inserts: self.correct_inserts,
            correct_cells: count(&[Cell::Correct, Cell::Corrected]) as u32,
            uncorrected: count(&[Cell::Wrong]) as u32,
        }
    }

    /// Fautes de la session : une par mot fautif, une par caractère fautif hors mot.
    /// Une faute corrigée compte : le mot a été mal écrit au premier passage.
    pub fn errors(&self) -> Vec<TypingError> {
        let mut errors = Vec::new();
        let mut in_word = vec![false; self.text.len()];
        for span in &self.words {
            let (start, end) = (span.start as usize, span.end as usize);
            in_word[start..end].fill(true);
            let wrong: Vec<usize> = (start..end)
                .filter(|&i| self.first_wrong[i].is_some())
                .collect();
            if wrong.is_empty() {
                continue;
            }
            let types: Vec<ErrorType> = wrong.iter().map(|&i| self.classify_at(i)).collect();
            let typed: String = (start..end)
                .map(|i| self.first_wrong[i].as_deref().unwrap_or(&self.text[i]))
                .collect();
            errors.push(TypingError {
                expected: self.text[start..end].concat(),
                typed,
                error_type: dominant(&types),
                position: span.start,
                word: true,
            });
        }
        for (i, wrong) in self.first_wrong.iter().enumerate() {
            if let (Some(typed), false) = (wrong, in_word[i]) {
                errors.push(TypingError {
                    expected: self.text[i].clone(),
                    typed: typed.clone(),
                    error_type: self.classify_at(i),
                    position: i as u32,
                    word: false,
                });
            }
        }
        errors.sort_by_key(|e| e.position);
        errors
    }

    fn classify_at(&self, i: usize) -> ErrorType {
        classify(&self.text[i], self.first_wrong[i].as_deref().unwrap_or(""))
    }
}

fn resolved(
    position: usize,
    kind: KeyKind,
    expected: String,
    typed: Option<String>,
    correct: bool,
) -> ResolvedKeystroke {
    ResolvedKeystroke {
        seq: 0,
        position: position as u32,
        kind,
        expected: Some(expected),
        typed,
        t_ms: 0,
        correct,
    }
}

/// Type le plus fréquent ; à égalité, le premier rencontré.
fn dominant(types: &[ErrorType]) -> ErrorType {
    let mut best = types[0];
    let mut best_count = 0;
    for &t in types {
        let count = types.iter().filter(|&&o| o == t).count();
        if count > best_count {
            best = t;
            best_count = count;
        }
    }
    best
}

/// Compteurs utilisés par les statistiques.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    /// Insertions acceptées, justes ou non.
    pub inserts: u32,
    pub correct_inserts: u32,
    /// Caractères justes à la fin (corrigés compris).
    pub correct_cells: u32,
    /// Caractères encore faux à la fin.
    pub uncorrected: u32,
}

#[cfg(test)]
mod tests;
