//! Carte de chaleur du clavier et bigrammes lents (F-34, ADR 0017).
//!
//! Calculées à partir des frappes brutes (F-31), sessions abandonnées comprises.
//! Les touches suivent l'AZERTY français : é, è, ç, à, ù ont leur propre touche ;
//! â, ê, ë, ï… se tapent avec une touche morte (^ ou ¨) puis la lettre de base,
//! et comptent donc sur les deux touches.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::text::is_whitespace;
use crate::typing::{KeyKind, ResolvedKeystroke};

/// Écart au-delà duquel le temps entre deux frappes n'est pas une mesure de
/// vitesse mais une hésitation (lecture, distraction) : il est ignoré.
pub const MAX_GAP_MS: u32 = 3_000;

/// Lettres accentuées qui ont leur propre touche sur l'AZERTY.
const DIRECT_KEYS: [char; 5] = ['é', 'è', 'ç', 'à', 'ù'];

/// Accent circonflexe combinant (U+0302), tapé avec la touche morte ^.
const COMBINING_CIRCUMFLEX: char = '\u{302}';

/// Tréma combinant (U+0308), tapé avec la touche morte ¨.
const COMBINING_DIAERESIS: char = '\u{308}';

/// Statistiques d'une touche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct KeyStat {
    /// Touche, en minuscules (« a », « é », « ^ »…).
    pub key: String,
    /// Frappes attendues sur cette touche.
    pub presses: u32,
    /// Frappes fausses.
    pub errors: u32,
    /// Temps moyen d'une frappe juste, en ms, s'il a pu être mesuré.
    pub avg_ms: Option<f64>,
}

/// Temps moyen d'un enchaînement de deux caractères.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BigramStat {
    /// Les deux caractères, en minuscules.
    pub bigram: String,
    /// Nombre d'occurrences mesurées.
    pub count: u32,
    /// Temps moyen du second caractère après le premier, en ms.
    pub avg_ms: f64,
}

/// Résultat de l'analyse des frappes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct KeyReport {
    /// Une entrée par touche rencontrée, triée par touche.
    pub keys: Vec<KeyStat>,
    /// Bigrammes les plus lents, du plus lent au moins lent.
    pub slow_bigrams: Vec<BigramStat>,
    /// Frappes fausses, toutes touches confondues.
    pub total_errors: u32,
    /// Frappes fausses sur un caractère accentué (part des accents, maquette).
    pub accent_errors: u32,
}

/// Cumul pour une touche.
#[derive(Default)]
struct KeyAcc {
    /// Frappes attendues.
    presses: u32,
    /// Frappes fausses.
    errors: u32,
    /// Somme des temps mesurés.
    total_ms: u64,
    /// Nombre de temps mesurés.
    timed: u32,
}

/// Cumul pour un bigramme.
#[derive(Default)]
struct BigramAcc {
    /// Somme des temps.
    total_ms: u64,
    /// Nombre d'occurrences.
    count: u32,
}

/// Accumule les frappes, session par session, puis produit le rapport.
#[derive(Default)]
pub struct KeyAnalyzer {
    /// Cumuls par touche.
    keys: BTreeMap<String, KeyAcc>,
    /// Cumuls par bigramme.
    bigrams: BTreeMap<String, BigramAcc>,
    /// Frappes fausses.
    total_errors: u32,
    /// Frappes fausses sur un caractère accentué.
    accent_errors: u32,
}

impl KeyAnalyzer {
    /// Analyseur vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute les frappes d'une session, dans l'ordre de frappe. Les temps ne sont
    /// comparés qu'à l'intérieur d'une session.
    pub fn add_session(&mut self, keystrokes: &[ResolvedKeystroke]) {
        let mut previous: Option<&ResolvedKeystroke> = None;
        for k in keystrokes {
            // Seules les insertions sur un caractère connu sont analysées.
            if let (KeyKind::Insert, Some(expected)) = (k.kind, k.expected.as_deref()) {
                // Temps depuis la frappe précédente, s'il mesure une vraie frappe.
                let gap = previous
                    .map(|p| k.t_ms.saturating_sub(p.t_ms))
                    .filter(|&gap| gap <= MAX_GAP_MS);
                self.add_key(expected, k.correct, gap);
                // Bigramme : deux insertions justes à la suite, à des positions voisines.
                if let (Some(p), Some(gap)) = (previous, gap) {
                    self.add_bigram(p, k, gap);
                }
            }
            previous = Some(k);
        }
    }

    /// Compte une frappe attendue sur les touches du caractère `expected`.
    fn add_key(&mut self, expected: &str, correct: bool, gap: Option<u32>) {
        let keys = keys_of(expected);
        // Les espaces et retours à la ligne ne sont pas des touches de la carte.
        if keys.is_empty() {
            return;
        }
        if !correct {
            self.total_errors += 1;
            if is_accented(expected) {
                self.accent_errors += 1;
            }
        }
        for key in keys {
            let acc = self.keys.entry(key).or_default();
            acc.presses += 1;
            // Une frappe fausse compte comme erreur ; seule une juste donne un temps.
            match (correct, gap) {
                (false, _) => acc.errors += 1,
                (true, Some(gap)) => {
                    acc.total_ms += u64::from(gap);
                    acc.timed += 1;
                }
                (true, None) => {}
            }
        }
    }

