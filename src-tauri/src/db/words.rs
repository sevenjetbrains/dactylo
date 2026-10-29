//! Dépôt des mots suivis (`words`) et de leurs tentatives (`word_attempts`),
//! F-40 à F-45.

use dactylo_core::srs::SrsState;
use dactylo_core::srs::words::{Mastery, WordRecord};
use dactylo_core::typing::{ErrorType, WordAttempt};
use sqlx::{SqliteConnection, SqlitePool};

use super::new_id;
use crate::error::{AppError, AppResult};

/// Formes fautives différentes gardées par mot pour l'affichage (F-41).
const TYPED_FORMS: u32 = 3;

/// Ligne de la table `words`.
#[derive(sqlx::FromRow)]
struct WordRow {
    /// Identifiant UUIDv7.
    id: String,
    /// Clé du mot.
    word: String,
    /// Rencontres.
    encounters: u32,
    /// Erreurs.
    errors: u32,
    /// Durée moyenne, en ms.
    avg_ms: Option<f64>,
    /// Maîtrise (`learning`, `mastered`, `manual_mastered`).
    mastery: String,
    /// Série de réussites.
    streak: u32,
    /// Dernière faute, en ms Unix.
    last_error_at: Option<i64>,
    /// Prochaine révision, en ms Unix.
    next_review_at: Option<i64>,
    /// État du planificateur (JSON).
    srs_state: String,
}

impl WordRow {
    /// Convertit la ligne en `(identifiant, état du mot)`.
    fn into_record(self) -> (String, WordRecord) {
        // Un état illisible repart des valeurs par défaut plutôt que d'échouer.
        let srs: SrsState = serde_json::from_str(&self.srs_state).unwrap_or_default();
        let record = WordRecord {
            word: self.word,
            encounters: self.encounters,
            errors: self.errors,
            avg_ms: self.avg_ms,
            mastery: Mastery::parse(&self.mastery),
            streak: self.streak,
            last_error_at: self.last_error_at,
            next_review_at: self.next_review_at,
            srs,
        };
        (self.id, record)
    }
}

/// Colonnes lues par `WordRow`.
macro_rules! select_words {
    () => {
        "SELECT id, word, encounters, errors, avg_ms, mastery, streak, last_error_at, \
         next_review_at, srs_state FROM words"
    };
}

/// Mot d'un profil, avec son identifiant ; `None` s'il n'a jamais été vu.
pub async fn find(
    conn: &mut SqliteConnection,
    profile_id: &str,
    word: &str,
) -> AppResult<Option<(String, WordRecord)>> {
    let row: Option<WordRow> = sqlx::query_as(concat!(
        select_words!(),
        " WHERE profile_id = ? AND word = ?"
    ))
    .bind(profile_id)
    .bind(word)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(WordRow::into_record))
}

