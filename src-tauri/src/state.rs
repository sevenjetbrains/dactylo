//! État partagé de l'application et cache des sessions en cours.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dactylo_core::dictation::segment::Segment;
use dactylo_core::dictation::settings::DictationSettings;
use dactylo_core::events::EventBus;
use dactylo_core::settings::PracticeSettings;
use dactylo_core::speech::SpeechEngine;
use dactylo_core::typing::TypingSession;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::speech::UnavailableEngine;

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
    /// Texte tapé ; `None` pour une révision sur un texte généré (ADR 0018).
    pub text_id: Option<String>,
    /// Réglages figés au début de la session.
    pub settings: PracticeSettings,
    /// Exercice tapé.
    pub exercise: SessionExercise,
    /// Comparaison de frappe sur l'exercice, rejouée jusqu'à la dernière frappe reçue.
    pub typing: TypingSession,
}

/// Dictée en cours, gardée en mémoire entre deux commandes (ADR 0020). Le texte n'en
/// sort qu'à la correction ; elle est reconstruite depuis la base si elle manque.
#[derive(Debug, Clone)]
pub struct LiveDictation {
    /// Profil qui fait la dictée.
    pub profile_id: String,
    /// Texte dicté.
    pub text_id: String,
    /// Titre du texte.
    pub title: String,
    /// Exercice dicté (F-24).
    pub exercise: SessionExercise,
    /// Réglages de la dictée ; voix, vitesse et pause modifiables en cours de route.
    pub settings: DictationSettings,
    /// Texte de l'exercice, attendu à la correction.
    pub content: String,
    /// Segments lus l'un après l'autre (F-11).
    pub segments: Vec<Segment>,
    /// Répétitions déjà faites de chaque segment (F-14).
    pub repeats: Vec<u32>,
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
    /// Moteur de synthèse vocale de la dictée (ADR 0019).
    pub speech: Arc<dyn SpeechEngine>,
    /// Dictées en cours, par identifiant de session.
    pub live_dictations: Mutex<HashMap<String, LiveDictation>>,
}

impl AppState {
    /// Crée l'état avec un cache de sessions vide, sans moteur vocal (`with_speech`).
    pub fn new(pool: SqlitePool, bus: Arc<EventBus>, profile_id: String) -> Self {
        Self {
            pool,
            bus,
            profile_id,
            live_sessions: Mutex::default(),
            live_dictations: Mutex::default(),
            speech: Arc::new(UnavailableEngine("moteur vocal non démarré".into())),
        }
    }

    /// Remplace le moteur vocal.
    pub fn with_speech(self, speech: Arc<dyn SpeechEngine>) -> Self {
        Self { speech, ..self }
    }

    /// Retire une session du cache (le verrou n'est jamais tenu pendant une E/S).
    pub fn take_live(&self, session_id: &str) -> Option<LiveSession> {
        // Un verrou empoisonné ne doit pas rendre les sessions inaccessibles.
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session_id)
    }

    /// Retire une dictée du cache.
    pub fn take_dictation(&self, session_id: &str) -> Option<LiveDictation> {
        self.live_dictations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session_id)
    }

    /// Remet une dictée dans le cache.
    pub fn put_dictation(&self, session_id: String, dictation: LiveDictation) {
        self.live_dictations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id, dictation);
    }

    /// Remet une session dans le cache après traitement.
    pub fn put_live(&self, session_id: String, session: LiveSession) {
        self.live_sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id, session);
    }
}
