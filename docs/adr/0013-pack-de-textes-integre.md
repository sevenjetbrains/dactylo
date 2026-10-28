# 0013 — Pack de textes intégré

- Statut : accepté — 2026-10-28

## Décision

- Le pack (F-25) se compose de textes français du domaine public, copiés depuis Wikisource : trois fables de La Fontaine, *Demain, dès l'aube* (Hugo), *L'Albatros* (Baudelaire), le chapitre I du *Tour du monde en quatre-vingts jours* (Verne) et *La Parure* (Maupassant). Chaque texte est un fichier de `src-tauri/builtin/fr/`, embarqué par `include_str!` : le pack fonctionne hors ligne (NF-03).
- Préparation des fichiers :
  - appels et notes de bas de page retirés ;
  - « … » remplacé par « ... », introuvable sur un clavier AZERTY ;
  - strophes séparées par une ligne vide.

  Un test vérifie que chaque fichier est déjà en NFC et ne contient aucun de ces restes.
- Insertion **une seule fois**. La version du pack installée est retenue dans la table `app_meta` (clé `builtin_pack_version`, migration 0002). Un texte supprimé par l'utilisateur ne revient pas. Pour ajouter des textes plus tard, on augmente `PACK_VERSION` et on marque les nouveaux textes `since` : seuls eux sont insérés dans les bases existantes.
- Chaque texte a un découpage adapté : fables entières, poèmes par strophe, Verne par 4 paragraphes (8 exercices), Maupassant par 10 paragraphes.

## Conséquences

On peut s'entraîner dès le premier lancement. Les textes intégrés se modifient, se dupliquent ou se suppriment comme les autres. Les « œ » (« œil », « cœur ») restent, car ce sont les textes d'origine.
