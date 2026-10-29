//! Cœur métier de Dactylo.
//!
//! Ce crate ne dépend pas de Tauri : toutes les règles (comparaison de frappe,
//! statistiques, répétition espacée, traitement du texte) sont testables seules.

/// Jours locaux à partir d'horodatages en ms (séries, révisions du jour).
pub mod days;
/// Mode Dictée : correction, découpage, ponctuation lue, réglages.
pub mod dictation;
/// Événements de domaine et bus de diffusion entre modules.
pub mod events;
/// Découpage des textes longs en exercices et progression.
pub mod exercise;
/// Dispositions de clavier et touches par caractère (clavier virtuel).
pub mod keyboard;
/// Recherche, filtre et tri de la bibliothèque.
pub mod library;
/// Réglages du mode Copie, avec valeurs par défaut et bornes.
pub mod settings;
/// Sources de texte (collage, PDF, pack intégré) derrière le trait `TextSource`.
pub mod source;
/// Synthèse vocale derrière le trait `SpeechEngine`.
pub mod speech;
/// Répétition espacée et suivi des mots difficiles.
pub mod srs;
/// Calcul des statistiques de session (MPM, CPM, précision).
pub mod stats;
/// Découpage du texte en graphèmes, mots et lignes.
pub mod text;
/// Comparaison de la frappe avec le texte attendu.
pub mod typing;
