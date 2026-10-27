//! Dépôt des profils utilisateurs.

use serde::Serialize;
use sqlx::SqlitePool;

use super::{new_id, now_ms};
use crate::error::{AppError, AppResult};

/// Nom du profil créé au premier lancement.
pub const DEFAULT_PROFILE_NAME: &str = "Par défaut";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub keyboard_layout: String,
    pub created_at: i64,
}

pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<Profile> {
    sqlx::query_as("SELECT id, name, keyboard_layout, created_at FROM profiles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("profil {id}")))
}

/// Renvoie le plus ancien profil, ou crée le profil par défaut si la base est vide.
pub async fn ensure_default(pool: &SqlitePool) -> AppResult<Profile> {
    let existing: Option<Profile> = sqlx::query_as(
        "SELECT id, name, keyboard_layout, created_at FROM profiles ORDER BY created_at, id LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    if let Some(profile) = existing {
        return Ok(profile);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_in_memory;

    #[tokio::test]
    async fn first_launch_creates_default_azerty_profile() {
        let pool = connect_in_memory().await.unwrap();
        let profile = ensure_default(&pool).await.unwrap();
        assert_eq!(profile.name, DEFAULT_PROFILE_NAME);
        assert_eq!(profile.keyboard_layout, "azerty");
        assert_eq!(get(&pool, &profile.id).await.unwrap().id, profile.id);
    }

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

    #[tokio::test]
    async fn unknown_profile_is_not_found() {
        let pool = connect_in_memory().await.unwrap();
        assert!(matches!(
            get(&pool, "absent").await,
            Err(AppError::NotFound(_))
        ));
    }
}
