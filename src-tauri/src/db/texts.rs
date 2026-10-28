//! Dépôt des textes de la bibliothèque et de la progression par profil.

use dactylo_core::exercise::{Segmentation, exercises, next_exercise};
use dactylo_core::source::SourceKind;
use dactylo_core::text::graphemes;
use serde::Serialize;
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::error::{AppError, AppResult};

/// Longueur maximale d'un titre, en caractères.
pub const MAX_TITLE_CHARS: usize = 200;

/// Texte à enregistrer : contenu déjà normalisé et métadonnées.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRecord {
    /// Titre, obligatoire.
    pub title: String,
    /// Auteur, facultatif.
    pub author: Option<String>,
    /// Contenu normalisé, obligatoire.
    pub content: String,
    /// Provenance.
    pub kind: SourceKind,
    /// Fichier d'origine (PDF).
    pub source_file: Option<String>,
    /// Pages retenues (PDF), ex. « 3-5 ».
    pub pages: Option<String>,
    /// Catégorie et étiquettes libres.
    pub tags: Vec<String>,
    /// Code de langue (« fr » si vide).
    pub language: String,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
}

impl TextRecord {
    /// Texte collé minimal, sans étiquette, en français (tests et valeurs par défaut).
    pub fn pasted(title: &str, content: &str) -> Self {
        Self {
            title: title.into(),
            author: None,
            content: content.into(),
            kind: SourceKind::Paste,
            source_file: None,
            pages: None,
            tags: Vec::new(),
            language: "fr".into(),
            segmentation: Segmentation::Whole,
        }
    }
}

/// Entrée de la liste de la bibliothèque, avec la progression du profil.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextSummary {
    /// Identifiant UUIDv7.
    pub id: String,
    /// Titre.
    pub title: String,
    /// Auteur, s'il est connu.
    pub author: Option<String>,
    /// Code de langue.
    pub language: String,
    /// Étiquettes triées, sans doublon.
    pub tags: Vec<String>,
    /// Provenance.
    pub source: SourceKind,
    /// Fichier d'origine (PDF).
    pub source_file: Option<String>,
    /// Pages retenues (PDF).
    pub pages: Option<String>,
    /// Nombre de caractères du contenu.
    pub length: u32,
    /// Date d'ajout, en ms Unix.
    pub created_at: i64,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
    /// Nombre d'exercices (au moins 1).
    pub exercise_count: u32,
    /// Exercices terminés par le profil (au plus `exercise_count`).
    pub completed: u32,
    /// Exercice à proposer ensuite, à partir de 0.
    pub next_exercise: u32,
    /// Meilleur MPM net du profil sur ce texte.
    pub best_wpm: Option<f64>,
}

/// Texte complet, pour une session ou pour l'éditer.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// Identifiant.
    pub id: String,
    /// Titre.
    pub title: String,
    /// Auteur.
    pub author: Option<String>,
    /// Code de langue.
    pub language: String,
    /// Étiquettes.
    pub tags: Vec<String>,
    /// Provenance.
    pub source: SourceKind,
    /// Pages retenues (PDF).
    pub pages: Option<String>,
    /// Contenu normalisé.
    pub content: String,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
}

/// Ligne SQL d'un texte, avec la progression et le meilleur score du profil.
#[derive(sqlx::FromRow)]
struct TextRow {
    /// Identifiant.
    id: String,
    /// Titre.
    title: String,
    /// Auteur.
    author: Option<String>,
    /// Code de langue.
    language: String,
    /// Étiquettes en JSON.
    tags: String,
    /// Provenance (`paste`, `pdf`, `builtin`).
    source: String,
    /// Fichier d'origine.
    source_file: Option<String>,
    /// Pages retenues.
    pages: Option<String>,
    /// Contenu.
    content: String,
    /// Découpage en JSON.
    segmentation: String,
    /// Date d'ajout.
    created_at: i64,
    /// Exercices terminés par le profil (0 si jamais ouvert).
    progress: i64,
    /// Meilleur MPM net du profil.
    best_wpm: Option<f64>,
}

