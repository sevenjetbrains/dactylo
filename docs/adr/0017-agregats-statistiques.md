# 0017 — Agrégats statistiques

- Statut : accepté — 2026-10-29

## Contexte

Le tableau de bord (F-33), la carte de chaleur (F-34) et l'export (F-35) sont calculés à partir de l'historique : sessions et frappes brutes (F-31). Le calcul doit rester dans le cœur, testable sans base.

## Décision

- L'adaptateur lit des lignes brutes. Le cœur calcule :
  - `stats::dashboard` : courbe des sessions terminées de la période, temps actif, précision moyenne, record, série de jours, tendance et semaine en cours ;
  - `stats::keys::KeyAnalyzer` : statistiques par touche et bigrammes lents, à partir des frappes de chaque session ;
  - `stats::csv` : fichier CSV.
- **Périodes** : 7 jours, 30 jours (aujourd'hui compris, depuis minuit local), ou tout.
- **Jours locaux** : le cœur ne connaît pas le fuseau. L'interface passe son décalage UTC en minutes (`tzOffsetMinutes`, `days`). Les semaines commencent le lundi.
- **Sessions abandonnées** (ADR 0014) :
  - comptées dans le temps d'entraînement, la série de jours et la carte de chaleur, car leurs frappes sont réelles ;
  - exclues de la courbe, du record, des moyennes et de la tendance, car elles n'ont pas de statistiques.
- **Série** : jours locaux consécutifs avec au moins une session. Elle n'est pas cassée tant que la journée en cours n'est pas finie.
- **Tendance** (tuiles de l'accueil) : moyenne des 10 dernières sessions terminées, et écart avec les 10 précédentes.
- **Touches AZERTY** :
  - minuscules ;
  - é, è, ç, à, ù ont leur touche ;
  - â, ê, î, ô, û, ë, ï, ü… comptent sur la touche morte (^ ou ¨) **et** sur la lettre de base ;
  - les autres accents comptent sur la seule lettre de base ;
  - espaces hors carte.
  - Le temps d'une frappe juste est l'écart avec la frappe précédente. Un écart de plus de 3 s est une hésitation, ignorée.
- **Bigrammes** : deux insertions justes consécutives à des positions voisines, sans espace. Les 5 plus lents parmi ceux vus au moins 5 fois.
- **CSV** :
  - séparateur `;`, BOM UTF-8 et virgule décimale en français, pour qu'Excel ouvre le fichier directement ;
  - en-têtes, modes et états traduits par l'interface (NF-08) ;
  - le fichier est produit par Rust, puis téléchargé par la WebView (lien `blob:`), sans permission fichier (NF-11).

## Conséquences

- Le calcul lit toutes les sessions du profil et les frappes de la période. Pour quelques milliers de sessions, cela reste rapide. Un cache ou des agrégats en base pourront venir si la mesure le justifie.
- D'autres dispositions (QWERTY, BÉPO, F-08) demanderont une table de touches par disposition. `keys_of` est le seul endroit à adapter.
