//! Cœur métier de Dactylo.
//!
//! Ce crate ne dépend pas de Tauri : toutes les règles (comparaison de frappe,
//! statistiques, répétition espacée, traitement du texte) sont testables seules.

pub mod events;
pub mod srs;
pub mod stats;
pub mod text;
pub mod typing;
