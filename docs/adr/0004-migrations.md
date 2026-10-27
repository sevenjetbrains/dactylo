# 0004 — Migrations sqlx

- Statut : accepté — 2026-10-27

## Décision

- Les migrations SQL sont dans `src-tauri/migrations/NNNN_nom.sql`, embarquées dans l'exécutable par `sqlx::migrate!` et appliquées automatiquement à l'ouverture de la base.
- Le suivi utilise la table `_sqlx_migrations` de sqlx (version, date, somme de contrôle) **à la place** de la table `schema_migrations` prévue au §3.3 : sqlx détecte en plus toute modification d'une migration déjà appliquée.
- Une migration publiée n'est jamais modifiée ; toute évolution passe par une nouvelle migration.
- À l'ouverture : `foreign_keys = ON`, `journal_mode = WAL`.
- Requêtes vérifiées à l'exécution (`query_as`), pas de macros `query!` : la compilation n'a pas besoin de `DATABASE_URL`.

## Conséquences

Les tests (`db::tests`) vérifient la création sur base vierge et qu'une réouverture n'applique rien deux fois.
