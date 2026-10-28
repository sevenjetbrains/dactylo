//! Tests de `TypingSession` : cas partagés avec l'interface (ADR 0008)
//! et cas propres au cœur (fautes, idempotence, règles de comparaison).

use serde::Deserialize;

use super::*;
use crate::text::{graphemes, normalize};

/// Touche qui représente un retour arrière dans les cas partagés.
const DELETE_KEY: &str = "⌫";

/// Fichier de cas partagés `typing-cases.json`.
#[derive(Deserialize)]
struct Fixture {
    /// Liste des cas.
    cases: Vec<Case>,
}

/// Un cas : un texte, des règles, une suite de touches et le résultat attendu.
#[derive(Deserialize)]
struct Case {
    /// Nom du cas, affiché quand une assertion échoue.
    name: String,
    /// Texte à taper (normalisé avant usage).
    text: String,
    /// Règles de comparaison.
    rules: Rules,
    /// Touches tapées, `⌫` pour un retour arrière.
    keys: Vec<String>,
    /// État attendu après toutes les touches.
    expected: Expected,
}

/// État attendu d'une session après un cas.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    /// Position du curseur.
    position: usize,
    /// Codes des cellules (voir `Cell::code`).
    cells: String,
    /// Insertions acceptées.
    inserts: u32,
    /// Insertions justes.
    correct_inserts: u32,
    /// Caractères justes à la fin.
    correct_cells: u32,
    /// Caractères encore faux à la fin.
    uncorrected: u32,
    /// Frappes refusées.
    rejected: usize,
}

/// Transforme une suite de touches en frappes numérotées, espacées de 100 ms.
fn keystrokes(keys: &[&str]) -> Vec<Keystroke> {
    keys.iter()
        .enumerate()
        .map(|(i, key)| {
            // `⌫` devient un retour arrière ; toute autre touche, une insertion.
            let delete = *key == DELETE_KEY;
            Keystroke {
                seq: i as u32,
                kind: if delete {
                    KeyKind::Delete
                } else {
                    KeyKind::Insert
                },
                typed: (!delete).then(|| (*key).to_owned()),
                t_ms: 100 * i as u32,
            }
        })
        .collect()
}

/// Rejoue des touches sur un texte normalisé.
fn session(text: &str, rules: Rules, keys: &[&str]) -> TypingSession {
    TypingSession::replay(graphemes(&normalize(text)), rules, &keystrokes(keys))
}

/// Codes des cellules de la session, sous forme de chaîne.
fn cells(session: &TypingSession) -> String {
    session.cells().iter().map(|c| c.code()).collect()
}

/// Tous les cas partagés avec le miroir TypeScript donnent le résultat attendu.
#[test]
fn shared_fixture_cases() {
    // Le fichier est inclus à la compilation ; on vérifie qu'il n'a pas été vidé.
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../tests/fixtures/typing-cases.json")).unwrap();
    assert!(fixture.cases.len() >= 15);
    for case in fixture.cases {
        // Rejoue les touches une à une en comptant les frappes refusées.
        let keys: Vec<&str> = case.keys.iter().map(String::as_str).collect();
        let mut s = TypingSession::new(graphemes(&normalize(&case.text)), case.rules);
        let rejected = keystrokes(&keys)
            .iter()
            .filter(|k| s.apply(k).is_none())
            .count();
        // Compare chaque valeur ; le nom du cas figure dans le message d'échec.
        let e = &case.expected;
        let counters = s.counters();
        assert_eq!(s.position(), e.position, "{} : position", case.name);
        assert_eq!(cells(&s), e.cells, "{} : cellules", case.name);
        assert_eq!(counters.inserts, e.inserts, "{} : insertions", case.name);
        assert_eq!(
            counters.correct_inserts, e.correct_inserts,
            "{} : insertions justes",
            case.name
        );
        assert_eq!(counters.correct_cells, e.correct_cells, "{}", case.name);
        assert_eq!(counters.uncorrected, e.uncorrected, "{}", case.name);
        assert_eq!(rejected, e.rejected, "{} : refusées", case.name);
    }
}

/// Une frappe résolue porte sa position, le caractère attendu, son numéro et son temps.
#[test]
fn resolved_keystrokes_carry_position_and_expected() {
    // « a » juste, « x » faux, puis retour arrière.
    let mut s = TypingSession::new(graphemes("ab"), Rules::default());
    let ks = keystrokes(&["a", "x", DELETE_KEY]);
    let resolved: Vec<_> = ks.iter().filter_map(|k| s.apply(k)).collect();
    assert_eq!(resolved.len(), 3);
    // Première frappe : juste, en position 0.
    assert_eq!(resolved[0].position, 0);
    assert!(resolved[0].correct);
    // Deuxième frappe : « x » à la place de « b ».
    assert_eq!(resolved[1].position, 1);
    assert_eq!(resolved[1].expected.as_deref(), Some("b"));
    assert_eq!(resolved[1].typed.as_deref(), Some("x"));
    assert!(!resolved[1].correct);
    // Retour arrière : efface la position 1, sans graphème tapé.
    assert_eq!(resolved[2].kind, KeyKind::Delete);
    assert_eq!(resolved[2].position, 1);
    assert_eq!(resolved[2].typed, None);
    assert_eq!(resolved[2].seq, 2);
    assert_eq!(resolved[2].t_ms, 200);
    // La session retient la dernière frappe acceptée.
    assert_eq!(s.last_seq(), Some(2));
    assert_eq!(s.last_t_ms(), 200);
}

