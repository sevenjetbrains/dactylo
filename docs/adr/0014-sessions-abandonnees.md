# 0014 — Sessions abandonnées conservées

- Statut : accepté — 2026-10-28

## Contexte

Jusqu'ici, une session commencée puis non terminée était supprimée avec ses frappes dans trois cas : au démarrage d'une autre session, à l'abandon (« Recommencer à zéro », Ctrl+R, texte ou exercice suivant), ou si elle n'avait aucune frappe. Seule la dernière session en cours restait, pour la reprise (NF-06). Cela contredisait F-31 et la section 3.1 : chaque frappe doit être conservée, pour que de futures statistiques puissent être calculées après coup.

## Décision

- Une session a trois états :
  - **en cours** : ni `ended_at` ni `abandoned_at` ;
  - **terminée** : `ended_at` ;
  - **abandonnée** : `abandoned_at` (migration 0003).
- Abandonner une session (`sessions::abandon`, ou démarrer une autre session du profil) la **garde avec ses frappes**. Elle est datée, et `duration_ms` reçoit le temps actif de sa dernière frappe.
- Une session **sans aucune frappe** est toujours supprimée : elle n'a rien à garder.
- Une session abandonnée n'est plus proposée à la reprise. Elle ne reçoit plus de frappes et ne peut plus être close.
- La moyenne de l'écran de fin (F-32) et le meilleur score de la bibliothèque ne comptent que les sessions terminées ; les abandonnées en sont exclues d'office.

## Conséquences

Les statistiques de la phase 3 (F-33 à F-35) pourront compter les abandons à part, et inclure leurs frappes dans les analyses : temps de frappe total, fautes par touche… La base grossit un peu plus, d'une ligne par frappe, comme pour les sessions terminées.
