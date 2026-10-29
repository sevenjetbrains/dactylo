//! Tests des commandes de statistiques, sur une base en mémoire.

use dactylo_core::text::graphemes;
use dactylo_core::typing::{KeyKind, Keystroke};

use super::*;
use crate::commands::library::{self, NewText};
use crate::commands::practice;
use crate::test_support;

/// Ajoute un texte à la bibliothèque et renvoie son identifiant.
async fn add_text(state: &AppState, title: &str, content: &str) -> String {
    library::create(
        state,
        NewText {
            title: title.into(),
            content: content.into(),
            language: "fr".into(),
            ..NewText::default()
        },
    )
    .await
    .unwrap()
    .id
}

/// Insertions régulières (`step` ms) de chaque graphème de `typed`.
fn keys(typed: &str, step: u32) -> Vec<Keystroke> {
    graphemes(typed)
        .into_iter()
        .enumerate()
        .map(|(i, g)| Keystroke {
            seq: i as u32,
            kind: KeyKind::Insert,
            typed: Some(g),
            t_ms: step * i as u32,
        })
        .collect()
}

/// Joue une session ; la termine si `finish`, l'abandonne sinon.
async fn play(state: &AppState, text_id: &str, typed: &str, finish: bool) -> String {
    let s = practice::start(state, Some(text_id.into()), None, None)
        .await
        .unwrap();
    practice::record(state, &s.session_id, &keys(typed, 100))
        .await
        .unwrap();
    if finish {
        practice::finish(state, &s.session_id, 60_000)
            .await
            .unwrap();
    } else {
        practice::abandon(state, &s.session_id).await.unwrap();
    }
    s.session_id
}

/// Libellés de test pour l'export.
fn labels() -> CsvLabels {
    CsvLabels {
        headers: (1..=10).map(|i| format!("h{i}")).collect(),
        copy: "Copie".into(),
        dictation: "Dictée".into(),
        review: "Révision".into(),
        completed: "Terminée".into(),
        abandoned: "Abandonnée".into(),
        decimal_comma: true,
    }
}

/// Le tableau de bord compte terminées et abandonnées, sans session en cours.
#[tokio::test]
async fn dashboard_counts_completed_and_abandoned_sessions() {
    let (state, _) = test_support::state().await;
    let text = add_text(&state, "Chat", "le chat").await;
    play(&state, &text, "le chat", true).await;
    play(&state, &text, "le c", false).await;
    // Une session en cours n'est pas comptée.
    let current = practice::start(&state, Some(text.clone()), None, None)
        .await
        .unwrap();
    practice::record(&state, &current.session_id, &keys("l", 100))
        .await
        .unwrap();

    let d = compute_dashboard(&state, Period::Week, 0).await.unwrap();
    assert_eq!(d.points.len(), 1);
    assert_eq!(d.period.sessions, 1);
    // 60 s pour la terminée, 300 ms (dernière frappe) pour l'abandonnée.
    assert_eq!(d.period.training_ms, 60_300);
    assert_eq!(d.streak.current, 1);
    assert!(d.best_wpm_net.is_some());
    assert_eq!(d.week.sessions, 1);
}

/// La carte de chaleur compte les frappes et les fautes, abandons compris.
#[tokio::test]
async fn key_report_counts_presses_and_errors() {
    let (state, _) = test_support::state().await;
    let text = add_text(&state, "Fête", "fête").await;
    // « fete » : faute d'accent ; puis une session abandonnée avec une frappe juste.
    play(&state, &text, "fete", true).await;
    play(&state, &text, "f", false).await;
    let r = compute_keys(&state, Period::All, 0).await.unwrap();
    let f = r.keys.iter().find(|k| k.key == "f").unwrap();
    assert_eq!(f.presses, 2);
    let circumflex = r.keys.iter().find(|k| k.key == "^").unwrap();
    assert_eq!((circumflex.presses, circumflex.errors), (1, 1));
    assert_eq!((r.total_errors, r.accent_errors), (1, 1));
}

/// Sessions récentes : terminées seulement, la plus récente d'abord, avec le titre.
#[tokio::test]
async fn recent_sessions_are_completed_and_newest_first() {
    let (state, _) = test_support::state().await;
    let a = add_text(&state, "Premier", "un").await;
    let b = add_text(&state, "Second", "deux").await;
    play(&state, &a, "un", true).await;
    play(&state, &b, "de", false).await;
    let last = play(&state, &b, "deux", true).await;
    let list = recent(&state, 10).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].session_id, last);
    assert_eq!(list[0].title.as_deref(), Some("Second"));
    assert_eq!(list[0].mode, TrainingMode::Copy);
    assert_eq!(list[0].accuracy, 100.0);
    assert_eq!(recent(&state, 1).await.unwrap().len(), 1);
}

/// Révision : sans titre, et de mode `review`.
#[tokio::test]
async fn review_sessions_are_listed_without_title() {
    let (state, _) = test_support::state().await;
    let s = practice::start(&state, None, None, Some("œuvre".into()))
        .await
        .unwrap();
    practice::record(&state, &s.session_id, &keys("œuvre", 100))
        .await
        .unwrap();
    practice::finish(&state, &s.session_id, 1_000)
        .await
        .unwrap();
    let list = recent(&state, 5).await.unwrap();
    assert_eq!(list[0].mode, TrainingMode::Review);
    assert_eq!(list[0].title, None);
}

/// Export CSV : en-têtes, une ligne par session, libellés traduits, fautes comptées.
#[tokio::test]
async fn csv_export_lists_sessions_with_labels() {
    let (state, _) = test_support::state().await;
    let text = add_text(&state, "La forêt", "la forêt").await;
    play(&state, &text, "la foret", true).await;
    play(&state, &text, "la", false).await;
    let csv = export_csv(&state, Period::Month, 60, labels())
        .await
        .unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].ends_with("h1;h2;h3;h4;h5;h6;h7;h8;h9;h10"));
    // Session terminée : titre, mode, état, 60 s, une faute.
    assert!(
        lines[1].contains(";La forêt;Copie;Terminée;60,0;"),
        "{}",
        lines[1]
    );
    assert!(lines[1].ends_with(";1"));
    // Session abandonnée : statistiques vides, aucune faute relevée.
    assert!(lines[2].contains(";Abandonnée;"), "{}", lines[2]);
    assert!(lines[2].ends_with(";;;;;0"), "{}", lines[2]);
}

/// Les modes inconnus valent la copie.
#[test]
fn unknown_mode_is_copy() {
    assert_eq!(TrainingMode::parse("dictation"), TrainingMode::Dictation);
    assert_eq!(TrainingMode::parse("???"), TrainingMode::Copy);
}
