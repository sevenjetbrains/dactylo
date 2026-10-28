//! Commandes du mode Copie : réglages, session, frappes, fin et reprise
//! (F-01 à F-07, F-30 à F-32, NF-06).

use dactylo_core::events::DomainEvent;
use dactylo_core::settings::PracticeSettings;
use dactylo_core::stats::{self, AverageStats, SessionStats};
use dactylo_core::text::{WordSpan, graphemes, wrap};
use dactylo_core::typing::{Keystroke, TypingError, TypingSession};
use serde::Serialize;
use tauri::State;

use crate::db::{profiles, sessions, texts};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, LiveSession};

/// Clé des réglages du module dans `profiles.settings` (ADR 0010).
const SETTINGS_KEY: &str = "practice";
/// Nombre de sessions de la moyenne affichée en fin de session (F-32).
const AVERAGE_OVER: u32 = 10;

/// Tout ce qu'il faut à l'interface pour afficher une session.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionStart {
    pub session_id: String,
    pub text_id: String,
    pub title: String,
    /// Texte découpé en graphèmes : les positions des deux côtés coïncident (ADR 0008).
    pub graphemes: Vec<String>,
    pub words: Vec<WordSpan>,
    /// Réglages figés pour la session.
    pub settings: PracticeSettings,
    /// Frappes déjà enregistrées, à rejouer (reprise) ; vide pour une nouvelle session.
    pub keystrokes: Vec<Keystroke>,
    /// Temps actif déjà écoulé.
    pub active_ms: u32,
}

/// Session interrompue proposée à la reprise (NF-06).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PendingSession {
    pub session_id: String,
    pub title: String,
    /// Avancement, de 0 à 1.
    pub progress: f64,
    pub started_at: i64,
}

/// Résumé de l'écran de fin (F-32).
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub text_id: String,
    pub stats: SessionStats,
    pub errors: Vec<TypingError>,
    /// Moyenne des dernières sessions, hors celle-ci ; `None` pour la première.
    pub average: Option<AverageStats>,
}

pub async fn load_settings(state: &AppState) -> AppResult<PracticeSettings> {
    let value = profiles::get_settings(&state.pool, &state.profile_id, SETTINGS_KEY).await?;
    Ok(PracticeSettings::from_json(value.as_ref()))
}

pub async fn store_settings(
    state: &AppState,
    settings: PracticeSettings,
) -> AppResult<PracticeSettings> {
    let settings = settings.sanitized();
    let value = serde_json::to_value(settings).map_err(|e| AppError::Invalid(e.to_string()))?;
    profiles::save_settings(&state.pool, &state.profile_id, SETTINGS_KEY, &value).await?;
    Ok(settings)
}

/// Démarre une session sur `text_id`, ou sur le texte le plus récent.
pub async fn start(state: &AppState, text_id: Option<String>) -> AppResult<SessionStart> {
    let text_id = match text_id {
        Some(id) => id,
        None => texts::latest(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("aucun texte dans la bibliothèque".into()))?,
    };
    let text = texts::get(&state.pool, &text_id).await?;
    let settings = load_settings(state).await?;
    let settings_json =
        serde_json::to_string(&settings).map_err(|e| AppError::Invalid(e.to_string()))?;
    let row = sessions::start(&state.pool, &state.profile_id, &text.id, &settings_json).await?;

    let typing = TypingSession::new(graphemes(&text.content), settings.rules());
    let start = SessionStart {
        session_id: row.id.clone(),
        text_id: text.id.clone(),
        title: text.title,
        graphemes: typing.text().to_vec(),
        words: typing.words().to_vec(),
        settings,
        keystrokes: Vec::new(),
        active_ms: 0,
    };
    state.put_live(
        row.id,
        LiveSession {
            profile_id: state.profile_id.clone(),
            text_id: text.id,
            settings,
            typing,
        },
    );
    Ok(start)
}

/// Session en cours, depuis le cache ou reconstruite depuis la base.
/// Elle est retirée du cache : l'appelant la remet après une écriture réussie.
async fn take_live(state: &AppState, session_id: &str) -> AppResult<LiveSession> {
    if let Some(live) = state.take_live(session_id) {
        return Ok(live);
    }
    let row = sessions::get(&state.pool, session_id).await?;
    if row.ended_at.is_some() {
        return Err(AppError::Invalid(format!(
            "session {session_id} déjà terminée"
        )));
    }
    let text_id = row
        .text_id
        .ok_or_else(|| AppError::NotFound(format!("texte de la session {session_id}")))?;
    let text = texts::get(&state.pool, &text_id).await?;
    let settings_value = serde_json::from_str(&row.settings).ok();
    let settings = PracticeSettings::from_json(settings_value.as_ref());
    let keystrokes = sessions::keystrokes(&state.pool, session_id).await?;
    let typing = TypingSession::replay(graphemes(&text.content), settings.rules(), &keystrokes);
    Ok(LiveSession {
        profile_id: row.profile_id,
        text_id,
        settings,
        typing,
    })
}

/// Enregistre un lot de frappes ; renvoie le dernier numéro accepté.
pub async fn record(
    state: &AppState,
    session_id: &str,
    batch: &[Keystroke],
) -> AppResult<Option<u32>> {
    let mut live = take_live(state, session_id).await?;
    let resolved: Vec<_> = batch.iter().filter_map(|k| live.typing.apply(k)).collect();
    // En cas d'échec, la session n'est pas remise en cache : elle sera rejouée depuis la base.
    sessions::insert_keystrokes(&state.pool, session_id, &resolved).await?;
    let last_seq = live.typing.last_seq();
    state.put_live(session_id.to_owned(), live);
    Ok(last_seq)
}

