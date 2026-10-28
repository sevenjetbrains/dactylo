//! Commandes de la bibliothèque : collage (F-20), import PDF (F-21, F-22),
//! gestion des textes (F-23) et découpage en exercices (F-24).

use dactylo_core::events::DomainEvent;
use dactylo_core::exercise::Segmentation;
use dactylo_core::library::{self, TextFacts, TextQuery, duplicate_title};
use dactylo_core::source::{PageRange, PasteSource, PdfSource, SourceKind, SourceLine, TextSource};
use dactylo_core::text::normalize;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::texts::{self, TextChanges, TextContent, TextRecord, TextSummary};
use crate::error::AppResult;
use crate::state::AppState;

/// Texte collé par l'utilisateur (F-20).
#[derive(Debug, Clone, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewText {
    /// Titre, obligatoire.
    pub title: String,
    /// Contenu, obligatoire.
    pub content: String,
    /// Catégorie et étiquettes libres.
    pub tags: Vec<String>,
    /// Code de langue (« fr » si vide).
    pub language: String,
    /// Auteur, facultatif.
    #[serde(default)]
    pub author: Option<String>,
    /// Découpage en exercices ; texte entier par défaut.
    #[serde(default)]
    pub segmentation: Segmentation,
}

/// Aperçu d'un import PDF : lignes des pages et texte nettoyé (F-21, F-22).
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PdfPreview {
    /// Lignes des pages retenues, numérotées à partir de 1 dans l'ordre.
    pub lines: Vec<SourceLine>,
    /// Texte nettoyé et assemblé à partir des lignes choisies.
    pub content: String,
}

/// Texte importé d'un PDF, tel que validé (et éventuellement corrigé) par l'utilisateur.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PdfText {
    /// Titre, obligatoire.
    pub title: String,
    /// Auteur, facultatif.
    pub author: Option<String>,
    /// Étiquettes.
    pub tags: Vec<String>,
    /// Code de langue.
    pub language: String,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
    /// Nom du fichier d'origine.
    pub file_name: String,
    /// Pages retenues.
    pub range: PageRange,
    /// Texte de l'aperçu, modifiable avant validation (F-22).
    pub content: String,
}

/// Modification d'un texte (F-23).
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextUpdate {
    /// Titre.
    pub title: String,
    /// Auteur.
    pub author: Option<String>,
    /// Contenu.
    pub content: String,
    /// Étiquettes.
    pub tags: Vec<String>,
    /// Code de langue.
    pub language: String,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
}

/// Enregistre un texte, l'annonce aux autres modules et renvoie son résumé.
async fn save(state: &AppState, record: TextRecord) -> AppResult<TextSummary> {
    let id = texts::create(&state.pool, record).await?;
    // Prévient les autres modules qu'un texte est disponible.
    state.bus.publish(&DomainEvent::TextImported {
        text_id: id.clone(),
    });
    texts::summary(&state.pool, &state.profile_id, &id).await
}

/// Ajoute un texte collé ; son contenu est normalisé (NFC, fins de ligne).
pub async fn create(state: &AppState, input: NewText) -> AppResult<TextSummary> {
    // La source normalise le texte et refuse un texte vide.
    let draft = PasteSource {
        content: input.content,
    }
    .draft()?;
    save(
        state,
        TextRecord {
            title: input.title,
            author: input.author,
            content: draft.content,
            kind: draft.kind,
            source_file: None,
            pages: None,
            tags: input.tags,
            language: input.language,
            segmentation: input.segmentation,
        },
    )
    .await
}

/// Aperçu d'un import PDF : rien n'est enregistré.
pub fn preview_pdf_import(source: &PdfSource) -> AppResult<PdfPreview> {
    let (lines, draft) = source.preview()?;
    Ok(PdfPreview {
        lines,
        content: draft.content,
    })
}

/// Enregistre le texte d'un PDF, tel que corrigé dans l'aperçu.
pub async fn import_pdf_text(state: &AppState, input: PdfText) -> AppResult<TextSummary> {
    save(
        state,
        TextRecord {
            title: input.title,
            author: input.author,
            // L'aperçu a pu être modifié : on normalise de nouveau.
            content: normalize(&input.content),
            kind: SourceKind::Pdf,
            source_file: Some(input.file_name),
            pages: Some(input.range.label()),
            tags: input.tags,
            language: input.language,
            segmentation: input.segmentation,
        },
    )
    .await
}

/// Liste filtrée et triée (F-23).
pub async fn list(state: &AppState, query: &TextQuery) -> AppResult<Vec<TextSummary>> {
    let all = texts::list(&state.pool, &state.profile_id).await?;
    Ok(library::apply(all, query, |t| TextFacts {
        title: &t.title,
        tags: &t.tags,
        source_file: t.source_file.as_deref(),
        length: t.length,
        created_at: t.created_at,
        best_wpm: t.best_wpm,
    }))
}

