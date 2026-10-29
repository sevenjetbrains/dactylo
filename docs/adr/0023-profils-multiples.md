# 0023 — Profils multiples

- Statut : accepté — 2026-10-29

## Contexte

F-50 : plusieurs profils locaux (par exemple un parent et un enfant), chacun avec ses statistiques. Jusqu'ici, le profil actif était le plus ancien, fixé au démarrage.

## Décision

- `AppState` garde le profil actif derrière un `RwLock`, lu par `profile_id()` et changé par `set_profile()`. Les commandes lisent le profil actif à chaque appel.
- Le profil actif est retenu dans `app_meta` (clé `active_profile`). Au démarrage, on rouvre ce profil s'il existe encore, sinon le plus ancien (`profiles::active`).
- Commandes :
  - `list_profiles`, `create_profile(nom, disposition)`, `update_profile`, `switch_profile` ;
  - `delete_profile` : le dernier profil ne peut pas être supprimé. Supprimer le profil actif rend le plus ancien actif. Les données du profil supprimé partent en cascade (sessions, frappes, mots, profils de dictée, réglages).
- Changer de profil n'abandonne pas les sessions en cours de l'ancien profil : elles redeviennent reprenables quand il redevient actif (NF-06). L'interface envoie les frappes en attente, puis se recharge sur l'accueil du nouveau profil.
- La disposition clavier (F-08, F-51) est un réglage du profil (`profiles.keyboard_layout`). `keyboard_layout` et `keyboard_keys` servent celle du profil actif.

## Conséquences

- La bibliothèque reste commune à tous les profils ; la progression dans chaque texte est propre au profil (schéma initial).
- Le code ne lit plus `state.profile_id` comme une valeur figée : un changement de profil en cours de commande n'affecte que les commandes suivantes.
