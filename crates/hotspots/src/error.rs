//! Fallible operations shared across the library.

use std::fmt;

/// Error produced while parsing logs or running an analysis.
#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    /// Creates an error from a displayable message.
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::msg(value.to_string())
    }
}

/// Result alias for library operations.
pub type Result<T> = std::result::Result<T, Error>;
