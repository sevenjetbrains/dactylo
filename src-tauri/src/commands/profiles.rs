//! Commandes des profils (F-50, ADR 0023) et de la disposition clavier (F-08,
//! ADR 0022).

use dactylo_core::keyboard::{KeyboardLayout, Layout, Press, keys_for_all, layout};
use tauri::State;

use crate::db::profiles::{self, Profile};
use crate::error::AppResult;
use crate::state::AppState;

/// Profils existants.
pub async fn list(state: &AppState) -> AppResult<Vec<Profile>> {
    profiles::list(&state.pool).await
}

/// Crée un profil avec sa disposition.
pub async fn create(state: &AppState, name: &str, layout: Layout) -> AppResult<Profile> {
    profiles::create(&state.pool, name, layout.as_str()).await
}

/// Renomme un profil et change sa disposition.
pub async fn update(state: &AppState, id: &str, name: &str, layout: Layout) -> AppResult<Profile> {
    profiles::update(&state.pool, id, name, layout.as_str()).await
}

/// Supprime un profil. Si c'était le profil actif, le plus ancien prend sa place.
pub async fn delete(state: &AppState, id: &str) -> AppResult<Profile> {
    profiles::delete(&state.pool, id).await?;
    if state.profile_id() == id {
        let next = profiles::active(&state.pool).await?;
        switch(state, &next.id).await?;
    }
    profiles::get(&state.pool, &state.profile_id()).await
}

/// Rend un profil actif et le retient pour le prochain démarrage. Les sessions en
/// cours de l'ancien profil restent reprenables quand il redevient actif.
pub async fn switch(state: &AppState, id: &str) -> AppResult<Profile> {
    let profile = profiles::get(&state.pool, id).await?;
    profiles::remember_active(&state.pool, id).await?;
    state.set_profile(profile.id.clone());
    Ok(profile)
}

/// Disposition du profil actif.
pub async fn active_layout(state: &AppState) -> AppResult<KeyboardLayout> {
    let profile = profiles::get(&state.pool, &state.profile_id()).await?;
    Ok(layout(Layout::parse(&profile.keyboard_layout)))
}

/// Commande : profils existants (F-50).
#[tauri::command]
#[specta::specta]
pub async fn list_profiles(state: State<'_, AppState>) -> AppResult<Vec<Profile>> {
    list(&state).await
}

/// Commande : crée un profil (F-50).
#[tauri::command]
#[specta::specta]
pub async fn create_profile(
    state: State<'_, AppState>,
    name: String,
    layout: Layout,
) -> AppResult<Profile> {
    create(&state, &name, layout).await
}

/// Commande : renomme un profil et change sa disposition.
#[tauri::command]
#[specta::specta]
pub async fn update_profile(
    state: State<'_, AppState>,
    id: String,
    name: String,
    layout: Layout,
) -> AppResult<Profile> {
    update(&state, &id, &name, layout).await
}

/// Commande : supprime un profil ; renvoie le profil actif ensuite.
#[tauri::command]
#[specta::specta]
pub async fn delete_profile(state: State<'_, AppState>, id: String) -> AppResult<Profile> {
    delete(&state, &id).await
}

/// Commande : change de profil actif.
#[tauri::command]
#[specta::specta]
pub async fn switch_profile(state: State<'_, AppState>, id: String) -> AppResult<Profile> {
    switch(&state, &id).await
}

/// Commande : disposition du profil actif (clavier virtuel, F-08).
#[tauri::command]
#[specta::specta]
pub async fn keyboard_layout(state: State<'_, AppState>) -> AppResult<KeyboardLayout> {
    active_layout(&state).await
}

/// Commande : touches à frapper pour chaque graphème, dans la disposition du
/// profil actif (F-08).
#[tauri::command]
#[specta::specta]
pub async fn keyboard_keys(
    state: State<'_, AppState>,
    graphemes: Vec<String>,
) -> AppResult<Vec<Vec<Press>>> {
    Ok(keys_for_all(&graphemes, &active_layout(&state).await?))
}

/// Tests des commandes de profils.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{library, practice, stats};
    use crate::test_support;
    use dactylo_core::stats::dashboard::Period;
    use dactylo_core::typing::{KeyKind, Keystroke};

    /// Les données restent séparées d'un profil à l'autre ; la disposition suit le profil.
    #[tokio::test]
    async fn each_profile_has_its_own_data_and_layout() {
        let (state, _) = test_support::state().await;
        let parent = state.profile_id();
        // Une session terminée pour le premier profil.
        let text = library::create(
            &state,
            library::NewText {
                title: "T".into(),
                content: "ab".into(),
                language: "fr".into(),
                ..library::NewText::default()
            },
        )
        .await
        .unwrap()
        .id;
        let s = practice::start(&state, Some(text), None, None)
            .await
            .unwrap();
        let keys: Vec<Keystroke> = ["a", "b"]
            .iter()
            .enumerate()
            .map(|(i, k)| Keystroke {
                seq: i as u32,
                kind: KeyKind::Insert,
                typed: Some((*k).into()),
                t_ms: 100 * i as u32,
            })
            .collect();
        practice::record(&state, &s.session_id, &keys)
            .await
            .unwrap();
        practice::finish(&state, &s.session_id, 1_000)
            .await
            .unwrap();
        assert_eq!(stats::recent(&state, 5).await.unwrap().len(), 1);

        // Nouveau profil en BÉPO : statistiques vides, disposition propre.
        let child = create(&state, "Léa", Layout::Bepo).await.unwrap();
        switch(&state, &child.id).await.unwrap();
        assert_eq!(state.profile_id(), child.id);
        assert!(stats::recent(&state, 5).await.unwrap().is_empty());
        let d = stats::compute_dashboard(&state, Period::All, 0)
            .await
            .unwrap();
        assert!(d.points.is_empty());
        assert_eq!(active_layout(&state).await.unwrap().id, Layout::Bepo);

        // Retour au premier profil : ses données sont là ; le choix est retenu.
        switch(&state, &parent).await.unwrap();
        assert_eq!(stats::recent(&state, 5).await.unwrap().len(), 1);
        assert_eq!(profiles::active(&state.pool).await.unwrap().id, parent);
        assert_eq!(list(&state).await.unwrap().len(), 2);
    }

    /// Supprimer le profil actif rend l'autre actif ; le dernier ne part pas.
    #[tokio::test]
    async fn deleting_active_profile_switches_to_another() {
        let (state, _) = test_support::state().await;
        let first = state.profile_id();
        let second = create(&state, "Invité", Layout::Qwerty).await.unwrap();
        switch(&state, &second.id).await.unwrap();
        let now = delete(&state, &second.id).await.unwrap();
        assert_eq!(now.id, first);
        assert_eq!(state.profile_id(), first);
        assert!(delete(&state, &first).await.is_err());
        let renamed = update(&state, &first, "Maison", Layout::Azerty)
            .await
            .unwrap();
        assert_eq!(renamed.name, "Maison");
    }
}
