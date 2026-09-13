//! Fallible operations shared across the library.

use std::error::Error as StdError;
use std::fmt;
use std::io;

/// Category of a library error for callers that need more than the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Filesystem or process I/O failure.
    Io,
    /// Malformed log, date, or other input.
    Parse,
    /// Git invocation or repository lookup failure.
    Git,
    /// Analysis prerequisites or unknown analysis name.
    Analysis,
    /// Uncategorized message.
    Other,
}

/// Error produced while parsing logs or running an analysis.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    source: Option<io::Error>,
    git_missing_path: bool,
}

impl Error {
    /// Creates an uncategorized error from a displayable message.
    pub fn msg(message: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::Other, message)
    }

    /// Creates a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::Parse, message)
    }

    /// Creates a git error.
    pub fn git(message: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::Git, message)
    }

    /// Creates a git error for a missing `REV:PATH` blob.
    pub fn git_missing_path(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Git,
            message: message.into(),
            source: None,
            git_missing_path: true,
        }
    }

    /// Creates an analysis error.
    pub fn analysis(message: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::Analysis, message)
    }

    /// Returns the error category.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns whether this error is Git's missing-path signal for `show REV:PATH`.
    #[must_use]
    pub const fn is_git_missing_path(&self) -> bool {
        self.git_missing_path
    }

    fn with_kind(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
            git_missing_path: false,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|e| e as &(dyn StdError + 'static))
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self {
            kind: ErrorKind::Io,
            message: value.to_string(),
            source: Some(value),
            git_missing_path: false,
        }
    }
}

/// Result alias for library operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_kinds() {
        assert_eq!(Error::msg("boom").to_string(), "boom");
        assert_eq!(Error::msg("boom").kind(), ErrorKind::Other);
        assert_eq!(Error::parse("bad").kind(), ErrorKind::Parse);
        assert_eq!(Error::git("git").kind(), ErrorKind::Git);
        assert_eq!(Error::analysis("a").kind(), ErrorKind::Analysis);
        assert!(Error::git_missing_path("gone").is_git_missing_path());
        assert!(!Error::git("fail").is_git_missing_path());
    }

    #[test]
    fn io_conversion_preserves_source() {
        let from_io = Error::from(io::Error::other("disk"));
        assert_eq!(from_io.kind(), ErrorKind::Io);
        assert!(from_io.to_string().contains("disk"));
        assert!(from_io.source().is_some());
        let _: &dyn StdError = &from_io;
    }
}
