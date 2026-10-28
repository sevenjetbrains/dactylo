//! Bibliothèque (F-23) : recherche, filtre par étiquette et tri.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Ordre d'affichage de la bibliothèque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TextSort {
    /// Plus récents d'abord.
    #[default]
    Newest,
    /// Plus anciens d'abord.
    Oldest,
    /// Plus courts d'abord.
    Shortest,
    /// Plus longs d'abord.
    Longest,
    /// Meilleur MPM net d'abord ; les textes jamais tapés à la fin.
    BestScore,
}

/// Critères de la liste.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", default)]
pub struct TextQuery {
    /// Mots recherchés dans le titre, les étiquettes et le fichier d'origine.
    pub search: String,
    /// Étiquette exigée.
    pub tag: Option<String>,
    /// Ordre.
    pub sort: TextSort,
}

/// Ce que la requête a besoin de savoir d'un texte.
pub struct TextFacts<'a> {
    /// Titre.
    pub title: &'a str,
    /// Étiquettes.
    pub tags: &'a [String],
    /// Fichier d'origine, s'il y en a un.
    pub source_file: Option<&'a str>,
    /// Longueur en caractères.
    pub length: u32,
    /// Date d'ajout, en ms Unix.
    pub created_at: i64,
    /// Meilleur MPM net du profil sur ce texte.
    pub best_wpm: Option<f64>,
}

/// Filtre puis trie des textes. L'ordre d'entrée départage les égalités.
pub fn apply<T>(items: Vec<T>, query: &TextQuery, facts: impl Fn(&T) -> TextFacts<'_>) -> Vec<T> {
    // Mots recherchés, sans accents ni majuscules.
    let words: Vec<String> = fold(&query.search)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let mut kept: Vec<T> = items
        .into_iter()
        .filter(|item| {
            let f = facts(item);
            // Étiquette exigée, puis chaque mot présent quelque part.
            let tag_ok = query
                .tag
                .as_ref()
                .is_none_or(|tag| f.tags.iter().any(|t| t == tag));
            let haystack = fold(&format!(
                "{} {} {}",
                f.title,
                f.tags.join(" "),
                f.source_file.unwrap_or("")
            ));
            tag_ok && words.iter().all(|w| haystack.contains(w.as_str()))
        })
        .collect();
    // Tri stable : à égalité, l'ordre d'entrée est conservé.
    kept.sort_by(|a, b| compare(&facts(a), &facts(b), query.sort));
    kept
}

/// Comparaison de deux textes selon l'ordre demandé.
fn compare(a: &TextFacts<'_>, b: &TextFacts<'_>, sort: TextSort) -> Ordering {
    match sort {
        TextSort::Newest => b.created_at.cmp(&a.created_at),
        TextSort::Oldest => a.created_at.cmp(&b.created_at),
        TextSort::Shortest => a.length.cmp(&b.length),
        TextSort::Longest => b.length.cmp(&a.length),
        // Les textes sans score passent après tous les autres.
        TextSort::BestScore => match (a.best_wpm, b.best_wpm) {
            (Some(x), Some(y)) => y.total_cmp(&x),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
    }
}

/// Minuscules sans accents : « Été » → « ete ».
fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Titre d'une copie de texte (duplication, F-23).
pub fn duplicate_title(title: &str) -> String {
    format!("{title} (copie)")
}

/// Tests de la requête de bibliothèque.
#[cfg(test)]
mod tests {
    use super::*;

    /// Texte d'essai : titre, étiquettes, longueur, date, score.
    struct Item {
        /// Titre.
        title: &'static str,
        /// Étiquettes.
        tags: Vec<String>,
        /// Longueur.
        length: u32,
        /// Date d'ajout.
        created_at: i64,
        /// Meilleur score.
        best: Option<f64>,
    }

    /// Trois textes d'essai, du plus récent au plus ancien.
    fn items() -> Vec<Item> {
        vec![
            Item {
                title: "Le Corbeau et le Renard",
                tags: vec!["fable".into()],
                length: 600,
                created_at: 3,
                best: None,
            },
            Item {
                title: "Été indien",
                tags: vec!["chanson".into()],
                length: 200,
                created_at: 2,
                best: Some(40.0),
            },
            Item {
                title: "La Cigale et la Fourmi",
                tags: vec!["fable".into()],
                length: 500,
                created_at: 1,
                best: Some(55.0),
            },
        ]
    }

    /// Titres après application de la requête.
    fn titles(query: TextQuery) -> Vec<&'static str> {
        apply(items(), &query, |i| TextFacts {
            title: i.title,
            tags: &i.tags,
            source_file: None,
            length: i.length,
            created_at: i.created_at,
            best_wpm: i.best,
        })
        .iter()
        .map(|i| i.title)
        .collect()
    }

    /// La recherche ignore accents et majuscules, et exige tous les mots.
    #[test]
    fn search_ignores_accents_and_case() {
        let q = |search: &str| TextQuery {
            search: search.into(),
            ..TextQuery::default()
        };
        assert_eq!(titles(q("ete")), ["Été indien"]);
        assert_eq!(titles(q("FABLE renard")), ["Le Corbeau et le Renard"]);
        assert_eq!(titles(q("")).len(), 3);
    }

    /// Le filtre par étiquette garde les textes qui la portent.
    #[test]
    fn tag_filter() {
        let query = TextQuery {
            tag: Some("fable".into()),
            ..TextQuery::default()
        };
        assert_eq!(
            titles(query),
            ["Le Corbeau et le Renard", "La Cigale et la Fourmi"]
        );
    }

    /// Chaque ordre de tri.
    #[test]
    fn sorts() {
        let q = |sort| TextQuery {
            sort,
            ..TextQuery::default()
        };
        assert_eq!(titles(q(TextSort::Newest))[0], "Le Corbeau et le Renard");
        assert_eq!(titles(q(TextSort::Oldest))[0], "La Cigale et la Fourmi");
        assert_eq!(titles(q(TextSort::Shortest))[0], "Été indien");
        assert_eq!(titles(q(TextSort::Longest))[0], "Le Corbeau et le Renard");
        // Meilleur score d'abord, texte jamais tapé en dernier.
        assert_eq!(
            titles(q(TextSort::BestScore)),
            [
                "La Cigale et la Fourmi",
                "Été indien",
                "Le Corbeau et le Renard"
            ]
        );
    }

    /// Titre d'une copie.
    #[test]
    fn duplicate_title_is_marked() {
        assert_eq!(duplicate_title("Le loup"), "Le loup (copie)");
    }
}