    /// Compte un bigramme si les deux frappes justes se suivent dans le texte.
    fn add_bigram(&mut self, previous: &ResolvedKeystroke, current: &ResolvedKeystroke, gap: u32) {
        let follows = previous.kind == KeyKind::Insert
            && previous.correct
            && current.correct
            && previous.position + 1 == current.position;
        if !follows {
            return;
        }
        // Les deux caractères doivent être des caractères visibles.
        let (Some(a), Some(b)) = (previous.expected.as_deref(), current.expected.as_deref()) else {
            return;
        };
        if is_whitespace(a) || is_whitespace(b) {
            return;
        }
        let acc = self
            .bigrams
            .entry(format!("{a}{b}").to_lowercase())
            .or_default();
        acc.total_ms += u64::from(gap);
        acc.count += 1;
    }

    /// Produit le rapport : `top` bigrammes au plus, vus au moins `min_count` fois.
    pub fn report(self, top: usize, min_count: u32) -> KeyReport {
        // Touches, triées par touche (ordre de la `BTreeMap`).
        let keys = self
            .keys
            .into_iter()
            .map(|(key, a)| KeyStat {
                key,
                presses: a.presses,
                errors: a.errors,
                avg_ms: (a.timed > 0).then(|| a.total_ms as f64 / f64::from(a.timed)),
            })
            .collect();
        // Bigrammes assez fréquents, du plus lent au moins lent ; à égalité, alphabétique.
        let mut slow_bigrams: Vec<BigramStat> = self
            .bigrams
            .into_iter()
            .filter(|(_, a)| a.count >= min_count)
            .map(|(bigram, a)| BigramStat {
                bigram,
                count: a.count,
                avg_ms: a.total_ms as f64 / f64::from(a.count),
            })
            .collect();
        slow_bigrams.sort_by(|a, b| b.avg_ms.total_cmp(&a.avg_ms));
        slow_bigrams.truncate(top);
        KeyReport {
            keys,
            slow_bigrams,
            total_errors: self.total_errors,
            accent_errors: self.accent_errors,
        }
    }
}

/// Touches AZERTY à frapper pour un caractère ; vide pour une espace.
pub fn keys_of(grapheme: &str) -> Vec<String> {
    // Espaces : hors carte.
    if is_whitespace(grapheme) {
        return Vec::new();
    }
    let lower = grapheme.to_lowercase();
    // Lettre accentuée qui a sa propre touche.
    if lower.chars().count() == 1 && lower.chars().all(|c| DIRECT_KEYS.contains(&c)) {
        return vec![lower];
    }
    // Sinon : touche morte éventuelle, puis le caractère de base sans accent.
    let mut keys = Vec::new();
    let mut base = String::new();
    for c in lower.nfd() {
        match c {
            COMBINING_CIRCUMFLEX => keys.push("^".to_owned()),
            COMBINING_DIAERESIS => keys.push("¨".to_owned()),
            // Les autres accents (á, ñ…) n'existent pas sur l'AZERTY : seule la base compte.
            c if unicode_normalization::char::is_combining_mark(c) => {}
            c => base.push(c),
        }
    }
    keys.push(base);
    keys
}

/// Le caractère porte-t-il un accent (touche dédiée ou touche morte) ?
fn is_accented(grapheme: &str) -> bool {
    let keys = keys_of(grapheme);
    keys.len() > 1
        || keys
            .first()
            .is_some_and(|k| k.chars().all(|c| DIRECT_KEYS.contains(&c)))
}

/// Tests de la carte de chaleur et des bigrammes.
#[cfg(test)]
mod tests {
    use super::*;

    /// Insertion à la position `position`, au temps `t_ms`.
    fn ins(position: u32, expected: &str, correct: bool, t_ms: u32) -> ResolvedKeystroke {
        ResolvedKeystroke {
            seq: position,
            position,
            kind: KeyKind::Insert,
            expected: Some(expected.to_owned()),
            typed: Some(if correct { expected } else { "x" }.to_owned()),
            t_ms,
            correct,
        }
    }

    /// Frappes justes, une par graphème de `text`, tous les `step` ms.
    fn typed(text: &str, step: u32) -> Vec<ResolvedKeystroke> {
        crate::text::graphemes(text)
            .iter()
            .enumerate()
            .map(|(i, g)| ins(i as u32, g, true, i as u32 * step))
            .collect()
    }

    /// Touches AZERTY : directes, mortes, majuscules, espaces.
    #[test]
    fn azerty_keys() {
        assert_eq!(keys_of("a"), ["a"]);
        assert_eq!(keys_of("A"), ["a"]);
        assert_eq!(keys_of("é"), ["é"]);
        assert_eq!(keys_of("É"), ["é"]);
        assert_eq!(keys_of("ê"), ["^", "e"]);
        assert_eq!(keys_of("Ï"), ["¨", "i"]);
        assert_eq!(keys_of("á"), ["a"]);
        assert_eq!(keys_of(","), [","]);
        assert!(keys_of(" ").is_empty());
        assert!(keys_of("\n").is_empty());
    }

