use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum SneakerError {
    #[error("Generic error: {0}")]
    Generic(String),
}
