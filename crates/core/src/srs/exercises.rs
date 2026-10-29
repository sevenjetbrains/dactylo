//! Exercices générés à partir des mots à réviser (F-44), derrière le trait
//! `ExerciseGenerator` : mots mélangés, mots répétés, phrases de la bibliothèque.
//!
//! La dictée de ces mots (4ᵉ forme de F-44) viendra avec le mode Dictée (Phase 4).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::text::{graphemes, normalize, word_spans};

/// Répétitions par défaut d'un mot dans l'exercice « mots répétés ».
pub const DEFAULT_REPEAT: u32 = 3;

/// Répétitions maximales d'un mot.
pub const MAX_REPEAT: u32 = 10;

/// Phrases au plus dans l'exercice « phrases de la bibliothèque ».
pub const MAX_SENTENCES: usize = 10;

/// Forme d'exercice demandée par l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReviewExerciseKind {
    /// Chaque mot une fois, dans un ordre mélangé.
    Shuffled,
    /// Chaque mot répété `times` fois de suite.
    Repeated {
        /// Répétitions de chaque mot (1 à `MAX_REPEAT`).
        times: u32,
    },
    /// Phrases de la bibliothèque qui contiennent ces mots.
    Sentences,
}

/// Générateur d'exercice : produit le texte à taper à partir des mots.
pub trait ExerciseGenerator {
    /// Texte de l'exercice ; vide si aucun exercice n'est possible.
    fn generate(&self, words: &[String]) -> String;
}

/// Générateur pseudo-aléatoire SplitMix64 : déterministe à graine égale (tests),
/// sans dépendance externe.
struct SplitMix64(u64);

impl SplitMix64 {
    /// Nombre suivant de la suite.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Mélange de Fisher-Yates.
    fn shuffle<T>(&mut self, items: &mut [T]) {
        // On parcourt de la fin vers le début en échangeant avec une case tirée avant.
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

/// Chaque mot une fois, mélangé.
pub struct ShuffledWords {
    /// Graine du mélange.
    pub seed: u64,
}

impl ExerciseGenerator for ShuffledWords {
    /// Les mots, sans doublon, dans un ordre tiré au hasard.
    fn generate(&self, words: &[String]) -> String {
        let mut list = unique(words);
        SplitMix64(self.seed).shuffle(&mut list);
        list.join(" ")
    }
}

/// Chaque mot répété plusieurs fois de suite, les mots étant mélangés.
pub struct RepeatedWords {
    /// Répétitions de chaque mot.
    pub times: u32,
    /// Graine du mélange.
    pub seed: u64,
}

impl ExerciseGenerator for RepeatedWords {
    /// « mot mot mot autre autre autre… », `times` ramené entre 1 et `MAX_REPEAT`.
    fn generate(&self, words: &[String]) -> String {
        let times = self.times.clamp(1, MAX_REPEAT) as usize;
        let mut list = unique(words);
        SplitMix64(self.seed).shuffle(&mut list);
        list.iter()
            .flat_map(|w| std::iter::repeat_n(w.as_str(), times))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Phrases de la bibliothèque qui contiennent au moins un des mots.
pub struct SentencesContaining<'a> {
    /// Contenus des textes de la bibliothèque.
    pub texts: &'a [String],
}

impl ExerciseGenerator for SentencesContaining<'_> {
    /// Choix glouton : à chaque tour, la phrase qui couvre le plus de mots encore
    /// absents de l'exercice (à égalité, la plus courte), jusqu'à `MAX_SENTENCES`.
    /// Les phrases retenues gardent l'ordre de la bibliothèque.
    fn generate(&self, words: &[String]) -> String {
        let wanted: HashSet<String> = words.iter().map(|w| w.to_lowercase()).collect();
        // Phrases candidates, avec leur rang et les mots cherchés qu'elles contiennent.
        let mut candidates: Vec<(usize, String, HashSet<String>)> = self
            .texts
            .iter()
            .flat_map(|t| sentences(&normalize(t)))
            .enumerate()
            .filter_map(|(rank, s)| {
                let found: HashSet<String> = words_of(&s)
                    .into_iter()
                    .filter(|w| wanted.contains(w))
                    .collect();
                (!found.is_empty()).then_some((rank, s, found))
            })
            .collect();
        // Sélection gloutonne tant qu'une phrase apporte un mot nouveau.
        let mut covered: HashSet<String> = HashSet::new();
        let mut chosen: Vec<(usize, String)> = Vec::new();
        while chosen.len() < MAX_SENTENCES {
            // Gain, longueur et rang de chaque candidate ; à égalité, la plus tôt.
            let best = candidates
                .iter()
                .enumerate()
                .map(|(i, (rank, s, found))| {
                    (i, found.difference(&covered).count(), s.len(), *rank)
                })
                .filter(|&(_, gain, _, _)| gain > 0)
                .max_by(|a, b| a.1.cmp(&b.1).then(b.2.cmp(&a.2)).then(b.3.cmp(&a.3)));
            let Some((i, ..)) = best else { break };
            let (rank, sentence, found) = candidates.swap_remove(i);
            covered.extend(found);
            chosen.push((rank, sentence));
        }
        // Remise dans l'ordre de lecture.
        chosen.sort_by_key(|(rank, _)| *rank);
        chosen
            .into_iter()
            .map(|(_, s)| s)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Mots sans doublon, dans leur ordre d'arrivée.
fn unique(words: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    words
        .iter()
        .filter(|w| seen.insert(w.as_str()))
        .cloned()
        .collect()
}

/// Mots d'une phrase, en minuscules.
fn words_of(sentence: &str) -> Vec<String> {
    let g = graphemes(sentence);
    word_spans(&g)
        .iter()
        .map(|s| g[s.start as usize..s.end as usize].concat().to_lowercase())
        .collect()
}

/// Coupe un texte en phrases : après . ! ? … suivis d'une espace, et à chaque
/// retour à la ligne. Les espaces autour sont retirées, les phrases vides ignorées.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        // Un retour à la ligne termine la phrase sans être gardé.
        if c == '\n' {
            push_sentence(&mut out, &mut current);
            continue;
        }
        current.push(c);
        // Ponctuation finale suivie d'une espace (ou de la fin) : fin de phrase.
        let ends = matches!(c, '.' | '!' | '?' | '…');
        if ends && chars.peek().is_none_or(|n| n.is_whitespace()) {
            push_sentence(&mut out, &mut current);
        }
    }
    push_sentence(&mut out, &mut current);
    out
}

/// Range la phrase en cours si elle n'est pas vide, puis la remet à zéro.
fn push_sentence(out: &mut Vec<String>, current: &mut String) {
    let sentence = current.trim();
    if !sentence.is_empty() {
        out.push(sentence.to_owned());
    }
    current.clear();
}

/// Tests des exercices générés.
#[cfg(test)]
mod tests {
    use super::*;

    /// Liste de mots de test.
    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|w| (*w).to_owned()).collect()
    }

