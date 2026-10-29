//! Outils communs aux tests des commandes.

use std::sync::{Arc, Mutex};

use dactylo_core::events::{DomainEvent, EventBus, EventSubscriber};

use crate::db::{connect_in_memory, profiles};
use crate::speech::FakeSpeechEngine;
use crate::state::AppState;

/// Abonné qui garde chaque événement reçu.
#[derive(Default)]
pub struct Recorder(
    /// Événements reçus, dans l'ordre d'arrivée.
    pub Mutex<Vec<DomainEvent>>,
);

impl EventSubscriber for Recorder {
    /// Ajoute l'événement à la liste.
    fn on_event(&self, event: &DomainEvent) {
        self.0.lock().unwrap().push(event.clone());
    }
}

impl Recorder {
    /// Copie des événements reçus jusqu'ici.
    pub fn events(&self) -> Vec<DomainEvent> {
        self.0.lock().unwrap().clone()
    }
}

/// État d'application sur une base en mémoire, avec le profil par défaut.
pub async fn state() -> (AppState, Arc<Recorder>) {
    // Base vierge, migrations appliquées, profil par défaut créé.
    let pool = connect_in_memory().await.unwrap();
    let profile = profiles::ensure_default(&pool).await.unwrap();
    // Un enregistreur est abonné pour vérifier les événements publiés.
    let bus = Arc::new(EventBus::new());
    let recorder = Arc::new(Recorder::default());
    bus.subscribe(recorder.clone());
    (AppState::new(pool, bus, profile.id), recorder)
}

/// État de test avec un moteur vocal factice, rendu pour vérifier les textes lus.
pub async fn state_with_speech() -> (AppState, Arc<Recorder>, Arc<FakeSpeechEngine>) {
    let (state, recorder) = state().await;
    let speech = Arc::new(FakeSpeechEngine::default());
    (state.with_speech(speech.clone()), recorder, speech)
}
