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

/// Clé de `app_meta` qui retient le profil actif (F-50, ADR 0023).
const ACTIVE_KEY: &str = "active_profile";

/// Profils, du plus ancien au plus récent.
pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Profile>> {
    Ok(sqlx::query_as(
        "SELECT id, name, keyboard_layout, created_at FROM profiles ORDER BY created_at, id",
    )
    .fetch_all(pool)
    .await?)
}

/// Nom d'un profil : non vide, espaces de bord retirées.
fn clean_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("le profil doit avoir un nom".into()));
    }
    Ok(name.to_owned())
}

/// Crée un profil ; `layout` est une valeur de `profiles.keyboard_layout`.
pub async fn create(pool: &SqlitePool, name: &str, layout: &str) -> AppResult<Profile> {
    let profile = Profile {
        id: new_id(),
        name: clean_name(name)?,
        keyboard_layout: layout.to_owned(),
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

/// Change le nom et la disposition d'un profil ; `NotFound` s'il n'existe pas.
pub async fn update(pool: &SqlitePool, id: &str, name: &str, layout: &str) -> AppResult<Profile> {
    let updated = sqlx::query("UPDATE profiles SET name = ?, keyboard_layout = ? WHERE id = ?")
        .bind(clean_name(name)?)
        .bind(layout)
        .bind(id)
        .execute(pool)
        .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("profil {id}")));
    }
    get(pool, id).await
}

/// Supprime un profil et toutes ses données (sessions, mots, réglages) en cascade.
/// Le dernier profil ne peut pas être supprimé.
pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM profiles")
        .fetch_one(&mut *tx)
        .await?;
    if count <= 1 {
        return Err(AppError::Invalid(
            "le dernier profil ne peut pas être supprimé".into(),
        ));
    }
    let deleted = sqlx::query("DELETE FROM profiles WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("profil {id}")));
    }
    tx.commit().await?;
    Ok(())
}

/// Retient le profil actif pour le prochain démarrage.
pub async fn remember_active(pool: &SqlitePool, id: &str) -> AppResult<()> {
    sqlx::query("INSERT OR REPLACE INTO app_meta (key, value) VALUES (?, ?)")
        .bind(ACTIVE_KEY)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Profil à ouvrir au démarrage : celui retenu s'il existe encore, sinon le plus
/// ancien (créé s'il n'y en a aucun).
pub async fn active(pool: &SqlitePool) -> AppResult<Profile> {
    let remembered: Option<String> = sqlx::query_scalar("SELECT value FROM app_meta WHERE key = ?")
        .bind(ACTIVE_KEY)
        .fetch_optional(pool)
        .await?;
    // Le profil retenu a pu être supprimé entre-temps.
    if let Some(id) = remembered
        && let Ok(profile) = get(pool, &id).await
    {
        return Ok(profile);
    }
    ensure_default(pool).await
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

    /// Créer, renommer, changer de disposition, supprimer ; jamais le dernier.
    #[tokio::test]
    async fn manages_several_profiles() {
        let pool = connect_in_memory().await.unwrap();
        let first = ensure_default(&pool).await.unwrap();
        let child = create(&pool, " Léa ", "bepo").await.unwrap();
        assert_eq!(child.name, "Léa");
        assert_eq!(list(&pool).await.unwrap().len(), 2);
        let renamed = update(&pool, &child.id, "Léa B.", "qwerty").await.unwrap();
        assert_eq!(
            (renamed.name.as_str(), renamed.keyboard_layout.as_str()),
            ("Léa B.", "qwerty")
        );
        // Nom vide ou disposition inconnue refusés.
        assert!(matches!(
            create(&pool, "  ", "azerty").await,
            Err(AppError::Invalid(_))
        ));
        assert!(update(&pool, &child.id, "x", "dvorak").await.is_err());
        // Profil actif retenu ; supprimé, on revient au plus ancien.
        remember_active(&pool, &child.id).await.unwrap();
        assert_eq!(active(&pool).await.unwrap().id, child.id);
        delete(&pool, &child.id).await.unwrap();
        assert_eq!(active(&pool).await.unwrap().id, first.id);
        assert!(matches!(
            delete(&pool, &first.id).await,
            Err(AppError::Invalid(_))
        ));
    }
}