    /// Mélange : chaque mot une fois, ordre stable à graine égale.
    #[test]
    fn shuffled_words_appear_once() {
        let w = words(&["accueillir", "parallèle", "œuvre", "parallèle"]);
        let out = ShuffledWords { seed: 7 }.generate(&w);
        let mut got: Vec<&str> = out.split(' ').collect();
        assert_eq!(out, ShuffledWords { seed: 7 }.generate(&w));
        got.sort_unstable();
        assert_eq!(got, ["accueillir", "parallèle", "œuvre"]);
    }

    /// Le mélange change vraiment l'ordre selon la graine.
    #[test]
    fn different_seeds_give_different_orders() {
        let w: Vec<String> = (0..20).map(|i| format!("mot{i}")).collect();
        let a = ShuffledWords { seed: 1 }.generate(&w);
        let b = ShuffledWords { seed: 2 }.generate(&w);
        assert_ne!(a, b);
    }

    /// Répétition : chaque mot `times` fois de suite, bornée à `MAX_REPEAT`.
    #[test]
    fn repeated_words_are_grouped() {
        let out = RepeatedWords { times: 2, seed: 0 }.generate(&words(&["un"]));
        assert_eq!(out, "un un");
        let many = RepeatedWords { times: 99, seed: 0 }.generate(&words(&["a"]));
        assert_eq!(many.split(' ').count(), MAX_REPEAT as usize);
        let none = RepeatedWords { times: 0, seed: 0 }.generate(&words(&["a"]));
        assert_eq!(none, "a");
        // Deux mots : deux blocs consécutifs.
        let two = RepeatedWords { times: 3, seed: 5 }.generate(&words(&["x", "y"]));
        assert!(two == "x x x y y y" || two == "y y y x x x");
    }

    /// Phrases : celles qui contiennent les mots, en couvrant un maximum de mots.
    #[test]
    fn sentences_cover_words_greedily() {
        let texts = words(&[
            "Il faut accueillir les amis. Le chat dort.\nUne œuvre parallèle à accueillir !",
            "Rien ici. Encore une œuvre.",
        ]);
        let out = SentencesContaining { texts: &texts }.generate(&words(&["accueillir", "œuvre"]));
        // Une seule phrase couvre les deux mots : elle suffit.
        assert_eq!(out, "Une œuvre parallèle à accueillir !");
    }

    /// Mots trouvés dans des phrases différentes ; la casse ne compte pas.
    #[test]
    fn sentences_from_several_texts() {
        let texts = words(&["Accueillir est un art.", "Une œuvre rare. Une autre œuvre."]);
        let out = SentencesContaining { texts: &texts }.generate(&words(&["accueillir", "œuvre"]));
        assert_eq!(out, "Accueillir est un art. Une œuvre rare.");
    }

    /// Aucun mot trouvé : exercice vide.
    #[test]
    fn no_sentence_found_gives_empty_exercise() {
        let texts = words(&["Le chat dort."]);
        assert_eq!(
            SentencesContaining { texts: &texts }.generate(&words(&["œuvre"])),
            ""
        );
    }

    /// Le mot doit être un mot entier : « art » n'est pas dans « partir ».
    #[test]
    fn sentences_match_whole_words() {
        let texts = words(&["Il faut partir. Un art."]);
        assert_eq!(
            SentencesContaining { texts: &texts }.generate(&words(&["art"])),
            "Un art."
        );
    }

    /// Découpage : ponctuation finale suivie d'une espace, retours à la ligne.
    #[test]
    fn splits_sentences() {
        assert_eq!(
            sentences("Un. Deux ! 3.5 kg ? Quatre…\nCinq"),
            ["Un.", "Deux !", "3.5 kg ?", "Quatre…", "Cinq"]
        );
        assert!(sentences("  \n ").is_empty());
    }

    /// La forme d'exercice se sérialise avec une étiquette `type`.
    #[test]
    fn kind_serializes_with_type_tag() {
        let json = serde_json::to_value(ReviewExerciseKind::Repeated { times: 3 }).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "repeated", "times": 3 }));
    }
}
