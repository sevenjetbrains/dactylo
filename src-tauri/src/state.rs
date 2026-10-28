//! État partagé de l'application et cache des sessions en cours.

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
    /// Profil qui joue la session.
    pub profile_id: String,
    /// Texte tapé.
    pub text_id: String,
    /// Réglages figés au début de la session.
    pub settings: PracticeSettings,
    /// Comparaison de frappe, rejouée jusqu'à la dernière frappe reçue.
    pub typing: TypingSession,
}

/// État partagé, injecté dans les commandes via `tauri::State`.
pub struct AppState {
    /// Connexions à la base SQLite.
    pub pool: SqlitePool,
    /// Bus des événements de domaine.
    pub bus: Arc<EventBus>,
    /// Profil actif ; le changement de profil arrive en Phase 5 (F-50).
    pub profile_id: String,
    /// Sessions en cours, par identifiant de session.
    pub live_sessions: Mutex<HashMap<String, LiveSession>>,
}

impl AppState {
    /// Crée l'état avec un cache de sessions vide.
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
        // Un verrou empoisonné ne doit pas rendre les sessions inaccessibles.
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session_id)
    }

    /// Remet une session dans le cache après traitement.
    pub fn put_live(&self, session_id: String, session: LiveSession) {
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id, session);
    }
}
