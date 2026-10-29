# 0020 — Dictée menée par le backend, texte caché

- Statut : accepté — 2026-10-29

## Contexte

En dictée (F-10), l'élève tape sans voir le texte, qui n'est révélé qu'à la correction. La lecture se fait par segments (F-11), avec des répétitions (F-14) et un avancement adaptatif ou automatique (F-15). La dictée doit pouvoir reprendre après une fermeture brutale (NF-06).

## Décision

- **Le texte ne quitte pas le backend avant la correction.**
  - `start_dictation` renvoie seulement le nombre de mots de chaque segment.
  - `speak_dictation_segment(session, index, repeat)` fait lire le segment par le moteur vocal (ADR 0019) et renvoie un numéro de lecture. La limite de répétitions est vérifiée côté backend.
  - La correction (`finish_dictation`) renvoie enfin l'alignement complet (ADR 0021).
- **Saisie libre**, dans une zone de texte, comme sur la maquette. Il n'y a pas de comparaison caractère par caractère, faute de caractère « attendu » pendant une dictée. Les frappes brutes (F-31) ne concernent donc que la Copie. La dictée enregistre son **texte tapé** (`sessions.typed_text`, migration 0005).
- **Reprise** :
  - le texte tapé et le temps actif sont envoyés toutes les 10 s (`save_dictation_draft`) ;
  - la session (mode `dictation`) range ses réglages et son exercice dans `sessions.settings`, ce qui permet de la reconstruire après un redémarrage ;
  - `pending` distingue la famille Copie (copie, révision) de la dictée ;
  - démarrer une session de l'une abandonne celle de l'autre : une seule session en cours par profil.
- **Abandon** : une dictée sans frappes mais avec un texte tapé est gardée comme abandonnée (ADR 0014). Elle n'est supprimée que si rien n'a été tapé.
- **Réglages en cours de dictée** (`update_dictation_settings`) : voix, vitesse, hauteur, volume, pause, avancement et ponctuation lue s'appliquent au segment suivant. Le découpage reste celui du début. Le changer relance une dictée.
- **Profils de dictée (F-17)** : table `dictation_profiles`, complétée par la migration 0005 (avancement, répétitions, tolérance typographique). Les réglages courants restent sous la clé `dictation` de `profiles.settings` (ADR 0010).
- **Chiffres** :
  - MPM = mots tapés par minute ;
  - précision = mots attendus écrits juste ;
  - moyenne des 10 dernières dictées pour la comparaison.
- La correction publie `SessionCompleted`, avec un mot par mot attendu, et un `WordMistyped` par mot fautif : les mots de la dictée entrent en révision comme ceux de la copie.

## Conséquences

- Un texte de dictée ne peut pas être lu dans les outils de développement du navigateur pendant l'exercice.
- Le pilotage (minuteur de pause, comptage des mots tapés) reste dans l'interface, qui applique les règles du cœur (`dictation::advance`).
