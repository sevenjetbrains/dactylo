//! Lectures pour les statistiques (F-33 à F-35) : sessions et frappes brutes,
//! agrégées ensuite par le cœur (ADR 0017).

use dactylo_core::stats::dashboard::{SessionResult, SessionSample};
use dactylo_core::typing::{KeyKind, ResolvedKeystroke};
use sqlx::SqlitePool;

use crate::error::AppResult;

/// Session terminée ou abandonnée, telle que lue en base.
#[derive(sqlx::FromRow)]
struct SampleRow {
    /// Début, en ms Unix.
    started_at: i64,
    /// Temps actif, en ms (absent pour une session abandonnée sans frappe datée).
    duration_ms: Option<u32>,
    /// Fin : présente pour une session terminée.
    ended_at: Option<i64>,
    /// MPM net.
    wpm_net: Option<f64>,
    /// Précision.
    accuracy: Option<f64>,
}

/// Toutes les sessions terminées ou abandonnées du profil (les en-cours sont exclues).
pub async fn samples(pool: &SqlitePool, profile_id: &str) -> AppResult<Vec<SessionSample>> {
    let rows: Vec<SampleRow> = sqlx::query_as(
        "SELECT started_at, duration_ms, ended_at, wpm_net, accuracy FROM sessions \
         WHERE profile_id = ? AND (ended_at IS NOT NULL OR abandoned_at IS NOT NULL)",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SessionSample {
            started_at: r.started_at,
            duration_ms: r.duration_ms.unwrap_or(0),
            // Seule une session terminée a des statistiques.
            result: match (r.ended_at, r.wpm_net, r.accuracy) {
                (Some(_), Some(wpm_net), Some(accuracy)) => {
                    Some(SessionResult { wpm_net, accuracy })
                }
                _ => None,
            },
        })
        .collect())
}

/// Frappe brute, telle que lue en base.
#[derive(sqlx::FromRow)]
struct KeystrokeRow {
    /// Session de la frappe.
    session_id: String,
    /// Numéro d'ordre.
    seq: u32,
    /// Position dans le texte.
    position: u32,
    /// `insert` ou `delete`.
    kind: String,
    /// Graphème attendu.
    expected: Option<String>,
    /// Graphème tapé.
    typed: Option<String>,
    /// Temps actif, en ms.
    t_ms: u32,
    /// Frappe juste ?
    correct: bool,
}

/// Frappes des sessions du profil commencées depuis `from` (terminées ou
/// abandonnées), regroupées par session, dans l'ordre de frappe.
pub async fn keystrokes_since(
    pool: &SqlitePool,
    profile_id: &str,
    from: i64,
) -> AppResult<Vec<Vec<ResolvedKeystroke>>> {
    let rows: Vec<KeystrokeRow> = sqlx::query_as(
        "SELECT k.session_id, k.seq, k.position, k.kind, k.expected, k.typed, k.t_ms, k.correct \
         FROM keystrokes k JOIN sessions s ON s.id = k.session_id \
         WHERE s.profile_id = ? AND s.started_at >= ? \
         AND (s.ended_at IS NOT NULL OR s.abandoned_at IS NOT NULL) \
         ORDER BY s.started_at, k.session_id, k.seq",
    )
    .bind(profile_id)
    .bind(from)
    .fetch_all(pool)
    .await?;
    // Regroupement : une nouvelle liste à chaque changement de session.
    let mut sessions: Vec<Vec<ResolvedKeystroke>> = Vec::new();
    let mut current: Option<String> = None;
    for r in rows {
        if current.as_deref() != Some(r.session_id.as_str()) {
            sessions.push(Vec::new());
            current = Some(r.session_id.clone());
        }
        let kind = if r.kind == KeyKind::Delete.as_str() {
            KeyKind::Delete
        } else {
            KeyKind::Insert
        };
        if let Some(list) = sessions.last_mut() {
            list.push(ResolvedKeystroke {
                seq: r.seq,
                position: r.position,
                kind,
                expected: r.expected,
                typed: r.typed,
                t_ms: r.t_ms,
                correct: r.correct,
            });
        }
    }
    Ok(sessions)
}

/// Session listée (sessions récentes, export CSV).
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct SessionListRow {
    /// Identifiant.
    pub id: String,
    /// Début, en ms Unix.
    pub started_at: i64,
    /// Titre du texte, s'il existe encore.
    pub title: Option<String>,
    /// Mode (`copy`, `dictation`, `review`).
    pub mode: String,
    /// Terminée (sinon abandonnée).
    pub completed: bool,
    /// Temps actif, en ms.
    pub duration_ms: Option<u32>,
    /// MPM brut.
    pub wpm_raw: Option<f64>,
    /// MPM net.
    pub wpm_net: Option<f64>,
    /// CPM.
    pub cpm: Option<f64>,
    /// Précision, en %.
    pub accuracy: Option<f64>,
    /// Fautes relevées.
    pub errors: u32,
}

/// Colonnes de `SessionListRow`, communes aux deux listes.
macro_rules! select_sessions {
    () => {
        "SELECT s.id, s.started_at, t.title, s.mode, s.ended_at IS NOT NULL AS completed, \
         s.duration_ms, s.wpm_raw, s.wpm_net, s.cpm, s.accuracy, \
         (SELECT COUNT(*) FROM session_errors e WHERE e.session_id = s.id) AS errors \
         FROM sessions s LEFT JOIN texts t ON t.id = s.text_id "
    };
}

/// Les `limit` dernières sessions terminées du profil, la plus récente d'abord.
pub async fn recent(
    pool: &SqlitePool,
    profile_id: &str,
    limit: u32,
) -> AppResult<Vec<SessionListRow>> {
    Ok(sqlx::query_as(concat!(
        select_sessions!(),
        "WHERE s.profile_id = ? AND s.ended_at IS NOT NULL \
         ORDER BY s.started_at DESC, s.id DESC LIMIT ?"
    ))
    .bind(profile_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// Sessions terminées ou abandonnées commencées depuis `from`, dans l'ordre chronologique.
pub async fn since(
    pool: &SqlitePool,
    profile_id: &str,
    from: i64,
) -> AppResult<Vec<SessionListRow>> {
    Ok(sqlx::query_as(concat!(
        select_sessions!(),
        "WHERE s.profile_id = ? AND s.started_at >= ? \
         AND (s.ended_at IS NOT NULL OR s.abandoned_at IS NOT NULL) \
         ORDER BY s.started_at, s.id"
    ))
    .bind(profile_id)
    .bind(from)
    .fetch_all(pool)
    .await?)
}
