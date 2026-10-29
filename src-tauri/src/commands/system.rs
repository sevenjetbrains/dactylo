//! Commandes système : sauvegarde et restauration de la base (F-52, ADR 0024).
//!
//! Les chemins viennent de la boîte de dialogue du système, ouverte par
//! l'interface (plugin `dialog`) : l'application n'accède à aucun autre fichier
//! (NF-11).

use std::path::Path;

use tauri::{AppHandle, State};

use crate::db::backup;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Sauvegarde toute la base dans le fichier choisi.
pub async fn backup_to(state: &AppState, path: &str) -> AppResult<()> {
    backup::backup(&state.pool, Path::new(path)).await
}

/// Vérifie la sauvegarde choisie et la met en attente pour le prochain démarrage.
pub async fn stage(state: &AppState, path: &str) -> AppResult<()> {
    let db = state
        .db_path
        .as_deref()
        .ok_or_else(|| AppError::Invalid("base sans fichier : restauration impossible".into()))?;
    backup::stage_restore(db, Path::new(path)).await
}

/// Commande : sauvegarde la base dans `path` (F-52).
#[tauri::command]
#[specta::specta]
pub async fn backup_database(state: State<'_, AppState>, path: String) -> AppResult<()> {
    backup_to(&state, &path).await
}

/// Commande : restaure la sauvegarde `path`, puis redémarre l'application, qui
/// l'installe avant d'ouvrir la base (F-52).
#[tauri::command]
#[specta::specta]
pub async fn restore_database(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<()> {
    stage(&state, &path).await?;
    // La base en cours est fermée proprement avant le redémarrage.
    state.pool.close().await;
    app.restart()
}

/// Tests des commandes système, sur une base fichier : `VACUUM INTO` depuis une base
/// en mémoire écrirait la copie en mémoire, pas sur le disque.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dactylo_core::events::EventBus;

    use super::*;
    use crate::db::{connect, profiles};

    /// État sur une base fichier dans `dir`.
    async fn file_state(dir: &Path) -> AppState {
        let db = dir.join("dactylo.db");
        let pool = connect(&db).await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap();
        AppState::new(pool, Arc::new(EventBus::new()), profile.id)
    }

    /// Sauvegarde par la commande ; restauration refusée sans fichier de base connu.
    #[tokio::test]
    async fn backup_works_and_restore_needs_a_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = file_state(dir.path()).await;
        let file = dir.path().join("copie.dactylo");
        backup_to(&state, file.to_str().unwrap()).await.unwrap();
        backup::check(&file).await.unwrap();
        assert!(matches!(
            stage(&state, file.to_str().unwrap()).await,
            Err(AppError::Invalid(_))
        ));
    }

    /// Avec le fichier de base connu, la restauration est mise en attente à côté.
    #[tokio::test]
    async fn restore_is_staged_next_to_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let state = file_state(dir.path()).await;
        let file = dir.path().join("copie.dactylo");
        backup_to(&state, file.to_str().unwrap()).await.unwrap();
        let state = state.with_db_path(dir.path().join("dactylo.db"));
        stage(&state, file.to_str().unwrap()).await.unwrap();
        assert!(dir.path().join("dactylo.db.restore").exists());
    }
}
