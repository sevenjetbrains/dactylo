//! Registre des commandes exposées à l'interface.
//!
//! Chaque module a son fichier `commands/<nom>.rs`, ses lignes dans
//! `collect_commands!` ci-dessous et, s'il écoute le bus, une ligne dans
//! `subscribe_modules` (voir ADR 0005).

/// Informations sur l'application et profil actif.
pub mod app;
/// Dictée : voix, réglages, profils, session et correction.
pub mod dictation;
/// Bibliothèque de textes.
pub mod library;
/// Mode Copie : réglages et sessions.
pub mod practice;
/// Profils et disposition clavier.
pub mod profiles;
/// Révision : mots difficiles, révision du jour, exercices générés.
pub mod review;
/// Statistiques : tableau de bord, clavier, sessions récentes, export.
pub mod stats;
/// Système : sauvegarde et restauration.
pub mod system;

use std::sync::Arc;

use dactylo_core::events::EventBus;
use specta_typescript::Typescript;
use sqlx::SqlitePool;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::events::{DomainEventPayload, SpeechFinished, UpdateAvailable};

/// Chemin du client typé généré, résolu depuis le dossier `src-tauri/`.
pub const BINDINGS_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../src/shared/api/bindings.ts");

/// Registre des commandes et événements exposés à l'interface.
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            app::app_info,
            app::current_profile,
            dictation::list_voices,
            dictation::get_dictation_settings,
            dictation::save_dictation_settings,
            dictation::list_dictation_profiles,
            dictation::save_dictation_profile,
            dictation::delete_dictation_profile,
            dictation::start_dictation,
            dictation::speak_dictation_segment,
            dictation::stop_speech,
            dictation::update_dictation_settings,
            dictation::save_dictation_draft,
            dictation::finish_dictation,
            dictation::abandon_dictation,
            dictation::pending_dictation,
            dictation::resume_dictation,
            library::create_text,
            library::preview_pdf,
            library::import_pdf,
            library::list_texts,
            library::list_tags,
            library::get_text,
            library::update_text,
            library::delete_text,
            library::duplicate_text,
            practice::get_practice_settings,
            practice::save_practice_settings,
            practice::start_session,
            practice::record_keystrokes,
            practice::finish_session,
            practice::abandon_session,
            practice::pending_session,
            practice::resume_session,
            practice::wrap_text,
            practice::next_text_id,
            profiles::list_profiles,
            profiles::create_profile,
            profiles::update_profile,
            profiles::delete_profile,
            profiles::switch_profile,
            profiles::keyboard_layout,
            profiles::keyboard_keys,
            review::get_review_settings,
            review::save_review_settings,
            review::review_overview,
            review::difficult_words,
            review::add_word,
            review::remove_word,
            review::mark_word_mastered,
            review::review_exercise,
            review::words_exercise,
            stats::stats_dashboard,
            stats::key_report,
            stats::recent_sessions,
            stats::export_sessions_csv,
            system::backup_database,
            system::restore_database,
            system::get_update_check,
            system::save_update_check,
        ])
        .events(collect_events![
            DomainEventPayload,
            SpeechFinished,
            UpdateAvailable
        ])
        // Les i64 sont des horodatages en ms et des compteurs : très en deçà de 2^53.
        .dangerously_cast_bigints_to_number()
}

/// Abonne au bus les modules qui écoutent les événements de domaine (ADR 0006).
pub fn subscribe_modules(bus: &Arc<EventBus>, pool: &SqlitePool) {
    bus.subscribe(Arc::new(review::ReviewSubscriber::new(pool.clone(), bus)));
}

/// Écrit le client TypeScript typé dans `src/shared/api/bindings.ts`.
pub fn export_bindings(builder: &Builder<tauri::Wry>) -> Result<(), specta_typescript::Error> {
    builder.export(
        Typescript::default()
            .header("// Fichier généré par tauri-specta : ne pas modifier.\n/* eslint-disable */"),
        BINDINGS_PATH,
    )
}

/// Tests du registre de commandes.
#[cfg(test)]
mod tests {
    /// Régénère le client typé ; la CI vérifie ensuite qu'il n'a pas changé.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::specta_builder()).unwrap();
    }
}
