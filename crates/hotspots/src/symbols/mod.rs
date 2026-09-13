//! Language-agnostic symbol facts and change expansion to `path::symbol` entities.

mod expand;
mod types;

pub use expand::{expand_file_change, expand_with_diffs};
pub use types::{ExpandStats, FileDiff, Hunk, SymbolFact, SymbolResolver, entity_id, is_rust_path};

use crate::error::Result;
use crate::model::Change;
use crate::options::Grain;
use std::path::Path;

/// Expands changes for function grain, or returns them unchanged for file grain.
///
/// Non-`.rs` paths are dropped under function grain. Their count is returned in
/// [`ExpandStats::dropped_non_rust`].
///
/// # Errors
///
/// Returns an error when `grain` is function and `resolver` is missing, `repo`
/// is missing, or expansion fails.
pub fn apply_grain(
    changes: Vec<Change>,
    grain: Grain,
    repo: Option<&Path>,
    resolver: Option<&dyn SymbolResolver>,
) -> Result<(Vec<Change>, ExpandStats)> {
    match grain {
        Grain::File => Ok((changes, ExpandStats::default())),
        Grain::Function => expand_function_grain(&changes, repo, resolver),
    }
}

fn expand_function_grain(
    changes: &[Change],
    repo: Option<&Path>,
    resolver: Option<&dyn SymbolResolver>,
) -> Result<(Vec<Change>, ExpandStats)> {
    let repo = repo
        .ok_or_else(|| crate::error::Error::msg("`--grain function` requires `--repo <path>`"))?;
    let resolver = resolver
        .ok_or_else(|| crate::error::Error::msg("`--grain function` requires a symbol resolver"))?;
    resolver.expand(changes, repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Change;
    use crate::options::Grain;
    use std::path::PathBuf;

    #[test]
    fn file_grain_is_noop() {
        let changes = vec![Change::new(
            "1",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(1),
            Some(0),
        )];
        let out = apply_grain(changes.clone(), Grain::File, None, None);
        assert!(out.is_ok_and(|(rows, stats)| rows == changes && stats.dropped_non_rust == 0));
    }

    #[test]
    fn function_grain_requires_repo_and_resolver() {
        let changes = vec![Change::new(
            "1",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(1),
            Some(0),
        )];
        assert!(apply_grain(changes.clone(), Grain::Function, None, None).is_err());
        assert!(
            apply_grain(
                changes,
                Grain::Function,
                Some(PathBuf::from(".").as_path()),
                None
            )
            .is_err()
        );
    }
}
