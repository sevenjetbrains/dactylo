# 0006 — Événements de domaine

- Statut : accepté — 2026-10-27

## Décision

- `dactylo-core::events` définit `DomainEvent` (`SessionCompleted`, `WordMistyped`, `TextImported`), le trait `EventSubscriber` et un `EventBus` synchrone en mémoire.
- `src-tauri` crée le bus au démarrage et y abonne un `FrontendForwarder`, qui relaie chaque événement à l'interface sous forme d'événement Tauri typé (`DomainEventPayload`, généré par tauri-specta).
- Les modules Rust (statistiques, révision…) s'abonnent au bus ; les modules React écoutent `events.domainEventPayload`.

## Conséquences

- La publication est synchrone : un abonné lent ralentit l'émetteur. On passera à une file asynchrone si un abonné fait des E/S lourdes.
- Un abonné peut publier à son tour sans interblocage (testé).