/// Crée (sans `id`) ou met à jour le mot ; renvoie son identifiant.
pub async fn save(
    conn: &mut SqliteConnection,
    profile_id: &str,
    id: Option<&str>,
    record: &WordRecord,
) -> AppResult<String> {
    let srs = serde_json::to_string(&record.srs).map_err(|e| AppError::Invalid(e.to_string()))?;
    // Identifiant existant, ou nouveau pour un mot jamais vu.
    let id = id.map_or_else(new_id, str::to_owned);
    sqlx::query(
        "INSERT INTO words (id, profile_id, word, encounters, errors, avg_ms, mastery, streak, \
         last_error_at, next_review_at, srs_state) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (id) DO UPDATE SET encounters = excluded.encounters, \
         errors = excluded.errors, avg_ms = excluded.avg_ms, mastery = excluded.mastery, \
         streak = excluded.streak, last_error_at = excluded.last_error_at, \
         next_review_at = excluded.next_review_at, srs_state = excluded.srs_state",
    )
    .bind(&id)
    .bind(profile_id)
    .bind(&record.word)
    .bind(record.encounters)
    .bind(record.errors)
    .bind(record.avg_ms)
    .bind(record.mastery.as_str())
    .bind(record.streak)
    .bind(record.last_error_at)
    .bind(record.next_review_at)
    .bind(srs)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

/// Enregistre une tentative d'un mot pendant une session.
pub async fn insert_attempt(
    conn: &mut SqliteConnection,
    word_id: &str,
    session_id: &str,
    attempt: &WordAttempt,
    at: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO word_attempts (word_id, session_id, typed_form, correct, duration_ms, \
         attempted_at, error_type) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(word_id)
    .bind(session_id)
    .bind(&attempt.typed)
    .bind(attempt.correct)
    .bind(attempt.duration_ms)
    .bind(at)
    .bind(attempt.error_type.map(ErrorType::as_str))
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Mot suivi, avec ce qu'en montre le tableau « Mots difficiles » (F-41).
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedWord {
    /// État du mot.
    pub record: WordRecord,
    /// Dernières formes fautives distinctes, la plus récente d'abord.
    pub typed_forms: Vec<String>,
    /// Types des fautes enregistrées, pour la faute dominante.
    pub error_types: Vec<ErrorType>,
}

/// Mots en révision du profil, du plus urgent au moins urgent.
pub async fn tracked(pool: &SqlitePool, profile_id: &str) -> AppResult<Vec<TrackedWord>> {
    let rows: Vec<WordRow> = sqlx::query_as(concat!(
        select_words!(),
        " WHERE profile_id = ? AND mastery = 'learning' AND next_review_at IS NOT NULL \
         ORDER BY next_review_at, word"
    ))
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    // Formes et types de fautes, mot par mot.
    let mut words = Vec::with_capacity(rows.len());
    for row in rows {
        let (id, record) = row.into_record();
        words.push(TrackedWord {
            typed_forms: typed_forms(pool, &id).await?,
            error_types: error_types(pool, &id).await?,
            record,
        });
    }
    Ok(words)
}

/// Formes fautives distinctes d'un mot, les plus récentes d'abord.
async fn typed_forms(pool: &SqlitePool, word_id: &str) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT typed_form FROM word_attempts WHERE word_id = ? AND correct = 0 \
         GROUP BY typed_form ORDER BY MAX(attempted_at) DESC, MAX(id) DESC LIMIT ?",
    )
    .bind(word_id)
    .bind(TYPED_FORMS)
    .fetch_all(pool)
    .await?)
}

/// Types des fautes d'un mot, dans l'ordre des tentatives.
async fn error_types(pool: &SqlitePool, word_id: &str) -> AppResult<Vec<ErrorType>> {
    let raw: Vec<String> = sqlx::query_scalar(
        "SELECT error_type FROM word_attempts WHERE word_id = ? AND error_type IS NOT NULL \
         ORDER BY attempted_at, id",
    )
    .bind(word_id)
    .fetch_all(pool)
    .await?;
    // La colonne est contrainte : seules des valeurs connues peuvent s'y trouver.
    Ok(raw
        .iter()
        .filter_map(|t| serde_json::from_value(serde_json::Value::String(t.clone())).ok())
        .collect())
}

/// Mots à réviser avant `before` (borne exclue), du plus ancien au plus récent.
pub async fn due(pool: &SqlitePool, profile_id: &str, before: i64) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT word FROM words WHERE profile_id = ? AND mastery = 'learning' \
         AND next_review_at < ? ORDER BY next_review_at, word",
    )
    .bind(profile_id)
    .bind(before)
    .fetch_all(pool)
    .await?)
}

/// Nombre de mots suivis (en révision) et de mots maîtrisés du profil.
pub async fn counts(pool: &SqlitePool, profile_id: &str) -> AppResult<(u32, u32)> {
    let (tracked, mastered): (u32, u32) = sqlx::query_as(
        "SELECT \
         COALESCE(SUM(mastery = 'learning' AND next_review_at IS NOT NULL), 0), \
         COALESCE(SUM(mastery IN ('mastered', 'manual_mastered')), 0) \
         FROM words WHERE profile_id = ?",
    )
    .bind(profile_id)
    .fetch_one(pool)
    .await?;
    Ok((tracked, mastered))
}

/// Supprime un mot et ses tentatives ; `NotFound` s'il n'existe pas.
pub async fn delete(pool: &SqlitePool, profile_id: &str, word: &str) -> AppResult<()> {
    let deleted = sqlx::query("DELETE FROM words WHERE profile_id = ? AND word = ?")
        .bind(profile_id)
        .bind(word)
        .execute(pool)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("mot « {word} »")));
    }
    Ok(())
}

