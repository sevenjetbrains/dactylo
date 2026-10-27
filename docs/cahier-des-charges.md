# Cahier des charges — Application de dactylographie et d'orthographe

Oct 26, 2026 · @boualem

## 1. Contexte et objectifs

Le projet est une application desktop hors-ligne, nommée provisoirement **Dactylo**, qui entraîne à la fois la vitesse de frappe et l'orthographe en français, sur le modèle des sites de dactylographie en ligne.

**Objectifs mesurables**

- Afficher un texte à recopier, avec un nombre de lignes visibles configurable, et corriger la frappe caractère par caractère.
- Proposer un mode dictée : l'application lit le texte à voix haute, l'utilisateur l'écrit sans le voir.
- Gérer une bibliothèque de textes : copier-coller ou import d'un PDF (choix des pages et du nombre de lignes).
- Mesurer chaque session (MPM, précision, erreurs) et conserver l'historique.
- Repérer les mots mal écrits ou difficiles et générer des exercices ciblés sur ces mots.

**Utilisateur cible** : une personne francophone, clavier AZERTY par défaut, sur Windows en priorité (macOS et Linux ensuite).

**Hors périmètre V1** : comptes en ligne, synchronisation cloud, multijoueur, OCR de PDF scannés. L'architecture doit cependant permettre de les ajouter plus tard sans refonte.

**Glossaire**

| Terme | Définition |
| --- | --- |
| MPM | Mots par minute ; 1 mot = 5 caractères frappés, espaces compris (convention standard) |
| CPM | Caractères par minute |
| Précision | Caractères corrects ÷ caractères frappés, en % |
| Session | Une séance d'entraînement, du premier au dernier caractère |
| Mot difficile | Mot mal écrit au moins une fois, ou frappé nettement plus lentement que la moyenne |

## 2. Étude technologique

**Recommandation : Tauri 2 (cœur en Rust) + React/TypeScript (interface) + SQLite (données).** C'est le meilleur compromis pour une application légère, rapide au démarrage, hors-ligne, et que Claude Code sait très bien produire.

### 2.1 Comparatif des frameworks desktop

| Critère | Tauri 2 | Electron | Flutter Desktop | Qt / PySide |
| --- | --- | --- | --- | --- |
| Taille de l'installeur | 3 à 15 Mo | 80 à 250 Mo | 20 à 40 Mo | 40 à 100 Mo |
| Mémoire au repos | 30 à 80 Mo | 100 à 300 Mo | 60 à 120 Mo | 60 à 150 Mo |
| Interface | Web (React, etc.) | Web (React, etc.) | Dart / widgets | QML ou widgets |
| Lecture PDF | pdf.js ou Rust | pdf.js | paquets tiers | PyMuPDF (excellent) |
| Synthèse vocale | Rust (SAPI / AVSpeech / speech-dispatcher) | Web Speech API | plugin tiers | QtTextToSpeech |
| Mobile plus tard | Oui (iOS / Android) | Non | Oui | Partiel |
| Maîtrise par Claude Code | Très bonne | Très bonne | Bonne | Bonne |

