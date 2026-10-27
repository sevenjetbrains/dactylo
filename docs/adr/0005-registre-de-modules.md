# 0005 — Registre de modules

- Statut : accepté — 2026-10-27

## Décision

- **Interface** : chaque module exporte un `ModuleManifest` (`src/app/types.ts`) qui décrit sa route, son entrée de menu, son panneau de réglages et ses traductions. `src/app/registry.ts` contient le tableau des modules, **une ligne par module**. Le routeur, le menu, la page Réglages et i18n sont construits à partir de ce tableau.
- **Backend** : chaque module a `src-tauri/src/commands/<nom>.rs` et **une ligne** dans `collect_commands!` (`commands/mod.rs`).
- Ajouter un module coûte donc son dossier, plus une ligne par côté (une seule s'il n'a pas de commande).
- ESLint interdit à un module d'importer un autre module, et au shell d'importer un module ailleurs que dans le registre. Les modules qui ont besoin de la liste (Réglages) la lisent via `useModules()`.

## Conséquences

Le test « module Hello » (`registry.test.tsx`) prouve qu'un module ajouté au tableau obtient sa route, son menu et ses traductions.
