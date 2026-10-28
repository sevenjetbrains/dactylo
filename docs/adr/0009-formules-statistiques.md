# 0009 — Formules des statistiques de session

- Statut : accepté — 2026-10-28

## Décision

Implémentées dans `dactylo-core::stats::compute`, à partir des compteurs de `TypingSession` et du **temps actif** (pauses exclues, chronomètre lancé à la première frappe).

| Mesure    | Formule                                                       |
| --------- | ------------------------------------------------------------- |
| MPM brut  | (insertions ÷ 5) ÷ minutes                                    |
| MPM net   | MPM brut − (caractères restés faux ÷ minutes), plancher 0     |
| CPM       | caractères justes à la fin (corrigés compris) ÷ minutes       |
| Précision | insertions justes ÷ insertions × 100 ; 100 % si aucune frappe |
| Erreurs   | insertions fausses, même corrigées ensuite                    |

- Une « insertion » est une frappe de caractère acceptée ; les retours arrière ne comptent pas.
- Les fautes enregistrées (`session_errors`) : une ligne par mot mal écrit au premier passage (type dominant), une ligne par espace ou ponctuation fautive hors mot.
- La comparaison de l'écran de fin (F-32) porte sur la moyenne des 10 dernières sessions terminées du même mode, hors session courante.

## Conséquences

Les frappes brutes étant conservées (F-31), une autre formule pourra être recalculée rétroactivement sans migration.
