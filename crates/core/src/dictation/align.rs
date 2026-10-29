//! Correction d'une dictée (F-16, ADR 0021) : alignement mot à mot du texte tapé
//! sur le texte attendu, par distance d'édition, puis classement de chaque faute.

use serde::{Deserialize, Serialize};

use crate::text::{graphemes, is_whitespace, normalize, word_spans};
use crate::typing::{ErrorType, base_letters, graphemes_match};

/// Coût d'un jeton identique.
const MATCH: u32 = 0;
/// Coût d'un mot à une faute près, ou d'une ponctuation remplacée par une autre.
const NEAR: u32 = 1;
/// Coût d'un mot remplacé par un mot sans rapport.
const FAR: u32 = 3;
/// Coût d'un jeton oublié ou ajouté.
const GAP: u32 = 2;
/// Coût d'une fusion ou d'une scission (« soixante quinze » / « soixante-quinze »).
const JOIN: u32 = 1;
/// Coût interdit (un mot ne s'aligne pas sur une ponctuation).
const INF: u32 = u32::MAX / 4;
/// Nombre maximal de mots fusionnés ou scindés en une opération.
const MAX_JOIN: usize = 3;

/// Jeton aligné : un mot ou un signe de ponctuation, attendu et/ou tapé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct AlignedToken {
    /// Texte attendu ; `None` pour un jeton tapé en trop.
    pub expected: Option<String>,
    /// Texte tapé ; `None` pour un jeton oublié.
    pub typed: Option<String>,
    /// Mot (sinon ponctuation).
    pub word: bool,
    /// Faute, ou `None` si le jeton est juste.
    pub error: Option<ErrorType>,
    /// Index du premier graphème attendu, s'il y en a un.
    pub position: Option<u32>,
    /// Une espace précède-t-elle ce jeton dans le texte (pour l'affichage) ?
    pub space_before: bool,
}

/// Nombre de fautes d'un type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TypeCount {
    /// Type de faute.
    pub error_type: ErrorType,
    /// Nombre de fautes de ce type.
    pub count: u32,
}

/// Correction complète d'une dictée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Correction {
    /// Jetons dans l'ordre du texte.
    pub tokens: Vec<AlignedToken>,
    /// Mots du texte attendu.
    pub expected_words: u32,
    /// Mots attendus écrits juste.
    pub correct_words: u32,
    /// Fautes, ponctuation comprise.
    pub faults: u32,
    /// Fautes par type, dans un ordre fixe, types sans faute compris.
    pub by_type: Vec<TypeCount>,
}

/// Ordre d'affichage des types de faute de dictée.
const TYPE_ORDER: [ErrorType; 9] = [
    ErrorType::Accent,
    ErrorType::Case,
    ErrorType::MissingLetter,
    ErrorType::ExtraLetter,
    ErrorType::Punctuation,
    ErrorType::Transposition,
    ErrorType::MissingWord,
    ErrorType::ExtraWord,
    ErrorType::Substitution,
];

/// Jeton d'un texte avant alignement.
#[derive(Debug, Clone)]
struct Tok {
    /// Texte du jeton.
    text: String,
    /// Mot (sinon ponctuation).
    word: bool,
    /// Index du premier graphème.
    pos: u32,
    /// Précédé d'une espace ?
    space_before: bool,
}

/// Découpe un texte en mots (`word_spans`) et en signes de ponctuation isolés ;
/// les espaces séparent les jetons sans en être.
fn tokenize(text: &str) -> Vec<Tok> {
    let g = graphemes(&normalize(text));
    let spans = word_spans(&g);
    let mut tokens = Vec::new();
    let mut next_span = spans.iter().peekable();
    let mut space = false;
    let mut i = 0;
    while i < g.len() {
        // Début d'un mot : on le prend en entier.
        if let Some(span) = next_span.next_if(|s| s.start as usize == i) {
            tokens.push(Tok {
                text: g[span.start as usize..span.end as usize].concat(),
                word: true,
                pos: span.start,
                space_before: space,
            });
            space = false;
            i = span.end as usize;
            continue;
        }
        // Une espace ne fait que marquer le jeton suivant ; le reste est ponctuation.
        if is_whitespace(&g[i]) {
            space = true;
        } else {
            tokens.push(Tok {
                text: g[i].clone(),
                word: false,
                pos: i as u32,
                space_before: space,
            });
            space = false;
        }
        i += 1;
    }
    tokens
}

