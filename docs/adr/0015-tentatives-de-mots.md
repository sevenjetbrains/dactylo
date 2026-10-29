# 0015 — Tentatives de mots dans `SessionCompleted`

- Statut : accepté — 2026-10-29

## Contexte

Le suivi des mots difficiles (F-40 à F-42) doit connaître **tous** les mots tapés, pas seulement les fautes : un mot réussi compte comme rencontre et allonge la série de réussites, et un mot juste mais lent devient difficile. `WordMistyped` ne porte que les fautes, sans durée.

## Décision

- Le cœur calcule `TypingSession::word_attempts()`, qui renvoie des `SessionWords` :
  - `attempts` : un `WordAttempt` par mot entièrement tapé. Il porte la clé (minuscules, NFC), le mot du texte, la forme tapée au premier passage, la justesse, le type de faute, la durée et le nombre de caractères chronométrés ;
  - `ms_per_char` : la durée moyenne par caractère de tous les mots tapés de la session.
- Durée d'un mot : de la frappe qui le précède (souvent l'espace) à sa dernière insertion. Le premier mot du texte part de sa première lettre, qui n'a donc pas de durée propre.
- Les mots d'une lettre (« a », « l ») et les nombres ne sont pas suivis, mais ils comptent dans la moyenne.
- `SessionCompleted` gagne le champ `words`. `WordMistyped` reste publié tel quel.
- **La lenteur n'est pas jugée par le mode Copie.** Le seuil (F-40, 1,5 par défaut) est un réglage du module Révision. Celui-ci appelle `WordAttempt::is_slow(ms_per_char, seuil)` à la réception de l'événement. Ainsi, le mode Copie ne dépend d'aucun réglage de la révision.
- Nouvel événement `WordsUpdated { profileId }`, publié quand la liste des mots d'un profil change. L'interface s'en sert pour rafraîchir le badge Révision et la carte de l'accueil.

## Conséquences

- La référence de lenteur est la session, pas l'historique. Un texte entier tapé lentement ne rend aucun mot lent. C'est voulu : un mot lent est un mot plus lent que les autres.
- L'événement transporte tous les mots de la session vers l'interface, ce qui reste modeste même pour un long texte.
- La même clé en minuscules regroupe « Le » et « le ». Une faute de casse reste visible dans la forme tapée et le type de faute.
