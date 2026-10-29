//! Événements de domaine et bus de diffusion synchrone.
//!
//! Les modules ne s'appellent pas entre eux : ils publient des événements
//! et s'abonnent à ceux des autres (voir ADR 0006).

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::typing::SessionWords;

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
        /// Identifiant de la session terminée.
        session_id: String,
        /// Profil qui a joué la session.
        profile_id: String,
        /// Mots entièrement tapés, justes ou non, avec leur durée (ADR 0015).
        words: SessionWords,
    },
    /// Un mot a été mal écrit pendant une session.
    WordMistyped {
        /// Session pendant laquelle la faute a eu lieu.
        session_id: String,
        /// Profil qui a fait la faute.
        profile_id: String,
        /// Mot attendu, tel qu'il figure dans le texte.
        expected: String,
        /// Mot effectivement tapé.
        typed: String,
    },
    /// Un texte a été ajouté à la bibliothèque.
    TextImported {
        /// Identifiant du texte ajouté.
        text_id: String,
    },
    /// La liste des mots difficiles d'un profil a changé (révision du jour, badge).
    WordsUpdated {
        /// Profil concerné.
        profile_id: String,
    },
}

/// Abonné au bus : reçoit chaque événement publié.
pub trait EventSubscriber: Send + Sync {
    /// Appelé pour chaque événement publié après l'inscription de l'abonné.
    fn on_event(&self, event: &DomainEvent);
}

/// Bus synchrone en mémoire : les abonnés sont appelés dans l'ordre d'inscription.
#[derive(Default)]
pub struct EventBus {
    /// Abonnés inscrits, dans l'ordre d'inscription.
    subscribers: RwLock<Vec<Arc<dyn EventSubscriber>>>,
}

impl EventBus {
    /// Crée un bus sans abonné.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inscrit un abonné ; il recevra les événements publiés à partir de maintenant.
    pub fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) {
        // Un verrou empoisonné (abonné qui a paniqué) ne doit pas bloquer le bus.
        self.subscribers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(subscriber);
    }

    /// Diffuse un événement à tous les abonnés, de façon synchrone.
    pub fn publish(&self, event: &DomainEvent) {
        // Copie de la liste pour qu'un abonné puisse publier à son tour sans interblocage.
        let subscribers = self
            .subscribers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        // Les abonnés sont appelés dans l'ordre où ils se sont inscrits.
        for subscriber in subscribers {
            subscriber.on_event(event);
        }
    }

    /// Nombre d'abonnés inscrits (utile aux tests et au diagnostic).
    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }
}

/// Tests du bus d'événements.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Abonné de test qui mémorise les événements reçus.
    #[derive(Default)]
    struct Recorder {
        /// Événements reçus, dans l'ordre d'arrivée.
        received: Mutex<Vec<DomainEvent>>,
    }

    impl EventSubscriber for Recorder {
        /// Ajoute l'événement à la liste des événements reçus.
        fn on_event(&self, event: &DomainEvent) {
            self.received.lock().unwrap().push(event.clone());
        }
    }

    /// Fabrique un événement `TextImported` pour l'identifiant donné.
    fn imported(id: &str) -> DomainEvent {
        DomainEvent::TextImported {
            text_id: id.to_owned(),
        }
    }

    /// Publier sans abonné ne fait rien et ne panique pas.
    #[test]
    fn publish_without_subscriber_is_a_no_op() {
        let bus = EventBus::new();
        bus.publish(&imported("t1"));
        assert_eq!(bus.subscriber_count(), 0);
    }

    /// Chaque abonné reçoit tous les événements, dans l'ordre de publication.
    #[test]
    fn every_subscriber_receives_events_in_order() {
        // Deux abonnés inscrits l'un après l'autre.
        let bus = EventBus::new();
        let a = Arc::new(Recorder::default());
        let b = Arc::new(Recorder::default());
        bus.subscribe(a.clone());
        bus.subscribe(b.clone());

        // Deux publications successives.
        bus.publish(&imported("t1"));
        bus.publish(&imported("t2"));

        // Les deux abonnés ont tout reçu, dans le même ordre.
        let expected = vec![imported("t1"), imported("t2")];
        assert_eq!(*a.received.lock().unwrap(), expected);
        assert_eq!(*b.received.lock().unwrap(), expected);
        assert_eq!(bus.subscriber_count(), 2);
    }

    /// Abonné de test qui republie un événement quand il en reçoit un.
    struct Republisher {
        /// Bus sur lequel republier.
        bus: Arc<EventBus>,
        /// Enregistreur qui reçoit aussi chaque événement.
        recorder: Arc<Recorder>,
    }

    impl EventSubscriber for Republisher {
        /// Enregistre l'événement puis, pour une fin de session, en publie un autre.
        fn on_event(&self, event: &DomainEvent) {
            self.recorder.on_event(event);
            // Republier depuis un abonné : c'est ce cas qui provoquerait un interblocage.
            if let DomainEvent::SessionCompleted { session_id, .. } = event {
                self.bus.publish(&imported(session_id));
            }
        }
    }

    /// Un abonné peut publier pendant la diffusion sans bloquer le bus.
    #[test]
    fn subscriber_can_publish_without_deadlock() {
        // Un seul abonné, qui republie.
        let bus = Arc::new(EventBus::new());
        let recorder = Arc::new(Recorder::default());
        bus.subscribe(Arc::new(Republisher {
            bus: bus.clone(),
            recorder: recorder.clone(),
        }));

        // La publication déclenche une seconde publication imbriquée.
        bus.publish(&DomainEvent::SessionCompleted {
            session_id: "s1".into(),
            profile_id: "p1".into(),
            words: SessionWords::default(),
        });

        // Les deux événements sont arrivés : l'original puis le republié.
        let received = recorder.received.lock().unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(received[1], imported("s1"));
    }

    /// Le JSON utilise une étiquette `type` et des champs en camelCase, dans les deux sens.
    #[test]
    fn events_serialize_with_camel_case_tag() {
        // Sérialisation vers le format attendu par l'interface.
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
        // Aller-retour : la désérialisation redonne le même événement.
        let back: DomainEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }
}