/// Tests du dépôt des mots.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::texts::{self, TextRecord};
    use crate::db::{connect_in_memory, now_ms, profiles, sessions};

    /// Base en mémoire avec le profil par défaut.
    async fn setup() -> (SqlitePool, String) {
        let pool = connect_in_memory().await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap();
        (pool, profile.id)
    }

    /// Tentative fausse de « fête ».
    fn wrong(typed: &str, error_type: ErrorType) -> WordAttempt {
        WordAttempt {
            word: "fête".into(),
            expected: "fête".into(),
            typed: typed.into(),
            correct: false,
            error_type: Some(error_type),
            position: 0,
            duration_ms: 400,
            timed_chars: 4,
        }
    }

    /// Un mot créé puis relu redonne le même état, puis se met à jour sur place.
    #[tokio::test]
    async fn saves_and_finds_words() {
        let (pool, profile) = setup().await;
        let mut conn = pool.acquire().await.unwrap();
        // Absent avant la première sauvegarde.
        assert!(find(&mut conn, &profile, "fête").await.unwrap().is_none());
        let mut record = WordRecord::added("fête", 1_000);
        record.errors = 2;
        let id = save(&mut conn, &profile, None, &record).await.unwrap();
        let (found_id, found) = find(&mut conn, &profile, "fête").await.unwrap().unwrap();
        assert_eq!((found_id.as_str(), &found), (id.as_str(), &record));
        // Mise à jour avec le même identifiant : une seule ligne.
        record.mark_mastered();
        save(&mut conn, &profile, Some(&id), &record).await.unwrap();
        let (_, found) = find(&mut conn, &profile, "fête").await.unwrap().unwrap();
        assert_eq!(found.mastery, Mastery::ManualMastered);
        // Base de test à une connexion : on la rend avant d'interroger le pool.
        drop(conn);
        assert_eq!(counts(&pool, &profile).await.unwrap(), (0, 1));
    }

    /// Mots suivis : formes fautives récentes, types de fautes, ordre d'urgence.
    #[tokio::test]
    async fn tracked_words_with_forms_and_error_types() {
        let (pool, profile) = setup().await;
        // Une vraie session, pour la clé étrangère des tentatives.
        let text = texts::create(&pool, TextRecord::pasted("T", "fête"))
            .await
            .unwrap();
        let session = sessions::start(&pool, &profile, &text, "{}").await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        // « fête » à revoir après « œuvre » ; un mot non suivi n'apparaît pas.
        let fete = save(&mut conn, &profile, None, &WordRecord::added("fête", 2_000))
            .await
            .unwrap();
        save(
            &mut conn,
            &profile,
            None,
            &WordRecord::added("œuvre", 1_000),
        )
        .await
        .unwrap();
        save(&mut conn, &profile, None, &WordRecord::new("chat"))
            .await
            .unwrap();
        // Trois tentatives fausses dont deux formes identiques.
        let t = now_ms();
        for (i, (typed, kind)) in [
            ("fete", ErrorType::Accent),
            ("fète", ErrorType::Accent),
            ("fete", ErrorType::Accent),
        ]
        .into_iter()
        .enumerate()
        {
            insert_attempt(
                &mut conn,
                &fete,
                &session.id,
                &wrong(typed, kind),
                t + i as i64,
            )
            .await
            .unwrap();
        }
        drop(conn);
        let words = tracked(&pool, &profile).await.unwrap();
        let names: Vec<&str> = words.iter().map(|w| w.record.word.as_str()).collect();
        assert_eq!(names, ["œuvre", "fête"]);
        // Formes distinctes, la plus récente d'abord ; tous les types de fautes.
        assert_eq!(words[1].typed_forms, ["fete", "fète"]);
        assert_eq!(words[1].error_types, [ErrorType::Accent; 3]);
        assert!(words[0].typed_forms.is_empty());
        assert_eq!(due(&pool, &profile, 1_500).await.unwrap(), ["œuvre"]);
        assert_eq!(counts(&pool, &profile).await.unwrap(), (2, 0));
    }

    /// Supprimer un mot absent est une erreur ; un mot présent disparaît.
    #[tokio::test]
    async fn deletes_words() {
        let (pool, profile) = setup().await;
        let mut conn = pool.acquire().await.unwrap();
        save(&mut conn, &profile, None, &WordRecord::added("fête", 0))
            .await
            .unwrap();
        drop(conn);
        delete(&pool, &profile, "fête").await.unwrap();
        assert!(matches!(
            delete(&pool, &profile, "fête").await,
            Err(AppError::NotFound(_))
        ));
    }
}
