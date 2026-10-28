//! État partagé de l'application et cache des sessions en cours.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dactylo_core::events::EventBus;
use dactylo_core::settings::PracticeSettings;
use dactylo_core::typing::TypingSession;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Exercice d'une session : rang, nombre d'exercices du texte et portion tapée (F-24).
/// Il est enregistré avec les réglages de la session pour pouvoir la reconstruire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExercise {
    /// Rang de l'exercice, à partir de 0.
    pub index: u32,
    /// Nombre d'exercices du texte.
    pub count: u32,
    /// Premier graphème de l'exercice dans le texte.
    pub start: u32,
    /// Graphème qui suit le dernier (borne exclue).
    pub end: u32,
}

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
    /// Exercice tapé.
    pub exercise: SessionExercise,
    /// Comparaison de frappe sur l'exercice, rejouée jusqu'à la dernière frappe reçue.
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