/// Deux textes égaux graphème à graphème, avec la tolérance typographique au choix.
fn same(a: &str, b: &str, lenient: bool) -> bool {
    let (ga, gb) = (graphemes(a), graphemes(b));
    ga.len() == gb.len()
        && ga
            .iter()
            .zip(&gb)
            .all(|(x, y)| graphemes_match(x, y, lenient))
}

/// Classe la faute entre un mot attendu et un mot tapé différents.
pub fn classify_word(expected: &str, typed: &str) -> ErrorType {
    // Casse d'abord, puis accents : ce sont les fautes les plus fines.
    if expected.to_lowercase() == typed.to_lowercase() {
        return ErrorType::Case;
    }
    if base_letters(expected) == base_letters(typed) {
        return ErrorType::Accent;
    }
    // Comparaison lettre à lettre, casse ignorée, accents gardés.
    let a: Vec<char> = expected.to_lowercase().chars().collect();
    let b: Vec<char> = typed.to_lowercase().chars().collect();
    if a.len() == b.len() {
        let diffs: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        if let [i, j] = diffs[..]
            && j == i + 1
            && a[i] == b[j]
            && a[j] == b[i]
        {
            return ErrorType::Transposition;
        }
    }
    if b.len() + 1 == a.len() && one_deletion(&a, &b) {
        return ErrorType::MissingLetter;
    }
    if a.len() + 1 == b.len() && one_deletion(&b, &a) {
        return ErrorType::ExtraLetter;
    }
    ErrorType::Substitution
}

/// `short` s'obtient-il en retirant exactement une lettre de `long` ?
fn one_deletion(long: &[char], short: &[char]) -> bool {
    // Premier écart, puis le reste doit coïncider avec un décalage d'une lettre.
    let first = long
        .iter()
        .zip(short)
        .position(|(x, y)| x != y)
        .unwrap_or(short.len());
    long[first + 1..] == short[first..]
}

/// Le mot tapé est-il « proche » de l'attendu (une seule faute fine) ?
fn is_near(expected: &str, typed: &str) -> bool {
    match classify_word(expected, typed) {
        ErrorType::Substitution => {
            // Une seule lettre remplacée, même longueur : faute de frappe, mot reconnu.
            let (a, b): (Vec<char>, Vec<char>) =
                (expected.chars().collect(), typed.chars().collect());
            a.len() == b.len() && a.iter().zip(&b).filter(|(x, y)| x != y).count() == 1
        }
        _ => true,
    }
}

/// Lettres d'une suite de mots, traits d'union retirés (comparaison des fusions).
fn letters<'a>(words: impl Iterator<Item = &'a str>) -> String {
    words
        .flat_map(|w| w.chars())
        .filter(|&c| c != '-')
        .collect()
}

/// Opération retenue pour atteindre une case de la table d'alignement.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    /// Case de départ.
    Start,
    /// Un attendu face à un tapé (juste ou faux).
    Pair,
    /// Attendu oublié.
    Missing,
    /// Tapé en trop.
    Extra,
    /// Un mot attendu tapé en `k` morceaux.
    Merge(usize),
    /// `k` mots attendus tapés d'un seul tenant.
    Split(usize),
}

/// Coût d'appariement d'un jeton attendu et d'un jeton tapé.
fn pair_cost(e: &Tok, t: &Tok, lenient: bool) -> u32 {
    match (e.word, t.word) {
        // Un mot ne s'aligne jamais sur une ponctuation.
        (true, false) | (false, true) => INF,
        _ if same(&e.text, &t.text, lenient) => MATCH,
        (false, false) => NEAR,
        (true, true) if is_near(&e.text, &t.text) => NEAR,
        (true, true) => FAR,
    }
}