/// Une frappe déjà vue (même numéro) est ignorée.
#[test]
fn replaying_the_same_sequence_number_twice_is_ignored() {
    let mut s = TypingSession::new(graphemes("ab"), Rules::default());
    let ks = keystrokes(&["a", "b"]);
    assert!(s.apply(&ks[0]).is_some());
    assert!(s.apply(&ks[0]).is_none(), "doublon");
    assert!(s.apply(&ks[1]).is_some());
    assert_eq!(s.position(), 2);
}

/// Une insertion sans graphème est refusée et ne compte pas comme vue.
#[test]
fn insert_without_grapheme_is_rejected() {
    let mut s = TypingSession::new(graphemes("ab"), Rules::default());
    let k = Keystroke {
        seq: 0,
        kind: KeyKind::Insert,
        typed: None,
        t_ms: 0,
    };
    assert!(s.apply(&k).is_none());
    assert_eq!(s.last_seq(), None);
}

/// Une faute par mot fautif, une par caractère fautif hors mot.
#[test]
fn errors_report_one_entry_per_mistyped_word() {
    // « forêt » tapé « foret », « chat » juste, virgule remplacée par un point.
    let s = session(
        "la forêt, le chat",
        Rules::default(),
        &[
            "l", "a", " ", "f", "o", "r", "e", "t", ".", " ", "l", "e", " ", "c", "h", "a", "t",
        ],
    );
    let errors = s.errors();
    // Le mot fautif puis la virgule, dans l'ordre du texte.
    assert_eq!(
        errors,
        vec![
            TypingError {
                expected: "forêt".into(),
                typed: "foret".into(),
                error_type: ErrorType::Accent,
                position: 3,
                word: true,
            },
            TypingError {
                expected: ",".into(),
                typed: ".".into(),
                error_type: ErrorType::Punctuation,
                position: 8,
                word: false,
            },
        ]
    );
}

/// Une faute corrigée reste relevée, avec la forme tapée la première fois.
#[test]
fn corrected_error_still_counts_with_first_typed_form() {
    // « j » tapé à la place de « h », effacé puis corrigé.
    let s = session(
        "chat",
        Rules::default(),
        &["c", "j", DELETE_KEY, "h", "a", "t"],
    );
    let errors = s.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].typed, "cjat");
    assert_eq!(errors[0].error_type, ErrorType::Substitution);
    // Plus aucune erreur visible à la fin.
    assert_eq!(s.counters().uncorrected, 0);
}

/// Le type le plus fréquent l'emporte ; à égalité, le premier rencontré.
#[test]
fn dominant_error_type_wins_and_ties_keep_the_first() {
    // « Été » → « ete » : majuscule+accent (accent) puis accent → accent.
    let s = session("Été", Rules::default(), &["e", "t", "e"]);
    assert_eq!(s.errors()[0].error_type, ErrorType::Accent);
    // « Le » → « lz » : majuscule puis substitution, égalité → majuscule.
    let s = session("Le", Rules::default(), &["l", "z"]);
    assert_eq!(s.errors()[0].error_type, ErrorType::Case);
    // Majorité directe, même si un autre type arrive en premier.
    assert_eq!(
        dominant(&[ErrorType::Case, ErrorType::Accent, ErrorType::Accent]),
        ErrorType::Accent
    );
}

/// Les mots pas encore tapés ne produisent pas de faute.
#[test]
fn untyped_words_have_no_error() {
    let s = session("le chat", Rules::default(), &["l", "e"]);
    assert!(s.errors().is_empty());
    assert_eq!(s.words().len(), 2);
    assert_eq!(s.text().len(), 7);
    assert_eq!(s.rules(), Rules::default());
}

/// Règles de comparaison : NFC, espaces équivalentes, tolérance typographique.
#[test]
fn graphemes_match_rules() {
    // Touche morte décomposée = caractère précomposé.
    assert!(graphemes_match("ê", "e\u{302}", false));
    // Toute espace vaut toute espace, y compris le retour à la ligne.
    assert!(graphemes_match("\n", " ", false));
    assert!(graphemes_match("\u{202f}", " ", false));
    // L'apostrophe courbe n'est acceptée qu'avec la tolérance.
    assert!(!graphemes_match("’", "'", false));
    assert!(graphemes_match("’", "'", true));
    assert!(graphemes_match("—", "-", true));
    // La tolérance ne rend pas justes deux lettres différentes.
    assert!(!graphemes_match("a", "b", true));
}

/// Les règles sont sérialisées en camelCase ; `KeyKind` a ses valeurs de base.
#[test]
fn rules_serialize_in_camel_case() {
    let json = serde_json::to_value(Rules::default()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "correction": "free",
            "backspaceAllowed": true,
            "lenientPunctuation": true
        })
    );
    assert_eq!(KeyKind::Insert.as_str(), "insert");
    assert_eq!(KeyKind::Delete.as_str(), "delete");
}
