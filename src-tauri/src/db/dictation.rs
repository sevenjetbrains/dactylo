//! Dépôt de la dictée : profils de dictée (F-17), brouillon et fin d'une dictée
//! (ADR 0020), moyenne des dictées précédentes.

use dactylo_core::dictation::Correction;
use dactylo_core::dictation::settings::{Advance, DictationSettings, SegmentMode};
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::db::sessions::MODE_DICTATION;
use crate::error::{AppError, AppResult};

/// Profil de dictée enregistré (« Débutant lent », « Examen »…).
#[derive(Debug, Clone, PartialEq)]
pub struct ProfileRecord {
    /// Identifiant UUIDv7.
    pub id: String,
    /// Nom choisi par l'utilisateur.
    pub name: String,
    /// Réglages du profil.
    pub settings: DictationSettings,
}

/// Ligne de la table `dictation_profiles`.
#[derive(sqlx::FromRow)]
struct ProfileRow {
    /// Identifiant.
    id: String,
    /// Nom.
    name: String,
    /// Voix.
    voice: Option<String>,
    /// Vitesse relative.
    rate: f64,
    /// Hauteur relative.
    pitch: f64,
    /// Volume.
    volume: f64,
    /// Découpage.
    segmentation: String,
    /// Mots par groupe.
    group_size: u32,
    /// Pause entre segments.
    pause_ms: u32,
    /// Ponctuation lue.
    read_punctuation: bool,
    /// Avancement.
    advance: String,
    /// Répétitions autorisées.
    max_repeats: Option<u32>,
    /// Tolérance typographique.
    lenient_punctuation: bool,
}

impl ProfileRow {
    /// Profil métier, réglages bornés.
    fn into_record(self) -> ProfileRecord {
        let settings = DictationSettings {
            voice: self.voice,
            rate: self.rate,
            pitch: self.pitch,
            volume: self.volume,
            segmentation: SegmentMode::parse(&self.segmentation),
            group_size: self.group_size,
            pause_ms: self.pause_ms,
            read_punctuation: self.read_punctuation,
            advance: if self.advance == "automatic" {
                Advance::Automatic
            } else {
                Advance::Adaptive
            },
            max_repeats: self.max_repeats,
            lenient_punctuation: self.lenient_punctuation,
        }
        .sanitized();
        ProfileRecord {
            id: self.id,
            name: self.name,
            settings,
        }
    }
}

/// Valeur de la colonne `advance`.
fn advance_str(advance: Advance) -> &'static str {
    match advance {
        Advance::Adaptive => "adaptive",
        Advance::Automatic => "automatic",
    }
}

/// Profils de dictée du profil utilisateur, par nom.
pub async fn list_profiles(pool: &SqlitePool, profile_id: &str) -> AppResult<Vec<ProfileRecord>> {
    let rows: Vec<ProfileRow> = sqlx::query_as(
        "SELECT id, name, voice, rate, pitch, volume, segmentation, group_size, pause_ms, \
         read_punctuation, advance, max_repeats, lenient_punctuation FROM dictation_profiles \
         WHERE profile_id = ? ORDER BY name COLLATE NOCASE, id",
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(ProfileRow::into_record).collect())
}

/// Crée (sans `id`) ou remplace un profil de dictée ; renvoie son identifiant.
pub async fn save_profile(
    pool: &SqlitePool,
    profile_id: &str,
    id: Option<&str>,
    name: &str,
    settings: &DictationSettings,
) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid(
            "le profil de dictée doit avoir un nom".into(),
        ));
    }
    let s = settings.clone().sanitized();
    let id = id.map_or_else(new_id, str::to_owned);
    let saved = sqlx::query(
        "INSERT INTO dictation_profiles (id, profile_id, name, voice, rate, pitch, volume, \
         segmentation, group_size, pause_ms, read_punctuation, advance, max_repeats, \
         lenient_punctuation) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT (id) DO UPDATE SET name = excluded.name, voice = excluded.voice, \
         rate = excluded.rate, pitch = excluded.pitch, volume = excluded.volume, \
         segmentation = excluded.segmentation, group_size = excluded.group_size, \
         pause_ms = excluded.pause_ms, read_punctuation = excluded.read_punctuation, \
         advance = excluded.advance, max_repeats = excluded.max_repeats, \
         lenient_punctuation = excluded.lenient_punctuation \
         WHERE dictation_profiles.profile_id = excluded.profile_id",
    )
    .bind(&id)
    .bind(profile_id)
    .bind(name)
    .bind(&s.voice)
    .bind(s.rate)
    .bind(s.pitch)
    .bind(s.volume)
    .bind(s.segmentation.as_str())
    .bind(s.group_size)
    .bind(s.pause_ms)
    .bind(s.read_punctuation)
    .bind(advance_str(s.advance))
    .bind(s.max_repeats)
    .bind(s.lenient_punctuation)
    .execute(pool)
    .await?;
    // Un identifiant d'un autre profil utilisateur n'est pas modifiable.
    if saved.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("profil de dictée {id}")));
    }
    Ok(id)
}

