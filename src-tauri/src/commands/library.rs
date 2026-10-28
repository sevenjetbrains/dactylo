//! Commandes de la bibliothèque (F-20 ; F-21 à F-25 en Phase 2).

use dactylo_core::events::DomainEvent;
use dactylo_core::text::normalize;
use tauri::State;

use crate::db::texts::{self, NewText, TextSummary};
use crate::error::AppResult;
use crate::state::AppState;

/// Ajoute un texte collé ; son contenu est normalisé (NFC, fins de ligne).
pub async fn create(state: &AppState, input: NewText) -> AppResult<TextSummary> {
    // Seul le contenu est normalisé ; titre, étiquettes et langue sont repris tels quels.
    let input = NewText {
        content: normalize(&input.content),
        ..input
    };
    let text = texts::create(&state.pool, input).await?;
    // Prévient les autres modules qu'un texte est disponible.
    state.bus.publish(&DomainEvent::TextImported {
        text_id: text.id.clone(),
    });
    Ok(text)
}

/// Commande : ajoute un texte collé à la bibliothèque (F-20).
#[tauri::command]
#[specta::specta]
pub async fn create_text(state: State<'_, AppState>, input: NewText) -> AppResult<TextSummary> {
    create(&state, input).await
}

/// Commande : liste les textes de la bibliothèque.
#[tauri::command]
#[specta::specta]
pub async fn list_texts(state: State<'_, AppState>) -> AppResult<Vec<TextSummary>> {
    texts::list(&state.pool).await
}

/// Tests des commandes de bibliothèque.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    /// Un texte collé est normalisé et annoncé par `TextImported`.
    #[tokio::test]
    async fn pasted_text_is_normalized_and_announced() {
        let (state, recorder) = test_support::state().await;
        // Accent décomposé et fin de ligne Windows suivie d'espaces.
        let text = create(
            &state,
            NewText {
                title: "Forêt".into(),
                content: "fore\u{302}t  \r\n".into(),
                tags: vec![],
                language: "fr".into(),
            },
        )
        .await
        .unwrap();

        // Le contenu stocké est en NFC, sans espaces ni retour final.
        let stored = texts::get(&state.pool, &text.id).await.unwrap();
        assert_eq!(stored.content, "forêt");
        assert_eq!(
            recorder.events(),
            vec![DomainEvent::TextImported { text_id: text.id }]
        );
    }
}
