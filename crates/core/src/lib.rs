//! Cœur métier de Dactylo.
//!
//! Ce crate ne dépend pas de Tauri : toutes les règles (comparaison de frappe,
//! statistiques, répétition espacée, traitement du texte) sont testables seules.

/// Événements de domaine et bus de diffusion entre modules.
pub mod events;
/// Découpage des textes longs en exercices et progression.
pub mod exercise;
/// Recherche, filtre et tri de la bibliothèque.
pub mod library;
/// Réglages du mode Copie, avec valeurs par défaut et bornes.
pub mod settings;
/// Sources de texte (collage, PDF, pack intégré) derrière le trait `TextSource`.
pub mod source;
/// Répétition espacée des mots difficiles (Phase 3).
pub mod srs;
/// Calcul des statistiques de session (MPM, CPM, précision).
pub mod stats;
/// Découpage du texte en graphèmes, mots et lignes.
pub mod text;
/// Comparaison de la frappe avec le texte attendu.
pub mod typing;
