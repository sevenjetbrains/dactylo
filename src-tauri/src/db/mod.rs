//! Accès SQLite : ouverture, PRAGMA et migrations automatiques (NF-05).

pub mod profiles;

use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::AppResult;

/// Migrations embarquées dans l'exécutable ; jamais modifiées une fois publiées.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Ouvre (ou crée) la base et applique les migrations en attente.
pub async fn connect(path: &Path) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    open(options, 4).await
}

/// Base en mémoire, pour les tests.
pub async fn connect_in_memory() -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?;
    // Une seule connexion : chaque connexion en mémoire est une base distincte.
    open(options, 1).await
}

async fn open(options: SqliteConnectOptions, max_connections: u32) -> AppResult<SqlitePool> {
    let options = options
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Horodatage courant en millisecondes Unix (ADR 0003).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Nouvel identifiant UUIDv7 (triable par date de création).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: [&str; 9] = [
        "dictation_profiles",
        "keystrokes",
        "profiles",
        "session_errors",
        "sessions",
        "text_progress",
        "texts",
        "word_attempts",
        "words",
    ];

    #[tokio::test]
    async fn fresh_database_has_all_tables() {
        let pool = connect_in_memory().await.unwrap();
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(names, TABLES);
    }

    #[tokio::test]
    async fn reopening_applies_no_migration_twice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let pool = connect(&path).await.unwrap();
        pool.close().await;
        let pool = connect(&path).await.unwrap();

        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(applied as usize, MIGRATOR.iter().count());
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let pool = connect_in_memory().await.unwrap();
        let result = sqlx::query(
            "INSERT INTO sessions (id, profile_id, mode, started_at) VALUES ('s', 'absent', 'copy', 0)",
        )
        .execute(&pool)
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn invalid_settings_json_is_rejected() {
        let pool = connect_in_memory().await.unwrap();
        let result = sqlx::query(
            "INSERT INTO profiles (id, name, settings, created_at) VALUES ('p', 'x', 'pas du json', 0)",
        )
        .execute(&pool)
        .await;
        assert!(result.is_err());
    }

    #[test]
    fn new_ids_are_unique_and_sortable() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(a < b);
    }
}
