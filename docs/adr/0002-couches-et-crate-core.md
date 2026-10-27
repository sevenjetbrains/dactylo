# 0002 — Couches et nom du crate `dactylo-core`

- Statut : accepté — 2026-10-27

## Décision

- `crates/core` contient la logique métier en Rust pur, **sans dépendance à Tauri**. Seuls serde et, en option (fonctionnalité `specta`), specta sont autorisés.
- Le paquet s'appelle `dactylo-core`, et non `core`, pour éviter le conflit avec la bibliothèque standard `core::`.
- `src-tauri` contient les adaptateurs : commandes, dépôts SQLite, relais des événements, et plus tard la synthèse vocale.
- L'interface ne contient aucune règle métier ; elle passe par `src/shared/api`.

## Conséquences

Le cœur se teste avec `cargo test -p dactylo-core`, sans webview ; c'est sur lui que porte la couverture ≥ 80 % (NF-10).
