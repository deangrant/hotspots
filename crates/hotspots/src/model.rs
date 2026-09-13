//! Normalized change events mined from version-control logs.

/// One file modification recorded in a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Commit identifier from the VCS.
    pub rev: String,
    /// Author name as recorded in the log.
    pub author: String,
    /// Commit date in `YYYY-MM-DD` form.
    pub date: String,
    /// Changed path, or a layer name after grouping.
    pub entity: String,
    /// Lines added when numstat is present.
    pub added: Option<u64>,
    /// Lines deleted when numstat is present.
    pub deleted: Option<u64>,
}

impl Change {
    /// Builds a change with the given identity fields and optional churn.
    pub fn new(
        rev: impl Into<String>,
        author: impl Into<String>,
        date: impl Into<String>,
        entity: impl Into<String>,
        added: Option<u64>,
        deleted: Option<u64>,
    ) -> Self {
        Self {
            rev: rev.into(),
            author: author.into(),
            date: date.into(),
            entity: entity.into(),
            added,
            deleted,
        }
    }
}
