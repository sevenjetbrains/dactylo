# 0001 — Pile technique : Tauri 2 + React + SQLite

- Statut : accepté — 2026-10-27

## Contexte

Application desktop hors-ligne, légère (installeur < 20 Mo), Windows d'abord (cahier des charges §2).

## Décision

- Tauri **2.12** (stable). Tauri 3 est en alpha : on ne l'adopte pas avant sa version stable.
- React 19, TypeScript **6.0** en mode strict (TypeScript 7 n'est pas encore pris en charge par typescript-eslint), Vite 8, Tailwind 4.
- SQLite via sqlx 0.9.
- Types partagés Rust → TypeScript avec tauri-specta **2.0.0-rc.26**, épinglé exactement (ainsi que `specta =2.0.0-rc.26` et `specta-typescript =0.0.13`).

## Conséquences

- tauri-specta est encore en RC : une montée de version peut changer l'API générée. On l'épingle et on la teste en CI (contrôle de `bindings.ts`).
- Plan B pour Linux si WebKitGTK pose problème : Electron (cahier §2.1).