/// Clôt la session : statistiques, fautes, événements de domaine.
pub async fn finish(
    state: &AppState,
    session_id: &str,
    active_ms: u32,
) -> AppResult<SessionSummary> {
    let live = take_live(state, session_id).await?;
    let mut duration = active_ms.max(live.typing.last_t_ms());
    if let Some(limit) = live.settings.goal.duration_ms() {
        duration = duration.min(limit);
    }
    let session_stats = stats::compute(live.typing.counters(), duration);
    let errors = live.typing.errors();
    sessions::finish(&state.pool, session_id, &session_stats, &errors).await?;

    let recent =
        sessions::recent_stats(&state.pool, &live.profile_id, session_id, AVERAGE_OVER).await?;

    state.bus.publish(&DomainEvent::SessionCompleted {
        session_id: session_id.to_owned(),
        profile_id: live.profile_id.clone(),
    });
    for e in errors.iter().filter(|e| e.word) {
        state.bus.publish(&DomainEvent::WordMistyped {
            session_id: session_id.to_owned(),
            profile_id: live.profile_id.clone(),
            expected: e.expected.clone(),
            typed: e.typed.clone(),
        });
    }
    Ok(SessionSummary {
        session_id: session_id.to_owned(),
        text_id: live.text_id,
        stats: session_stats,
        errors,
        average: stats::average(&recent),
    })
}

pub async fn abandon(state: &AppState, session_id: &str) -> AppResult<()> {
    state.take_live(session_id);
    sessions::delete(&state.pool, session_id).await
}

pub async fn pending(state: &AppState) -> AppResult<Option<PendingSession>> {
    let Some(row) = sessions::pending(&state.pool, &state.profile_id).await? else {
        return Ok(None);
    };
    let live = take_live(state, &row.id).await?;
    // Une session ouverte sans aucune frappe n'a rien à reprendre.
    if live.typing.last_seq().is_none() {
        sessions::delete(&state.pool, &row.id).await?;
        return Ok(None);
    }
    let title = match &row.text_id {
        Some(id) => texts::get(&state.pool, id).await?.title,
        None => String::new(),
    };
    let len = live.typing.text().len().max(1);
    let pending = PendingSession {
        session_id: row.id.clone(),
        title,
        progress: live.typing.position() as f64 / len as f64,
        started_at: row.started_at,
    };
    state.put_live(row.id, live);
    Ok(Some(pending))
}

pub async fn resume(state: &AppState, session_id: &str) -> AppResult<SessionStart> {
    let live = take_live(state, session_id).await?;
    let text = texts::get(&state.pool, &live.text_id).await?;
    let start = SessionStart {
        session_id: session_id.to_owned(),
        text_id: live.text_id.clone(),
        title: text.title,
        graphemes: live.typing.text().to_vec(),
        words: live.typing.words().to_vec(),
        settings: live.settings,
        keystrokes: sessions::keystrokes(&state.pool, session_id).await?,
        active_ms: live.typing.last_t_ms(),
    };
    state.put_live(session_id.to_owned(), live);
    Ok(start)
}

pub async fn line_starts(state: &AppState, session_id: &str, columns: u32) -> AppResult<Vec<u32>> {
    let live = take_live(state, session_id).await?;
    let starts = wrap(live.typing.text(), columns);
    state.put_live(session_id.to_owned(), live);
    Ok(starts)
}

#[tauri::command]
#[specta::specta]
pub async fn get_practice_settings(state: State<'_, AppState>) -> AppResult<PracticeSettings> {
    load_settings(&state).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_practice_settings(
    state: State<'_, AppState>,
    settings: PracticeSettings,
) -> AppResult<PracticeSettings> {
    store_settings(&state, settings).await
}

#[tauri::command]
#[specta::specta]
pub async fn start_session(
    state: State<'_, AppState>,
    text_id: Option<String>,
) -> AppResult<SessionStart> {
    start(&state, text_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn record_keystrokes(
    state: State<'_, AppState>,
    session_id: String,
    keystrokes: Vec<Keystroke>,
) -> AppResult<Option<u32>> {
    record(&state, &session_id, &keystrokes).await
}

#[tauri::command]
#[specta::specta]
pub async fn finish_session(
    state: State<'_, AppState>,
    session_id: String,
    active_ms: u32,
) -> AppResult<SessionSummary> {
    finish(&state, &session_id, active_ms).await
}

#[tauri::command]
#[specta::specta]
pub async fn abandon_session(state: State<'_, AppState>, session_id: String) -> AppResult<()> {
    abandon(&state, &session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn pending_session(state: State<'_, AppState>) -> AppResult<Option<PendingSession>> {
    pending(&state).await
}

#[tauri::command]
#[specta::specta]
pub async fn resume_session(
    state: State<'_, AppState>,
    session_id: String,
) -> AppResult<SessionStart> {
    resume(&state, &session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn wrap_text(
    state: State<'_, AppState>,
    session_id: String,
    columns: u32,
) -> AppResult<Vec<u32>> {
    line_starts(&state, &session_id, columns).await
}

#[tauri::command]
#[specta::specta]
pub async fn next_text_id(
    state: State<'_, AppState>,
    text_id: String,
) -> AppResult<Option<String>> {
    texts::next_after(&state.pool, &text_id).await
}

#[cfg(test)]
mod tests;