/// Aligne le texte tapé sur le texte attendu et classe chaque faute (F-16).
pub fn correct(expected: &str, typed: &str, lenient: bool) -> Correction {
    let e = tokenize(expected);
    let t = tokenize(typed);
    let (n, m) = (e.len(), t.len());
    // Table des coûts minimaux et de l'opération qui y mène.
    let mut cost = vec![vec![INF; m + 1]; n + 1];
    let mut op = vec![vec![Op::Start; m + 1]; n + 1];
    cost[0][0] = 0;
    for i in 0..=n {
        for j in 0..=m {
            if i == 0 && j == 0 {
                continue;
            }
            // L'ordre des essais départage les égalités : appariement d'abord.
            let mut best = (INF, Op::Start);
            let mut consider = |c: u32, o: Op| {
                if c < best.0 {
                    best = (c, o);
                }
            };
            if i > 0 && j > 0 {
                consider(
                    cost[i - 1][j - 1] + pair_cost(&e[i - 1], &t[j - 1], lenient),
                    Op::Pair,
                );
            }
            // Fusion : un mot attendu tapé en plusieurs morceaux.
            for k in 2..=MAX_JOIN.min(j) {
                if i > 0
                    && e[i - 1].word
                    && t[j - k..j].iter().all(|x| x.word)
                    && letters(t[j - k..j].iter().map(|x| x.text.as_str()))
                        == letters(std::iter::once(e[i - 1].text.as_str()))
                {
                    consider(cost[i - 1][j - k] + JOIN, Op::Merge(k));
                }
            }
            // Scission : plusieurs mots attendus tapés d'un seul tenant.
            for k in 2..=MAX_JOIN.min(i) {
                if j > 0
                    && t[j - 1].word
                    && e[i - k..i].iter().all(|x| x.word)
                    && letters(e[i - k..i].iter().map(|x| x.text.as_str()))
                        == letters(std::iter::once(t[j - 1].text.as_str()))
                {
                    consider(cost[i - k][j - 1] + JOIN, Op::Split(k));
                }
            }
            if i > 0 {
                consider(cost[i - 1][j] + GAP, Op::Missing);
            }
            if j > 0 {
                consider(cost[i][j - 1] + GAP, Op::Extra);
            }
            cost[i][j] = best.0;
            op[i][j] = best.1;
        }
    }

    // Remontée depuis la dernière case, puis remise dans l'ordre du texte.
    let mut tokens = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        match op[i][j] {
            Op::Pair => {
                let (x, y) = (&e[i - 1], &t[j - 1]);
                let error = (!same(&x.text, &y.text, lenient)).then(|| {
                    if x.word {
                        classify_word(&x.text, &y.text)
                    } else {
                        ErrorType::Punctuation
                    }
                });
                tokens.push(token(Some(x), Some(y.text.clone()), x.word, error));
                i -= 1;
                j -= 1;
            }
            Op::Missing => {
                let x = &e[i - 1];
                let error = if x.word {
                    ErrorType::MissingWord
                } else {
                    ErrorType::Punctuation
                };
                tokens.push(token(Some(x), None, x.word, Some(error)));
                i -= 1;
            }
            Op::Extra => {
                let y = &t[j - 1];
                let error = if y.word {
                    ErrorType::ExtraWord
                } else {
                    ErrorType::Punctuation
                };
                tokens.push(AlignedToken {
                    expected: None,
                    typed: Some(y.text.clone()),
                    word: y.word,
                    error: Some(error),
                    position: None,
                    space_before: y.space_before,
                });
                j -= 1;
            }
            Op::Merge(k) => {
                let typed = t[j - k..j]
                    .iter()
                    .map(|x| x.text.as_str())
                    .collect::<Vec<_>>();
                let x = &e[i - 1];
                tokens.push(token(
                    Some(x),
                    Some(typed.join(" ")),
                    true,
                    Some(ErrorType::Punctuation),
                ));
                i -= 1;
                j -= k;
            }
            Op::Split(k) => {
                let words = e[i - k..i]
                    .iter()
                    .map(|x| x.text.as_str())
                    .collect::<Vec<_>>();
                let first = &e[i - k];
                tokens.push(AlignedToken {
                    expected: Some(words.join(" ")),
                    typed: Some(t[j - 1].text.clone()),
                    word: true,
                    error: Some(ErrorType::Punctuation),
                    position: Some(first.pos),
                    space_before: first.space_before,
                });
                i -= k;
                j -= 1;
            }
            // Impossible tant que i ou j est positif : la case de départ est (0, 0).
            Op::Start => break,
        }
    }
    tokens.reverse();
    summarize(tokens, e.iter().filter(|x| x.word).count() as u32)
}

