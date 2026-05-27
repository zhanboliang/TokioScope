use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TsError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("runner busy")]
    Busy,
    #[error("runner not ready: {0}")]
    NotReady(String),
    #[error("cargo failed: {0}")]
    Cargo(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for TsError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<anyhow::Error> for TsError {
    fn from(value: anyhow::Error) -> Self {
        TsError::Other(value.to_string())
    }
}

pub type TsResult<T> = std::result::Result<T, TsError>;
