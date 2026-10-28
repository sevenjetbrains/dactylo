//! Commandes générales de l'application.

use serde::Serialize;
use tauri::State;

use crate::db::profiles::{self, Profile};
use crate::error::AppResult;
use crate::state::AppState;

/// Nom et version de l'application.
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Nom affiché.
    pub name: String,
    /// Version du paquet Cargo.
    pub version: String,
}

/// Renvoie le nom et la version de l'application.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Dactylo".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// Renvoie le profil actif.
#[tauri::command]
#[specta::specta]
pub async fn current_profile(state: State<'_, AppState>) -> AppResult<Profile> {
    profiles::get(&state.pool, &state.profile_id).await
}
