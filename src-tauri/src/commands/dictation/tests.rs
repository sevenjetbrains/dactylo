//! Tests des commandes de dictée, sur une base en mémoire et un moteur vocal factice.

use dactylo_core::dictation::settings::SegmentMode;
use dactylo_core::typing::ErrorType;

use super::*;
use crate::commands::library::{self, NewText};
use crate::commands::practice;
use crate::test_support;

/// Paragraphe de 50 mots du critère transverse.
const PARAGRAPH: &str = "En 1815, monseigneur Myriel était évêque de Digne. C'était un vieillard \
    d'environ soixante-quinze ans ; il occupait le siège de Digne depuis 1806. Quoique ce \
    détail ne touche au fond de ce que nous avons à raconter, il n'est peut-être pas inutile \
    d'indiquer les bruits très répandus.";

/// Même paragraphe tapé avec 5 fautes : accent, lettre manquante, majuscule, lettre en
/// trop, mot oublié.
const TYPED: &str = "En 1815, monseigneur Myriel était évèque de Digne. C'était un viellard \
    d'environ soixante-quinze ans ; il occupait le siège de digne depuis 1806. Quoique ce \
    détail ne touche au fond de ce que nous avons à raconter, il n'est peut-être pas inutiles \
    d'indiquer les bruits répandus.";

/// Ajoute un texte à la bibliothèque et renvoie son identifiant.
async fn add_text(state: &AppState, content: &str) -> String {
    library::create(
        state,
        NewText {
            title: "Les Misérables".into(),
            content: content.into(),
            language: "fr".into(),
            ..NewText::default()
        },
    )
    .await
    .unwrap()
    .id
}

/// Le texte n'est pas transmis ; les segments sont lus par le backend, un à un.
#[tokio::test]
async fn dictation_hides_text_and_speaks_segments() {
    let (state, _, speech) = test_support::state_with_speech().await;
    let text = add_text(&state, "Le chat dort. Il fait beau, très beau.").await;
    let start = start(&state, Some(text), None).await.unwrap();
    // Groupes de 4 mots par défaut : 4 + 4 mots.
    assert_eq!(start.segment_words, [4, 4]);
    assert_eq!(start.total_words, 8);
    assert_eq!(start.title, "Les Misérables");
    // Lecture du premier segment, puis du second.
    let first = speak_segment(&state, &start.session_id, 0, false)
        .await
        .unwrap();
    let second = speak_segment(&state, &start.session_id, 1, false)
        .await
        .unwrap();
    assert!(second.utterance > first.utterance);
    assert_eq!(
        speech.texts(),
        ["Le chat dort. Il", "fait beau, très beau."]
    );
    // Segment absent refusé.
    assert!(matches!(
        speak_segment(&state, &start.session_id, 2, false).await,
        Err(AppError::Invalid(_))
    ));
}

/// Ponctuation lue, découpage par phrase et limite de répétitions (F-11, F-13, F-14).
#[tokio::test]
async fn settings_shape_reading_and_repeats() {
    let (state, _, speech) = test_support::state_with_speech().await;
    store_settings(
        &state,
        DictationSettings {
            segmentation: SegmentMode::Sentence,
            read_punctuation: true,
            max_repeats: Some(1),
            rate: 0.8,
            ..DictationSettings::default()
        },
    )
    .await
    .unwrap();
    let text = add_text(&state, "Oui, il vient. Non !").await;
    let s = start(&state, Some(text), None).await.unwrap();
    assert_eq!(s.segment_words, [3, 1]);
    speak_segment(&state, &s.session_id, 0, false)
        .await
        .unwrap();
    // Une répétition permise, pas deux.
    let again = speak_segment(&state, &s.session_id, 0, true).await.unwrap();
    assert_eq!(again.repeats, 1);
    assert!(matches!(
        speak_segment(&state, &s.session_id, 0, true).await,
        Err(AppError::Invalid(_))
    ));
    let spoken = speech.spoken.lock().unwrap().clone();
    assert_eq!(spoken[0].0, "Oui virgule, il vient point,");
    assert_eq!(spoken[0].1.rate, 0.8);
}

