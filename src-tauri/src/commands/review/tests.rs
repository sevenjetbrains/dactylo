//! Tests des commandes du module Révision, sur une base en mémoire.

use std::time::Duration;

use dactylo_core::days::DAY_MS;
use dactylo_core::typing::{KeyKind, Keystroke};

use super::*;
use crate::commands::library::{self, NewText};
use crate::commands::practice;
use crate::test_support;

/// Ajoute un texte à la bibliothèque et renvoie son identifiant.
async fn add_text(state: &AppState, content: &str) -> String {
    library::create(
        state,
        NewText {
            title: "Texte".into(),
            content: content.into(),
            language: "fr".into(),
            ..NewText::default()
        },
    )
    .await
    .unwrap()
    .id
}

/// Insertions régulières (100 ms) de chaque graphème de `typed`.
fn keys(typed: &str) -> Vec<Keystroke> {
    graphemes(typed)
        .into_iter()
        .enumerate()
        .map(|(i, g)| Keystroke {
            seq: i as u32,
            kind: KeyKind::Insert,
            typed: Some(g),
            t_ms: 100 * i as u32,
        })
        .collect()
}

/// Joue une session complète de copie ; renvoie son identifiant et les mots du
/// `SessionCompleted` publié.
async fn play(
    state: &AppState,
    recorder: &test_support::Recorder,
    text: &str,
    typed: &str,
) -> (String, SessionWords) {
    let text_id = add_text(state, text).await;
    let start = practice::start(state, Some(text_id), None, None)
        .await
        .unwrap();
    practice::record(state, &start.session_id, &keys(typed))
        .await
        .unwrap();
    practice::finish(state, &start.session_id, 5_000)
        .await
        .unwrap();
    // Les mots de la session sont ceux de l'événement de fin.
    let words = recorder
        .events()
        .into_iter()
        .find_map(|e| match e {
            DomainEvent::SessionCompleted {
                session_id, words, ..
            } if session_id == start.session_id => Some(words),
            _ => None,
        })
        .expect("SessionCompleted publié");
    (start.session_id, words)
}

/// Tentatives d'un seul mot, justes ou fausses, à vitesse normale.
fn single(word: &str, correct: bool) -> SessionWords {
    SessionWords {
        attempts: vec![WordAttempt {
            word: word.into(),
            expected: word.into(),
            typed: if correct {
                word.into()
            } else {
                format!("{word}x")
            },
            correct,
            error_type: (!correct).then_some(ErrorType::Substitution),
            position: 0,
            duration_ms: 400,
            timed_chars: 4,
        }],
        ms_per_char: 100.0,
    }
}

/// Applique des mots de session au profil actif, à l'instant `at`.
async fn apply_at(state: &AppState, session_id: &str, words: SessionWords, at: i64) {
    apply_session(&state.pool, &state.profile_id, session_id, &words, at)
        .await
        .unwrap();
}

/// Nombre de mots dus avant l'instant `before`.
async fn due_count(state: &AppState, before: i64) -> usize {
    words::due(&state.pool, &state.profile_id, before)
        .await
        .unwrap()
        .len()
}

/// Une faute de copie rend le mot à réviser (le lendemain de la faute), avec sa
/// forme fautive et son type ; un mot juste n'est pas suivi.
#[tokio::test]
async fn mistyped_word_becomes_due_with_details() {
    let (state, recorder) = test_support::state().await;
    let (session_id, words) = play(&state, &recorder, "la forêt", "la foret").await;
    // La faute date d'hier : le mot est à réviser aujourd'hui.
    apply_session(
        &state.pool,
        &state.profile_id,
        &session_id,
        &words,
        now_ms() - DAY_MS,
    )
    .await
    .unwrap();
    let o = overview(&state, 0).await.unwrap();
    assert_eq!(o.due, ["forêt"]);
    assert_eq!((o.tracked, o.mastered), (1, 0));
    // Détail du mot pour le tableau des mots difficiles.
    let list = difficult(&state).await.unwrap();
    assert_eq!(list.len(), 1);
    let w = &list[0];
    assert_eq!(w.word, "forêt");
    assert_eq!(w.typed_forms, ["foret"]);
    assert_eq!(w.dominant_error, Some(ErrorType::Accent));
    assert_eq!((w.errors, w.encounters, w.streak), (1, 1, 0));
    // « la » a été rencontré mais n'est pas difficile.
    let mut conn = state.pool.acquire().await.unwrap();
    let (_, la) = words::find(&mut conn, &state.profile_id, "la")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(la.encounters, 1);
    assert!(!la.is_tracked());
}

