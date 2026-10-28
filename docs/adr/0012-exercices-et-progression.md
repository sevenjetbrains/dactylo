# 0012 — Exercices et progression dans un texte

- Statut : accepté — 2026-10-28

## Décision

- Un texte porte son **découpage** (`texts.segmentation`, JSON de `Segmentation` : texte entier, N paragraphes ou N lignes par exercice ; migration 0002). Les exercices ne sont pas stockés : `dactylo-core::exercise::exercises` les recalcule à partir du texte. Ce sont des portions `[début, fin[` en graphèmes.
- Un paragraphe est un bloc de lignes séparé par une ligne vide si le texte en contient. Sinon, chaque ligne compte comme un paragraphe : c'est le cas d'un PDF reconstitué ou de beaucoup de textes collés.
- `text_progress.position` contient le **nombre d'exercices terminés** par le profil. On ne le fait jamais reculer : refaire un exercice ancien ne fait pas perdre la progression. Une fois le dernier exercice fait, le prochain proposé redevient le premier.
- Une session porte sur un exercice. Son rang, le nombre d'exercices et la portion du texte sont enregistrés dans `sessions.settings`, sous la clé `exercise`, à côté des réglages. La reprise après redémarrage (NF-06) reconstruit donc la même portion. Une session antérieure à cette décision vaut « texte entier ».
- Un exercice ne compte comme terminé que s'il a été tapé jusqu'au bout. Une session chronométrée arrêtée avant la fin ne fait pas avancer la progression.
- Modifier le contenu ou le découpage d'un texte remet la progression de tous les profils à zéro, puisque les exercices changent.
- La commande de fin de session met à jour la progression directement, sans passer par un événement : la progression appartient au parcours du mode Copie.

## Conséquences

Pas de table d'exercices à garder cohérente avec le texte. L'interface reçoit déjà la portion à taper (`SessionStart.graphemes`) ; le miroir TypeScript (ADR 0008) n'a pas changé.
