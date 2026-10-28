//! Erreurs des commandes et leur forme sérialisée pour l'interface.

use serde::Serialize;

/// Erreur renvoyée à l'interface : un code stable et un message lisible.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Échec d'une requête SQLite.
    #[error("base de données : {0}")]
    Database(#[from] sqlx::Error),
    /// Échec d'une migration au démarrage.
    #[error("migration : {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// Enregistrement absent (texte, session, profil…).
    #[error("introuvable : {0}")]
    NotFound(String),
    /// Donnée refusée par une règle métier.
    #[error("{0}")]
    Invalid(String),
}

impl AppError {
    /// Code stable correspondant à la variante.
    fn code(&self) -> ErrorCode {
        match self {
            AppError::Database(_) => ErrorCode::Database,
            AppError::Migration(_) => ErrorCode::Migration,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::Invalid(_) => ErrorCode::Invalid,
        }
    }
}

/// Code d'erreur stable, sur lequel l'interface peut s'appuyer.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// Erreur SQLite.
    Database,
    /// Erreur de migration.
    Migration,
    /// Enregistrement introuvable.
    NotFound,
    /// Donnée invalide.
    Invalid,
}

/// Forme sérialisée de `AppError`, décrite pour specta.
#[derive(Serialize, specta::Type)]
pub struct ApiError {
    /// Code stable.
    pub code: ErrorCode,
    /// Message lisible, en français.
    pub message: String,
}

/// `AppError` est envoyé à l'interface sous la forme d'un `ApiError`.
impl Serialize for AppError {
    /// Sérialise le code et le message.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ApiError {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

/// specta décrit `AppError` comme un `ApiError`, sa forme sérialisée.
impl specta::Type for AppError {
    /// Reprend la définition de `ApiError`.
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        ApiError::definition(types)
    }
}

/// Résultat des commandes.
pub type AppResult<T> = Result<T, AppError>;
