use dactylo_core::settings::SessionGoal;
use dactylo_core::typing::{CorrectionMode, ErrorType, KeyKind};

use super::*;
use crate::commands::library;
use crate::db::texts::NewText;
use crate::test_support;

async fn add_text(state: &AppState, content: &str) -> String {
    library::create(
        state,
        NewText {
            title: "Texte".into(),
            content: content.into(),
            tags: vec![],
            language: "fr".into(),
        },
    )
    .await
    .unwrap()
    .id
}

/// Frappes numérotées à partir de `first`, 200 ms d'écart ; « ⌫ » = retour arrière.
fn keys(first: u32, typed: &[&str]) -> Vec<Keystroke> {
    typed
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let seq = first + i as u32;
            let delete = *t == "⌫";
            Keystroke {
                seq,
                kind: if delete {
                    KeyKind::Delete
                } else {
                    KeyKind::Insert
                },
                typed: (!delete).then(|| (*t).to_owned()),
                t_ms: seq * 200,
            }
        })
        .collect()
}

async fn stored_keystrokes(state: &AppState, session_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM keystrokes WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn complete_session_is_saved_with_stats_errors_and_events() {
    let (state, recorder) = test_support::state().await;
    let text_id = add_text(&state, "la forêt").await;

    let start = start(&state, Some(text_id.clone())).await.unwrap();
    assert_eq!(start.graphemes.len(), 8);
    assert_eq!(start.words.len(), 2);
    assert!(start.keystrokes.is_empty());

    record(&state, &start.session_id, &keys(0, &["l", "a", " ", "f"]))
        .await
        .unwrap();
    let last = record(&state, &start.session_id, &keys(4, &["o", "r", "e", "t"]))
        .await
        .unwrap();
    assert_eq!(last, Some(7));

    let summary = finish(&state, &start.session_id, 2_000).await.unwrap();
    assert_eq!(summary.text_id, text_id);
    assert_eq!(summary.stats.typed, 8);
    assert_eq!(summary.stats.errors, 1);
    assert_eq!(summary.stats.duration_ms, 2_000);
    assert_eq!(summary.stats.accuracy, 87.5);
    assert!(summary.average.is_none(), "première session");
    assert_eq!(summary.errors.len(), 1);
    assert_eq!(summary.errors[0].expected, "forêt");
    assert_eq!(summary.errors[0].error_type, ErrorType::Accent);

    assert_eq!(stored_keystrokes(&state, &start.session_id).await, 8);
    let row = sessions::get(&state.pool, &start.session_id).await.unwrap();
    assert!(row.ended_at.is_some());

    let events = recorder.events();
    assert!(events.contains(&DomainEvent::SessionCompleted {
        session_id: start.session_id.clone(),
        profile_id: state.profile_id.clone(),
    }));
    assert!(events.contains(&DomainEvent::WordMistyped {
        session_id: start.session_id.clone(),
        profile_id: state.profile_id.clone(),
        expected: "forêt".into(),
        typed: "foret".into(),
    }));

    assert!(matches!(
        finish(&state, &start.session_id, 2_000).await,
        Err(AppError::Invalid(_))
    ));
}

#[tokio::test]
async fn second_session_is_compared_with_the_first() {
    let (state, _) = test_support::state().await;
    let text_id = add_text(&state, "ab").await;
    for _ in 0..2 {
        let s = start(&state, Some(text_id.clone())).await.unwrap();
        record(&state, &s.session_id, &keys(0, &["a", "b"]))
            .await
            .unwrap();
        let summary = finish(&state, &s.session_id, 1_000).await.unwrap();
        if let Some(avg) = summary.average {
            assert_eq!(avg.sessions, 1);
            assert_eq!(avg.accuracy, 100.0);
        }
    }
}

#[tokio::test]
async fn batches_are_idempotent_even_after_losing_the_cache() {
    let (state, _) = test_support::state().await;
    let text_id = add_text(&state, "chat").await;
    let s = start(&state, Some(text_id)).await.unwrap();

    record(&state, &s.session_id, &keys(0, &["c", "x"]))
        .await
        .unwrap();
    // Simule un redémarrage : la session n'est plus en mémoire.
    state.take_live(&s.session_id);
    // Le lot précédent est renvoyé avec le suivant (envoi interrompu).
    record(&state, &s.session_id, &keys(0, &["c", "x", "⌫", "h"]))
        .await
        .unwrap();

    assert_eq!(stored_keystrokes(&state, &s.session_id).await, 4);
    let resumed = resume(&state, &s.session_id).await.unwrap();
    assert_eq!(resumed.keystrokes.len(), 4);
    assert_eq!(resumed.active_ms, 600);
}

#[tokio::test]
async fn interrupted_session_is_offered_for_resumption() {
    let (state, _) = test_support::state().await;
    assert!(pending(&state).await.unwrap().is_none());

    let text_id = add_text(&state, "abcd").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    record(&state, &s.session_id, &keys(0, &["a"]))
        .await
        .unwrap();

    // Nouvel état sur la même base : comme après une fermeture brutale.
    let restarted = AppState::new(
        state.pool.clone(),
        state.bus.clone(),
        state.profile_id.clone(),
    );
    let offer = pending(&restarted).await.unwrap().unwrap();
    assert_eq!(offer.session_id, s.session_id);
    assert_eq!(offer.title, "Texte");
    assert_eq!(offer.progress, 0.25);

    let resumed = resume(&restarted, &s.session_id).await.unwrap();
    assert_eq!(resumed.graphemes, ["a", "b", "c", "d"]);
    assert_eq!(resumed.keystrokes.len(), 1);
    record(&restarted, &s.session_id, &keys(1, &["b", "c", "d"]))
        .await
        .unwrap();
    let summary = finish(&restarted, &s.session_id, 800).await.unwrap();
    assert_eq!(summary.stats.typed, 4);
    assert!(pending(&restarted).await.unwrap().is_none());
}

#[tokio::test]
async fn session_without_keystrokes_is_not_offered() {
    let (state, _) = test_support::state().await;
    let text_id = add_text(&state, "ab").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    assert!(pending(&state).await.unwrap().is_none());
    assert!(matches!(
        sessions::get(&state.pool, &s.session_id).await,
        Err(AppError::NotFound(_))
    ));
}

#[tokio::test]
async fn abandoned_session_is_deleted() {
    let (state, _) = test_support::state().await;
    let text_id = add_text(&state, "ab").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    abandon(&state, &s.session_id).await.unwrap();
    assert!(pending(&state).await.unwrap().is_none());
    assert!(matches!(
        record(&state, &s.session_id, &keys(0, &["a"])).await,
        Err(AppError::NotFound(_))
    ));
}

#[tokio::test]
async fn start_without_id_uses_the_latest_text() {
    let (state, _) = test_support::state().await;
    assert!(matches!(
        start(&state, None).await,
        Err(AppError::NotFound(_))
    ));
    add_text(&state, "ancien").await;
    let latest = add_text(&state, "récent").await;
    assert_eq!(start(&state, None).await.unwrap().text_id, latest);
}

#[tokio::test]
async fn settings_are_sanitized_and_frozen_for_the_session() {
    let (state, _) = test_support::state().await;
    assert_eq!(
        load_settings(&state).await.unwrap(),
        PracticeSettings::default()
    );

    let blocking = PracticeSettings {
        correction: CorrectionMode::Blocking,
        visible_lines: 42,
        ..PracticeSettings::default()
    };
    let saved = store_settings(&state, blocking).await.unwrap();
    assert_eq!(saved.visible_lines, 10);
    assert_eq!(load_settings(&state).await.unwrap(), saved);

    let text_id = add_text(&state, "ab").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    assert_eq!(s.settings.correction, CorrectionMode::Blocking);

    // Changer les réglages en cours de session ne change pas ses règles.
    store_settings(&state, PracticeSettings::default())
        .await
        .unwrap();
    state.take_live(&s.session_id);
    record(&state, &s.session_id, &keys(0, &["x", "a"]))
        .await
        .unwrap();
    let resumed = resume(&state, &s.session_id).await.unwrap();
    assert_eq!(resumed.settings.correction, CorrectionMode::Blocking);
    let summary = finish(&state, &s.session_id, 0).await.unwrap();
    // Bloquant : « x » n'avance pas, « a » est juste à la position 0.
    assert_eq!(summary.stats.uncorrected, 0);
    assert_eq!(summary.stats.errors, 1);
}

#[tokio::test]
async fn timed_session_duration_is_capped() {
    let (state, _) = test_support::state().await;
    store_settings(
        &state,
        PracticeSettings {
            goal: SessionGoal::Duration { minutes: 1 },
            ..PracticeSettings::default()
        },
    )
    .await
    .unwrap();
    let text_id = add_text(&state, "ab").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    let summary = finish(&state, &s.session_id, 61_250).await.unwrap();
    assert_eq!(summary.stats.duration_ms, 60_000);
}

#[tokio::test]
async fn wrap_text_returns_line_starts() {
    let (state, _) = test_support::state().await;
    let text_id = add_text(&state, "le chat dort").await;
    let s = start(&state, Some(text_id)).await.unwrap();
    assert_eq!(line_starts(&state, &s.session_id, 8).await.unwrap(), [0, 8]);
    state.take_live(&s.session_id);
    assert_eq!(line_starts(&state, &s.session_id, 100).await.unwrap(), [0]);
}