Les chiffres de taille et de mémoire sont des ordres de grandeur issus de comparatifs de 2026 ([tech-insider](https://tech-insider.org/tauri-vs-electron-2026/), [OpenReplay](https://blog.openreplay.com/comparing-electron-tauri-desktop-applications/)) ; Flutter et Qt sont des estimations.

**Pourquoi Tauri plutôt qu'Electron** : un exécutable 10 à 20 fois plus petit, un démarrage plus rapide et un modèle de permissions plus strict. Electron reste le plan B si le moteur web de Linux (WebKitGTK) pose problème.

### 2.2 Pile technique retenue

| Couche | Technologie | Rôle |
| --- | --- | --- |
| Shell desktop | Tauri 2.x | Fenêtre, menus, accès fichiers, installeur, mises à jour |
| Cœur métier | Rust (crate `core` sans dépendance Tauri) | Comparaison de frappe, calcul des statistiques, répétition espacée |
| Interface | React 19 + TypeScript + Vite | Écrans, zone de frappe |
| Style | Tailwind CSS + shadcn/ui | Composants accessibles, thèmes clair/sombre |
| État UI | Zustand | État de la session en cours |
| Graphiques | Recharts | Courbes de progression |
| Base de données | SQLite via `sqlx` (Rust), migrations versionnées | Textes, sessions, erreurs, mots difficiles |
| PDF | `pdfjs-dist` côté interface (aperçu + extraction par page) | Import de texte depuis un PDF |
| Synthèse vocale | crate Rust `tts` (voix système) derrière une interface `SpeechEngine` | Mode dictée ; voix neuronale Piper en option future |
| Tests | Vitest + Testing Library, `cargo test`, Playwright | Unitaires, intégration, bout en bout |
| Qualité | ESLint, Prettier, Clippy, rustfmt, GitHub Actions | CI multi-plateforme |

### 2.3 Points techniques sensibles

- **Accents et touches mortes (AZERTY)** : ê, ë, ç, œ passent par des touches mortes. La saisie doit écouter les événements `beforeinput` / `input` et `compositionend`, jamais seulement `keydown`, sinon les caractères accentués sont mal comptés.
- **Normalisation Unicode** : comparer en NFC ; option pour assimiler apostrophes (’ / ') et guillemets (« » / "), fréquents dans les PDF.
- **Synthèse vocale** : la Web Speech API est inégale selon les moteurs web (faible sous Linux), d'où le passage par Rust. Vérifier la présence d'une voix française installée et guider l'utilisateur sinon.
- **PDF** : le texte extrait a des césures (« exem-\nple »), des en-têtes et des numéros de page. Prévoir un nettoyage avant enregistrement. Les PDF scannés (images) n'ont pas de texte : message clair en V1, OCR en module futur.

## 3. Architecture évolutive

L'application est découpée en trois couches et en modules indépendants : ajouter une fonction revient à ajouter un dossier, sans modifier l'existant.

```
┌──────────────────────────────────────────────────────────┐
│ Interface React — modules : Copie, Dictée, Bibliothèque, │
│ Statistiques, Révision, Réglages (+ registre)            │
└───────────────────────────┬──────────────────────────────┘
                            │ client API typé (tauri-specta)
┌───────────────────────────▼──────────────────────────────┐
│ Adaptateurs (src-tauri) — commandes, dépôts SQLite,      │
│ SpeechEngine, TextSource                                 │
└───────────────────────────┬──────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────┐
│ Cœur métier (crates/core) — frappe, stats, SRS, texte,   │
│ événements de domaine                                    │
└──────────────────────────────────────────────────────────┘
```

L'interface ne fait qu'afficher ; les règles (comparaison, calculs, révisions) sont dans le cœur, testable sans lancer l'application.

### 3.1 Principes d'extensibilité

- **Modules (feature slices)** : chaque fonction (Copie, Dictée, Bibliothèque...) a son dossier côté interface et ses commandes côté Rust. Elle se déclare dans un **registre** (route, entrée de menu, panneau de réglages).
- **Interfaces abstraites (traits)** pour tout ce qui peut changer : `SpeechEngine` (voix système aujourd'hui, Piper demain), `TextSource` (collage, PDF, puis OCR, EPUB, site web), `Scheduler` (SM-2 ou FSRS), `ExerciseGenerator` (listes de mots, phrases, dictées).
- **Événements de domaine** : `SessionCompleted`, `WordMistyped`, `TextImported`. Les statistiques et les mots difficiles s'abonnent à ces événements ; un futur module (badges, objectifs) s'abonne aussi, sans toucher au mode Copie.
- **Données brutes conservées** : chaque frappe est stockée, donc toute nouvelle statistique pourra être calculée rétroactivement.
- **Types partagés générés** depuis Rust vers TypeScript (`tauri-specta`) : pas de décalage entre les deux côtés.
- **Décisions tracées** : chaque choix d'architecture fait l'objet d'un court fichier ADR dans `docs/adr/`.

### 3.2 Flux d'une session

1. L'utilisateur choisit un texte et un mode ; l'interface demande au backend de créer une session.
2. Chaque frappe est comparée localement pour l'affichage immédiat, puis envoyée par lots (toutes les 2 s) pour enregistrement.
3. En fin de session, le cœur calcule les statistiques et classe les fautes, puis émet `SessionCompleted` et un `WordMistyped` par mot fautif.
4. Le module Révision met à jour les mots difficiles et leur date de révision.

### 3.3 Modèle de données (SQLite)

| Table | Champs principaux |
| --- | --- |
| `profiles` | id, nom, disposition clavier, réglages (JSON), créé le |
| `texts` | id, titre, contenu, langue, source (paste / pdf), fichier source, pages, étiquettes, créé le |
| `text_progress` | profile\_id, text\_id, position, dernière ouverture |
| `sessions` | id, profile\_id, text\_id, mode (copy / dictation / review), début, fin, MPM brut, MPM net, CPM, précision, réglages utilisés (JSON) |
| `keystrokes` | session\_id, index, attendu, tapé, horodatage (ms), correct |
| `session_errors` | session\_id, mot attendu, mot tapé, type de faute, position |
| `words` | id, profile\_id, mot (normalisé), rencontres, erreurs, temps moyen, maîtrise, prochaine révision, paramètres SRS |
| `word_attempts` | word\_id, session\_id, forme tapée, correct, durée (ms), date |
| `dictation_profiles` | id, nom, voix, vitesse, hauteur, volume, découpage, pause, ponctuation lue |
| `schema_migrations` | version, appliquée le |

Les champs `réglages (JSON)` permettent d'ajouter des options sans migration ; les données interrogées souvent restent en colonnes.

## 4. Spécifications fonctionnelles

Chaque exigence porte un identifiant (F-xx) que Claude Code réutilise dans les commits et les tests.

### 4.1 Interface de frappe (mode Copie)

- **F-01** Le texte à recopier s'affiche dans une zone centrale, police monospace lisible (taille réglable 14 à 32 px).
- **F-02** Coloration en temps réel : caractère correct en vert/neutre, erreur en rouge (le caractère attendu reste visible), caractère courant souligné par un curseur.
- **F-03** **Nombre de lignes visibles configurable** (1 à 10, défaut 3). Le texte défile automatiquement quand la ligne courante est terminée ; la ligne active reste toujours en position haute ou centrée (option).
- **F-04** Mode de correction au choix : *bloquant* (impossible d'avancer tant que l'erreur n'est pas corrigée), *libre* (on avance, l'erreur est comptée), *Retour arrière autorisé ou non*.
- **F-05** Bandeau en direct : MPM, précision, temps écoulé, progression en %. Option pour le masquer (concentration).
- **F-06** Types de session : texte entier, durée fixe (1, 2, 5, 10 min), ou nombre de mots fixe.
- **F-07** Raccourcis : Échap = pause, Ctrl+R = recommencer, Ctrl+Entrée = passer au texte suivant.
- **F-08** Clavier virtuel optionnel sous la zone de frappe, mettant en évidence la touche attendue et le doigt conseillé (disposition AZERTY par défaut, QWERTY/BÉPO sélectionnables).

### 4.2 Mode Dictée

- **F-10** L'application lit le texte à voix haute ; l'utilisateur tape sans voir le texte. Le texte n'est révélé qu'à la fin, avec la correction.
- **F-11** Découpage de la lecture configurable : par mot, par groupe de N mots (défaut 4), par phrase ou par paragraphe.
- **F-12** Réglages audio : voix (liste des voix françaises installées), vitesse (0,5× à 2×), hauteur, volume, pause entre segments (0 à 10 s).
- **F-13** Lecture de la ponctuation à voix haute en option (« virgule », « point », « à la ligne »), comme une vraie dictée scolaire.
- **F-14** Commandes : répéter le segment (Ctrl+Espace), segment précédent, pause/reprise, nombre de répétitions autorisées (illimité par défaut).
- **F-15** Avancement : automatique (temps fixe) ou adaptatif (le segment suivant est lu quand l'utilisateur a tapé le nombre de mots du segment courant).
- **F-16** Correction finale : alignement mot à mot entre texte tapé et texte attendu (algorithme de distance d'édition), affichage des fautes par type : accent, lettre manquante, lettre en trop, inversion, mot oublié, ponctuation, majuscule.
- **F-17** Profils de dictée enregistrables (ex. « Débutant lent », « Examen »).

### 4.3 Sources de texte (bibliothèque)

- **F-20** Ajout par copier-coller : titre, texte, catégorie/étiquettes, langue. Le texte est stocké en base.
- **F-21** Import PDF : ouverture du fichier, aperçu des pages, sélection d'une page ou d'une plage (ex. 3-5), puis choix du nombre de lignes (ou sélection libre de lignes de début et de fin) avant enregistrement.
- **F-22** Nettoyage du texte importé : recoller les mots coupés par une césure, supprimer numéros de page et en-têtes répétés, normaliser espaces et apostrophes. Aperçu modifiable avant validation.
- **F-23** Bibliothèque : liste avec recherche, filtres par étiquette, tri (date, longueur, meilleur score), modification, suppression, duplication.
- **F-24** Un texte long peut être découpé en exercices (par paragraphe ou par N lignes) ; la progression dans le texte est mémorisée pour reprendre où l'on s'est arrêté.
- **F-25** Pack de textes français intégré (domaine public) pour démarrer sans import.

### 4.4 Statistiques

- **F-30** Chaque session est enregistrée : date, durée, mode, texte, MPM brut et net, CPM, précision, nombre et liste des erreurs.
- **F-31** Enregistrement de chaque frappe (caractère attendu, caractère tapé, horodatage en ms) pour permettre des analyses futures sans changer le schéma.
- **F-32** Écran de fin de session : résumé chiffré, mots fautifs, comparaison avec la moyenne des 10 dernières sessions.
- **F-33** Tableau de bord : courbe MPM et précision dans le temps, temps total d'entraînement, record personnel, série de jours consécutifs.
- **F-34** Carte de chaleur du clavier : taux d'erreur et lenteur par touche ; liste des bigrammes les plus lents (ex. « qu », « ée »).
- **F-35** Export des statistiques en CSV.

### 4.5 Mots difficiles et entraînement ciblé

- **F-40** Un mot entre dans la liste « difficiles » s'il est mal écrit, ou s'il est tapé plus de 1,5 fois plus lentement que la vitesse moyenne de l'utilisateur (seuil réglable).
- **F-41** Pour chaque mot : nombre de rencontres, nombre d'erreurs, formes erronées tapées, type de faute dominant, date de dernière erreur, niveau de maîtrise.
- **F-42** Répétition espacée (algorithme SM-2 ou FSRS) : chaque mot a une prochaine date de révision ; un mot réussi 3 fois de suite sans faute et à vitesse normale passe « maîtrisé ».
- **F-43** L'accueil propose : « 12 mots à réviser aujourd'hui », en un clic.
- **F-44** Exercices générés : liste de mots mélangés, mots répétés N fois, phrases de la bibliothèque qui contiennent ces mots, et dictée de ces mots (mode Dictée).
- **F-45** L'utilisateur peut ajouter, retirer ou marquer un mot comme maîtrisé manuellement.

### 4.6 Réglages et profils

- **F-50** Plusieurs profils utilisateurs locaux (ex. parent et enfant), chacun avec ses statistiques.
- **F-51** Réglages : thème clair/sombre, police, taille, disposition clavier, sons de frappe (on/off), langue de l'interface (français par défaut, anglais/arabe prévus).
- **F-52** Sauvegarde et restauration de toutes les données dans un fichier unique (copie de la base SQLite).

## 5. Exigences non fonctionnelles

| Id | Exigence | Critère de validation |
| --- | --- | --- |
| NF-01 | Latence de frappe | Affichage du caractère en moins de 16 ms (60 i/s), même sur un texte de 10 000 mots |
| NF-02 | Démarrage | Fenêtre utilisable en moins de 1,5 s sur un PC moyen |
| NF-03 | Hors-ligne | Toutes les fonctions V1 marchent sans Internet |
| NF-04 | Taille | Installeur Windows de moins de 20 Mo (hors voix neuronales) |
| NF-05 | Données | Base SQLite locale dans le dossier utilisateur ; migrations automatiques sans perte entre versions |
| NF-06 | Robustesse | Session sauvegardée toutes les 10 s ; reprise après fermeture brutale |
| NF-07 | Accessibilité | Navigation 100 % clavier, contrastes WCAG AA, taille de police jusqu'à 32 px |
| NF-08 | Internationalisation | Tous les textes d'interface dans des fichiers de traduction (i18next) ; prêt pour l'arabe (droite à gauche) |
| NF-09 | Plateformes | Windows 10/11 en priorité, puis macOS 12+ et Ubuntu 22.04+ |
| NF-10 | Qualité | Couverture de tests de 80 % minimum sur le cœur métier (comparaison, statistiques, répétition espacée) |
| NF-11 | Sécurité | Permissions Tauri minimales : lecture de fichiers uniquement via la boîte de dialogue, aucun accès réseau en V1 |
| NF-12 | Vie privée | Aucune donnée envoyée à l'extérieur ; pas de télémétrie |

## 6. Structure du dépôt et feuille de route

### 6.1 Arborescence

```
dactylo/
├─ CLAUDE.md                  # règles pour Claude Code (section 7)
├─ docs/
│  ├─ cahier-des-charges.md     # ce document
│  └─ adr/                      # décisions d'architecture (1 fichier par décision)
├─ crates/
│  └─ core/                     # Rust pur, sans Tauri : testable seul
│     ├─ src/typing/            # comparaison frappe/texte, classification des fautes
│     ├─ src/stats/             # MPM, précision, bigrammes, chaleur clavier
│     ├─ src/srs/               # répétition espacée (trait Scheduler)
│     ├─ src/text/              # découpage, nettoyage, normalisation
│     └─ src/events.rs          # événements de domaine
├─ src-tauri/
│  ├─ src/commands/             # commandes exposées à l'interface, par module
│  ├─ src/db/                   # dépôts SQLite (sqlx)
│  ├─ src/speech/               # trait SpeechEngine + implémentation système
│  ├─ migrations/               # 0001_init.sql, 0002_...
│  └─ capabilities/             # permissions Tauri
├─ src/                         # interface React
│  ├─ app/                      # routes, shell, registre des modules
│  ├─ modules/
│  │  ├─ practice/              # mode Copie
│  │  ├─ dictation/             # mode Dictée
│  │  ├─ library/               # textes + import PDF
│  │  ├─ stats/                 # tableau de bord
│  │  ├─ review/                # mots difficiles
│  │  └─ settings/
│  ├─ shared/                   # composants UI, hooks, client API typé
│  └─ i18n/
└─ tests/e2e/                   # Playwright
```

### 6.2 Feuille de route

1. **Phase 0 — Socle (1 semaine)** : projet Tauri + React, CI, base SQLite et migrations, registre de modules, écran d'accueil vide. *Fin* : l'application s'installe et s'ouvre sur Windows.
2. **Phase 1 — Mode Copie (2 semaines)** : F-01 à F-07, collage de texte (F-20), enregistrement de session (F-30, F-31), écran de fin (F-32). *Fin* : une session complète est jouable et sauvegardée.
3. **Phase 2 — Bibliothèque et PDF (1,5 semaine)** : F-21 à F-25. *Fin* : import des pages 3 à 5 d'un PDF, nettoyage, exercice.
4. **Phase 3 — Statistiques et mots difficiles (2 semaines)** : F-33 à F-35, F-40 à F-45. *Fin* : l'accueil propose les mots à réviser.
5. **Phase 4 — Mode Dictée (2 semaines)** : F-10 à F-17. *Fin* : dictée d'un paragraphe avec correction classée par type de faute.
6. **Phase 5 — Finitions (1 semaine)** : clavier virtuel (F-08), profils (F-50), sauvegarde (F-52), installeurs signés, mise à jour automatique.

**Modules futurs prévus par l'architecture** : voix neuronales hors-ligne (Piper), OCR des PDF scannés (Tesseract), leçons progressives par rangée de touches, défis chronométrés et badges, autres langues, synchronisation cloud, version mobile (Tauri 2 le permet).

## 7. Instructions pour Claude Code

Copiez ce cahier des charges dans `docs/cahier-des-charges.md`, créez le fichier `CLAUDE.md` à la racine (contenu en 7.1), puis lancez le prompt de démarrage.

### 7.1 Contenu de CLAUDE.md

Voir le fichier `CLAUDE.md` à la racine du dépôt (source de vérité).

### 7.2 Prompt de démarrage

```
Lis docs/cahier-des-charges.md et CLAUDE.md.
Passe en mode plan et propose le plan détaillé de la Phase 0 :
fichiers à créer, dépendances et leurs versions, schéma SQL initial
(section 3.3), registre de modules, pipeline CI.
Pose-moi tes questions s'il reste des ambiguïtés.
N'écris pas de code avant que j'aie validé le plan.
```

Pour chaque phase suivante : « Implémente la Phase N. Commence par les tests du cœur métier, puis les commandes, puis l'interface. Termine en vérifiant les critères d'acceptation de la phase. »

### 7.3 Critères d'acceptation transverses

- [ ] Tous les tests passent en CI sur Windows, macOS et Linux.
- [ ] Une frappe avec touche morte (ê, ï) est comptée comme **un** caractère correct.
- [ ] Un texte collé de 10 000 mots s'affiche et se tape sans ralentissement (NF-01).
- [ ] Fermer l'application en pleine session puis la rouvrir propose la reprise (NF-06).
- [ ] Ajouter un module factice « Hello » ne modifie que son dossier et une ligne du registre (preuve d'extensibilité).
- [ ] Une dictée de 50 mots avec 5 fautes connues produit exactement ces 5 fautes, bien classées par type.
- [ ] Un mot mal écrit apparaît dans les révisions du lendemain et disparaît après 3 réussites.

### Sources

- [Tauri vs Electron 2026 — tech-insider](https://tech-insider.org/tauri-vs-electron-2026/)
- [Comparing Electron and Tauri — OpenReplay](https://blog.openreplay.com/comparing-electron-tauri-desktop-applications/)
- [Electron vs Tauri 2026 — PkgPulse](https://www.pkgpulse.com/guides/electron-vs-tauri-2026)
- [Extraction de texte avec pdfjs-dist — Nutrient](https://www.nutrient.io/blog/pdfjs-server-side-text-extraction/)
