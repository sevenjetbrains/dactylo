use std::sync::Arc;

use dactylo_core::events::EventBus;
use sqlx::SqlitePool;

/// État partagé, injecté dans les commandes via `tauri::State`.
pub struct AppState {
    pub pool: SqlitePool,
    pub bus: Arc<EventBus>,
    /// Profil actif ; le changement de profil arrive en Phase 5 (F-50).
    pub profile_id: String,
}
