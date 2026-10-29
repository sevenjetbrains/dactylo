# 0024 — Sauvegarde et restauration

- Statut : accepté — 2026-10-29

## Contexte

F-52 : sauvegarder et restaurer toutes les données dans un fichier unique, qui est une copie de la base SQLite. NF-11 n'autorise l'accès aux fichiers qu'à travers la boîte de dialogue.

## Décision

- **Choix des fichiers** : plugin `dialog`. L'interface ouvre la boîte « Enregistrer » ou « Ouvrir » du système, puis passe le chemin choisi au backend. Seules permissions ajoutées : `dialog:allow-save` et `dialog:allow-open`, sans aucune permission du plugin `fs`.
- **Sauvegarde** : `VACUUM INTO` vers le fichier choisi, remplacé s'il existe (la boîte de dialogue a demandé confirmation). Le résultat est une copie cohérente et compacte, même pendant l'utilisation.
- **Restauration** :
  1. Le fichier est vérifié en lecture seule : base SQLite lisible, tables `_sqlx_migrations` et `profiles` présentes, version du schéma inférieure ou égale à celle de l'application.
  2. Il est copié à côté de la base (`dactylo.db.restore`), puis l'application ferme la base et redémarre.
  3. Au démarrage, avant d'ouvrir la base, `apply_pending` installe la copie. L'ancienne base et ses journaux WAL (`-wal`, `-shm`), qui n'appartiennent qu'à elle, sont gardés en `dactylo.db.bak`.
  4. Les migrations manquantes s'appliquent ensuite, comme à toute ouverture (NF-05).

## Conséquences

- Une restauration se défait en remettant `dactylo.db.bak` à la place de la base.
- `VACUUM INTO` depuis une base en mémoire écrit en mémoire : les tests de sauvegarde utilisent une base fichier.