/// Jeton aligné à partir d'un jeton attendu.
fn token(
    x: Option<&Tok>,
    typed: Option<String>,
    word: bool,
    error: Option<ErrorType>,
) -> AlignedToken {
    AlignedToken {
        expected: x.map(|x| x.text.clone()),
        typed,
        word,
        error,
        position: x.map(|x| x.pos),
        space_before: x.is_some_and(|x| x.space_before),
    }
}

/// Compte les fautes et les mots justes.
fn summarize(tokens: Vec<AlignedToken>, expected_words: u32) -> Correction {
    let faults = tokens.iter().filter(|t| t.error.is_some()).count() as u32;
    let correct_words = tokens
        .iter()
        .filter(|t| t.word && t.error.is_none() && t.expected.is_some())
        .count() as u32;
    let by_type = TYPE_ORDER
        .iter()
        .map(|&error_type| TypeCount {
            error_type,
            count: tokens
                .iter()
                .filter(|t| t.error == Some(error_type))
                .count() as u32,
        })
        .collect();
    Correction {
        tokens,
        expected_words,
        correct_words,
        faults,
        by_type,
    }
}

/// Tests de l'alignement et du classement des fautes.
#[cfg(test)]
mod tests {
    use super::*;

    /// Fautes d'une correction : (attendu, tapé, type).
    fn faults(c: &Correction) -> Vec<(Option<&str>, Option<&str>, ErrorType)> {
        c.tokens
            .iter()
            .filter_map(|t| {
                t.error
                    .map(|e| (t.expected.as_deref(), t.typed.as_deref(), e))
            })
            .collect()
    }

    /// Nombre de fautes d'un type.
    fn count(c: &Correction, kind: ErrorType) -> u32 {
        c.by_type
            .iter()
            .find(|t| t.error_type == kind)
            .unwrap()
            .count
    }

    /// Une copie parfaite : aucune faute, tous les mots justes.
    #[test]
    fn perfect_copy_has_no_fault() {
        let c = correct("Le chat dort, paisible.", "Le chat dort, paisible.", true);
        assert_eq!(c.faults, 0);
        assert_eq!((c.expected_words, c.correct_words), (4, 4));
        assert_eq!(c.tokens.len(), 6);
        assert!(c.tokens[3].expected.as_deref() == Some(",") && !c.tokens[3].space_before);
        assert!(c.tokens[4].space_before);
    }

    /// Chaque type de faute sur un mot.
    #[test]
    fn classifies_word_faults() {
        assert_eq!(classify_word("Digne", "digne"), ErrorType::Case);
        assert_eq!(classify_word("évêque", "évèque"), ErrorType::Accent);
        assert_eq!(classify_word("évêque", "Eveque"), ErrorType::Accent);
        assert_eq!(
            classify_word("vieillard", "viellard"),
            ErrorType::MissingLetter
        );
        assert_eq!(classify_word("environ", "environs"), ErrorType::ExtraLetter);
        assert_eq!(
            classify_word("accueillir", "acceuillir"),
            ErrorType::Transposition
        );
        assert_eq!(classify_word("chat", "chien"), ErrorType::Substitution);
        // Une lettre manquante et une majuscule : la lettre l'emporte.
        assert_eq!(
            classify_word("Vieillard", "viellard"),
            ErrorType::MissingLetter
        );
    }

    /// Cas de la maquette : chaque faute à sa place, avec son type.
    #[test]
    fn mockup_sentence() {
        let expected =
            "était évêque de Digne. C'était un vieillard d'environ soixante-quinze ans ;";
        let typed = "était évèque de digne. C'était un viellard d'environs soixante quinze ans ;";
        let c = correct(expected, typed, true);
        assert_eq!(
            faults(&c),
            [
                (Some("évêque"), Some("évèque"), ErrorType::Accent),
                (Some("Digne"), Some("digne"), ErrorType::Case),
                (
                    Some("vieillard"),
                    Some("viellard"),
                    ErrorType::MissingLetter
                ),
                (Some("environ"), Some("environs"), ErrorType::ExtraLetter),
                (
                    Some("soixante-quinze"),
                    Some("soixante quinze"),
                    ErrorType::Punctuation
                ),
            ]
        );
        assert_eq!(c.faults, 5);
        assert_eq!(count(&c, ErrorType::Punctuation), 1);
        assert_eq!(count(&c, ErrorType::Transposition), 0);
    }

