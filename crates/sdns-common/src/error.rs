#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> http::StatusCode {
        match self {
            Self::Unauthenticated(_) => http::StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => http::StatusCode::FORBIDDEN,
            Self::NotFound(_) => http::StatusCode::NOT_FOUND,
            Self::Validation(_) | Self::Config(_) => http::StatusCode::BAD_REQUEST,
            Self::Internal(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
