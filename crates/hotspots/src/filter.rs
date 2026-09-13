//! Path include and exclude filtering for change entities.

use crate::model::Change;

/// Filters entities by optional include and exclude path prefixes.
#[derive(Debug, Clone, Default)]
pub struct PathFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl PathFilter {
    /// Builds a filter from include and exclude prefix lists.
    #[must_use]
    pub fn new(include: Vec<String>, exclude: Vec<String>) -> Self {
        Self {
            include: normalize_all(include),
            exclude: normalize_all(exclude),
        }
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

    #[test]
    fn exclude_drops_vendor_paths() {
        let filter = PathFilter::new(vec![], vec![String::from("vendor")]);
        assert!(!filter.allows("vendor/lib.js"));
        assert!(filter.allows("src/main.rs"));
    }

    #[test]
    fn include_requires_match() {
        let filter = PathFilter::new(vec![String::from("src")], vec![]);
        assert!(filter.allows("src/a.rs"));
        assert!(!filter.allows("docs/a.md"));
    }
}
