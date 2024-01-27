use std::sync::OnceLock;

use regex::Regex;

#[derive(thiserror::Error, Debug)]
pub enum NameError {
    #[error("table names cannot be longer than 63 characters")]
    TooLong,

    #[error("table name {name:?} contains invalid characters")]
    InvalidCharacters { name: String },
}

static RE_CELL: OnceLock<Regex> = OnceLock::new();
const MAX_IDENTIFIER_LENGTH: usize = 64;

pub fn is_valid(name: &str) -> Result<(), NameError> {
    let re = RE_CELL.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());
    let parts = name.split('.').collect::<Vec<&str>>();

    if parts.len() > 2 {
        return Err(NameError::InvalidCharacters { name: name.into() });
    }

    if parts.iter().any(|name| name.len() >= MAX_IDENTIFIER_LENGTH) {
        return Err(NameError::TooLong);
    }

    if parts.iter().any(|name| !re.is_match(name)) {
        Err(NameError::InvalidCharacters { name: name.into() })
    } else {
        Ok(())
    }
}
