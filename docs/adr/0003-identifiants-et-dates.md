# 0003 — Identifiants UUIDv7 et dates en millisecondes

- Statut : accepté — 2026-10-27

## Décision

- Les entités (profils, textes, sessions, mots…) ont un identifiant UUIDv7 stocké en `TEXT`. Ces identifiants sont uniques sans coordination (synchronisation cloud future) et triables par date de création.
- Les tables de détail volumineuses (`session_errors`, `word_attempts`) utilisent un `INTEGER PRIMARY KEY` local ; `keystrokes` a une clé composite `(session_id, seq)`.
- Toutes les dates sont des `INTEGER` en millisecondes Unix (UTC). Le fuseau horaire est appliqué à l'affichage.

## Conséquences

Les identifiants prennent un peu plus de place que des entiers ; c'est sans importance aux volumes visés.
