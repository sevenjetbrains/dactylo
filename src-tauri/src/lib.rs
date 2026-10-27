//! Adaptateurs Tauri : commandes, base SQLite, relais des événements.

pub mod commands;
pub mod db;
pub mod error;
pub mod events;
pub mod state;

use std::sync::Arc;

use dactylo_core::events::EventBus;
use tauri::Manager;

use crate::events::FrontendForwarder;
use crate::state::AppState;

/// Nom du fichier de base dans le dossier de données de l'application (NF-05).
const DB_FILE: &str = "dactylo.db";

pub fn run() {
    let builder = commands::specta_builder();

    #[cfg(debug_assertions)]
    commands::export_bindings(&builder).expect("export des types TypeScript");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let pool = tauri::async_runtime::block_on(db::connect(&data_dir.join(DB_FILE)))?;
            let profile = tauri::async_runtime::block_on(db::profiles::ensure_default(&pool))?;

            let bus = Arc::new(EventBus::new());
            bus.subscribe(Arc::new(FrontendForwarder::new(app.handle().clone())));

            app.manage(AppState {
                pool,
                bus,
                profile_id: profile.id,
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Dactylo");
}
