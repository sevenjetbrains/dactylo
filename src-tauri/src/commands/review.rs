//! Commandes du module Révision : suivi des mots difficiles (F-40 à F-42),
//! révision du jour (F-43), exercices générés (F-44) et gestion manuelle (F-45).
//!
//! Le module ne dépend pas du mode Copie : il reçoit les mots de chaque session
//! par l'événement `SessionCompleted` (ADR 0015) et publie `WordsUpdated`.

use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use dactylo_core::days::end_of_day;
use dactylo_core::events::{DomainEvent, EventBus, EventSubscriber};
use dactylo_core::srs::exercises::{
    ExerciseGenerator, RepeatedWords, ReviewExerciseKind, SentencesContaining, ShuffledWords,
};
use dactylo_core::srs::words::{SessionReview, WordRecord};
use dactylo_core::srs::{ReviewSettings, Sm2};
use dactylo_core::text::{graphemes, normalize, word_spans};
use dactylo_core::typing::{ErrorType, SessionWords, WordAttempt, dominant_error};
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

use crate::db::{now_ms, profiles, texts, words};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Clé des réglages du module dans `profiles.settings` (ADR 0010).
const SETTINGS_KEY: &str = "review";

/// Mots au plus dans un exercice de révision : au-delà, la séance devient une corvée.
pub const MAX_REVIEW_WORDS: usize = 20;

/// Planificateur utilisé (ADR 0016).
static SCHEDULER: Sm2 = Sm2;

/// Vue d'ensemble de la révision : carte de l'accueil et badge du menu (F-43).
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ReviewOverview {
    /// Mots à réviser aujourd'hui, du plus en retard au plus récent.
    pub due: Vec<String>,
    /// Mots en révision, dus ou non.
    pub tracked: u32,
    /// Mots maîtrisés (automatiquement ou à la main).
    pub mastered: u32,
}

/// Ligne du tableau « Mots difficiles » (F-41).
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DifficultWord {
    /// Mot (clé en minuscules).
    pub word: String,
    /// Dernières formes fautives distinctes, la plus récente d'abord.
    pub typed_forms: Vec<String>,
    /// Type de faute le plus fréquent.
    pub dominant_error: Option<ErrorType>,
    /// Nombre d'erreurs.
    pub errors: u32,
    /// Nombre de rencontres.
    pub encounters: u32,
    /// Sessions réussies d'affilée (sur 3 pour la maîtrise).
    pub streak: u32,
    /// Dernière faute, en ms Unix.
    pub last_error_at: Option<i64>,
    /// Prochaine révision, en ms Unix.
    pub next_review_at: Option<i64>,
}

/// Lit les réglages du profil actif ; défauts si absents.
pub async fn load_settings(pool: &SqlitePool, profile_id: &str) -> AppResult<ReviewSettings> {
    let value = profiles::get_settings(pool, profile_id, SETTINGS_KEY).await?;
    Ok(ReviewSettings::from_json(value.as_ref()))
}

/// Borne puis enregistre les réglages ; renvoie la version réellement stockée.
pub async fn store_settings(
    state: &AppState,
    settings: ReviewSettings,
) -> AppResult<ReviewSettings> {
    let settings = settings.sanitized();
    let value = serde_json::to_value(settings).map_err(|e| AppError::Invalid(e.to_string()))?;
    profiles::save_settings(&state.pool, &state.profile_id, SETTINGS_KEY, &value).await?;
    Ok(settings)
}

