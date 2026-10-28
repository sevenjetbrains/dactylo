# 0010 — Réglages par module dans `profiles.settings`

- Statut : accepté — 2026-10-28

## Décision

- Chaque module range ses réglages sous sa propre clé du JSON `profiles.settings` (ex. `practice`). Écrire une clé ne touche pas les autres (`json_set`).
- Le type des réglages, leurs valeurs par défaut et leurs bornes vivent dans le cœur (ex. `dactylo-core::settings::PracticeSettings`). La lecture est tolérante : un champ absent ou invalide prend sa valeur par défaut, une valeur hors bornes est ramenée dans ses bornes.
- Une session copie les réglages utilisés dans `sessions.settings` : la reprise (NF-06) et les statistiques utilisent les règles de la session, même si les réglages changent ensuite.

## Conséquences

Ajouter un réglage ne demande aucune migration (§3.3). Les réglages purement visuels et propres à l'appareil (thème) restent côté interface.
