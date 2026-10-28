//! Outils communs aux tests des commandes.

use std::sync::{Arc, Mutex};

use dactylo_core::events::{DomainEvent, EventBus, EventSubscriber};

use crate::db::{connect_in_memory, profiles};
use crate::state::AppState;

/// Abonné qui garde chaque événement reçu.
#[derive(Default)]
pub struct Recorder(pub Mutex<Vec<DomainEvent>>);

impl EventSubscriber for Recorder {
    fn on_event(&self, event: &DomainEvent) {
        self.0.lock().unwrap().push(event.clone());
    }
}

impl Recorder {
    pub fn events(&self) -> Vec<DomainEvent> {
        self.0.lock().unwrap().clone()
    }
}

/// État d'application sur une base en mémoire, avec le profil par défaut.
pub async fn state() -> (AppState, Arc<Recorder>) {
    let pool = connect_in_memory().await.unwrap();
    let profile = profiles::ensure_default(&pool).await.unwrap();
    let bus = Arc::new(EventBus::new());
    let recorder = Arc::new(Recorder::default());
    bus.subscribe(recorder.clone());
    (AppState::new(pool, bus, profile.id), recorder)
}
