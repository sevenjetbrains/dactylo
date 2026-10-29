//! Relais des événements de domaine vers l'interface.

use dactylo_core::events::{DomainEvent, EventSubscriber};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// Événement Tauri typé portant un `DomainEvent` ; les modules React s'y abonnent.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct DomainEventPayload(pub DomainEvent);

/// Fin de lecture d'un texte par le moteur vocal (dictée, ADR 0019). `utterance` est
/// le numéro renvoyé par la commande de lecture : l'interface ignore les fins
/// d'anciennes lectures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SpeechFinished {
    /// Numéro de la lecture terminée.
    pub utterance: u64,
}

/// Nouvelle version trouvée par la vérification au démarrage (opt-in, ADR 0025).
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvailable {
    /// Version disponible.
    pub version: String,
}

/// Abonné du bus qui retransmet chaque événement à la fenêtre.
pub struct FrontendForwarder<R: Runtime> {
    /// Application Tauri vers laquelle émettre.
    app: AppHandle<R>,
}

impl<R: Runtime> FrontendForwarder<R> {
    /// Crée un relais vers l'application donnée.
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> EventSubscriber for FrontendForwarder<R> {
    /// Émet l'événement vers l'interface.
    fn on_event(&self, event: &DomainEvent) {
        // Une fenêtre fermée ne doit pas faire échouer le métier.
        let _ = DomainEventPayload(event.clone()).emit(&self.app);
    }
}
