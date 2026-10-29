# 0016 — Planificateur SM-2 et maîtrise des mots

- Statut : accepté — 2026-10-29

## Contexte

F-42 demande une répétition espacée (SM-2 ou FSRS) et la maîtrise d'un mot après 3 réussites de suite, sans faute et à vitesse normale. Le planificateur doit rester remplaçable (trait `Scheduler`, CLAUDE.md).

## Décision

- `dactylo_core::srs::Scheduler` calcule l'état suivant (`SrsState` : répétitions, intervalle en jours, facilité). Ce dernier est stocké en JSON dans `words.srs_state`.
- Implémentation `Sm2` :
  - intervalles de 1 jour, puis 6 jours, puis l'intervalle précédent × la facilité ;
  - facilité initiale 2,5, minimum 1,3 ;
  - un échec remet le compteur à zéro et redonne 1 jour.
- Note d'une session pour un mot : faute = `Again` (q = 1) ; juste mais lent = `Hard` (q = 3) ; juste à vitesse normale = `Good` (q = 5).
- **Une note par mot et par session**, même si le mot y figure plusieurs fois : une seule faute suffit à rater la session. Ainsi, un exercice « mot répété 5 fois » ne rend pas un mot maîtrisé d'un coup.
- Chaque mot rencontré est compté (`encounters`, `errors`, `avg_ms`). Il n'entre en révision (`next_review_at`) que s'il est raté ou lent.
- `streak` compte les sessions `Good` d'affilée. À 3 (`MASTERY_STREAK`), le mot passe `mastered` et sort de la révision. Une faute ou une lenteur ultérieure le remet en `learning`, avec un état SM-2 neuf.
- `manual_mastered` (F-45) est compté, mais jamais replanifié. Un mot ajouté à la main est à réviser tout de suite.
- « À réviser aujourd'hui » = `next_review_at` avant le minuit local suivant (`days::end_of_day`). L'interface fournit le décalage horaire.

## Conséquences

- Critère transverse vérifié par les tests : un mot raté est à réviser le lendemain, puis maîtrisé après 3 sessions réussies.
- FSRS pourra remplacer SM-2 en implémentant `Scheduler`. Les états existants seront lus avec les valeurs par défaut de leurs champs manquants (`serde(default)`).
