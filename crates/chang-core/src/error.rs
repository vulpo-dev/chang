use serde_json;
use sqlx;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error("")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
