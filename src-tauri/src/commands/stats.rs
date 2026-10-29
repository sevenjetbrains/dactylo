//! Commandes du module Statistiques : tableau de bord (F-33), carte de chaleur
//! et bigrammes (F-34), sessions récentes et export CSV (F-35).

use dactylo_core::stats::csv::{CsvFormat, CsvSession, sessions_csv};
use dactylo_core::stats::dashboard::{Dashboard, Period, dashboard};
use dactylo_core::stats::keys::{KeyAnalyzer, KeyReport};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::now_ms;
use crate::db::stats::{self, SessionListRow};
use crate::error::AppResult;
use crate::state::AppState;

/// Bigrammes lents affichés (maquette : 5).
const TOP_BIGRAMS: usize = 5;

/// Occurrences minimales d'un bigramme pour que sa moyenne ait un sens.
const MIN_BIGRAM_COUNT: u32 = 5;

/// Mode d'une session ; valeurs identiques à la contrainte `sessions.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrainingMode {
    /// Copie d'un texte.
    Copy,
    /// Dictée (Phase 4).
    Dictation,
    /// Révision des mots difficiles.
    Review,
}

impl TrainingMode {
    /// Lit la colonne `sessions.mode` ; une valeur inconnue vaut `Copy`.
    fn parse(value: &str) -> Self {
        match value {
            "dictation" => TrainingMode::Dictation,
            "review" => TrainingMode::Review,
            _ => TrainingMode::Copy,
        }
    }
}

/// Ligne du tableau « Sessions récentes » de l'accueil.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RecentSession {
    /// Identifiant de la session.
    pub session_id: String,
    /// Début, en ms Unix.
    pub started_at: i64,
    /// Titre du texte ; `None` pour une révision ou un texte supprimé.
    pub title: Option<String>,
    /// Mode de la session.
    pub mode: TrainingMode,
    /// MPM net.
    pub wpm_net: f64,
    /// Précision, en %.
    pub accuracy: f64,
}

/// Libellés traduits de l'export CSV (NF-08) : l'interface les fournit.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CsvLabels {
    /// En-têtes des 10 colonnes : date, texte, mode, état, durée (s), MPM brut,
    /// MPM net, CPM, précision (%), erreurs.
    pub headers: Vec<String>,
    /// Libellé du mode copie.
    pub copy: String,
    /// Libellé du mode dictée.
    pub dictation: String,
    /// Libellé du mode révision.
    pub review: String,
    /// Libellé d'une session terminée.
    pub completed: String,
    /// Libellé d'une session abandonnée.
    pub abandoned: String,
    /// Virgule décimale (français) plutôt que point.
    pub decimal_comma: bool,
}

/// Début de la période en ms, ou le tout début pour « tout ».
fn period_start(period: Period, tz_offset_min: i32) -> i64 {
    period.start(now_ms(), tz_offset_min).unwrap_or(i64::MIN)
}

/// Tableau de bord du profil actif sur la période (F-33).
pub async fn compute_dashboard(
    state: &AppState,
    period: Period,
    tz_offset_min: i32,
) -> AppResult<Dashboard> {
    let samples = stats::samples(&state.pool, &state.profile_id).await?;
    Ok(dashboard(&samples, period, now_ms(), tz_offset_min))
}

/// Carte de chaleur et bigrammes lents sur la période (F-34).
pub async fn compute_keys(
    state: &AppState,
    period: Period,
    tz_offset_min: i32,
) -> AppResult<KeyReport> {
    let from = period_start(period, tz_offset_min);
    let sessions = stats::keystrokes_since(&state.pool, &state.profile_id, from).await?;
    // Chaque session est analysée à part : les temps ne se comparent qu'en son sein.
    let mut analyzer = KeyAnalyzer::new();
    for keystrokes in &sessions {
        analyzer.add_session(keystrokes);
    }
    Ok(analyzer.report(TOP_BIGRAMS, MIN_BIGRAM_COUNT))
}

/// Les `limit` dernières sessions terminées.
pub async fn recent(state: &AppState, limit: u32) -> AppResult<Vec<RecentSession>> {
    let rows = stats::recent(&state.pool, &state.profile_id, limit).await?;
    Ok(rows
        .into_iter()
        .map(|r| RecentSession {
            session_id: r.id,
            started_at: r.started_at,
            title: r.title,
            mode: TrainingMode::parse(&r.mode),
            wpm_net: r.wpm_net.unwrap_or(0.0),
            accuracy: r.accuracy.unwrap_or(0.0),
        })
        .collect())
}

/// Fichier CSV des sessions de la période (F-35).
pub async fn export_csv(
    state: &AppState,
    period: Period,
    tz_offset_min: i32,
    labels: CsvLabels,
) -> AppResult<String> {
    let from = period_start(period, tz_offset_min);
    let rows = stats::since(&state.pool, &state.profile_id, from).await?;
    // Mode et état traduits avec les libellés de l'interface.
    let to_csv = |r: SessionListRow| CsvSession {
        started_at: r.started_at,
        title: r.title,
        mode: match TrainingMode::parse(&r.mode) {
            TrainingMode::Copy => labels.copy.clone(),
            TrainingMode::Dictation => labels.dictation.clone(),
            TrainingMode::Review => labels.review.clone(),
        },
        status: if r.completed {
            labels.completed.clone()
        } else {
            labels.abandoned.clone()
        },
        duration_ms: r.duration_ms.unwrap_or(0),
        wpm_raw: r.wpm_raw,
        wpm_net: r.wpm_net,
        cpm: r.cpm,
        accuracy: r.accuracy,
        errors: r.errors,
    };
    let sessions: Vec<CsvSession> = rows.into_iter().map(to_csv).collect();
    let format = CsvFormat {
        headers: labels.headers.clone(),
        decimal_comma: labels.decimal_comma,
        tz_offset_min,
    };
    Ok(sessions_csv(&sessions, &format))
}

/// Commande : tableau de bord sur la période (F-33).
#[tauri::command]
#[specta::specta]
pub async fn stats_dashboard(
    state: State<'_, AppState>,
    period: Period,
    tz_offset_minutes: i32,
) -> AppResult<Dashboard> {
    compute_dashboard(&state, period, tz_offset_minutes).await
}

/// Commande : carte de chaleur et bigrammes lents (F-34).
#[tauri::command]
#[specta::specta]
pub async fn key_report(
    state: State<'_, AppState>,
    period: Period,
    tz_offset_minutes: i32,
) -> AppResult<KeyReport> {
    compute_keys(&state, period, tz_offset_minutes).await
}

/// Commande : dernières sessions terminées (accueil).
#[tauri::command]
#[specta::specta]
pub async fn recent_sessions(
    state: State<'_, AppState>,
    limit: u32,
) -> AppResult<Vec<RecentSession>> {
    recent(&state, limit).await
}

/// Commande : export CSV des sessions de la période (F-35).
#[tauri::command]
#[specta::specta]
pub async fn export_sessions_csv(
    state: State<'_, AppState>,
    period: Period,
    tz_offset_minutes: i32,
    labels: CsvLabels,
) -> AppResult<String> {
    export_csv(&state, period, tz_offset_minutes, labels).await
}

/// Tests des commandes de statistiques (dans `stats/tests.rs`).
#[cfg(test)]
mod tests;
