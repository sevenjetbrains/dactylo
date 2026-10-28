//! Dépôt des sessions, des frappes (F-31) et des fautes (F-30).

use dactylo_core::stats::SessionStats;
use dactylo_core::typing::{KeyKind, Keystroke, ResolvedKeystroke, TypingError};
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::error::{AppError, AppResult};

/// Mode d'entraînement, valeur de `sessions.mode`.
pub const MODE_COPY: &str = "copy";

/// Ligne de la table `sessions` utile au mode Copie.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    /// Identifiant UUIDv7.
    pub id: String,
    /// Profil qui joue la session.
    pub profile_id: String,
    /// Texte tapé (absent pour une dictée libre, plus tard).
    pub text_id: Option<String>,
    /// Début, en ms Unix.
    pub started_at: i64,
    /// Fin, en ms Unix ; `None` tant que la session est en cours.
    pub ended_at: Option<i64>,
    /// Réglages utilisés (JSON).
    pub settings: String,
}

/// Crée une session de copie. Les sessions non terminées du profil sont
/// supprimées : on ne reprend que la dernière (NF-06).
pub async fn start(
    pool: &SqlitePool,
    profile_id: &str,
    text_id: &str,
    settings_json: &str,
) -> AppResult<SessionRow> {
    // Suppression et création dans la même transaction : jamais deux sessions en cours.
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM sessions WHERE profile_id = ? AND ended_at IS NULL")
        .bind(profile_id)
        .execute(&mut *tx)
        .await?;
    // Nouvelle session, sans date de fin.
    let row = SessionRow {
        id: new_id(),
        profile_id: profile_id.to_owned(),
        text_id: Some(text_id.to_owned()),
        started_at: now_ms(),
        ended_at: None,
        settings: settings_json.to_owned(),
    };
    sqlx::query(
        "INSERT INTO sessions (id, profile_id, text_id, mode, started_at, settings) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row.id)
    .bind(&row.profile_id)
    .bind(&row.text_id)
    .bind(MODE_COPY)
    .bind(row.started_at)
    .bind(&row.settings)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(row)
}

/// Lit une session ; `NotFound` si elle n'existe pas.
pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<SessionRow> {
    sqlx::query_as(
        "SELECT id, profile_id, text_id, started_at, ended_at, settings FROM sessions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("session {id}")))
}

/// Dernière session non terminée du profil (NF-06).
pub async fn pending(pool: &SqlitePool, profile_id: &str) -> AppResult<Option<SessionRow>> {
    Ok(sqlx::query_as(
        "SELECT id, profile_id, text_id, started_at, ended_at, settings FROM sessions \
         WHERE profile_id = ? AND ended_at IS NULL AND text_id IS NOT NULL \
         ORDER BY started_at DESC, id DESC LIMIT 1",
    )
    .bind(profile_id)
    .fetch_optional(pool)
    .await?)
}

/// Enregistre un lot de frappes en une transaction ; un `seq` déjà présent est ignoré.
pub async fn insert_keystrokes(
    pool: &SqlitePool,
    session_id: &str,
    keystrokes: &[ResolvedKeystroke],
) -> AppResult<()> {
    // Une seule transaction pour tout le lot : bien plus rapide qu'une par frappe.
    let mut tx = pool.begin().await?;
    for k in keystrokes {
        // `OR IGNORE` : un lot renvoyé après une coupure ne crée pas de doublon.
        sqlx::query(
            "INSERT OR IGNORE INTO keystrokes \
             (session_id, seq, position, kind, expected, typed, t_ms, correct) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(k.seq)
        .bind(k.position)
        .bind(k.kind.as_str())
        .bind(&k.expected)
        .bind(&k.typed)
        .bind(k.t_ms)
        .bind(k.correct)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Ligne brute de la table `keystrokes`, réduite à ce qu'il faut pour rejouer.
#[derive(sqlx::FromRow)]
struct KeystrokeRow {
    /// Numéro d'ordre.
    seq: u32,
    /// « insert » ou « delete ».
    kind: String,
    /// Graphème tapé.
    typed: Option<String>,
    /// Temps actif, en ms.
    t_ms: u32,
}

/// Frappes brutes d'une session, dans l'ordre, pour les rejouer.
pub async fn keystrokes(pool: &SqlitePool, session_id: &str) -> AppResult<Vec<Keystroke>> {
    let rows: Vec<KeystrokeRow> = sqlx::query_as(
        "SELECT seq, kind, typed, t_ms FROM keystrokes WHERE session_id = ? ORDER BY seq",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    // Reconversion en frappes brutes ; toute valeur autre que « delete » est une insertion.
    Ok(rows
        .into_iter()
        .map(|r| Keystroke {
            seq: r.seq,
            kind: if r.kind == "delete" {
                KeyKind::Delete
            } else {
                KeyKind::Insert
            },
            typed: r.typed,
            t_ms: r.t_ms,
        })
        .collect())
}

/// Clôt la session : statistiques et fautes, en une transaction.
pub async fn finish(
    pool: &SqlitePool,
    session_id: &str,
    stats: &SessionStats,
    errors: &[TypingError],
) -> AppResult<()> {
    // Statistiques et fautes sont écrites ensemble ou pas du tout.
    let mut tx = pool.begin().await?;
    // `ended_at IS NULL` empêche de clore deux fois la même session.
    let updated = sqlx::query(
        "UPDATE sessions SET ended_at = ?, duration_ms = ?, wpm_raw = ?, wpm_net = ?, \
         cpm = ?, accuracy = ? WHERE id = ? AND ended_at IS NULL",
    )
    .bind(now_ms())
    .bind(stats.duration_ms)
    .bind(stats.wpm_raw)
    .bind(stats.wpm_net)
    .bind(stats.cpm)
    .bind(stats.accuracy)
    .bind(session_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::Invalid(format!(
            "session {session_id} déjà terminée"
        )));
    }
    // Une ligne par faute relevée.
    for e in errors {
        sqlx::query(
            "INSERT INTO session_errors (session_id, expected_word, typed_word, error_type, position) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(&e.expected)
        .bind(&e.typed)
        .bind(e.error_type.as_str())
        .bind(e.position)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Supprime une session (abandon) ; ses frappes et fautes partent en cascade.
pub async fn delete(pool: &SqlitePool, session_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Statistiques stockées d'une session terminée.
#[derive(sqlx::FromRow)]
struct StatsRow {
    /// Temps actif, en ms.
    duration_ms: u32,
    /// MPM brut.
    wpm_raw: f64,
    /// MPM net.
    wpm_net: f64,
    /// CPM.
    cpm: f64,
    /// Précision, en %.
    accuracy: f64,
}

/// Statistiques des `limit` dernières sessions terminées du profil, hors `exclude`.
pub async fn recent_stats(
    pool: &SqlitePool,
    profile_id: &str,
    exclude: &str,
    limit: u32,
) -> AppResult<Vec<SessionStats>> {
    let rows: Vec<StatsRow> = sqlx::query_as(
        "SELECT duration_ms, wpm_raw, wpm_net, cpm, accuracy FROM sessions \
         WHERE profile_id = ? AND mode = ? AND ended_at IS NOT NULL AND id <> ? \
         ORDER BY ended_at DESC, id DESC LIMIT ?",
    )
    .bind(profile_id)
    .bind(MODE_COPY)
    .bind(exclude)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    // Les compteurs ne sont pas stockés : ils restent à zéro, seule la moyenne compte.
    Ok(rows
        .into_iter()
        .map(|r| SessionStats {
            duration_ms: r.duration_ms,
            wpm_raw: r.wpm_raw,
            wpm_net: r.wpm_net,
            cpm: r.cpm,
            accuracy: r.accuracy,
            ..SessionStats::default()
        })
        .collect())
}

/// Tests du dépôt des sessions.
#[cfg(test)]
mod tests {
    use dactylo_core::typing::ErrorType;

    use super::*;
    use crate::db::texts::{self, TextRecord};
    use crate::db::{connect_in_memory, profiles};

    /// Base en mémoire avec le profil par défaut et un texte « ab ».
    async fn setup() -> (SqlitePool, String, String) {
        let pool = connect_in_memory().await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap();
        let text = texts::create(&pool, TextRecord::pasted("T", "ab"))
            .await
            .unwrap();
        (pool, profile.id, text)
    }

    /// Insertion résolue numéro `seq`, à la position `seq`.
    fn resolved(seq: u32, typed: &str, correct: bool) -> ResolvedKeystroke {
        ResolvedKeystroke {
            seq,
            position: seq,
            kind: KeyKind::Insert,
            expected: Some("a".into()),
            typed: Some(typed.into()),
            t_ms: seq * 100,
            correct,
        }
    }

    /// Les frappes sont relues dans l'ordre ; un numéro déjà enregistré est ignoré.
    #[tokio::test]
    async fn keystrokes_round_trip_and_duplicates_are_ignored() {
        let (pool, profile, text) = setup().await;
        let s = start(&pool, &profile, &text, "{}").await.unwrap();
        insert_keystrokes(&pool, &s.id, &[resolved(0, "a", true)])
            .await
            .unwrap();
        // Second lot : le doublon du numéro 0 et un retour arrière.
        let delete = ResolvedKeystroke {
            kind: KeyKind::Delete,
            typed: None,
            ..resolved(1, "", true)
        };
        insert_keystrokes(&pool, &s.id, &[resolved(0, "z", false), delete])
            .await
            .unwrap();

        // La première version de la frappe 0 est conservée.
        let ks = keystrokes(&pool, &s.id).await.unwrap();
        assert_eq!(ks.len(), 2);
        assert_eq!(ks[0].typed.as_deref(), Some("a"), "le doublon est ignoré");
        assert_eq!(ks[1].kind, KeyKind::Delete);
        assert_eq!(ks[1].t_ms, 100);
    }

    /// Démarrer une session supprime la session en cours du même profil.
    #[tokio::test]
    async fn starting_a_session_replaces_the_unfinished_one() {
        let (pool, profile, text) = setup().await;
        let first = start(&pool, &profile, &text, "{}").await.unwrap();
        assert_eq!(
            pending(&pool, &profile).await.unwrap().unwrap().id,
            first.id
        );

        // La seconde session remplace la première.
        let second = start(&pool, &profile, &text, "{}").await.unwrap();
        assert!(matches!(
            get(&pool, &first.id).await,
            Err(AppError::NotFound(_))
        ));
        assert_eq!(
            pending(&pool, &profile).await.unwrap().unwrap().id,
            second.id
        );

        // Après suppression, plus rien à reprendre.
        delete(&pool, &second.id).await.unwrap();
        assert!(pending(&pool, &profile).await.unwrap().is_none());
    }

    /// Clore une session écrit statistiques et fautes, une seule fois.
    #[tokio::test]
    async fn finish_writes_stats_and_errors_once() {
        let (pool, profile, text) = setup().await;
        let s = start(&pool, &profile, &text, "{}").await.unwrap();
        let stats = SessionStats {
            duration_ms: 1000,
            wpm_net: 30.0,
            cpm: 150.0,
            accuracy: 90.0,
            ..SessionStats::default()
        };
        let errors = [TypingError {
            expected: "forêt".into(),
            typed: "foret".into(),
            error_type: ErrorType::Accent,
            position: 3,
            word: true,
        }];
        finish(&pool, &s.id, &stats, &errors).await.unwrap();

        // La session est close et n'est plus proposée à la reprise.
        let row = get(&pool, &s.id).await.unwrap();
        assert!(row.ended_at.is_some());
        assert!(pending(&pool, &profile).await.unwrap().is_none());
        // La faute est stockée avec son type en base.
        let stored: (String, String, String) = sqlx::query_as(
            "SELECT expected_word, typed_word, error_type FROM session_errors WHERE session_id = ?",
        )
        .bind(&s.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, ("forêt".into(), "foret".into(), "accent".into()));

        // Une seconde clôture est refusée.
        assert!(matches!(
            finish(&pool, &s.id, &stats, &[]).await,
            Err(AppError::Invalid(_))
        ));
    }

    /// La moyenne exclut la session courante et les sessions non terminées.
    #[tokio::test]
    async fn recent_stats_excludes_current_and_unfinished_sessions() {
        let (pool, profile, text) = setup().await;
        let stats = |wpm| SessionStats {
            wpm_net: wpm,
            ..SessionStats::default()
        };
        // Deux sessions terminées (20 et 40 MPM), puis une en cours.
        let a = start(&pool, &profile, &text, "{}").await.unwrap();
        finish(&pool, &a.id, &stats(20.0), &[]).await.unwrap();
        let b = start(&pool, &profile, &text, "{}").await.unwrap();
        finish(&pool, &b.id, &stats(40.0), &[]).await.unwrap();
        start(&pool, &profile, &text, "{}").await.unwrap();

        // En excluant B, seule A reste ; la limite est respectée.
        let recent = recent_stats(&pool, &profile, &b.id, 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].wpm_net, 20.0);
        assert_eq!(recent_stats(&pool, &profile, "", 1).await.unwrap().len(), 1);
    }
}