/// Supprime un profil de dictée ; `NotFound` s'il n'existe pas.
pub async fn delete_profile(pool: &SqlitePool, profile_id: &str, id: &str) -> AppResult<()> {
    let deleted = sqlx::query("DELETE FROM dictation_profiles WHERE id = ? AND profile_id = ?")
        .bind(id)
        .bind(profile_id)
        .execute(pool)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("profil de dictée {id}")));
    }
    Ok(())
}

/// Remplace les réglages rangés avec une dictée en cours (changement de voix…).
pub async fn update_session_settings(
    pool: &SqlitePool,
    session_id: &str,
    settings_json: &str,
) -> AppResult<()> {
    sqlx::query("UPDATE sessions SET settings = ? WHERE id = ? AND mode = ?")
        .bind(settings_json)
        .bind(session_id)
        .bind(MODE_DICTATION)
        .execute(pool)
        .await?;
    Ok(())
}

/// Brouillon d'une dictée en cours (NF-06) : texte tapé et temps actif.
pub async fn save_draft(
    pool: &SqlitePool,
    session_id: &str,
    typed: &str,
    active_ms: u32,
) -> AppResult<()> {
    let updated = sqlx::query(
        "UPDATE sessions SET typed_text = ?, duration_ms = ? \
         WHERE id = ? AND mode = ? AND ended_at IS NULL AND abandoned_at IS NULL",
    )
    .bind(typed)
    .bind(active_ms)
    .bind(session_id)
    .bind(MODE_DICTATION)
    .execute(pool)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::Invalid(format!(
            "dictée {session_id} terminée, abandonnée ou absente"
        )));
    }
    Ok(())
}

/// Brouillon enregistré : texte tapé et temps actif.
pub async fn draft(pool: &SqlitePool, session_id: &str) -> AppResult<(String, u32)> {
    let row: Option<(Option<String>, Option<u32>)> =
        sqlx::query_as("SELECT typed_text, duration_ms FROM sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    let (typed, ms) = row.ok_or_else(|| AppError::NotFound(format!("session {session_id}")))?;
    Ok((typed.unwrap_or_default(), ms.unwrap_or(0)))
}

/// Chiffres d'une dictée terminée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DictationNumbers {
    /// Temps actif, en ms.
    pub duration_ms: u32,
    /// Mots tapés par minute.
    pub wpm: f64,
    /// Caractères tapés par minute.
    pub cpm: f64,
    /// Précision en mots, en %.
    pub accuracy: f64,
}