/// Critère transverse, de bout en bout : 50 mots, 5 fautes connues, exactement
/// ces 5 fautes en base, bien classées ; les mots partent à la révision.
#[tokio::test]
async fn fifty_word_dictation_records_exactly_five_faults() {
    let (state, recorder, _) = test_support::state_with_speech().await;
    store_settings(
        &state,
        DictationSettings {
            segmentation: SegmentMode::Paragraph,
            ..DictationSettings::default()
        },
    )
    .await
    .unwrap();
    let text = add_text(&state, PARAGRAPH).await;
    let s = start(&state, Some(text), None).await.unwrap();
    assert_eq!(s.total_words, 50);
    let summary = finish(&state, &s.session_id, TYPED, 120_000).await.unwrap();
    assert_eq!(summary.stats.faults, 5);
    assert_eq!(summary.stats.accuracy, 90.0);
    assert!(summary.average.is_none());
    // Exactement ces cinq lignes dans `session_errors`, dans l'ordre du texte.
    let rows: Vec<(Option<String>, String)> = sqlx::query_as(
        "SELECT expected_word, error_type FROM session_errors WHERE session_id = ? ORDER BY position",
    )
    .bind(&s.session_id)
    .fetch_all(&state.pool)
    .await
    .unwrap();
    let rows: Vec<(&str, &str)> = rows
        .iter()
        .map(|(w, t)| (w.as_deref().unwrap_or(""), t.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("évêque", "accent"),
            ("vieillard", "missing_letter"),
            ("Digne", "case"),
            ("inutile", "extra_letter"),
            ("très", "missing_word"),
        ]
    );
    // Texte tapé gardé, session close en mode dictée.
    let (typed, ms) = repo::draft(&state.pool, &s.session_id).await.unwrap();
    assert_eq!((typed.as_str(), ms), (TYPED, 120_000));
    // Les mots (justes et faux) accompagnent la fin de session ; un `WordMistyped` par faute.
    let events = recorder.events();
    let words = events
        .iter()
        .find_map(|e| match e {
            DomainEvent::SessionCompleted { words, .. } => Some(words.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(words.attempts.iter().filter(|a| !a.correct).count(), 5);
    let mistyped = events
        .iter()
        .filter(|e| matches!(e, DomainEvent::WordMistyped { .. }))
        .count();
    assert_eq!(mistyped, 5);
    // Une seconde dictée est comparée à la première.
    let text = add_text(&state, PARAGRAPH).await;
    let second = start(&state, Some(text), None).await.unwrap();
    let summary = finish(&state, &second.session_id, PARAGRAPH, 60_000)
        .await
        .unwrap();
    let average = summary.average.unwrap();
    assert_eq!((average.sessions, average.faults), (1, 5.0));
    // Copie parfaite : aucune faute, d'aucun type.
    assert_eq!(summary.stats.faults, 0);
    assert!(summary.correction.by_type.iter().all(|t| t.count == 0));
    assert!(
        summary
            .correction
            .by_type
            .iter()
            .any(|t| t.error_type == ErrorType::MissingWord)
    );
}

/// Brouillon, perte du cache, reprise ; la Copie ne propose pas la dictée (NF-06).
#[tokio::test]
async fn draft_survives_restart_and_resumes() {
    let (state, _, _) = test_support::state_with_speech().await;
    let text = add_text(&state, PARAGRAPH).await;
    let s = start(&state, Some(text), None).await.unwrap();
    speak_segment(&state, &s.session_id, 0, true).await.unwrap();
    save_draft(&state, &s.session_id, "En 1815, monseigneur", 9_000)
        .await
        .unwrap();
    // Redémarrage : la dictée n'est plus en mémoire.
    state.take_dictation(&s.session_id);
    let offer = pending(&state).await.unwrap().expect("dictée reprenable");
    assert_eq!(offer.typed_words, 3);
    assert_eq!(offer.total_words, 50);
    assert!(practice::pending(&state).await.unwrap().is_none());
    let resumed = resume(&state, &s.session_id).await.unwrap();
    assert_eq!(resumed.typed, "En 1815, monseigneur");
    assert_eq!(resumed.active_ms, 9_000);
    assert_eq!(resumed.segment_words, s.segment_words);
}

/// Une dictée ouverte sans rien taper n'est pas proposée à la reprise : elle disparaît.
#[tokio::test]
async fn empty_dictation_is_not_offered() {
    let (state, _, _) = test_support::state_with_speech().await;
    let text = add_text(&state, PARAGRAPH).await;
    let s = start(&state, Some(text), None).await.unwrap();
    speak_segment(&state, &s.session_id, 0, false)
        .await
        .unwrap();
    assert!(pending(&state).await.unwrap().is_none());
    assert!(matches!(
        sessions::get(&state.pool, &s.session_id).await,
        Err(AppError::NotFound(_))
    ));
}

/// Abandon : gardée avec son brouillon, ou supprimée si rien n'a été tapé.
#[tokio::test]
async fn abandon_keeps_typed_dictations_only() {
    let (state, _, speech) = test_support::state_with_speech().await;
    let text = add_text(&state, PARAGRAPH).await;
    let kept = start(&state, Some(text.clone()), None).await.unwrap();
    abandon(&state, &kept.session_id, "En 1815", 3_000)
        .await
        .unwrap();
    let row = sessions::get(&state.pool, &kept.session_id).await.unwrap();
    assert!(row.abandoned_at.is_some());
    assert!(speech.stops.load(std::sync::atomic::Ordering::SeqCst) >= 1);
    let empty = start(&state, Some(text), None).await.unwrap();
    abandon(&state, &empty.session_id, "  ", 0).await.unwrap();
    assert!(matches!(
        sessions::get(&state.pool, &empty.session_id).await,
        Err(AppError::NotFound(_))
    ));
    // Plus rien à reprendre, plus rien à corriger.
    assert!(pending(&state).await.unwrap().is_none());
    assert!(finish(&state, &kept.session_id, "x", 1).await.is_err());
}

/// En cours de dictée, la voix change ; le découpage, lui, reste figé.
#[tokio::test]
async fn update_keeps_segmentation() {
    let (state, _, speech) = test_support::state_with_speech().await;
    let text = add_text(&state, PARAGRAPH).await;
    let s = start(&state, Some(text), None).await.unwrap();
    let updated = update_settings(
        &state,
        &s.session_id,
        DictationSettings {
            voice: Some("fake-fr".into()),
            segmentation: SegmentMode::Word,
            pause_ms: 3_000,
            ..DictationSettings::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.segmentation, SegmentMode::Group);
    assert_eq!(updated.pause_ms, 3_000);
    speak_segment(&state, &s.session_id, 0, false)
        .await
        .unwrap();
    let spoken = speech.spoken.lock().unwrap().clone();
    assert_eq!(spoken[0].1.voice.as_deref(), Some("fake-fr"));
    // Après redémarrage, les réglages changés sont relus depuis la session.
    state.take_dictation(&s.session_id);
    assert_eq!(
        resume(&state, &s.session_id)
            .await
            .unwrap()
            .settings
            .pause_ms,
        3_000
    );
}

/// Sans texte dans la bibliothèque, ou sur un texte sans mot, la dictée est refusée.
#[tokio::test]
async fn refuses_missing_or_wordless_texts() {
    let (state, _, _) = test_support::state_with_speech().await;
    assert!(matches!(
        start(&state, None, None).await,
        Err(AppError::NotFound(_))
    ));
    let text = add_text(&state, "… !").await;
    assert!(matches!(
        start(&state, Some(text.clone()), None).await,
        Err(AppError::Invalid(_))
    ));
    assert!(matches!(
        start(&state, Some(text), Some(9)).await,
        Err(AppError::Invalid(_))
    ));
}