    /// Critère transverse : 50 mots, 5 fautes connues, exactement ces 5 fautes.
    #[test]
    fn fifty_words_with_five_known_faults() {
        let expected = "En 1815, monseigneur Myriel était évêque de Digne. C'était un vieillard \
            d'environ soixante-quinze ans ; il occupait le siège de Digne depuis 1806. Quoique \
            ce détail ne touche au fond de ce que nous avons à raconter, il n'est peut-être \
            pas inutile d'indiquer les bruits très répandus.";
        // Accent, lettre manquante, majuscule, lettre en trop, mot oublié (« très »).
        let typed = "En 1815, monseigneur Myriel était évèque de Digne. C'était un viellard \
            d'environ soixante-quinze ans ; il occupait le siège de digne depuis 1806. Quoique \
            ce détail ne touche au fond de ce que nous avons à raconter, il n'est peut-être \
            pas inutiles d'indiquer les bruits répandus.";
        let c = correct(expected, typed, true);
        assert_eq!(c.expected_words, 50);
        assert_eq!(
            faults(&c),
            [
                (Some("évêque"), Some("évèque"), ErrorType::Accent),
                (
                    Some("vieillard"),
                    Some("viellard"),
                    ErrorType::MissingLetter
                ),
                (Some("Digne"), Some("digne"), ErrorType::Case),
                (Some("inutile"), Some("inutiles"), ErrorType::ExtraLetter),
                (Some("très"), None, ErrorType::MissingWord),
            ]
        );
        assert_eq!(c.correct_words, 45);
    }

    /// Mot en trop, mot sans rapport, mots collés.
    #[test]
    fn extra_words_substitutions_and_splits() {
        let c = correct("un chat noir", "un gros chien noir", true);
        assert_eq!(
            faults(&c),
            [
                (None, Some("gros"), ErrorType::ExtraWord),
                (Some("chat"), Some("chien"), ErrorType::Substitution),
            ]
        );
        let c = correct("parce que tu ris", "parceque tu ris", true);
        assert_eq!(
            faults(&c),
            [(Some("parce que"), Some("parceque"), ErrorType::Punctuation)]
        );
        assert_eq!(c.faults, 1);
    }

    /// Ponctuation oubliée, ajoutée ou remplacée ; apostrophes typographiques tolérées.
    #[test]
    fn punctuation_faults() {
        let c = correct("Oui, il vient.", "Oui il vient!", true);
        assert_eq!(
            faults(&c),
            [
                (Some(","), None, ErrorType::Punctuation),
                (Some("."), Some("!"), ErrorType::Punctuation),
            ]
        );
        assert_eq!(correct("l’école", "l'école", true).faults, 0);
        assert_eq!(correct("l’école", "l'école", false).faults, 1);
        let extra = correct("Oui", "Oui !", true);
        assert_eq!(faults(&extra), [(None, Some("!"), ErrorType::Punctuation)]);
    }

    /// Texte tapé vide : tous les mots oubliés ; texte attendu vide : tout en trop.
    #[test]
    fn empty_sides() {
        let c = correct("deux mots", "", true);
        assert_eq!(c.faults, 2);
        assert!(
            c.tokens
                .iter()
                .all(|t| t.error == Some(ErrorType::MissingWord))
        );
        let c = correct("", "rien", true);
        assert_eq!(faults(&c), [(None, Some("rien"), ErrorType::ExtraWord)]);
        assert_eq!(c.expected_words, 0);
    }

    /// Une faute de frappe d'une lettre aligne le mot, sans le compter oublié.
    #[test]
    fn single_letter_typo_stays_aligned() {
        let c = correct("le chat dort", "le chap dort", true);
        assert_eq!(
            faults(&c),
            [(Some("chat"), Some("chap"), ErrorType::Substitution)]
        );
    }
}