/// Critère transverse : un mot raté apparaît dans les révisions du lendemain et
/// disparaît après 3 réussites.
#[tokio::test]
async fn mistyped_word_is_due_tomorrow_and_leaves_after_three_successes() {
    let (state, recorder) = test_support::state().await;
    let (session_id, _) = play(&state, &recorder, "mot", "mot").await;
    let day = |n: i64| now_ms() - 30 * DAY_MS + n * DAY_MS;
    // Jour 0 : faute ; à réviser avant la fin du jour 1, pas du jour 0.
    apply_at(&state, &session_id, single("accueillir", false), day(0)).await;
    assert_eq!(due_count(&state, day(0) + 1).await, 0);
    assert_eq!(due_count(&state, day(1) + 1).await, 1);
    // Trois sessions réussies, chacune à sa date de révision.
    for n in [1, 2, 8] {
        apply_at(&state, &session_id, single("accueillir", true), day(n)).await;
    }
    let o = overview(&state, 0).await.unwrap();
    assert!(o.due.is_empty());
    assert_eq!((o.tracked, o.mastered), (0, 1));
    assert!(difficult(&state).await.unwrap().is_empty());
}

/// Le seuil de lenteur réglé décide si un mot lent devient difficile (F-40).
#[tokio::test]
async fn slow_threshold_comes_from_settings() {
    let (state, recorder) = test_support::state().await;
    let (session_id, _) = play(&state, &recorder, "mot", "mot").await;
    // 4 caractères en 1 000 ms pour une moyenne de 100 ms : 2,5 fois plus lent.
    let mut slow = single("néanmoins", true);
    slow.attempts[0].duration_ms = 1_000;
    // Seuil 3 : pas assez lent pour être suivi.
    store_settings(&state, ReviewSettings { slow_factor: 3.0 })
        .await
        .unwrap();
    apply_session(&state.pool, &state.profile_id, &session_id, &slow, now_ms())
        .await
        .unwrap();
    assert_eq!(overview(&state, 0).await.unwrap().tracked, 0);
    // Seuil 2 (et valeur hors bornes ramenée) : il l'est.
    let saved = store_settings(&state, ReviewSettings { slow_factor: 2.0 })
        .await
        .unwrap();
    assert_eq!(saved.slow_factor, 2.0);
    apply_session(&state.pool, &state.profile_id, &session_id, &slow, now_ms())
        .await
        .unwrap();
    assert_eq!(overview(&state, 0).await.unwrap().tracked, 1);
    let bounded = store_settings(&state, ReviewSettings { slow_factor: 0.0 })
        .await
        .unwrap();
    assert_eq!(bounded.slow_factor, dactylo_core::srs::MIN_SLOW_FACTOR);
}

/// Ajouter, marquer maîtrisé et retirer un mot ; chaque changement est publié.
#[tokio::test]
async fn manual_word_management() {
    let (state, recorder) = test_support::state().await;
    // Ajout : normalisé et à réviser tout de suite.
    assert_eq!(add(&state, "  Parallèle ").await.unwrap(), "parallèle");
    assert_eq!(overview(&state, 0).await.unwrap().due, ["parallèle"]);
    // Saisie invalide : vide, deux mots, ou ponctuation.
    for bad in ["", "deux mots", "!!", "l'eau"] {
        assert!(matches!(add(&state, bad).await, Err(AppError::Invalid(_))));
    }
    // Maîtrisé à la main : il sort de la révision.
    mark_mastered(&state, "parallèle").await.unwrap();
    let o = overview(&state, 0).await.unwrap();
    assert!(o.due.is_empty());
    assert_eq!(o.mastered, 1);
    // Rajouté : il revient en révision.
    add(&state, "parallèle").await.unwrap();
    assert_eq!(overview(&state, 0).await.unwrap().tracked, 1);
    // Retiré : il disparaît ; le retirer deux fois est une erreur.
    remove(&state, "parallèle").await.unwrap();
    assert_eq!(overview(&state, 0).await.unwrap().tracked, 0);
    assert!(matches!(
        remove(&state, "parallèle").await,
        Err(AppError::NotFound(_))
    ));
    assert!(matches!(
        mark_mastered(&state, "inconnu").await,
        Err(AppError::NotFound(_))
    ));
    // Quatre changements réussis, quatre `WordsUpdated`.
    let updates = recorder
        .events()
        .into_iter()
        .filter(|e| matches!(e, DomainEvent::WordsUpdated { .. }))
        .count();
    assert_eq!(updates, 4);
}