/// Modifie un texte et renvoie son nouveau résumé.
pub async fn update(state: &AppState, id: &str, input: TextUpdate) -> AppResult<TextSummary> {
    texts::update(
        &state.pool,
        id,
        TextChanges {
            title: input.title,
            author: input.author,
            content: normalize(&input.content),
            tags: input.tags,
            language: input.language,
            segmentation: input.segmentation,
        },
    )
    .await?;
    texts::summary(&state.pool, &state.profile_id, id).await
}

/// Duplique un texte : même contenu et réglages, titre marqué « (copie) ».
pub async fn duplicate(state: &AppState, id: &str) -> AppResult<TextSummary> {
    let text = texts::get(&state.pool, id).await?;
    save(
        state,
        TextRecord {
            title: duplicate_title(&text.title),
            author: text.author,
            content: text.content,
            kind: text.source,
            source_file: None,
            pages: text.pages,
            tags: text.tags,
            language: text.language,
            segmentation: text.segmentation,
        },
    )
    .await
}

/// Commande : ajoute un texte collé à la bibliothèque (F-20).
#[tauri::command]
#[specta::specta]
pub async fn create_text(state: State<'_, AppState>, input: NewText) -> AppResult<TextSummary> {
    create(&state, input).await
}

/// Commande : aperçu d'un import PDF (lignes et texte nettoyé).
#[tauri::command]
#[specta::specta]
pub fn preview_pdf(source: PdfSource) -> AppResult<PdfPreview> {
    preview_pdf_import(&source)
}

/// Commande : enregistre le texte d'un PDF (F-21).
#[tauri::command]
#[specta::specta]
pub async fn import_pdf(state: State<'_, AppState>, input: PdfText) -> AppResult<TextSummary> {
    import_pdf_text(&state, input).await
}

/// Commande : liste les textes de la bibliothèque, filtrés et triés.
#[tauri::command]
#[specta::specta]
pub async fn list_texts(
    state: State<'_, AppState>,
    query: TextQuery,
) -> AppResult<Vec<TextSummary>> {
    list(&state, &query).await
}

/// Commande : étiquettes utilisées, pour le filtre.
#[tauri::command]
#[specta::specta]
pub async fn list_tags(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    texts::tags(&state.pool).await
}

/// Commande : texte complet, pour l'éditer.
#[tauri::command]
#[specta::specta]
pub async fn get_text(state: State<'_, AppState>, id: String) -> AppResult<TextContent> {
    texts::get(&state.pool, &id).await
}

/// Commande : modifie un texte.
#[tauri::command]
#[specta::specta]
pub async fn update_text(
    state: State<'_, AppState>,
    id: String,
    input: TextUpdate,
) -> AppResult<TextSummary> {
    update(&state, &id, input).await
}

/// Commande : supprime un texte.
#[tauri::command]
#[specta::specta]
pub async fn delete_text(state: State<'_, AppState>, id: String) -> AppResult<()> {
    texts::delete(&state.pool, &id).await
}

/// Commande : duplique un texte.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_text(state: State<'_, AppState>, id: String) -> AppResult<TextSummary> {
    duplicate(&state, &id).await
}

/// Tests des commandes de bibliothèque.
#[cfg(test)]
mod tests {
    use dactylo_core::library::TextSort;
    use dactylo_core::source::{CleanOptions, LineSelection, PdfPage};

    use super::*;
    use crate::error::AppError;
    use crate::test_support;

    /// Texte collé minimal.
    fn pasted(title: &str, content: &str) -> NewText {
        NewText {
            title: title.into(),
            content: content.into(),
            language: "fr".into(),
            ..NewText::default()
        }
    }

    /// Un texte collé est normalisé et annoncé par `TextImported`.
    #[tokio::test]
    async fn pasted_text_is_normalized_and_announced() {
        let (state, recorder) = test_support::state().await;
        // Accent décomposé et fin de ligne Windows suivie d'espaces.
        let text = create(&state, pasted("Forêt", "fore\u{302}t  \r\n"))
            .await
            .unwrap();
        // Le contenu stocké est en NFC, sans espaces ni retour final.
        let stored = texts::get(&state.pool, &text.id).await.unwrap();
        assert_eq!(stored.content, "forêt");
        assert_eq!(
            recorder.events(),
            vec![DomainEvent::TextImported { text_id: text.id }]
        );
        // Un texte vide est refusé avec un message clair.
        assert!(matches!(
            create(&state, pasted("Vide", " ")).await,
            Err(AppError::Invalid(_))
        ));
    }

    /// Pages d'un petit livre : en-tête courant, numéro de page, césure.
    /// Le corps varie d'une page à l'autre, sinon il passerait pour un pied de page.
    fn pdf_pages() -> Vec<PdfPage> {
        let words = ["un", "deux", "trois", "quatre", "cinq"];
        (1..=5)
            .map(|n| PdfPage {
                number: n,
                lines: vec![
                    "MON LIVRE".into(),
                    format!("Page {n} : un texte avec une césu-"),
                    format!("re qui se recolle, {}.", words[n as usize - 1]),
                    n.to_string(),
                ],
            })
            .collect()
    }

