//! Symbol and hunk types shared by resolvers.

use std::path::Path;

use crate::error::Result;
use crate::model::Change;

/// Counts recorded while expanding to function grain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExpandStats {
    /// File changes dropped because the path was not Rust.
    pub dropped_non_rust: u64,
}

/// One symbol range in a source file at a given revision blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolFact {
    /// Repository-relative path.
    pub path: String,
    /// Inclusive start line (1-based).
    pub start_line: u32,
    /// Inclusive end line (1-based).
    pub end_line: u32,
    /// Symbol name (`fn`, or `Type::method`).
    pub name: String,
}

/// Unified-diff hunk line span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hunk {
    /// Old-file start line (1-based); `0` when absent.
    pub old_start: u32,
    /// Old-file line count.
    pub old_count: u32,
    /// New-file start line (1-based); `0` when absent.
    pub new_start: u32,
    /// New-file line count.
    pub new_count: u32,
}

/// Per-commit file diff facts used to expand a path-level change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// Commit id matching [`Change::rev`].
    pub rev: String,
    /// Path matching [`Change::entity`] before expansion.
    pub path: String,
    /// Symbols parsed from the file at `rev` (new side).
    pub symbols_new: Vec<SymbolFact>,
    /// Symbols parsed from the parent blob (old side); empty when unavailable.
    pub symbols_old: Vec<SymbolFact>,
    /// Zero-context hunks for this path at `rev`.
    pub hunks: Vec<Hunk>,
}

/// Builds function-level changes from file-level changes using a Git repo.
pub trait SymbolResolver {
    /// Expands file changes into `path::symbol` entities.
    ///
    /// # Errors
    ///
    /// Returns an error when git, parse, or expansion fails.
    fn expand(&self, changes: &[Change], repo: &Path) -> Result<(Vec<Change>, ExpandStats)>;
}

/// Returns whether `path` should be expanded under function grain.
#[must_use]
pub fn is_rust_path(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
}

/// Formats a function-grain entity id.
#[must_use]
pub fn entity_id(path: &str, symbol: &str) -> String {
    format!("{path}::{symbol}")
}
