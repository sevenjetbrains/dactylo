//! Sauvegarde et restauration de toute la base dans un fichier unique (F-52,
//! ADR 0024).

use std::fs;
use std::path::{Path, PathBuf};

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, SqlitePool};

use super::MIGRATOR;
use crate::error::{AppError, AppResult};

/// Suffixe de la base en attente de restauration, installée au prochain démarrage.
const PENDING: &str = "restore";
/// Suffixe de l'ancienne base, gardée après une restauration.
const PREVIOUS: &str = "bak";
/// Fichiers compagnons de SQLite en mode WAL, à déplacer avec la base.
const COMPANIONS: [&str; 2] = ["-wal", "-shm"];

/// `dactylo.db` → `dactylo.db.<suffixe>`.
fn sibling(db: &Path, suffix: &str) -> PathBuf {
    let mut name = db.as_os_str().to_owned();
    name.push(".");
    name.push(suffix);
    PathBuf::from(name)
}

/// `dactylo.db` → `dactylo.db-wal` (ou `-shm`).
fn companion(db: &Path, suffix: &str) -> PathBuf {
    let mut name = db.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Erreur d'entrée-sortie traduite pour l'interface.
fn io_error(context: &str, e: std::io::Error) -> AppError {
    AppError::Invalid(format!("{context} : {e}"))
}

/// Copie cohérente de la base, même pendant l'utilisation (`VACUUM INTO`). Un fichier
/// existant au même endroit est remplacé (la boîte de dialogue l'a fait confirmer).
pub async fn backup(pool: &SqlitePool, target: &Path) -> AppResult<()> {
    // `VACUUM INTO` refuse d'écraser un fichier.
    if target.exists() {
        fs::remove_file(target).map_err(|e| io_error("remplacement de la sauvegarde", e))?;
    }
    let path = target
        .to_str()
        .ok_or_else(|| AppError::Invalid("chemin de sauvegarde illisible".into()))?;
    sqlx::query("VACUUM INTO ?")
        .bind(path)
        .execute(pool)
        .await?;
    Ok(())
}

/// Vérifie qu'un fichier est une sauvegarde Dactylo utilisable : base SQLite
/// lisible, profils présents, schéma pas plus récent que l'application.
pub async fn check(candidate: &Path) -> AppResult<()> {
    let invalid = || AppError::Invalid("ce fichier n'est pas une sauvegarde de Dactylo".into());
    // Ouverture en lecture seule : le fichier choisi n'est jamais modifié.
    let mut conn = SqliteConnectOptions::new()
        .filename(candidate)
        .read_only(true)
        .connect()
        .await
        .map_err(|_| invalid())?;
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('_sqlx_migrations', 'profiles')",
    )
    .fetch_all(&mut conn)
    .await
    .map_err(|_| invalid())?;
    if tables.len() != 2 {
        return Err(invalid());
    }
    // Une sauvegarde d'une version plus récente de l'application ne se relit pas ici.
    let version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_one(&mut conn)
        .await
        .map_err(|_| invalid())?;
    let known = MIGRATOR.iter().map(|m| m.version).max().unwrap_or(0);
    if version.unwrap_or(0) > known {
        return Err(AppError::Invalid(
            "cette sauvegarde vient d'une version plus récente de Dactylo".into(),
        ));
    }
    Ok(())
}

/// Met une sauvegarde vérifiée en attente : elle remplacera la base au prochain
/// démarrage (`apply_pending`).
pub async fn stage_restore(db: &Path, candidate: &Path) -> AppResult<()> {
    check(candidate).await?;
    fs::copy(candidate, sibling(db, PENDING)).map_err(|e| io_error("copie de la sauvegarde", e))?;
    Ok(())
}

