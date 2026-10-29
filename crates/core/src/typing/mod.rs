//! Comparaison frappe / texte attendu et classification des fautes.
//!
//! Le client n'envoie que des frappes brutes (insertion ou retour arrière) ;
//! `TypingSession` les rejoue et décide seul de leur position et de leur
//! justesse. L'interface en possède un miroir pour l'affichage immédiat
//! (ADR 0008), vérifié sur les mêmes cas : `tests/fixtures/typing-cases.json`.

/// Classification des fautes (accent, casse, ponctuation…).
mod errors;
/// Tentatives de mots : forme tapée, justesse et durée (F-40).
mod words;

pub use errors::{ErrorType, TypingError, classify};
pub use words::{DEFAULT_SLOW_FACTOR, SessionWords, WordAttempt};

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
    /// Bloquant ou libre (F-04).
    pub correction: CorrectionMode,
    /// Le retour arrière est-il accepté ? (F-04)
    pub backspace_allowed: bool,
    /// Assimile apostrophes (’ / '), guillemets (« » / ") et tirets (– / -).
    pub lenient_punctuation: bool,
}

impl Default for Rules {
    /// Mode libre, retour arrière autorisé, ponctuation tolérante.
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
    /// Saisie d'un graphème.
    Insert,
    /// Retour arrière.
    Delete,
}

impl KeyKind {
    /// Valeur stockée dans la colonne `keystrokes.kind`.
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
    /// Insertion ou retour arrière.
    pub kind: KeyKind,
    /// Graphème tapé (insertion uniquement).
    pub typed: Option<String>,
    /// Temps actif écoulé depuis la première frappe, pauses exclues.
    pub t_ms: u32,
}

/// Frappe acceptée et rattachée au texte, telle qu'enregistrée (F-31).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKeystroke {
    /// Numéro d'ordre repris de la frappe brute.
    pub seq: u32,
    /// Indice du graphème concerné dans le texte.
    pub position: u32,
    /// Insertion ou retour arrière.
    pub kind: KeyKind,
    /// Graphème attendu à cette position.
    pub expected: Option<String>,
    /// Graphème tapé (insertion uniquement).
    pub typed: Option<String>,
    /// Temps actif de la frappe, en ms.
    pub t_ms: u32,
    /// Frappe juste ? (toujours vrai pour un retour arrière)
    pub correct: bool,
}

/// État d'un caractère du texte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Pas encore tapé.
    Pending,
    /// Juste du premier coup.
    Correct,
    /// Juste, après au moins une erreur à cette position.
    Corrected,
    /// Faux pour l'instant.
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
    // Le texte est déjà en NFC ; la frappe peut arriver décomposée (touche morte).
    let typed: String = typed.nfc().collect();
    // Égalité stricte, ou deux espaces de nature différente (insécable, tabulation…).
    if expected == typed || (is_whitespace(expected) && is_whitespace(&typed)) {
        return true;
    }
    // Sinon, seule la tolérance typographique peut encore rendre la frappe juste.
    lenient && fold_punctuation(expected) == fold_punctuation(&typed)
}

/// Ramène les variantes typographiques à leur forme clavier (’ → ', « → ", – → -).
fn fold_punctuation(grapheme: &str) -> String {
    grapheme
        .chars()
        .map(|c| match c {
            // Apostrophes et primes.
            '\u{2019}' | '\u{2018}' | '\u{02bc}' | '\u{201b}' | '\u{2032}' => '\'',
            // Guillemets français, anglais et doubles primes.
            '\u{ab}' | '\u{bb}' | '\u{201c}' | '\u{201d}' | '\u{201e}' | '\u{2033}' => '"',
            // Tirets et signe moins.
            '\u{2013}' | '\u{2014}' | '\u{2010}' | '\u{2011}' | '\u{2212}' => '-',
            other => other,
        })
        .collect()
}

