// Pas de console supplémentaire sous Windows en version publiée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Point d'entrée : délègue au crate `dactylo_lib`, partagé avec les tests.
fn main() {
    dactylo_lib::run()
}
