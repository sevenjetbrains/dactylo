# 0007 — Stratégie de tests de bout en bout

- Statut : accepté — 2026-10-27

## Contexte

Playwright ne pilote pas directement le webview de Tauri sur toutes les plateformes (WKWebView sous macOS).

## Décision

- Phase 0 : Playwright tourne contre le serveur Vite, avec le pont IPC (`window.__TAURI_INTERNALS__`) simulé. Ces tests couvrent l'interface et la navigation au clavier sur les 3 OS de la CI.
- Le backend est couvert par les tests Rust (`cargo test`), base SQLite comprise.
- Plus tard : un test de fumée sur l'application réelle sous Windows, via le protocole CDP de WebView2 (`connectOverCDP`).
