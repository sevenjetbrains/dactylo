use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dactylo_core::events::EventBus;
use dactylo_core::settings::PracticeSettings;
use dactylo_core::typing::TypingSession;
use sqlx::SqlitePool;

/// Session de copie en cours, gardée en mémoire entre deux lots de frappes.
/// Elle est reconstruite depuis la base si elle manque (redémarrage, NF-06).
#[derive(Debug, Clone)]
pub struct LiveSession {
    pub profile_id: String,
    pub text_id: String,
    pub settings: PracticeSettings,
    pub typing: TypingSession,
}

/// État partagé, injecté dans les commandes via `tauri::State`.
pub struct AppState {
    pub pool: SqlitePool,
    pub bus: Arc<EventBus>,
    /// Profil actif ; le changement de profil arrive en Phase 5 (F-50).
    pub profile_id: String,
    pub live_sessions: Mutex<HashMap<String, LiveSession>>,
}

impl AppState {
    pub fn new(pool: SqlitePool, bus: Arc<EventBus>, profile_id: String) -> Self {
        Self {
            pool,
            bus,
            profile_id,
            live_sessions: Mutex::default(),
        }
    }

    /// Retire une session du cache (le verrou n'est jamais tenu pendant une E/S).
    pub fn take_live(&self, session_id: &str) -> Option<LiveSession> {
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session_id)
    }

    pub fn put_live(&self, session_id: String, session: LiveSession) {
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id, session);
    }
}
