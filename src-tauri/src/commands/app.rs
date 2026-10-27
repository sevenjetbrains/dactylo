//! Commandes générales de l'application.

use serde::Serialize;
use tauri::State;

use crate::db::profiles::{self, Profile};
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[tauri::command]
#[specta::specta]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Dactylo".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn current_profile(state: State<'_, AppState>) -> AppResult<Profile> {
    profiles::get(&state.pool, &state.profile_id).await
}
