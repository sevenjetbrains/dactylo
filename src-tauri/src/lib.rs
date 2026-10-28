//! Adaptateurs Tauri : commandes, base SQLite, relais des événements.

/// Commandes Tauri exposées à l'interface.
pub mod commands;
/// Accès à la base SQLite.
pub mod db;
/// Erreurs renvoyées à l'interface.
pub mod error;
/// Relais des événements de domaine vers l'interface.
pub mod events;
/// État partagé entre les commandes.
pub mod state;
/// Outils communs aux tests.
#[cfg(test)]
mod test_support;

use std::sync::Arc;

use dactylo_core::events::EventBus;
use tauri::Manager;

use crate::events::FrontendForwarder;
use crate::state::AppState;

/// Nom du fichier de base dans le dossier de données de l'application (NF-05).
const DB_FILE: &str = "dactylo.db";

/// Construit et lance l'application Tauri.
pub fn run() {
    // Registre des commandes et des événements typés.
    let builder = commands::specta_builder();

    // En développement, le client TypeScript est régénéré à chaque lancement.
    #[cfg(debug_assertions)]
    commands::export_bindings(&builder).expect("export des types TypeScript");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            // Branche les événements typés sur l'application.
            builder.mount_events(app);

            // Base dans le dossier de données de l'utilisateur, migrations appliquées à l'ouverture.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let pool = tauri::async_runtime::block_on(db::connect(&data_dir.join(DB_FILE)))?;
            // Un profil existe toujours : celui par défaut est créé au premier lancement.
            let profile = tauri::async_runtime::block_on(db::profiles::ensure_default(&pool))?;

            // Bus d'événements ; l'interface y est abonnée via le relais.
            let bus = Arc::new(EventBus::new());
            bus.subscribe(Arc::new(FrontendForwarder::new(app.handle().clone())));

            // État accessible à toutes les commandes.
            app.manage(AppState::new(pool, bus, profile.id));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de Dactylo");
}
