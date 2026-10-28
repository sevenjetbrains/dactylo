# Dactylo — règles du projet

## Référence

- Spécifications : docs/cahier-des-charges.md (exigences F-xx et NF-xx).
- Toute décision d'architecture nouvelle = un fichier dans docs/adr/.

## Pile

Tauri 2, Rust (crate core + src-tauri), React 19 + TypeScript strict, Vite,
Tailwind + shadcn/ui, Zustand, SQLite via sqlx, pdfjs-dist, crate tts, i18next.

## Règles d'architecture

- La logique métier vit dans crates/core (Rust pur, sans Tauri). Jamais dans React.
- L'interface n'appelle le backend que via src/shared/api (client typé).
  Les types sont générés depuis Rust (specta / tauri-specta).
- Chaque fonctionnalité = un module dans src/modules/<nom> + commandes
  dans src-tauri/src/commands/<nom>. Un module s'enregistre dans le registre
  (route, entrée de menu, réglages) ; aucun autre fichier ne doit changer.
- Les modules ne s'importent pas entre eux : ils communiquent par les
  événements de domaine (SessionCompleted, WordMistyped...).
- Toute modification du schéma = nouvelle migration, jamais d'édition
  d'une migration existante.
- Synthèse vocale, planificateur de révision, import de source :
  toujours derrière un trait (SpeechEngine, Scheduler, TextSource).

## Saisie clavier

- Utiliser beforeinput / input / compositionend, pas seulement keydown
  (touches mortes AZERTY : ê, ë...). Comparer en Unicode NFC.

## Qualité

- TDD sur crates/core. Couverture >= 80 % sur core.
- Avant chaque commit : cargo fmt, cargo clippy -D warnings, cargo test,
  pnpm lint, pnpm typecheck, pnpm test.
- Commits conventionnels, référençant l'exigence : feat(practice): F-03 ...
- Une phase à la fois (section 6.2). Ne pas commencer la suivante
  sans validation.

## Langue

- Code et noms en anglais ; interface en français via i18n ;
  commentaires et docs en français.

## Commentaires

- Doc-commentaire (`//!`, `///`, `/** */`) sur chaque fichier, fonction,
  type, champ et constante, publics ou privés, tests compris.
- Un commentaire au-dessus de chaque bloc logique (boucle, condition,
  étape d'un traitement) qui explique le pourquoi, pas la paraphrase.
- Exceptions : code généré (src/shared/api/bindings.ts) et migrations SQL
  existantes.
