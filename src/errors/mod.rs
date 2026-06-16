use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Discord error: {0}")]
    Discord(#[from] serenity::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

impl axum::response::IntoResponse for BotError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            BotError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            BotError::Unauthorized(_) => axum::http::StatusCode::UNAUTHORIZED,
            BotError::Validation(_) => axum::http::StatusCode::BAD_REQUEST,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = serde_json::json!({
            "error": self.to_string(),
            "status": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}
