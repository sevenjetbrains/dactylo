//! Accès SQLite : ouverture, PRAGMA et migrations automatiques (NF-05).

/// Profils utilisateurs et leurs réglages.
pub mod profiles;
/// Sessions, frappes et fautes.
pub mod sessions;
/// Textes de la bibliothèque.
pub mod texts;

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
    // Le fichier est créé au premier lancement.
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

/// Applique les options communes, ouvre le pool et lance les migrations.
async fn open(options: SqliteConnectOptions, max_connections: u32) -> AppResult<SqlitePool> {
    // Clés étrangères vérifiées ; journal WAL pour écrire sans bloquer les lectures.
    let options = options
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    // Seules les migrations pas encore appliquées sont jouées.
    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

/// Horodatage courant en millisecondes Unix (ADR 0003).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        // Horloge avant 1970 : impossible en pratique, on renvoie 0 plutôt que d'échouer.
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Nouvel identifiant UUIDv7 (triable par date de création).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Tests de l'ouverture de la base et des migrations.
#[cfg(test)]
mod tests {
    use super::*;

    /// Tables attendues après les migrations, par ordre alphabétique.
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

    /// Une base neuve contient toutes les tables du schéma.
    #[tokio::test]
    async fn fresh_database_has_all_tables() {
        let pool = connect_in_memory().await.unwrap();
        // Tables de l'application, hors tables internes de SQLite et de sqlx.
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(names, TABLES);
    }

    /// Rouvrir une base existante ne rejoue aucune migration.
    #[tokio::test]
    async fn reopening_applies_no_migration_twice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        // Deux ouvertures successives du même fichier.
        let pool = connect(&path).await.unwrap();
        pool.close().await;
        let pool = connect(&path).await.unwrap();

        // Chaque migration n'est inscrite qu'une fois.
        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(applied as usize, MIGRATOR.iter().count());
    }

    /// Une session vers un profil inexistant est refusée.
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

    /// La contrainte CHECK refuse des réglages qui ne sont pas du JSON.
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

    /// Deux identifiants successifs diffèrent et sont triés par date.
    #[test]
    fn new_ids_are_unique_and_sortable() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(a < b);
    }
}
