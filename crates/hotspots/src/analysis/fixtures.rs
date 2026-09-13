//! Shared change fixtures for analysis unit tests.

use crate::model::Change;

/// Two entities co-changed across two revisions on the given dates.
pub fn pair_changes_on(dates: [&str; 2]) -> Vec<Change> {
    vec![
        Change::new("1", "Ada", dates[0], "a.rs", None, None),
        Change::new("1", "Ada", dates[0], "b.rs", None, None),
        Change::new("2", "Ada", dates[1], "a.rs", None, None),
        Change::new("2", "Ada", dates[1], "b.rs", None, None),
    ]
}

/// Same as [`pair_changes_on`] with both revisions on `2024-01-01`.
pub fn same_day_pair_changes() -> Vec<Change> {
    pair_changes_on(["2024-01-01", "2024-01-01"])
}
