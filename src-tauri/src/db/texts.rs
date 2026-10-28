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
    /// Titre, obligatoire.
    pub title: String,
    /// Contenu, obligatoire.
    pub content: String,
    /// Catégorie et étiquettes libres.
    pub tags: Vec<String>,
    /// Code de langue (« fr » si vide).
    pub language: String,
}

/// Entrée de la liste de la bibliothèque.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextSummary {
    /// Identifiant UUIDv7.
    pub id: String,
    /// Titre.
    pub title: String,
    /// Code de langue.
    pub language: String,
    /// Étiquettes triées, sans doublon.
    pub tags: Vec<String>,
    /// Nombre de caractères du contenu.
    pub length: u32,
    /// Date d'ajout, en ms Unix.
    pub created_at: i64,
}

/// Ligne SQL brute d'un résumé, avant conversion.
#[derive(sqlx::FromRow)]
struct SummaryRow {
    /// Identifiant.
    id: String,
    /// Titre.
    title: String,
    /// Code de langue.
    language: String,
    /// Étiquettes en JSON.
    tags: String,
    /// Longueur du contenu, calculée par SQLite.
    length: i64,
    /// Date d'ajout.
    created_at: i64,
}

impl From<SummaryRow> for TextSummary {
    /// Décode les étiquettes JSON ; un JSON illisible donne une liste vide.
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
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub title: String,
    /// Contenu normalisé.
    pub content: String,
}

/// Valide et enregistre un texte collé. Le contenu doit déjà être normalisé.
pub async fn create(pool: &SqlitePool, input: NewText) -> AppResult<TextSummary> {
    // Titre obligatoire et borné.
    let title = input.title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("le titre est vide".into()));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::Invalid(format!(
            "le titre dépasse {MAX_TITLE_CHARS} caractères"
        )));
    }
    // Un texte fait uniquement d'espaces est vide.
    if input.content.trim().is_empty() {
        return Err(AppError::Invalid("le texte est vide".into()));
    }
    // Étiquettes nettoyées : sans espaces autour, sans vide, triées, sans doublon.
    let mut tags: Vec<String> = input
        .tags
        .iter()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    // Le français est la langue par défaut.
    let language = match input.language.trim() {
        "" => "fr",
        l => l,
    };

    // Un texte collé a la source « paste » ; création et modification à la même date.
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

/// Lit le contenu d'un texte ; `NotFound` s'il n'existe pas.
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
    // Tous les identifiants, dans l'ordre d'affichage de la bibliothèque.
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM texts ORDER BY created_at DESC, id DESC")
            .fetch_all(pool)
            .await?;
    // Texte inconnu (supprimé entre-temps) : on repart du premier.
    let Some(index) = ids.iter().position(|i| i == id) else {
        return Ok(ids.into_iter().next());
    };
    // Le suivant, en revenant au début après le dernier ; rien s'il n'y a qu'un texte.
    let next = &ids[(index + 1) % ids.len()];
    Ok((next != id).then(|| next.clone()))
}

/// Tests du dépôt des textes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_in_memory;

    /// Texte minimal sans étiquette, en français.
    fn sample(title: &str, content: &str) -> NewText {
        NewText {
            title: title.into(),
            content: content.into(),
            tags: vec![],
            language: "fr".into(),
        }
    }

    /// Un texte créé est nettoyé, listé et relu à l'identique.
    #[tokio::test]
    async fn create_then_list_and_get() {
        let pool = connect_in_memory().await.unwrap();
        // Titre entouré d'espaces, étiquettes vides et en double.
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

        // La liste et la lecture donnent le même texte.
        let listed = list(&pool).await.unwrap();
        assert_eq!(listed, vec![created.clone()]);

        let text = get(&pool, &created.id).await.unwrap();
        assert_eq!(text.content, "Il était une fois.");
        assert_eq!(latest(&pool).await.unwrap(), Some(created.id));
    }

    /// Titre vide, contenu vide ou titre trop long sont refusés.
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
        // Rien n'a été enregistré.
        assert!(list(&pool).await.unwrap().is_empty());
        assert_eq!(latest(&pool).await.unwrap(), None);
    }

    /// Un texte inconnu donne `NotFound`.
    #[tokio::test]
    async fn unknown_text_is_not_found() {
        let pool = connect_in_memory().await.unwrap();
        assert!(matches!(
            get(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
    }

    /// Le texte suivant parcourt la bibliothèque en boucle.
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
