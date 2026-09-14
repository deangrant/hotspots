//! Shared path-prefix matching for filters and layer maps.

/// Returns whether `path` equals `prefix` or is a child under `prefix/`.
///
/// Empty prefixes never match.
#[must_use]
pub fn matches_prefix(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_child_non_child_and_empty() {
        assert!(matches_prefix("src", "src"));
        assert!(matches_prefix("src/a.rs", "src"));
        assert!(!matches_prefix("src2/a.rs", "src"));
        assert!(!matches_prefix("anything.rs", ""));
    }
}