/// Clôt la dictée : chiffres, texte tapé et fautes, en une transaction.
pub async fn finish(
    pool: &SqlitePool,
    session_id: &str,
    typed: &str,
    numbers: DictationNumbers,
    correction: &Correction,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    // Seule une dictée en cours peut être close.
    let updated = sqlx::query(
        "UPDATE sessions SET ended_at = ?, duration_ms = ?, wpm_raw = ?, wpm_net = ?, cpm = ?, \
         accuracy = ?, typed_text = ? \
         WHERE id = ? AND mode = ? AND ended_at IS NULL AND abandoned_at IS NULL",
    )
    .bind(now_ms())
    .bind(numbers.duration_ms)
    .bind(numbers.wpm)
    .bind(numbers.wpm)
    .bind(numbers.cpm)
    .bind(numbers.accuracy)
    .bind(typed)
    .bind(session_id)
    .bind(MODE_DICTATION)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::Invalid(format!(
            "dictée {session_id} déjà terminée, abandonnée ou absente"
        )));
    }
    // Une ligne par faute, ponctuation comprise.
    for t in correction.tokens.iter() {
        let Some(error) = t.error else { continue };
        sqlx::query(
            "INSERT INTO session_errors (session_id, expected_word, typed_word, error_type, position) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session_id)
        .bind(&t.expected)
        .bind(&t.typed)
        .bind(error.as_str())
        .bind(t.position.unwrap_or(0))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Moyenne des dictées terminées précédentes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DictationAverage {
    /// Nombre de dictées prises en compte.
    pub sessions: u32,
    /// Précision moyenne, en %.
    pub accuracy: f64,
    /// Fautes moyennes.
    pub faults: f64,
    /// MPM moyen.
    pub wpm: f64,
}

/// Moyenne des `limit` dernières dictées terminées du profil, hors `exclude`.
pub async fn average(
    pool: &SqlitePool,
    profile_id: &str,
    exclude: &str,
    limit: u32,
) -> AppResult<Option<DictationAverage>> {
    let rows: Vec<(f64, f64, u32)> = sqlx::query_as(
        "SELECT accuracy, wpm_net, \
         (SELECT COUNT(*) FROM session_errors e WHERE e.session_id = s.id) \
         FROM sessions s WHERE profile_id = ? AND mode = ? AND ended_at IS NOT NULL AND id <> ? \
         ORDER BY ended_at DESC, id DESC LIMIT ?",
    )
    .bind(profile_id)
    .bind(MODE_DICTATION)
    .bind(exclude)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    // Pas de moyenne sans dictée précédente.
    if rows.is_empty() {
        return Ok(None);
    }
    let n = rows.len() as f64;
    Ok(Some(DictationAverage {
        sessions: rows.len() as u32,
        accuracy: rows.iter().map(|r| r.0).sum::<f64>() / n,
        wpm: rows.iter().map(|r| r.1).sum::<f64>() / n,
        faults: rows.iter().map(|r| f64::from(r.2)).sum::<f64>() / n,
    }))
}

/// Tests du dépôt de la dictée.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{connect_in_memory, profiles};

    /// Base en mémoire avec le profil par défaut.
    async fn setup() -> (SqlitePool, String) {
        let pool = connect_in_memory().await.unwrap();
        let profile = profiles::ensure_default(&pool).await.unwrap();
        (pool, profile.id)
    }

    /// Créer, relire, modifier et supprimer un profil de dictée (F-17).
    #[tokio::test]
    async fn dictation_profiles_round_trip() {
        let (pool, profile) = setup().await;
        let slow = DictationSettings {
            rate: 0.8,
            pause_ms: 2_000,
            advance: Advance::Automatic,
            max_repeats: Some(3),
            segmentation: SegmentMode::Word,
            ..DictationSettings::default()
        };
        let id = save_profile(&pool, &profile, None, " Débutant lent ", &slow)
            .await
            .unwrap();
        save_profile(
            &pool,
            &profile,
            None,
            "Examen",
            &DictationSettings::default(),
        )
        .await
        .unwrap();
        let list = list_profiles(&pool, &profile).await.unwrap();
        assert_eq!(
            list.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Débutant lent", "Examen"]
        );
        assert_eq!(list[0].settings, slow);
        // Renommage et modification sur place.
        save_profile(
            &pool,
            &profile,
            Some(&id),
            "Lent",
            &DictationSettings::default(),
        )
        .await
        .unwrap();
        let list = list_profiles(&pool, &profile).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(
            list.iter()
                .any(|p| p.name == "Lent" && p.settings.rate == 1.0)
        );
        // Nom vide refusé ; suppression, puis suppression d'un absent.
        assert!(matches!(
            save_profile(&pool, &profile, None, "  ", &slow).await,
            Err(AppError::Invalid(_))
        ));
        delete_profile(&pool, &profile, &id).await.unwrap();
        assert!(matches!(
            delete_profile(&pool, &profile, &id).await,
            Err(AppError::NotFound(_))
        ));
    }
}