/// Applique les mots d'une session au suivi du profil, en une transaction :
/// chaque mot est noté une fois (ADR 0016) et chaque tentative est enregistrée.
pub async fn apply_session(
    pool: &SqlitePool,
    profile_id: &str,
    session_id: &str,
    session: &SessionWords,
    now_ms: i64,
) -> AppResult<()> {
    let settings = load_settings(pool, profile_id).await?;
    let review = SessionReview {
        scheduler: &SCHEDULER,
        ms_per_char: session.ms_per_char,
        slow_factor: settings.slow_factor,
        now_ms,
    };
    // Tentatives regroupées par mot : un mot répété n'est noté qu'une fois.
    let mut by_word: BTreeMap<&str, Vec<&WordAttempt>> = BTreeMap::new();
    for a in &session.attempts {
        by_word.entry(a.word.as_str()).or_default().push(a);
    }
    // Tout ou rien : un mot à moitié mis à jour fausserait sa planification.
    let mut tx = pool.begin().await?;
    for (word, attempts) in by_word {
        let existing = words::find(&mut tx, profile_id, word).await?;
        let (id, mut record) = match existing {
            Some((id, record)) => (Some(id), record),
            None => (None, WordRecord::new(word)),
        };
        record.apply_session(&attempts, &review);
        let id = words::save(&mut tx, profile_id, id.as_deref(), &record).await?;
        for a in attempts {
            words::insert_attempt(&mut tx, &id, session_id, a, now_ms).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Abonné du bus : met à jour les mots à chaque fin de session, en tâche de fond,
/// puis publie `WordsUpdated`.
pub struct ReviewSubscriber {
    /// Base de données.
    pool: SqlitePool,
    /// Bus, pour publier `WordsUpdated` ; faible, car le bus garde l'abonné.
    bus: Weak<EventBus>,
}

impl ReviewSubscriber {
    /// Abonné qui écrit dans `pool` et publie sur `bus`.
    pub fn new(pool: SqlitePool, bus: &Arc<EventBus>) -> Self {
        Self {
            pool,
            bus: Arc::downgrade(bus),
        }
    }
}

impl EventSubscriber for ReviewSubscriber {
    /// Seule la fin de session intéresse la révision.
    fn on_event(&self, event: &DomainEvent) {
        let DomainEvent::SessionCompleted {
            session_id,
            profile_id,
            words,
        } = event
        else {
            return;
        };
        // Le bus est synchrone : l'écriture en base part dans une tâche à part.
        let (pool, bus) = (self.pool.clone(), self.bus.clone());
        let (session_id, profile_id, words) =
            (session_id.clone(), profile_id.clone(), words.clone());
        tauri::async_runtime::spawn(async move {
            match apply_session(&pool, &profile_id, &session_id, &words, now_ms()).await {
                Ok(()) => {
                    // L'interface rafraîchit le badge et la carte de l'accueil.
                    if let Some(bus) = bus.upgrade() {
                        bus.publish(&DomainEvent::WordsUpdated { profile_id });
                    }
                }
                // Rien à renvoyer à l'interface : la session est déjà enregistrée.
                Err(e) => eprintln!("révision : session {session_id} non prise en compte : {e}"),
            }
        });
    }
}

/// Vue d'ensemble : mots dus avant le prochain minuit local, suivis et maîtrisés.
pub async fn overview(state: &AppState, tz_offset_min: i32) -> AppResult<ReviewOverview> {
    let before = end_of_day(now_ms(), tz_offset_min);
    let due = words::due(&state.pool, &state.profile_id, before).await?;
    let (tracked, mastered) = words::counts(&state.pool, &state.profile_id).await?;
    Ok(ReviewOverview {
        due,
        tracked,
        mastered,
    })
}

/// Mots en révision, du plus urgent au moins urgent, avec leur détail (F-41).
pub async fn difficult(state: &AppState) -> AppResult<Vec<DifficultWord>> {
    let tracked = words::tracked(&state.pool, &state.profile_id).await?;
    Ok(tracked
        .into_iter()
        .map(|w| DifficultWord {
            dominant_error: dominant_error(&w.error_types),
            typed_forms: w.typed_forms,
            word: w.record.word,
            errors: w.record.errors,
            encounters: w.record.encounters,
            streak: w.record.streak,
            last_error_at: w.record.last_error_at,
            next_review_at: w.record.next_review_at,
        })
        .collect())
}

/// Clé d'un mot saisi à la main : un seul mot, en minuscules NFC.
fn word_key(input: &str) -> AppResult<String> {
    // `normalize` ne retire que les espaces de fin de ligne : on retire aussi celles du début.
    let key = normalize(input.trim()).to_lowercase();
    let g = graphemes(&key);
    // Exactement un mot, qui couvre toute la saisie.
    match word_spans(&g).as_slice() {
        [span] if span.start == 0 && span.end as usize == g.len() => Ok(key),
        _ => Err(AppError::Invalid(format!(
            "« {} » n'est pas un mot",
            input.trim()
        ))),
    }
}

/// Publie `WordsUpdated` pour le profil actif.
fn notify(state: &AppState) {
    state.bus.publish(&DomainEvent::WordsUpdated {
        profile_id: state.profile_id.clone(),
    });
}

/// Ajoute un mot à la révision (F-45), à réviser tout de suite. Un mot déjà connu
/// garde son historique et revient en révision.
pub async fn add(state: &AppState, input: &str) -> AppResult<String> {
    let key = word_key(input)?;
    let now = now_ms();
    let mut conn = state.pool.acquire().await?;
    // Mot connu : il garde ses compteurs ; sinon, nouveau mot ajouté à la main.
    let (id, record) = match words::find(&mut conn, &state.profile_id, &key).await? {
        Some((id, mut record)) => {
            record.track_now(now);
            (Some(id), record)
        }
        None => (None, WordRecord::added(key.clone(), now)),
    };
    words::save(&mut conn, &state.profile_id, id.as_deref(), &record).await?;
    drop(conn);
    notify(state);
    Ok(key)
}

/// Retire un mot de la liste (F-45) : il est oublié avec ses tentatives.
pub async fn remove(state: &AppState, word: &str) -> AppResult<()> {
    words::delete(&state.pool, &state.profile_id, &word_key(word)?).await?;
    notify(state);
    Ok(())
}

/// Marque un mot maîtrisé à la main (F-45) : il sort de la révision.
pub async fn mark_mastered(state: &AppState, word: &str) -> AppResult<()> {
    let key = word_key(word)?;
    let mut conn = state.pool.acquire().await?;
    let (id, mut record) = words::find(&mut conn, &state.profile_id, &key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("mot « {key} »")))?;
    record.mark_mastered();
    words::save(&mut conn, &state.profile_id, Some(&id), &record).await?;
    drop(conn);
    notify(state);
    Ok(())
}

/// Texte d'un exercice de révision (F-44) sur les mots dus aujourd'hui, ou à défaut
/// les mots suivis les plus urgents ; vide s'il n'y a rien à réviser ou, pour les
/// phrases, si aucune phrase de la bibliothèque ne contient ces mots.
pub async fn exercise(
    state: &AppState,
    kind: ReviewExerciseKind,
    tz_offset_min: i32,
) -> AppResult<String> {
    // Les mots dus d'abord ; sinon les prochains à réviser, pour s'avancer.
    let mut list = overview(state, tz_offset_min).await?.due;
    if list.is_empty() {
        list = words::tracked(&state.pool, &state.profile_id)
            .await?
            .into_iter()
            .map(|w| w.record.word)
            .collect();
    }
    generate(state, list, kind).await
}

/// Texte d'un exercice sur des mots donnés (« S'entraîner sur ces mots » après une
/// dictée) ; les mots qui n'en sont pas sont ignorés.
pub async fn exercise_for_words(
    state: &AppState,
    words: &[String],
    kind: ReviewExerciseKind,
) -> AppResult<String> {
    let mut list: Vec<String> = words.iter().filter_map(|w| word_key(w).ok()).collect();
    list.dedup();
    generate(state, list, kind).await
}

/// Génère l'exercice de la forme demandée sur au plus `MAX_REVIEW_WORDS` mots.
async fn generate(
    state: &AppState,
    mut list: Vec<String>,
    kind: ReviewExerciseKind,
) -> AppResult<String> {
    list.truncate(MAX_REVIEW_WORDS);
    // La graine change à chaque exercice : l'ordre n'est jamais le même.
    let seed = now_ms() as u64;
    let content = match kind {
        ReviewExerciseKind::Shuffled => ShuffledWords { seed }.generate(&list),
        ReviewExerciseKind::Repeated { times } => RepeatedWords { times, seed }.generate(&list),
        ReviewExerciseKind::Sentences => {
            let library = texts::contents(&state.pool).await?;
            SentencesContaining { texts: &library }.generate(&list)
        }
    };
    Ok(content)
}

/// Commande : réglages du module Révision.
#[tauri::command]
#[specta::specta]
pub async fn get_review_settings(state: State<'_, AppState>) -> AppResult<ReviewSettings> {
    load_settings(&state.pool, &state.profile_id).await
}

/// Commande : enregistre les réglages du module Révision.
#[tauri::command]
#[specta::specta]
pub async fn save_review_settings(
    state: State<'_, AppState>,
    settings: ReviewSettings,
) -> AppResult<ReviewSettings> {
    store_settings(&state, settings).await
}

/// Commande : mots à réviser aujourd'hui, suivis et maîtrisés (F-43).
#[tauri::command]
#[specta::specta]
pub async fn review_overview(
    state: State<'_, AppState>,
    tz_offset_minutes: i32,
) -> AppResult<ReviewOverview> {
    overview(&state, tz_offset_minutes).await
}

/// Commande : tableau des mots difficiles (F-41).
#[tauri::command]
#[specta::specta]
pub async fn difficult_words(state: State<'_, AppState>) -> AppResult<Vec<DifficultWord>> {
    difficult(&state).await
}

/// Commande : ajoute un mot à la révision ; renvoie sa forme normalisée (F-45).
#[tauri::command]
#[specta::specta]
pub async fn add_word(state: State<'_, AppState>, word: String) -> AppResult<String> {
    add(&state, &word).await
}

/// Commande : retire un mot de la liste (F-45).
#[tauri::command]
#[specta::specta]
pub async fn remove_word(state: State<'_, AppState>, word: String) -> AppResult<()> {
    remove(&state, &word).await
}

/// Commande : marque un mot maîtrisé (F-45).
#[tauri::command]
#[specta::specta]
pub async fn mark_word_mastered(state: State<'_, AppState>, word: String) -> AppResult<()> {
    mark_mastered(&state, &word).await
}

/// Commande : texte d'un exercice de révision (F-44).
#[tauri::command]
#[specta::specta]
pub async fn review_exercise(
    state: State<'_, AppState>,
    kind: ReviewExerciseKind,
    tz_offset_minutes: i32,
) -> AppResult<String> {
    exercise(&state, kind, tz_offset_minutes).await
}

/// Commande : texte d'un exercice sur des mots donnés (F-44).
#[tauri::command]
#[specta::specta]
pub async fn words_exercise(
    state: State<'_, AppState>,
    words: Vec<String>,
    kind: ReviewExerciseKind,
) -> AppResult<String> {
    exercise_for_words(&state, &words, kind).await
}

/// Tests des commandes du module Révision (dans `review/tests.rs`).
#[cfg(test)]
mod tests;
