# 0022 — Clavier virtuel et dispositions

- Statut : accepté — 2026-10-29

## Contexte

F-08 demande un clavier virtuel optionnel sous la zone de frappe. Il met en évidence la touche attendue et le doigt conseillé, en AZERTY par défaut, avec QWERTY et BÉPO au choix. Savoir quelles touches frapper pour un caractère (« ê » = touche morte ^ puis E) est une règle métier : elle va dans le cœur.

## Décision

- `dactylo_core::keyboard` décrit les trois dispositions sur cinq rangées.
  - Une touche (`KeyDef`) a un code physique (`KeyboardEvent.code`), des légendes normale, Maj et AltGr, un drapeau « touche morte », un doigt et une largeur.
  - Les doigts suivent la méthode classique à dix doigts et dépendent de la position physique : ils sont communs aux trois dispositions.
- `keys_for(graphème, disposition)` renvoie les frappes successives (`Press` : code, Maj, AltGr, verrouillage des majuscules), dans cet ordre de recherche :
  1. espaces, retour à la ligne, tabulation ;
  2. caractère présent sur une touche ;
  3. majuscule, avec Maj sur une touche de lettre, sinon verrouillage des majuscules (AZERTY Windows : « É » = Verr. Maj + é) ;
  4. lettre accentuée : touche morte, puis la lettre de base (« ê », « ë », « Ê ») ;
  5. variante clavier d'un signe typographique (’ → ', « » → ") ;
  6. à défaut, la lettre sans accent (« é » en QWERTY → E).

  Une liste vide veut dire qu'aucune touche ne convient.
- `PracticeSettings` gagne `show_keyboard` (vrai par défaut, comme la maquette) et `typing_sounds` (F-51, faux par défaut). La lecture tolérante évite une migration (ADR 0010).
- La disposition vient du profil (`profiles.keyboard_layout`, F-50).

## Conséquences

- La carte de chaleur (ADR 0017) reste en AZERTY pour l'instant. La faire suivre la disposition du profil demandera de réutiliser `keys_for` dans `stats::keys`.
- Une nouvelle disposition (BÉPO 2, QWERTZ…) se décrit dans `keyboard::layouts`, sans rien changer ailleurs.