/// Exercices générés sur les mots dus, ou à défaut les mots suivis.
#[tokio::test]
async fn exercises_use_due_then_tracked_words() {
    let (state, _) = test_support::state().await;
    // Rien à réviser : exercice vide.
    assert_eq!(
        exercise(&state, ReviewExerciseKind::Shuffled, 0)
            .await
            .unwrap(),
        ""
    );
    add(&state, "œuvre").await.unwrap();
    add(&state, "vieillard").await.unwrap();
    // Mélange : chaque mot une fois.
    let shuffled = exercise(&state, ReviewExerciseKind::Shuffled, 0)
        .await
        .unwrap();
    let mut got: Vec<&str> = shuffled.split(' ').collect();
    got.sort_unstable();
    assert_eq!(got, ["vieillard", "œuvre"]);
    // Répétition : 2 fois chaque mot.
    let repeated = exercise(&state, ReviewExerciseKind::Repeated { times: 2 }, 0)
        .await
        .unwrap();
    assert_eq!(repeated.split(' ').count(), 4);
    // Phrases : prises dans la bibliothèque.
    add_text(&state, "Le vieillard dort. Le chat aussi.").await;
    let sentences = exercise(&state, ReviewExerciseKind::Sentences, 0)
        .await
        .unwrap();
    assert_eq!(sentences, "Le vieillard dort.");
}

/// Sans mot dû aujourd'hui, l'exercice prend les prochains mots à réviser.
#[tokio::test]
async fn exercise_falls_back_to_upcoming_words() {
    let (state, recorder) = test_support::state().await;
    let (session_id, _) = play(&state, &recorder, "mot", "mot").await;
    // Faute maintenant : à réviser demain, donc pas due aujourd'hui.
    apply_session(
        &state.pool,
        &state.profile_id,
        &session_id,
        &single("demain", false),
        now_ms(),
    )
    .await
    .unwrap();
    assert!(overview(&state, 0).await.unwrap().due.is_empty());
    assert_eq!(
        exercise(&state, ReviewExerciseKind::Shuffled, 0)
            .await
            .unwrap(),
        "demain"
    );
}

/// L'abonné met à jour les mots en tâche de fond, puis publie `WordsUpdated`.
#[tokio::test]
async fn subscriber_applies_completed_sessions() {
    let (state, recorder) = test_support::state().await;
    state.bus.subscribe(Arc::new(ReviewSubscriber::new(
        state.pool.clone(),
        &state.bus,
    )));
    let text_id = add_text(&state, "une fête").await;
    let start = practice::start(&state, Some(text_id), None, None)
        .await
        .unwrap();
    practice::record(&state, &start.session_id, &keys("une fete"))
        .await
        .unwrap();
    practice::finish(&state, &start.session_id, 2_000)
        .await
        .unwrap();
    // La tâche de fond finit par publier `WordsUpdated` ; on l'attend au plus 5 s.
    let mut updated = false;
    for _ in 0..100 {
        updated = recorder
            .events()
            .iter()
            .any(|e| matches!(e, DomainEvent::WordsUpdated { .. }));
        if updated {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(updated, "WordsUpdated publié");
    assert_eq!(overview(&state, 0).await.unwrap().tracked, 1);
}

/// Seule une fin de session déclenche une mise à jour.
#[tokio::test]
async fn subscriber_ignores_other_events() {
    let (state, recorder) = test_support::state().await;
    let subscriber = ReviewSubscriber::new(state.pool.clone(), &state.bus);
    subscriber.on_event(&DomainEvent::TextImported {
        text_id: "t".into(),
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(recorder.events().is_empty());
}
