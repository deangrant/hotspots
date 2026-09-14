//! Path include and exclude filtering for change entities.

use crate::error::{Error, Result};
use crate::model::Change;

/// Filters entities by optional include and exclude path prefixes.
#[derive(Debug, Clone, Default)]
pub struct PathFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl PathFilter {
    /// Builds a filter from include and exclude prefix lists.
    ///
    /// # Errors
    ///
    /// Returns an error when any include prefix is empty after normalization.
    pub fn try_new(include: Vec<String>, exclude: Vec<String>) -> Result<Self> {
        ensure_include_prefixes(&include)?;
        Ok(Self {
            include: normalize_all(include),
            exclude: normalize_all(exclude),
        })
    }

    /// Returns whether `path` should be kept.
    #[must_use]
    pub fn allows(&self, path: &str) -> bool {
        if self.exclude.iter().any(|prefix| matches_prefix(path, prefix)) {
            return false;
        }
        if self.include.is_empty() {
            return true;
        }
        self.include.iter().any(|prefix| matches_prefix(path, prefix))
    }

    /// Keeps only changes whose entity paths pass the filter.
    #[must_use]
    pub fn apply(&self, changes: Vec<Change>) -> Vec<Change> {
        changes.into_iter().filter(|change| self.allows(&change.entity)).collect()
    }
}

/// Rejects include prefixes that normalize to empty.
///
/// # Errors
///
/// Returns an error when any prefix is empty after trim and trailing-`/` strip.
pub fn ensure_include_prefixes(include: &[String]) -> Result<()> {
    if include.iter().any(|raw| normalize_prefix(raw).is_empty()) {
        return Err(Error::msg("`--include` prefix must be non-empty"));
    }
    Ok(())
}

fn normalize_all(prefixes: Vec<String>) -> Vec<String> {
    prefixes.into_iter().map(|p| normalize_prefix(&p)).collect()
}

fn normalize_prefix(prefix: &str) -> String {
    prefix.trim().trim_end_matches('/').to_owned()
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(include: Vec<&str>, exclude: Vec<&str>) -> PathFilter {
        let include: Vec<String> = include.into_iter().map(String::from).collect();
        let exclude: Vec<String> = exclude.into_iter().map(String::from).collect();
        assert!(ensure_include_prefixes(&include).is_ok());
        PathFilter {
            include: normalize_all(include),
            exclude: normalize_all(exclude),
        }
    }

    #[test]
    fn exclude_drops_vendor_paths() {
        let filter = filter(vec![], vec!["vendor"]);
        assert!(!filter.allows("vendor/lib.js"));
        assert!(filter.allows("src/main.rs"));
    }

    #[test]
    fn include_requires_match() {
        let filter = filter(vec!["src"], vec![]);
        assert!(filter.allows("src/a.rs"));
        assert!(!filter.allows("docs/a.md"));
    }

    #[test]
    fn empty_exclude_prefix_never_matches() {
        let filter = filter(vec![], vec!["", "  /"]);
        assert!(filter.allows("anything.rs"));
    }

    #[test]
    fn rejects_empty_include_prefixes() {
        for raw in ["", "   ", "/", "  /  "] {
            assert!(
                PathFilter::try_new(vec![String::from(raw)], vec![])
                    .is_err_and(|e| e.to_string().contains("non-empty"))
            );
        }
        assert!(ensure_include_prefixes(&[String::from("/")]).is_err());
    }

    #[test]
    fn trailing_slash_and_exact_prefix() {
        let filter = filter(vec!["src/"], vec![]);
        assert!(filter.allows("src"));
        assert!(filter.allows("src/a.rs"));
        assert!(!filter.allows("src2/a.rs"));
    }

    #[test]
    fn apply_filters_change_entities() {
        let filter = filter(vec!["src"], vec![]);
        let kept = filter.apply(vec![
            Change::new("1", "Ada", "2024-01-01", "src/a.rs", None, None),
            Change::new("2", "Ada", "2024-01-01", "docs/b.rs", None, None),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].entity, "src/a.rs");
    }
}
