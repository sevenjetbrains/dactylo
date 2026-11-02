# Dactylo

Application desktop hors-ligne d'entraînement à la frappe et à l'orthographe en français.

- Spécifications : [docs/cahier-des-charges.md](docs/cahier-des-charges.md)
- Décisions d'architecture : [docs/adr/](docs/adr/)
- Règles de contribution (y compris pour Claude Code) : [CLAUDE.md](CLAUDE.md)

## Prérequis

- Rust stable (via [rustup](https://rustup.rs)) ; sous Windows, les MSVC Build Tools (C++)
- Node.js 22 ou plus et pnpm 10
- Windows : WebView2 (préinstallé sur Windows 10/11)
- Linux : `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libspeechd-dev libclang-dev` (les deux derniers pour la synthèse vocale, crate `tts`)

## Commandes

```sh
pnpm install
pnpm tauri dev      # application en développement
pnpm tauri build    # installeur (NSIS sous Windows)

# Vérifications avant chaque commit
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
pnpm lint && pnpm typecheck && pnpm test
pnpm e2e            # Playwright, IPC simulé
```

`cargo test` régénère aussi le client typé `src/shared/api/bindings.ts` à partir des commandes Rust.

## Structure

| Dossier             | Rôle                                            |
| ------------------- | ----------------------------------------------- |
| `crates/core`       | Cœur métier en Rust pur (paquet `dactylo-core`) |
| `src-tauri`         | Commandes, base SQLite, migrations, permissions |
| `src/app`           | Shell, routeur et registre des modules          |
| `src/modules/<nom>` | Un dossier par fonctionnalité                   |
| `src/shared`        | Client API typé, composants UI, thème           |

## Ajouter un module

1. Créer `src/modules/<nom>/index.ts`, qui exporte un `ModuleManifest` (route, menu, réglages, traductions).
2. L'ajouter au tableau de `src/app/registry.ts`.
3. S'il a des commandes : créer `src-tauri/src/commands/<nom>.rs` et ajouter ses commandes sur une ligne dans `collect_commands!` (`src-tauri/src/commands/mod.rs`).