    /// Fin de parcours de la phase 2 : pages 3 à 5, nettoyage, enregistrement.
    #[tokio::test]
    async fn pdf_pages_three_to_five_are_imported() {
        let (state, recorder) = test_support::state().await;
        // Aperçu : six lignes (deux par page), texte nettoyé.
        let source = PdfSource {
            file_name: "livre.pdf".into(),
            pages: pdf_pages(),
            range: PageRange { first: 3, last: 5 },
            lines: None,
            options: CleanOptions::default(),
        };
        let preview = preview_pdf_import(&source).unwrap();
        assert_eq!(preview.lines.len(), 6);
        assert!(
            preview
                .content
                .starts_with("Page 3 : un texte avec une césure qui se recolle, trois.")
        );
        assert!(!preview.content.contains("MON LIVRE"));
        // L'utilisateur corrige l'aperçu puis enregistre, découpé par paragraphe.
        let text = import_pdf_text(
            &state,
            PdfText {
                title: "Mon livre".into(),
                author: None,
                tags: vec!["pdf".into()],
                language: "fr".into(),
                segmentation: Segmentation::Paragraphs { count: 1 },
                file_name: "livre.pdf".into(),
                range: source.range,
                content: format!("{}\nCorrigé à la main.", preview.content),
            },
        )
        .await
        .unwrap();
        assert_eq!(text.source, SourceKind::Pdf);
        assert_eq!(text.pages.as_deref(), Some("3-5"));
        assert_eq!(text.source_file.as_deref(), Some("livre.pdf"));
        assert_eq!(text.exercise_count, 2);
        assert_eq!(recorder.events().len(), 1);
    }

    /// Une sélection de lignes impossible ou un PDF scanné donnent une erreur lisible.
    #[test]
    fn pdf_preview_errors_are_readable() {
        let mut source = PdfSource {
            file_name: "livre.pdf".into(),
            pages: pdf_pages(),
            range: PageRange { first: 1, last: 1 },
            lines: Some(LineSelection { first: 1, last: 9 }),
            options: CleanOptions::default(),
        };
        assert!(matches!(
            preview_pdf_import(&source),
            Err(AppError::Invalid(_))
        ));
        // Pages sans texte : message sur les PDF scannés.
        source.pages = vec![PdfPage {
            number: 1,
            lines: vec![],
        }];
        source.lines = None;
        let Err(AppError::Invalid(message)) = preview_pdf_import(&source) else {
            panic!("erreur attendue");
        };
        assert!(message.contains("scanné"));
    }

    /// Liste filtrée, modification, duplication puis suppression.
    #[tokio::test]
    async fn texts_are_listed_updated_duplicated_and_deleted() {
        let (state, _) = test_support::state().await;
        let fable = create(
            &state,
            NewText {
                tags: vec!["fable".into()],
                ..pasted("Le Corbeau", "Maître corbeau")
            },
        )
        .await
        .unwrap();
        create(&state, pasted("Été", "Il fait chaud"))
            .await
            .unwrap();
        // Recherche sans accent et filtre par étiquette.
        let query = |search: &str, tag: Option<&str>| TextQuery {
            search: search.into(),
            tag: tag.map(str::to_owned),
            sort: TextSort::Newest,
        };
        let titles = |list: Vec<TextSummary>| list.into_iter().map(|t| t.title).collect::<Vec<_>>();
        assert_eq!(
            titles(list(&state, &query("ete", None)).await.unwrap()),
            ["Été"]
        );
        assert_eq!(
            titles(list(&state, &query("", Some("fable"))).await.unwrap()),
            ["Le Corbeau"]
        );
        // Modification : titre, auteur et découpage.
        let updated = update(
            &state,
            &fable.id,
            TextUpdate {
                title: "Le Corbeau et le Renard".into(),
                author: Some("La Fontaine".into()),
                content: "Maître corbeau\nsur un arbre perché".into(),
                tags: vec!["fable".into()],
                language: "fr".into(),
                segmentation: Segmentation::Lines { count: 1 },
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.exercise_count, 2);
        assert_eq!(updated.author.as_deref(), Some("La Fontaine"));
        // Duplication : même contenu, titre marqué.
        let copy = duplicate(&state, &fable.id).await.unwrap();
        assert_eq!(copy.title, "Le Corbeau et le Renard (copie)");
        assert_eq!(copy.exercise_count, 2);
        // Suppression : le texte disparaît de la liste.
        texts::delete(&state.pool, &fable.id).await.unwrap();
        assert_eq!(list(&state, &TextQuery::default()).await.unwrap().len(), 2);
    }
}