impl TextRow {
    /// Résumé : longueur, nombre d'exercices et prochain exercice calculés ici.
    fn summary(self) -> TextSummary {
        let segmentation = Segmentation::from_json(&self.segmentation);
        let count = exercises(&graphemes(&self.content), segmentation).len();
        let progress = self.progress.max(0) as u32;
        TextSummary {
            id: self.id,
            title: self.title,
            author: self.author,
            language: self.language,
            tags: parse_tags(&self.tags),
            source: SourceKind::from_db(&self.source),
            source_file: self.source_file,
            pages: self.pages,
            length: self.content.chars().count() as u32,
            created_at: self.created_at,
            segmentation,
            exercise_count: count as u32,
            completed: progress.min(count as u32),
            next_exercise: next_exercise(progress, count),
            best_wpm: self.best_wpm,
        }
    }

    /// Texte complet.
    fn content(self) -> TextContent {
        TextContent {
            segmentation: Segmentation::from_json(&self.segmentation),
            tags: parse_tags(&self.tags),
            source: SourceKind::from_db(&self.source),
            id: self.id,
            title: self.title,
            author: self.author,
            language: self.language,
            pages: self.pages,
            content: self.content,
        }
    }
}

/// Étiquettes JSON ; un JSON illisible donne une liste vide.
fn parse_tags(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Requête commune : textes, progression et meilleur score du profil `?1`.
/// Macro plutôt que constante : sqlx n'accepte que du SQL littéral (`concat!`).
macro_rules! select_texts {
    () => {
        "SELECT t.id, t.title, t.author, t.language, t.tags, t.source, \
     t.source_file, t.pages, t.content, t.segmentation, t.created_at, \
     COALESCE(p.position, 0) AS progress, \
     (SELECT MAX(s.wpm_net) FROM sessions s \
      WHERE s.text_id = t.id AND s.profile_id = ?1 AND s.ended_at IS NOT NULL) AS best_wpm \
     FROM texts t LEFT JOIN text_progress p ON p.text_id = t.id AND p.profile_id = ?1"
    };
}

/// Titre, contenu et étiquettes validés et nettoyés.
fn validated(mut record: TextRecord) -> AppResult<TextRecord> {
    // Titre obligatoire et borné.
    record.title = record.title.trim().to_owned();
    if record.title.is_empty() {
        return Err(AppError::Invalid("le titre est vide".into()));
    }
    if record.title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::Invalid(format!(
            "le titre dépasse {MAX_TITLE_CHARS} caractères"
        )));
    }
    // Un texte fait uniquement d'espaces est vide.
    if record.content.trim().is_empty() {
        return Err(AppError::Invalid("le texte est vide".into()));
    }
    // Étiquettes nettoyées : sans espaces autour, sans vide, triées, sans doublon.
    let mut tags: Vec<String> = record
        .tags
        .iter()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect();
    tags.sort();
    tags.dedup();
    record.tags = tags;
    // Auteur vide → absent ; français par défaut ; découpage borné.
    record.author = record
        .author
        .map(|a| a.trim().to_owned())
        .filter(|a| !a.is_empty());
    if record.language.trim().is_empty() {
        record.language = "fr".into();
    }
    record.segmentation = record.segmentation.sanitized();
    Ok(record)
}