/// Au démarrage, avant d'ouvrir la base : installe la sauvegarde en attente. L'ancienne
/// base (et ses fichiers WAL, qui ne vont qu'avec elle) est gardée en `.bak`.
/// Renvoie `true` si une restauration a eu lieu.
pub fn apply_pending(db: &Path) -> AppResult<bool> {
    let pending = sibling(db, PENDING);
    if !pending.exists() {
        return Ok(false);
    }
    let previous = sibling(db, PREVIOUS);
    // Les fichiers de l'ancienne sauvegarde `.bak` laissent la place.
    for path in std::iter::once(previous.clone()).chain(COMPANIONS.map(|c| companion(&previous, c)))
    {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| io_error("nettoyage de l'ancienne copie", e))?;
        }
    }
    // Base actuelle et ses compagnons → `.bak` ; puis la sauvegarde devient la base.
    if db.exists() {
        fs::rename(db, &previous).map_err(|e| io_error("mise de côté de la base", e))?;
    }
    for c in COMPANIONS {
        let from = companion(db, c);
        if from.exists() {
            fs::rename(&from, companion(&previous, c))
                .map_err(|e| io_error("mise de côté du journal", e))?;
        }
    }
    fs::rename(&pending, db).map_err(|e| io_error("installation de la sauvegarde", e))?;
    Ok(true)
}

/// Tests de la sauvegarde et de la restauration.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{connect, profiles};

    /// Une base sauvegardée se vérifie, se met en attente, puis remplace la base au
    /// démarrage suivant ; l'ancienne est gardée.
    #[tokio::test]
    async fn backup_then_restore_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("dactylo.db");
        // Base d'origine avec un profil renommé, sauvegardée.
        let pool = connect(&db).await.unwrap();
        let p = profiles::ensure_default(&pool).await.unwrap();
        profiles::update(&pool, &p.id, "Avant", "azerty")
            .await
            .unwrap();
        let file = dir.path().join("sauvegarde.dactylo");
        backup(&pool, &file).await.unwrap();
        // Une seconde sauvegarde au même endroit remplace la première.
        backup(&pool, &file).await.unwrap();
        // La base change après la sauvegarde.
        profiles::update(&pool, &p.id, "Après", "azerty")
            .await
            .unwrap();
        stage_restore(&db, &file).await.unwrap();
        pool.close().await;

        // Démarrage suivant : la sauvegarde est installée.
        assert!(apply_pending(&db).unwrap());
        assert!(!apply_pending(&db).unwrap(), "plus rien en attente");
        assert!(sibling(&db, PREVIOUS).exists());
        let pool = connect(&db).await.unwrap();
        assert_eq!(profiles::get(&pool, &p.id).await.unwrap().name, "Avant");
    }

    /// Un fichier qui n'est pas une base Dactylo est refusé, sans rien mettre en attente.
    #[tokio::test]
    async fn rejects_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("dactylo.db");
        let text = dir.path().join("notes.txt");
        fs::write(&text, "pas une base").unwrap();
        assert!(matches!(
            stage_restore(&db, &text).await,
            Err(AppError::Invalid(_))
        ));
        // Base SQLite étrangère : lisible, mais sans les tables de Dactylo.
        let other = dir.path().join("autre.db");
        let mut conn = SqliteConnectOptions::new()
            .filename(&other)
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE t (x)")
            .execute(&mut conn)
            .await
            .unwrap();
        drop(conn);
        assert!(check(&other).await.is_err());
        assert!(!sibling(&db, PENDING).exists());
    }

    /// Une sauvegarde d'une version plus récente du schéma est refusée.
    #[tokio::test]
    async fn rejects_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("future.db");
        let pool = connect(&db).await.unwrap();
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) \
             VALUES (9999, 'futur', CURRENT_TIMESTAMP, 1, x'00', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
        let err = check(&db).await.unwrap_err();
        assert!(err.to_string().contains("plus récente"), "{err}");
    }

    /// Les fichiers WAL de l'ancienne base partent avec elle, jamais avec la nouvelle.
    #[test]
    fn wal_files_follow_the_old_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("dactylo.db");
        fs::write(&db, "ancienne").unwrap();
        fs::write(companion(&db, "-wal"), "journal").unwrap();
        fs::write(sibling(&db, PENDING), "nouvelle").unwrap();
        assert!(apply_pending(&db).unwrap());
        assert_eq!(fs::read_to_string(&db).unwrap(), "nouvelle");
        assert!(!companion(&db, "-wal").exists());
        let previous = sibling(&db, PREVIOUS);
        assert_eq!(
            fs::read_to_string(companion(&previous, "-wal")).unwrap(),
            "journal"
        );
    }
}
