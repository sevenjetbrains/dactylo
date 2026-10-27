//! Événements de domaine et bus de diffusion synchrone.
//!
//! Les modules ne s'appellent pas entre eux : ils publient des événements
//! et s'abonnent à ceux des autres (voir ADR 0006).

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

/// Événement métier émis par un module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DomainEvent {
    /// Une session est terminée et ses statistiques sont calculées.
    SessionCompleted {
        session_id: String,
        profile_id: String,
    },
    /// Un mot a été mal écrit pendant une session.
    WordMistyped {
        session_id: String,
        profile_id: String,
        expected: String,
        typed: String,
    },
    /// Un texte a été ajouté à la bibliothèque.
    TextImported { text_id: String },
}

/// Abonné au bus : reçoit chaque événement publié.
pub trait EventSubscriber: Send + Sync {
    fn on_event(&self, event: &DomainEvent);
}

/// Bus synchrone en mémoire : les abonnés sont appelés dans l'ordre d'inscription.
#[derive(Default)]
pub struct EventBus {
    subscribers: RwLock<Vec<Arc<dyn EventSubscriber>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) {
        self.subscribers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(subscriber);
    }

    pub fn publish(&self, event: &DomainEvent) {
        // Copie de la liste pour qu'un abonné puisse publier à son tour sans interblocage.
        let subscribers = self
            .subscribers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        for subscriber in subscribers {
            subscriber.on_event(event);
        }
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder {
        received: Mutex<Vec<DomainEvent>>,
    }

    impl EventSubscriber for Recorder {
        fn on_event(&self, event: &DomainEvent) {
            self.received.lock().unwrap().push(event.clone());
        }
    }

    fn imported(id: &str) -> DomainEvent {
        DomainEvent::TextImported {
            text_id: id.to_owned(),
        }
    }

    #[test]
    fn publish_without_subscriber_is_a_no_op() {
        let bus = EventBus::new();
        bus.publish(&imported("t1"));
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn every_subscriber_receives_events_in_order() {
        let bus = EventBus::new();
        let a = Arc::new(Recorder::default());
        let b = Arc::new(Recorder::default());
        bus.subscribe(a.clone());
        bus.subscribe(b.clone());

        bus.publish(&imported("t1"));
        bus.publish(&imported("t2"));

        let expected = vec![imported("t1"), imported("t2")];
        assert_eq!(*a.received.lock().unwrap(), expected);
        assert_eq!(*b.received.lock().unwrap(), expected);
        assert_eq!(bus.subscriber_count(), 2);
    }

    struct Republisher {
        bus: Arc<EventBus>,
        recorder: Arc<Recorder>,
    }

    impl EventSubscriber for Republisher {
        fn on_event(&self, event: &DomainEvent) {
            self.recorder.on_event(event);
            if let DomainEvent::SessionCompleted { session_id, .. } = event {
                self.bus.publish(&imported(session_id));
            }
        }
    }

    #[test]
    fn subscriber_can_publish_without_deadlock() {
        let bus = Arc::new(EventBus::new());
        let recorder = Arc::new(Recorder::default());
        bus.subscribe(Arc::new(Republisher {
            bus: bus.clone(),
            recorder: recorder.clone(),
        }));

        bus.publish(&DomainEvent::SessionCompleted {
            session_id: "s1".into(),
            profile_id: "p1".into(),
        });

        let received = recorder.received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[1], imported("s1"));
    }

    #[test]
    fn events_serialize_with_camel_case_tag() {
        let event = DomainEvent::WordMistyped {
            session_id: "s1".into(),
            profile_id: "p1".into(),
            expected: "être".into(),
            typed: "etre".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "wordMistyped",
                "sessionId": "s1",
                "profileId": "p1",
                "expected": "être",
                "typed": "etre"
            })
        );
        let back: DomainEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }
}