/// Valide et enregistre un texte ; renvoie son identifiant.
pub async fn create(pool: &SqlitePool, record: TextRecord) -> AppResult<String> {
    let record = validated(record)?;
    // Création et modification à la même date.
    let id = new_id();
    let now = now_ms();
    sqlx::query(
        "INSERT INTO texts (id, title, author, content, language, source, source_file, pages, \
         tags, segmentation, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&record.title)
    .bind(&record.author)
    .bind(&record.content)
    .bind(&record.language)
    .bind(record.kind.as_str())
    .bind(&record.source_file)
    .bind(&record.pages)
    .bind(serde_json::to_string(&record.tags).unwrap_or_else(|_| "[]".into()))
    .bind(record.segmentation.to_json())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Modification d'un texte par l'utilisateur (F-23).
#[derive(Debug, Clone, PartialEq)]
pub struct TextChanges {
    /// Titre.
    pub title: String,
    /// Auteur.
    pub author: Option<String>,
    /// Contenu normalisé.
    pub content: String,
    /// Étiquettes.
    pub tags: Vec<String>,
    /// Code de langue.
    pub language: String,
    /// Découpage en exercices.
    pub segmentation: Segmentation,
}

/// Modifie un texte. Si le contenu ou le découpage change, les exercices changent :
/// la progression de tous les profils repart de zéro.
pub async fn update(pool: &SqlitePool, id: &str, changes: TextChanges) -> AppResult<()> {
    // Mêmes règles qu'à la création ; provenance et fichier d'origine ne changent pas.
    let before = get(pool, id).await?;
    let record = validated(TextRecord {
        title: changes.title,
        author: changes.author,
        content: changes.content,
        kind: before.source,
        source_file: None,
        pages: before.pages,
        tags: changes.tags,
        language: changes.language,
        segmentation: changes.segmentation,
    })?;
    // Modification et remise à zéro éventuelle dans une seule transaction.
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE texts SET title = ?, author = ?, content = ?, tags = ?, language = ?, \
         segmentation = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&record.title)
    .bind(&record.author)
    .bind(&record.content)
    .bind(serde_json::to_string(&record.tags).unwrap_or_else(|_| "[]".into()))
    .bind(&record.language)
    .bind(record.segmentation.to_json())
    .bind(now_ms())
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if record.content != before.content || record.segmentation != before.segmentation {
        sqlx::query("UPDATE text_progress SET position = 0 WHERE text_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Supprime un texte ; sa progression part en cascade, ses sessions restent (sans texte).
pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let deleted = sqlx::query("DELETE FROM texts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("texte {id}")));
    }
    Ok(())
}

/// Textes du plus récent au plus ancien, avec la progression du profil.
pub async fn list(pool: &SqlitePool, profile_id: &str) -> AppResult<Vec<TextSummary>> {
    let rows: Vec<TextRow> = sqlx::query_as(concat!(
        select_texts!(),
        " ORDER BY t.created_at DESC, t.id DESC"
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(TextRow::summary).collect())
}

/// Résumé d'un texte pour le profil ; `NotFound` s'il n'existe pas.
pub async fn summary(pool: &SqlitePool, profile_id: &str, id: &str) -> AppResult<TextSummary> {
    let row: Option<TextRow> = sqlx::query_as(concat!(select_texts!(), " WHERE t.id = ?2"))
        .bind(profile_id)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(TextRow::summary)
        .ok_or_else(|| AppError::NotFound(format!("texte {id}")))
}

/// Lit le contenu d'un texte ; `NotFound` s'il n'existe pas.
pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<TextContent> {
    // Le profil n'intervient pas : progression et score sont ignorés ici.
    let row: Option<TextRow> = sqlx::query_as(concat!(select_texts!(), " WHERE t.id = ?2"))
        .bind("")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.map(TextRow::content)
        .ok_or_else(|| AppError::NotFound(format!("texte {id}")))
}

/// Toutes les étiquettes utilisées, triées.
pub async fn tags(pool: &SqlitePool) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT j.value FROM texts, json_each(texts.tags) AS j ORDER BY j.value",
    )
    .fetch_all(pool)
    .await?)
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

/// Exercices terminés par le profil sur ce texte (0 si jamais ouvert).
pub async fn progress(pool: &SqlitePool, profile_id: &str, text_id: &str) -> AppResult<u32> {
    let position: Option<i64> = sqlx::query_scalar(
        "SELECT position FROM text_progress WHERE profile_id = ? AND text_id = ?",
    )
    .bind(profile_id)
    .bind(text_id)
    .fetch_optional(pool)
    .await?;
    Ok(position.unwrap_or(0).max(0) as u32)
}

/// Note l'ouverture d'un texte et, si `completed` est donné, porte la progression
/// à au moins cette valeur (refaire un exercice ancien ne la fait pas reculer).
pub async fn touch_progress(
    pool: &SqlitePool,
    profile_id: &str,
    text_id: &str,
    completed: Option<u32>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO text_progress (profile_id, text_id, position, last_opened_at) \
         VALUES (?1, ?2, COALESCE(?3, 0), ?4) \
         ON CONFLICT (profile_id, text_id) DO UPDATE SET \
         position = MAX(position, COALESCE(?3, position)), last_opened_at = ?4",
    )
    .bind(profile_id)
    .bind(text_id)
    .bind(completed)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}

/// Tests du dépôt des textes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{connect_in_memory, profiles};

    /// Base en mémoire avec le profil par défaut ; renvoie aussi son identifiant.
    async fn setup() -> (SqlitePool, String) {
        let pool = connect_in_memory().await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap();
        (pool, profile.id)
    }

    /// Un texte créé est nettoyé, listé et relu à l'identique.
    #[tokio::test]
    async fn create_then_list_and_get() {
        let (pool, profile) = setup().await;
        // Titre entouré d'espaces, étiquettes vides et en double, auteur vide.
        let id = create(
            &pool,
            TextRecord {
                tags: vec![" conte ".into(), "".into(), "conte".into(), "a".into()],
                author: Some("  ".into()),
                language: " ".into(),
                ..TextRecord::pasted("  Le loup  ", "Il était une fois.")
            },
        )
        .await
        .unwrap();
        // La liste donne le résumé nettoyé, sans progression ni score.
        let listed = list(&pool, &profile).await.unwrap();
        assert_eq!(listed.len(), 1);
        let s = &listed[0];
        assert_eq!(s.title, "Le loup");
        assert_eq!(s.tags, ["a", "conte"]);
        assert_eq!(s.author, None);
        assert_eq!(s.language, "fr");
        assert_eq!(s.length, 18);
        assert_eq!((s.exercise_count, s.completed, s.next_exercise), (1, 0, 0));
        assert_eq!(s.best_wpm, None);
        assert_eq!(s.source, SourceKind::Paste);
        // La lecture rend le contenu ; c'est aussi le texte le plus récent.
        let text = get(&pool, &id).await.unwrap();
        assert_eq!(text.content, "Il était une fois.");
        assert_eq!(latest(&pool).await.unwrap(), Some(id.clone()));
        assert_eq!(summary(&pool, &profile, &id).await.unwrap(), listed[0]);
        assert_eq!(tags(&pool).await.unwrap(), ["a", "conte"]);
    }

    /// Titre vide, contenu vide ou titre trop long sont refusés.
    #[tokio::test]
    async fn empty_title_or_content_is_rejected() {
        let (pool, _) = setup().await;
        let long = "x".repeat(MAX_TITLE_CHARS + 1);
        for record in [
            TextRecord::pasted(" ", "texte"),
            TextRecord::pasted("titre", " \n "),
            TextRecord::pasted(&long, "texte"),
        ] {
            assert!(matches!(
                create(&pool, record).await,
                Err(AppError::Invalid(_))
            ));
        }
        // Rien n'a été enregistré.
        assert_eq!(latest(&pool).await.unwrap(), None);
    }

    /// Un texte inconnu donne `NotFound`, en lecture comme en suppression.
    #[tokio::test]
    async fn unknown_text_is_not_found() {
        let (pool, profile) = setup().await;
        assert!(matches!(
            get(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            summary(&pool, &profile, "absent").await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            delete(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
    }

    /// Le texte suivant parcourt la bibliothèque en boucle.
    #[tokio::test]
    async fn next_after_cycles_through_the_library() {
        let (pool, _) = setup().await;
        let a = create(&pool, TextRecord::pasted("A", "a")).await.unwrap();
        assert_eq!(next_after(&pool, &a).await.unwrap(), None, "texte unique");

        let b = create(&pool, TextRecord::pasted("B", "b")).await.unwrap();
        // Ordre de la bibliothèque : B (plus récent), puis A.
        assert_eq!(next_after(&pool, &b).await.unwrap(), Some(a.clone()));
        assert_eq!(next_after(&pool, &a).await.unwrap(), Some(b.clone()));
        assert_eq!(next_after(&pool, "absent").await.unwrap(), Some(b));
    }

    /// Exercices et progression : la progression ne recule pas et revient au début à la fin.
    #[tokio::test]
    async fn progress_moves_forward_and_wraps() {
        let (pool, profile) = setup().await;
        // Trois exercices d'une ligne.
        let id = create(
            &pool,
            TextRecord {
                segmentation: Segmentation::Lines { count: 1 },
                ..TextRecord::pasted("T", "un\ndeux\ntrois")
            },
        )
        .await
        .unwrap();
        // Ouverture sans fin d'exercice : aucune progression.
        touch_progress(&pool, &profile, &id, None).await.unwrap();
        assert_eq!(progress(&pool, &profile, &id).await.unwrap(), 0);
        // Deux exercices faits, puis l'exercice 1 refait : on reste à 2.
        touch_progress(&pool, &profile, &id, Some(2)).await.unwrap();
        touch_progress(&pool, &profile, &id, Some(1)).await.unwrap();
        let s = summary(&pool, &profile, &id).await.unwrap();
        assert_eq!((s.exercise_count, s.completed, s.next_exercise), (3, 2, 2));
        // Les trois faits : le prochain est de nouveau le premier.
        touch_progress(&pool, &profile, &id, Some(3)).await.unwrap();
        let s = summary(&pool, &profile, &id).await.unwrap();
        assert_eq!((s.completed, s.next_exercise), (3, 0));
    }

    /// Modifier le contenu remet la progression à zéro ; changer le titre non.
    #[tokio::test]
    async fn update_resets_progress_only_when_exercises_change() {
        let (pool, profile) = setup().await;
        let id = create(&pool, TextRecord::pasted("T", "un")).await.unwrap();
        touch_progress(&pool, &profile, &id, Some(1)).await.unwrap();
        let changes = |title: &str, content: &str| TextChanges {
            title: title.into(),
            author: Some("Verne".into()),
            content: content.into(),
            tags: vec!["roman".into()],
            language: "fr".into(),
            segmentation: Segmentation::Whole,
        };
        // Titre seul : progression gardée.
        update(&pool, &id, changes("Titre", "un")).await.unwrap();
        assert_eq!(progress(&pool, &profile, &id).await.unwrap(), 1);
        let text = get(&pool, &id).await.unwrap();
        assert_eq!(
            (text.title.as_str(), text.author.as_deref()),
            ("Titre", Some("Verne"))
        );
        // Contenu changé : progression à zéro.
        update(&pool, &id, changes("Titre", "deux")).await.unwrap();
        assert_eq!(progress(&pool, &profile, &id).await.unwrap(), 0);
        // Les règles de validation s'appliquent aussi à la modification.
        assert!(matches!(
            update(&pool, &id, changes(" ", "deux")).await,
            Err(AppError::Invalid(_))
        ));
    }

    /// Le meilleur score ne compte que les sessions terminées du profil.
    #[tokio::test]
    async fn best_score_uses_finished_sessions() {
        let (pool, profile) = setup().await;
        let id = create(&pool, TextRecord::pasted("T", "ab")).await.unwrap();
        // Une session terminée à 30 MPM, une autre en cours.
        sqlx::query(
            "INSERT INTO sessions (id, profile_id, text_id, mode, started_at, ended_at, wpm_net) \
             VALUES ('s1', ?1, ?2, 'copy', 0, 1, 30.0), ('s2', ?1, ?2, 'copy', 0, NULL, 99.0)",
        )
        .bind(&profile)
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            summary(&pool, &profile, &id).await.unwrap().best_wpm,
            Some(30.0)
        );
        // Supprimer le texte garde les sessions, sans texte.
        delete(&pool, &id).await.unwrap();
        let orphan: Option<String> =
            sqlx::query_scalar("SELECT text_id FROM sessions WHERE id = 's1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(orphan, None);
    }
}
