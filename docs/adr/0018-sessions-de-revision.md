# 0018 — Sessions de révision sans texte stocké

- Statut : accepté — 2026-10-29

## Contexte

Les exercices de révision (F-44) sont générés à la demande à partir des mots à réviser. Ce sont des textes jetables. Les ranger dans `texts` remplirait la bibliothèque d'exercices morts. De plus, la contrainte `texts.source` n'admet que `paste`, `pdf` et `builtin`, et l'élargir demanderait de reconstruire la table.

## Décision

- `start_session` reçoit un paramètre `generated` : le texte de l'exercice. La session est créée avec `mode = 'review'` et `text_id = NULL`.
- Le texte est rangé dans `sessions.settings` (clé `content`), avec les réglages et l'exercice. La reprise après fermeture (NF-06) reconstruit la session à partir de là.
- `SessionStart`, `PendingSession` et `SessionSummary` exposent `mode` (`copy` ou `review`) et un `textId` facultatif. Une révision n'a pas de titre stocké : l'interface affiche le sien.
- Le module Révision génère le texte (`review_exercise`), puis ouvre la page Copie en le passant dans l'état de navigation (`navigate('/practice', { state: { review } })`). Aucun module n'en importe un autre.
- Dans une révision :
  - Ctrl+R recommence sur le même texte ;
  - « suivant » (Ctrl+Entrée, écran de fin) et « Recommencer à zéro » ramènent à la page Révision, qui propose un nouvel exercice.
- Pas de progression de texte (F-24) pour une révision. La moyenne de l'écran de fin (F-32) reste celle des sessions de copie.

## Conséquences

- Les mots tapés pendant une révision passent par le même `SessionCompleted` que la copie (ADR 0015). Une révision réussie fait donc avancer leur planification.
- Une session de copie dont le texte a été supprimé (`text_id` mis à `NULL`) n'est toujours pas reprenable : seule une révision porte son texte.
