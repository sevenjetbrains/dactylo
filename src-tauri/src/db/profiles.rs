//! Dépôt des profils utilisateurs.

use serde::Serialize;
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::error::{AppError, AppResult};

/// Nom du profil créé au premier lancement.
pub const DEFAULT_PROFILE_NAME: &str = "Par défaut";

/// Profil utilisateur.
#[derive(Debug, Clone, Serialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// Identifiant UUIDv7.
    pub id: String,
    /// Nom affiché.
    pub name: String,
    /// Disposition du clavier (« azerty » par défaut).
    pub keyboard_layout: String,
    /// Date de création, en ms Unix.
    pub created_at: i64,
}

/// Lit un profil ; `NotFound` s'il n'existe pas.
pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<Profile> {
    sqlx::query_as("SELECT id, name, keyboard_layout, created_at FROM profiles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("profil {id}")))
}

/// Renvoie le plus ancien profil, ou crée le profil par défaut si la base est vide.
pub async fn ensure_default(pool: &SqlitePool) -> AppResult<Profile> {
    // Le plus ancien profil est le profil actif tant que F-50 n'existe pas.
    let existing: Option<Profile> = sqlx::query_as(
        "SELECT id, name, keyboard_layout, created_at FROM profiles ORDER BY created_at, id LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    if let Some(profile) = existing {
        return Ok(profile);
    }

    // Base vide : on crée le profil par défaut, en AZERTY.
    let profile = Profile {
        id: new_id(),
        name: DEFAULT_PROFILE_NAME.into(),
        keyboard_layout: "azerty".into(),
        created_at: now_ms(),
    };
    sqlx::query("INSERT INTO profiles (id, name, keyboard_layout, created_at) VALUES (?, ?, ?, ?)")
        .bind(&profile.id)
        .bind(&profile.name)
        .bind(&profile.keyboard_layout)
        .bind(profile.created_at)
        .execute(pool)
        .await?;
    Ok(profile)
}

/// Réglages d'un module : la clé `key` du JSON `profiles.settings` (ADR 0010).
pub async fn get_settings(
    pool: &SqlitePool,
    id: &str,
    key: &str,
) -> AppResult<Option<serde_json::Value>> {
    // Ligne absente → profil inconnu ; clé absente → `None` dans la ligne.
    let raw: Option<Option<String>> =
        sqlx::query_scalar("SELECT settings -> ('$.' || ?) FROM profiles WHERE id = ?")
            .bind(key)
            .bind(id)
            .fetch_optional(pool)
            .await?;
    // Une valeur illisible est traitée comme absente : le module prendra ses défauts.
    let raw = raw.ok_or_else(|| AppError::NotFound(format!("profil {id}")))?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()))
}

/// Remplace la clé `key` du JSON `profiles.settings`, sans toucher aux autres.
pub async fn save_settings(
    pool: &SqlitePool,
    id: &str,
    key: &str,
    value: &serde_json::Value,
) -> AppResult<()> {
    // `json_set` ne modifie que la clé du module, les autres réglages restent intacts.
    let updated = sqlx::query(
        "UPDATE profiles SET settings = json_set(settings, '$.' || ?, json(?)) WHERE id = ?",
    )
    .bind(key)
    .bind(value.to_string())
    .bind(id)
    .execute(pool)
    .await?;
    // Aucune ligne modifiée : le profil n'existe pas.
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("profil {id}")));
    }
    Ok(())
}

/// Tests du dépôt des profils.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_in_memory;

    /// Le premier lancement crée un profil AZERTY par défaut.
    #[tokio::test]
    async fn first_launch_creates_default_azerty_profile() {
        let pool = connect_in_memory().await.unwrap();
        let profile = ensure_default(&pool).await.unwrap();
        assert_eq!(profile.name, DEFAULT_PROFILE_NAME);
        assert_eq!(profile.keyboard_layout, "azerty");
        assert_eq!(get(&pool, &profile.id).await.unwrap().id, profile.id);
    }

    /// Les lancements suivants réutilisent ce profil sans en créer d'autre.
    #[tokio::test]
    async fn second_launch_reuses_existing_profile() {
        let pool = connect_in_memory().await.unwrap();
        let first = ensure_default(&pool).await.unwrap();
        let second = ensure_default(&pool).await.unwrap();
        assert_eq!(first.id, second.id);

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    /// Chaque module a sa clé de réglages, indépendante des autres.
    #[tokio::test]
    async fn settings_are_stored_per_key() {
        let pool = connect_in_memory().await.unwrap();
        let id = ensure_default(&pool).await.unwrap().id;
        // Aucune clé au départ.
        assert_eq!(get_settings(&pool, &id, "practice").await.unwrap(), None);

        // Deux clés écrites l'une après l'autre.
        let practice = serde_json::json!({ "visibleLines": 5 });
        save_settings(&pool, &id, "practice", &practice)
            .await
            .unwrap();
        save_settings(&pool, &id, "other", &serde_json::json!(true))
            .await
            .unwrap();
        // La seconde écriture n'a pas écrasé la première.
        assert_eq!(
            get_settings(&pool, &id, "practice").await.unwrap(),
            Some(practice)
        );
        assert_eq!(
            get_settings(&pool, &id, "other").await.unwrap(),
            Some(serde_json::json!(true))
        );
        // Profil inconnu, en lecture comme en écriture.
        assert!(matches!(
            get_settings(&pool, "absent", "practice").await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            save_settings(&pool, "absent", "practice", &serde_json::json!({})).await,
            Err(AppError::NotFound(_))
        ));
    }

    /// Un profil inconnu donne `NotFound`.
    #[tokio::test]
    async fn unknown_profile_is_not_found() {
        let pool = connect_in_memory().await.unwrap();
        assert!(matches!(
            get(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
    }
}
