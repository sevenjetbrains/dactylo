# 0026 — Signature des installeurs

- Statut : accepté — 2026-10-29

## Contexte

Un installeur Windows non signé déclenche l'alerte SmartScreen, et une application macOS non notarisée est bloquée par Gatekeeper. Les certificats ne sont pas encore disponibles. L'utilisateur a choisi de câbler la signature dans la CI dès maintenant, pour qu'elle s'active dès que les secrets seront fournis.

## Décision

- `release.yml` calcule pour chaque job ce qui est disponible (`HAS_WINDOWS_CERT`, `HAS_APPLE_CERT`, `HAS_UPDATER_KEY`) et n'active que les étapes correspondantes :
  - **Windows** : le PFX (secret en base64) est importé dans le magasin de certificats de l'utilisateur. Son empreinte est passée à Tauri par `--config` (`certificateThumbprint`, SHA-256, horodatage DigiCert) ;
  - **macOS** : les variables `APPLE_*` ne sont exportées que si elles existent. Tauri signe et notarise ;
  - **Linux** : pas de signature (AppImage et deb).
- Sans secrets, la release sort non signée, comme avant.
- `docs/release.md` décrit les secrets, comment obtenir les certificats et comment vérifier une release.

## Conséquences

- Aucun secret n'est requis pour construire en local ou en CI ordinaire (`ci.yml`).
- Un certificat Windows sur clé matérielle (cas fréquent des certificats récents) ne s'exporte pas en PFX. Il faudra alors remplacer l'étape Windows par Azure Trusted Signing.
