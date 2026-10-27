use serde::Serialize;

/// Erreur renvoyée à l'interface : un code stable et un message lisible.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("base de données : {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration : {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("introuvable : {0}")]
    NotFound(String),
}

impl AppError {
    fn code(&self) -> ErrorCode {
        match self {
            AppError::Database(_) => ErrorCode::Database,
            AppError::Migration(_) => ErrorCode::Migration,
            AppError::NotFound(_) => ErrorCode::NotFound,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    Database,
    Migration,
    NotFound,
}

/// Forme sérialisée de `AppError`, décrite pour specta.
#[derive(Serialize, specta::Type)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ApiError {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl specta::Type for AppError {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        ApiError::definition(types)
    }
}

pub type AppResult<T> = Result<T, AppError>;