/// Session de frappe : rejoue les frappes une à une.
#[derive(Debug, Clone)]
pub struct TypingSession {
    /// Graphèmes du texte à taper.
    text: Vec<String>,
    /// Mots du texte, calculés une fois pour toutes.
    words: Vec<WordSpan>,
    /// Règles de comparaison.
    rules: Rules,
    /// État de chaque graphème du texte.
    cells: Vec<Cell>,
    /// Premier graphème erroné tapé à chaque position.
    first_wrong: Vec<Option<String>>,
    /// Temps actif de la dernière insertion à chaque position, en ms.
    insert_t: Vec<Option<u32>>,
    /// Indice du prochain graphème à taper.
    position: usize,
    /// Nombre d'insertions acceptées.
    inserts: u32,
    /// Nombre d'insertions justes.
    correct_inserts: u32,
    /// Numéro de la dernière frappe acceptée.
    last_seq: Option<u32>,
    /// Temps actif le plus grand vu jusqu'ici, en ms.
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
            insert_t: vec![None; len],
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
        // Une frappe déjà vue (lot renvoyé deux fois) est ignorée : idempotence.
        if self.last_seq.is_some_and(|last| k.seq <= last) {
            return None;
        }
        // Une insertion sans graphème est refusée, comme une frappe impossible.
        let resolved = match k.kind {
            KeyKind::Insert => self.insert(k.typed.as_deref()?)?,
            KeyKind::Delete => self.delete()?,
        };
        // La frappe est acceptée : on retient son numéro et son temps.
        self.last_seq = Some(k.seq);
        self.last_t_ms = self.last_t_ms.max(k.t_ms);
        // Une insertion date sa position : la durée des mots en découle.
        if k.kind == KeyKind::Insert {
            self.insert_t[resolved.position as usize] = Some(k.t_ms);
        }
        Some(ResolvedKeystroke {
            seq: k.seq,
            t_ms: k.t_ms,
            ..resolved
        })
    }

    /// Applique une insertion à la position courante.
    fn insert(&mut self, typed: &str) -> Option<ResolvedKeystroke> {
        // Une frappe doit être exactement un graphème une fois composée.
        let typed: String = typed.nfc().collect();
        if typed.graphemes(true).count() != 1 {
            return None;
        }
        // Texte terminé : plus rien à comparer.
        let pos = self.position;
        let expected = self.text.get(pos)?.clone();
        let correct = graphemes_match(&expected, &typed, self.rules.lenient_punctuation);

        self.inserts += 1;
        if correct {
            // Juste : « corrigé » s'il y a eu une erreur ici, sinon « correct ».
            self.correct_inserts += 1;
            self.cells[pos] = if self.first_wrong[pos].is_some() {
                Cell::Corrected
            } else {
                Cell::Correct
            };
            self.position += 1;
        } else {
            // Faux : on garde la première forme fautive pour le relevé des fautes.
            self.cells[pos] = Cell::Wrong;
            self.first_wrong[pos].get_or_insert_with(|| typed.clone());
            // En mode bloquant, le curseur reste sur l'erreur.
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

    /// Applique un retour arrière.
    fn delete(&mut self) -> Option<ResolvedKeystroke> {
        // Retour arrière désactivé par les réglages.
        if !self.rules.backspace_allowed {
            return None;
        }
        let pos = self.position;
        // En mode bloquant, l'erreur se trouve sous le curseur.
        let target = if self.cells.get(pos) == Some(&Cell::Wrong) {
            pos
        } else {
            // Sinon on recule d'un graphème ; impossible au début du texte.
            self.position = pos.checked_sub(1)?;
            self.position
        };
        // Le graphème effacé redevient à taper ; sa première faute reste mémorisée.
        self.cells[target] = Cell::Pending;
        let expected = self.text[target].clone();
        Some(resolved(target, KeyKind::Delete, expected, None, true))
    }

    /// Graphèmes du texte.
    pub fn text(&self) -> &[String] {
        &self.text
    }

    /// Mots du texte.
    pub fn words(&self) -> &[WordSpan] {
        &self.words
    }

    /// Règles de la session.
    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// État de chaque graphème.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Index du prochain graphème à taper.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Numéro de la dernière frappe acceptée.
    pub fn last_seq(&self) -> Option<u32> {
        self.last_seq
    }

    /// Temps actif le plus grand vu jusqu'ici, en ms.
    pub fn last_t_ms(&self) -> u32 {
        self.last_t_ms
    }

    /// Compteurs pour le calcul des statistiques.
    pub fn counters(&self) -> Counters {
        // Nombre de cellules dans l'un des états voulus.
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
        // Marque les positions couvertes par un mot, pour traiter le reste ensuite.
        let mut in_word = vec![false; self.text.len()];
        // Première passe : une faute par mot qui a au moins un caractère fautif.
        for span in &self.words {
            let (start, end) = (span.start as usize, span.end as usize);
            in_word[start..end].fill(true);
            // Un mot juste au premier passage n'est pas une faute.
            let Some(error_type) = self.word_error_type(span) else {
                continue;
            };
            errors.push(TypingError {
                expected: self.text[start..end].concat(),
                typed: self.first_typed(span),
                error_type,
                position: span.start,
                word: true,
            });
        }
        // Seconde passe : caractères fautifs hors mot (espace, ponctuation).
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
        // Les deux passes sont fusionnées dans l'ordre du texte.
        errors.sort_by_key(|e| e.position);
        errors
    }

    /// Type de faute dominant d'un mot, ou `None` s'il a été juste au premier passage.
    fn word_error_type(&self, span: &WordSpan) -> Option<ErrorType> {
        // Le type retenu est le plus fréquent parmi les caractères fautifs.
        let types: Vec<ErrorType> = (span.start as usize..span.end as usize)
            .filter(|&i| self.first_wrong[i].is_some())
            .map(|i| self.classify_at(i))
            .collect();
        dominant_error(&types)
    }

    /// Forme tapée d'un mot : la première faute à chaque position fautive, le texte ailleurs.
    fn first_typed(&self, span: &WordSpan) -> String {
        (span.start as usize..span.end as usize)
            .map(|i| self.first_wrong[i].as_deref().unwrap_or(&self.text[i]))
            .collect()
    }

    /// Classe la première faute faite à la position `i`.
    fn classify_at(&self, i: usize) -> ErrorType {
        classify(&self.text[i], self.first_wrong[i].as_deref().unwrap_or(""))
    }
}

/// Frappe résolue sans numéro ni temps ; `apply` les complète ensuite.
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

/// Type de faute dominant d'une liste (F-41) ; `None` si elle est vide.
pub fn dominant_error(types: &[ErrorType]) -> Option<ErrorType> {
    (!types.is_empty()).then(|| dominant(types))
}

/// Type le plus fréquent ; à égalité, le premier rencontré.
fn dominant(types: &[ErrorType]) -> ErrorType {
    let mut best = types[0];
    let mut best_count = 0;
    // Comparaison stricte : à égalité, le premier type rencontré est gardé.
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
    /// Insertions justes.
    pub correct_inserts: u32,
    /// Caractères justes à la fin (corrigés compris).
    pub correct_cells: u32,
    /// Caractères encore faux à la fin.
    pub uncorrected: u32,
}

/// Tests de la comparaison de frappe (dans `typing/tests.rs`).
#[cfg(test)]
mod tests;
