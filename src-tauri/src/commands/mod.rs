//! Registre des commandes exposées à l'interface.
//!
//! Chaque module a son fichier `commands/<nom>.rs` et **une ligne** dans
//! `collect_commands!` ci-dessous (voir ADR 0005).

pub mod app;

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::events::DomainEventPayload;

/// Chemin du client typé généré, résolu depuis le dossier `src-tauri/`.
pub const BINDINGS_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../src/shared/api/bindings.ts");

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![app::app_info, app::current_profile,])
        .events(collect_events![DomainEventPayload])
        // Les i64 sont des horodatages en ms et des compteurs : très en deçà de 2^53.
        .dangerously_cast_bigints_to_number()
}

pub fn export_bindings(builder: &Builder<tauri::Wry>) -> Result<(), specta_typescript::Error> {
    builder.export(
        Typescript::default()
            .header("// Fichier généré par tauri-specta : ne pas modifier.\n/* eslint-disable */"),
        BINDINGS_PATH,
    )
}

#[cfg(test)]
mod tests {
    /// Régénère le client typé ; la CI vérifie ensuite qu'il n'a pas changé.
    #[test]
    fn export_typescript_bindings() {
        super::export_bindings(&super::specta_builder()).unwrap();
    }
}
