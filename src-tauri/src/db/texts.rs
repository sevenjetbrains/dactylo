//! Dépôt des textes de la bibliothèque.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::error::{AppError, AppResult};

/// Longueur maximale d'un titre, en caractères.
pub const MAX_TITLE_CHARS: usize = 200;

/// Texte saisi par l'utilisateur (F-20).
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NewText {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub language: String,
}

/// Entrée de la liste de la bibliothèque.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextSummary {
    pub id: String,
    pub title: String,
    pub language: String,
    pub tags: Vec<String>,
    /// Nombre de caractères du contenu.
    pub length: u32,
    pub created_at: i64,
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: String,
    title: String,
    language: String,
    tags: String,
    length: i64,
    created_at: i64,
}

impl From<SummaryRow> for TextSummary {
    fn from(row: SummaryRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            language: row.language,
            tags: serde_json::from_str(&row.tags).unwrap_or_default(),
            length: row.length as u32,
            created_at: row.created_at,
        }
    }
}

/// Contenu d'un texte, pour une session.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TextContent {
    pub id: String,
    pub title: String,
    pub content: String,
}

/// Valide et enregistre un texte collé. Le contenu doit déjà être normalisé.
pub async fn create(pool: &SqlitePool, input: NewText) -> AppResult<TextSummary> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("le titre est vide".into()));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::Invalid(format!(
            "le titre dépasse {MAX_TITLE_CHARS} caractères"
        )));
    }
    if input.content.trim().is_empty() {
        return Err(AppError::Invalid("le texte est vide".into()));
    }
    let mut tags: Vec<String> = input
        .tags
        .iter()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    let language = match input.language.trim() {
        "" => "fr",
        l => l,
    };

    let id = new_id();
    let now = now_ms();
    sqlx::query(
        "INSERT INTO texts (id, title, content, language, source, tags, created_at, updated_at) \
         VALUES (?, ?, ?, ?, 'paste', ?, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(&input.content)
    .bind(language)
    .bind(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into()))
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(TextSummary {
        id,
        title: title.to_owned(),
        language: language.to_owned(),
        tags,
        length: input.content.chars().count() as u32,
        created_at: now,
    })
}

/// Textes du plus récent au plus ancien.
pub async fn list(pool: &SqlitePool) -> AppResult<Vec<TextSummary>> {
    let rows: Vec<SummaryRow> = sqlx::query_as(
        "SELECT id, title, language, tags, length(content) AS length, created_at \
         FROM texts ORDER BY created_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<TextContent> {
    sqlx::query_as("SELECT id, title, content FROM texts WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("texte {id}")))
}

/// Texte le plus récent, s'il y en a un.
pub async fn latest(pool: &SqlitePool) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM texts ORDER BY created_at DESC, id DESC LIMIT 1")
            .fetch_optional(pool)
            .await?,
    )
}

/// Texte suivant dans l'ordre de la bibliothèque (Ctrl+Entrée, F-07) ;
/// revient au premier après le dernier. `None` s'il n'y a pas d'autre texte.
pub async fn next_after(pool: &SqlitePool, id: &str) -> AppResult<Option<String>> {
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM texts ORDER BY created_at DESC, id DESC")
            .fetch_all(pool)
            .await?;
    let Some(index) = ids.iter().position(|i| i == id) else {
        return Ok(ids.into_iter().next());
    };
    let next = &ids[(index + 1) % ids.len()];
    Ok((next != id).then(|| next.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_in_memory;

    fn sample(title: &str, content: &str) -> NewText {
        NewText {
            title: title.into(),
            content: content.into(),
            tags: vec![],
            language: "fr".into(),
        }
    }

    #[tokio::test]
    async fn create_then_list_and_get() {
        let pool = connect_in_memory().await.unwrap();
        let created = create(
            &pool,
            NewText {
                tags: vec![" conte ".into(), "".into(), "conte".into(), "a".into()],
                ..sample("  Le loup  ", "Il était une fois.")
            },
        )
        .await
        .unwrap();
        assert_eq!(created.title, "Le loup");
        assert_eq!(created.tags, ["a", "conte"]);
        assert_eq!(created.length, 18);

        let listed = list(&pool).await.unwrap();
        assert_eq!(listed, vec![created.clone()]);

        let text = get(&pool, &created.id).await.unwrap();
        assert_eq!(text.content, "Il était une fois.");
        assert_eq!(latest(&pool).await.unwrap(), Some(created.id));
    }

    #[tokio::test]
    async fn empty_title_or_content_is_rejected() {
        let pool = connect_in_memory().await.unwrap();
        for input in [sample(" ", "texte"), sample("titre", " \n ")] {
            assert!(matches!(
                create(&pool, input).await,
                Err(AppError::Invalid(_))
            ));
        }
        let long = "x".repeat(MAX_TITLE_CHARS + 1);
        assert!(matches!(
            create(&pool, sample(&long, "texte")).await,
            Err(AppError::Invalid(_))
        ));
        assert!(list(&pool).await.unwrap().is_empty());
        assert_eq!(latest(&pool).await.unwrap(), None);
    }

    #[tokio::test]
    async fn unknown_text_is_not_found() {
        let pool = connect_in_memory().await.unwrap();
        assert!(matches!(
            get(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn next_after_cycles_through_the_library() {
        let pool = connect_in_memory().await.unwrap();
        let a = create(&pool, sample("A", "a")).await.unwrap().id;
        assert_eq!(next_after(&pool, &a).await.unwrap(), None, "texte unique");

        let b = create(&pool, sample("B", "b")).await.unwrap().id;
        // Ordre de la bibliothèque : B (plus récent), puis A.
        assert_eq!(next_after(&pool, &b).await.unwrap(), Some(a.clone()));
        assert_eq!(next_after(&pool, &a).await.unwrap(), Some(b.clone()));
        assert_eq!(next_after(&pool, "absent").await.unwrap(), Some(b));
    }
}
