//! Commandes de la bibliothèque (F-20 ; F-21 à F-25 en Phase 2).

use dactylo_core::events::DomainEvent;
use dactylo_core::text::normalize;
use tauri::State;

use crate::db::texts::{self, NewText, TextSummary};
use crate::error::AppResult;
use crate::state::AppState;

/// Ajoute un texte collé ; son contenu est normalisé (NFC, fins de ligne).
pub async fn create(state: &AppState, input: NewText) -> AppResult<TextSummary> {
    let input = NewText {
        content: normalize(&input.content),
        ..input
    };
    let text = texts::create(&state.pool, input).await?;
    state.bus.publish(&DomainEvent::TextImported {
        text_id: text.id.clone(),
    });
    Ok(text)
}

#[tauri::command]
#[specta::specta]
pub async fn create_text(state: State<'_, AppState>, input: NewText) -> AppResult<TextSummary> {
    create(&state, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_texts(state: State<'_, AppState>) -> AppResult<Vec<TextSummary>> {
    texts::list(&state.pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    #[tokio::test]
    async fn pasted_text_is_normalized_and_announced() {
        let (state, recorder) = test_support::state().await;
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

        let stored = texts::get(&state.pool, &text.id).await.unwrap();
        assert_eq!(stored.content, "forêt");
        assert_eq!(
            recorder.events(),
            vec![DomainEvent::TextImported { text_id: text.id }]
        );
    }
}
