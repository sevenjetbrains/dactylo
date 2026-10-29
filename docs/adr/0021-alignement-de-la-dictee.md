# 0021 — Alignement et classement des fautes de dictée

- Statut : accepté — 2026-10-29

## Contexte

F-16 demande un alignement mot à mot du texte tapé sur le texte attendu, par distance d'édition, et un classement des fautes : accent, lettre manquante, lettre en trop, inversion, mot oublié, ponctuation, majuscule. Critère transverse : une dictée de 50 mots avec 5 fautes connues produit exactement ces 5 fautes, bien classées.

## Décision

- **Jetons** : les mots de `text::word_spans` et les signes de ponctuation isolés. Le trait d'union interne fait partie du mot ; l'apostrophe sépare (« d'environ » → « d », « ' », « environ »). Les espaces ne sont pas des jetons. Chacun retient seulement s'il était précédé d'une espace, pour l'affichage.
- **Alignement** : distance d'édition sur les jetons (`dictation::correct`), avec ces coûts :

  | Opération | Coût |
  | --- | --- |
  | Jeton identique (tolérance typographique au choix) | 0 |
  | Mot à une faute près, ou ponctuation remplacée | 1 |
  | Mot sans rapport | 3 |
  | Jeton oublié ou en trop | 2 |
  | Fusion ou scission de 2 ou 3 mots dont les lettres coïncident, traits d'union mis à part | 1 |
  | Mot face à ponctuation | interdit |

  À coût égal, l'appariement est préféré, puis la fusion ou la scission, puis l'oubli, puis l'ajout.
- **Classement** d'un mot faux, dans cet ordre :
  1. majuscule (égal sans casse) ;
  2. accent (égal sans diacritiques) ;
  3. inversion (deux lettres voisines échangées) ;
  4. lettre manquante, puis lettre en trop (une lettre de moins ou de plus) ;
  5. sinon lettre erronée (`substitution`).
- **Cas particuliers** : un mot sans correspondant est « mot oublié », un mot tapé en plus est « mot en trop ». Une ponctuation oubliée, ajoutée ou remplacée, ainsi qu'une fusion ou une scission (« soixante quinze » pour « soixante-quinze », « parceque » pour « parce que ») comptent comme « ponctuation ».
- **Comptes** : une faute par jeton aligné faux. La précision est le nombre de mots attendus écrits juste, divisé par le nombre de mots attendus.
- **Révision** : chaque mot attendu d'un seul tenant devient un `WordAttempt` (`dictation::session_words`), avec une durée nulle, donc jamais « lent ». Le module Révision le traite comme un mot de copie (ADR 0015).

## Conséquences

- La table est de taille (mots attendus × mots tapés). Pour une dictée de quelques centaines de mots, cela reste instantané.
- Une faute combinée (accent + lettre manquante) est classée selon son premier critère. Le mot tapé et le mot attendu restent visibles dans la correction.
