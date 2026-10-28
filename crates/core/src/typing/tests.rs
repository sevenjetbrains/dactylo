use serde::Deserialize;

use super::*;
use crate::text::{graphemes, normalize};

const DELETE_KEY: &str = "⌫";

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    text: String,
    rules: Rules,
    keys: Vec<String>,
    expected: Expected,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    position: usize,
    cells: String,
    inserts: u32,
    correct_inserts: u32,
    correct_cells: u32,
    uncorrected: u32,
    rejected: usize,
}

fn keystrokes(keys: &[&str]) -> Vec<Keystroke> {
    keys.iter()
        .enumerate()
        .map(|(i, key)| {
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

fn session(text: &str, rules: Rules, keys: &[&str]) -> TypingSession {
    TypingSession::replay(graphemes(&normalize(text)), rules, &keystrokes(keys))
}

fn cells(session: &TypingSession) -> String {
    session.cells().iter().map(|c| c.code()).collect()
}

#[test]
fn shared_fixture_cases() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../tests/fixtures/typing-cases.json")).unwrap();
    assert!(fixture.cases.len() >= 15);
    for case in fixture.cases {
        let keys: Vec<&str> = case.keys.iter().map(String::as_str).collect();
        let mut s = TypingSession::new(graphemes(&normalize(&case.text)), case.rules);
        let rejected = keystrokes(&keys)
            .iter()
            .filter(|k| s.apply(k).is_none())
            .count();
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

#[test]
fn resolved_keystrokes_carry_position_and_expected() {
    let mut s = TypingSession::new(graphemes("ab"), Rules::default());
    let ks = keystrokes(&["a", "x", DELETE_KEY]);
    let resolved: Vec<_> = ks.iter().filter_map(|k| s.apply(k)).collect();
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0].position, 0);
    assert!(resolved[0].correct);
    assert_eq!(resolved[1].position, 1);
    assert_eq!(resolved[1].expected.as_deref(), Some("b"));
    assert_eq!(resolved[1].typed.as_deref(), Some("x"));
    assert!(!resolved[1].correct);
    assert_eq!(resolved[2].kind, KeyKind::Delete);
    assert_eq!(resolved[2].position, 1);
    assert_eq!(resolved[2].typed, None);
    assert_eq!(resolved[2].seq, 2);
    assert_eq!(resolved[2].t_ms, 200);
    assert_eq!(s.last_seq(), Some(2));
    assert_eq!(s.last_t_ms(), 200);
}

#[test]
fn replaying_the_same_sequence_number_twice_is_ignored() {
    let mut s = TypingSession::new(graphemes("ab"), Rules::default());
    let ks = keystrokes(&["a", "b"]);
    assert!(s.apply(&ks[0]).is_some());
    assert!(s.apply(&ks[0]).is_none(), "doublon");
    assert!(s.apply(&ks[1]).is_some());
    assert_eq!(s.position(), 2);
}

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

#[test]
fn corrected_error_still_counts_with_first_typed_form() {
    let s = session(
        "chat",
        Rules::default(),
        &["c", "j", DELETE_KEY, "h", "a", "t"],
    );
    let errors = s.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].typed, "cjat");
    assert_eq!(errors[0].error_type, ErrorType::Substitution);
    assert_eq!(s.counters().uncorrected, 0);
}

#[test]
fn dominant_error_type_wins_and_ties_keep_the_first() {
    // « Été » → « ete » : majuscule+accent (accent) puis accent → accent.
    let s = session("Été", Rules::default(), &["e", "t", "e"]);
    assert_eq!(s.errors()[0].error_type, ErrorType::Accent);
    // « Le » → « lz » : majuscule puis substitution, égalité → majuscule.
    let s = session("Le", Rules::default(), &["l", "z"]);
    assert_eq!(s.errors()[0].error_type, ErrorType::Case);
    assert_eq!(
        dominant(&[ErrorType::Case, ErrorType::Accent, ErrorType::Accent]),
        ErrorType::Accent
    );
}

#[test]
fn untyped_words_have_no_error() {
    let s = session("le chat", Rules::default(), &["l", "e"]);
    assert!(s.errors().is_empty());
    assert_eq!(s.words().len(), 2);
    assert_eq!(s.text().len(), 7);
    assert_eq!(s.rules(), Rules::default());
}

#[test]
fn graphemes_match_rules() {
    assert!(graphemes_match("ê", "e\u{302}", false));
    assert!(graphemes_match("\n", " ", false));
    assert!(graphemes_match("\u{202f}", " ", false));
    assert!(!graphemes_match("’", "'", false));
    assert!(graphemes_match("’", "'", true));
    assert!(graphemes_match("—", "-", true));
    assert!(!graphemes_match("a", "b", true));
}

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