    /// Fréquences, erreurs et temps moyens par touche.
    #[test]
    fn counts_presses_errors_and_times() {
        let mut an = KeyAnalyzer::new();
        // « aa » juste puis « a » faux : la 1ʳᵉ frappe n'a pas de temps.
        an.add_session(&[
            ins(0, "a", true, 0),
            ins(1, "a", true, 200),
            ins(2, "a", false, 300),
        ]);
        let r = an.report(5, 1);
        assert_eq!(
            r.keys,
            [KeyStat {
                key: "a".into(),
                presses: 3,
                errors: 1,
                avg_ms: Some(200.0),
            }]
        );
        assert_eq!(r.total_errors, 1);
        assert_eq!(r.accent_errors, 0);
    }

    /// Une lettre à touche morte compte sur la touche morte et sur la lettre.
    #[test]
    fn dead_key_letters_count_on_both_keys() {
        let mut an = KeyAnalyzer::new();
        an.add_session(&[ins(0, "ê", false, 0), ins(1, "é", false, 100)]);
        let r = an.report(5, 1);
        let keys: Vec<(&str, u32)> = r.keys.iter().map(|k| (k.key.as_str(), k.errors)).collect();
        assert_eq!(keys, [("^", 1), ("e", 1), ("é", 1)]);
        // Deux frappes fausses, toutes deux sur un accent.
        assert_eq!(r.total_errors, 2);
        assert_eq!(r.accent_errors, 2);
    }

    /// Une hésitation trop longue n'est pas un temps de frappe ; les retours
    /// arrière et les espaces ne sont pas des touches.
    #[test]
    fn long_gaps_deletes_and_spaces_are_ignored() {
        let mut an = KeyAnalyzer::new();
        let delete = ResolvedKeystroke {
            kind: KeyKind::Delete,
            typed: None,
            ..ins(1, "b", true, 150)
        };
        an.add_session(&[
            ins(0, "a", true, 0),
            ins(1, " ", true, 100),
            delete,
            ins(1, "b", true, 100 + MAX_GAP_MS + 200),
        ]);
        let r = an.report(5, 1);
        let b = r.keys.iter().find(|k| k.key == "b").unwrap();
        assert_eq!(b.avg_ms, None);
        assert!(r.keys.iter().all(|k| k.key != " "));
    }

    /// Bigrammes : les plus lents d'abord, filtrés par fréquence, sans espace.
    #[test]
    fn slowest_bigrams() {
        let mut an = KeyAnalyzer::new();
        // « que » deux fois : q→u à 300 ms, u→e à 100 ms.
        for _ in 0..2 {
            an.add_session(&[
                ins(0, "q", true, 0),
                ins(1, "u", true, 300),
                ins(2, "e", true, 400),
            ]);
        }
        // « le » une fois, très lent, et une espace (ignorée).
        an.add_session(&[
            ins(0, "l", true, 0),
            ins(1, "e", true, 900),
            ins(2, " ", true, 1000),
        ]);
        let r = an.report(5, 2);
        let names: Vec<&str> = r.slow_bigrams.iter().map(|b| b.bigram.as_str()).collect();
        assert_eq!(names, ["qu", "ue"]);
        assert_eq!(r.slow_bigrams[0].avg_ms, 300.0);
        assert_eq!(r.slow_bigrams[0].count, 2);
        // Sans seuil de fréquence, « le » passe en tête ; `top` limite la liste.
        let all = {
            let mut an = KeyAnalyzer::new();
            an.add_session(&typed("le", 900));
            an.add_session(&typed("que", 100));
            an.report(1, 1)
        };
        assert_eq!(all.slow_bigrams.len(), 1);
        assert_eq!(all.slow_bigrams[0].bigram, "le");
    }

    /// Une faute, ou deux positions non voisines, ne forment pas de bigramme.
    #[test]
    fn bigrams_need_two_correct_neighbours() {
        let mut an = KeyAnalyzer::new();
        an.add_session(&[
            ins(0, "a", true, 0),
            ins(1, "b", false, 100),
            ins(1, "b", true, 200),
            ins(5, "c", true, 300),
        ]);
        assert!(an.report(5, 1).slow_bigrams.is_empty());
    }

    /// Les majuscules sont ramenées aux minuscules dans les bigrammes.
    #[test]
    fn bigrams_are_lowercase() {
        let mut an = KeyAnalyzer::new();
        an.add_session(&typed("Qu", 100));
        assert_eq!(an.report(5, 1).slow_bigrams[0].bigram, "qu");
    }

    /// Aucun envoi : rapport vide.
    #[test]
    fn empty_report() {
        assert_eq!(KeyAnalyzer::new().report(5, 1), KeyReport::default());
    }
}
