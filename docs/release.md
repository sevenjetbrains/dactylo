# Publier une version de Dactylo

Ce document explique comment signer les installeurs et publier une version, mises à jour comprises (ADR 0025 et 0026).

## 1. Publier

1. Mettre la même version dans `package.json`, `Cargo.toml` (section `workspace.package`) et `src-tauri/tauri.conf.json`.
2. Committer, puis pousser une étiquette : `git tag v0.2.0 && git push origin v0.2.0`.
3. Le workflow **Release** construit les installeurs Windows, macOS et Linux dans une release GitHub **en brouillon**.
4. Relire la release, écrire ses notes (elles s'affichent dans « Mises à jour »), puis la publier.

Tant que la release reste en brouillon, les applications installées ne la voient pas : elles lisent `releases/latest/download/latest.json`.

## 2. Secrets GitHub

Chaque signature s'active d'elle-même quand ses secrets existent. Sans eux, la release sort quand même, mais non signée. Les secrets se créent dans _Settings → Secrets and variables → Actions_.

### Mises à jour (obligatoire pour la mise à jour en un clic)

| Secret                               | Contenu                                                                  |
| ------------------------------------ | ------------------------------------------------------------------------ |
| `TAURI_SIGNING_PRIVATE_KEY`          | Contenu du fichier `~/.tauri/dactylo-updater.key` (clé privée minisign). |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Vide : la clé a été générée sans mot de passe.                           |

La clé publique correspondante est dans `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

- **La clé privée ne doit jamais être commitée.** Gardez-en une copie hors de l'ordinateur, par exemple dans un gestionnaire de mots de passe.
- Si elle est perdue, les versions déjà installées ne pourront plus se mettre à jour automatiquement. Il faudra générer une nouvelle paire (`pnpm tauri signer generate -w ~/.tauri/dactylo-updater.key`), mettre la nouvelle clé publique dans la configuration, et réinstaller une fois à la main.

### Windows (Authenticode)

| Secret                         | Contenu                                                                                                                                      |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `WINDOWS_CERTIFICATE`          | Le certificat de signature de code au format PFX, encodé en base64 : `[Convert]::ToBase64String([IO.File]::ReadAllBytes("certificat.pfx"))`. |
| `WINDOWS_CERTIFICATE_PASSWORD` | Mot de passe du PFX.                                                                                                                         |

- Le certificat s'achète auprès d'une autorité (Certum, Sectigo, DigiCert…). Un certificat « OV » fait disparaître l'alerte SmartScreen au fil des téléchargements ; un certificat « EV » la supprime tout de suite.
- Les certificats récents sont souvent livrés sur clé matérielle et ne s'exportent pas en PFX. Dans ce cas, Azure Trusted Signing est l'alternative : il faudra alors remplacer l'étape « Certificat Windows » du workflow.

### macOS (signature et notarisation)

| Secret                       | Contenu                                                                      |
| ---------------------------- | ---------------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | Certificat « Developer ID Application » exporté en `.p12`, encodé en base64. |
| `APPLE_CERTIFICATE_PASSWORD` | Mot de passe du `.p12`.                                                      |
| `APPLE_SIGNING_IDENTITY`     | Par exemple `Developer ID Application: Nom (TEAMID)`.                        |
| `APPLE_ID`                   | Identifiant Apple du compte développeur.                                     |
| `APPLE_PASSWORD`             | Mot de passe d'application (appleid.apple.com → Sécurité).                   |
| `APPLE_TEAM_ID`              | Identifiant d'équipe.                                                        |

Il faut une adhésion à l'Apple Developer Program, à 99 $ par an.

## 3. Vérifier une release

- **Windows** : propriétés de l'installeur → onglet _Signatures numériques_, ou `Get-AuthenticodeSignature .\Dactylo_*.exe`.
- **Mises à jour** : la release doit contenir `latest.json` et un fichier `.sig` par installeur. Dans une version plus ancienne installée, _Réglages → Données → Vérifier maintenant_ doit proposer la nouvelle version.
