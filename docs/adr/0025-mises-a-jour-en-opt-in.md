# 0025 — Mises à jour en opt-in

- Statut : accepté — 2026-10-29
- Amende : NF-11 (« aucun accès réseau en V1 ») et NF-12 (aucune donnée envoyée), pour ce seul cas.

## Contexte

La Phase 5 prévoit la mise à jour automatique. Elle suppose d'interroger un serveur, ce que NF-11 interdit. L'utilisateur a choisi une mise à jour **opt-in, désactivée par défaut**.

## Décision

- Plugins `updater` et `process` de Tauri. Le point d'accès est `https://github.com/sevenjetbrains/dactylo/releases/latest/download/latest.json`.
- **Aucune requête réseau tant que l'utilisateur ne l'a pas demandé** :
  - la case « Vérifier les mises à jour au démarrage » (Réglages → Données) est décochée par défaut. Ce réglage est commun aux profils et rangé dans `app_meta` (`check_updates_on_start`) ;
  - le bouton « Vérifier maintenant » fait une seule vérification.
- La requête est un simple GET du fichier `latest.json`, sans identifiant ni donnée d'usage. Le téléchargement n'a lieu que sur « Installer et redémarrer ».
- Au démarrage, si la case est cochée, le backend vérifie en tâche de fond. Une nouvelle version s'annonce par l'événement `UpdateAvailable` et un bandeau. Une erreur réseau est ignorée.
- Les paquets sont signés avec une clé minisign. La clé publique est dans `tauri.conf.json`. La clé privée reste hors du dépôt (`~/.tauri/dactylo-updater.key`, puis le secret `TAURI_SIGNING_PRIVATE_KEY`).
- Les artefacts de mise à jour (`createUpdaterArtifacts`) ne sont produits qu'en CI, et seulement si ce secret existe. Une construction locale n'a donc pas besoin de la clé.
- Permissions ajoutées : `updater:default` et `process:allow-restart`.

## Conséquences

- Sans action de l'utilisateur, Dactylo reste entièrement hors ligne (NF-03).
- Perdre la clé privée empêche de mettre à jour les installations existantes (voir `docs/release.md`).
