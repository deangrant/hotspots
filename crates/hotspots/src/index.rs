//! Changeset indexes used by coupling and related metrics.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Change;
use crate::options::TemporalPeriod;

/// One logical commit and the entities it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changeset {
    /// Logical revision key (hash or day-merged id).
    pub rev: String,
    /// Author associated with the changeset.
    pub author: String,
    /// Calendar date for the changeset.
    pub date: String,
    /// Distinct entities modified in this changeset.
    pub entities: BTreeSet<String>,
}

/// Indexed view of changes grouped into changesets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangesetIndex {
    /// Changesets keyed by logical revision.
    pub by_rev: BTreeMap<String, Changeset>,
}

impl ChangesetIndex {
    /// Builds an index from change rows using the temporal strategy.
    #[must_use]
    pub fn build(changes: &[Change], period: TemporalPeriod) -> Self {
        let mut by_rev: BTreeMap<String, Changeset> = BTreeMap::new();
        for change in changes {
            let key = logical_rev(change, period);
            let entry = by_rev.entry(key.clone()).or_insert_with(|| Changeset {
                rev: key,
                author: change.author.clone(),
                date: change.date.clone(),
                entities: BTreeSet::new(),
            });
            entry.entities.insert(change.entity.clone());
        }
        Self { by_rev }
    }

    /// Returns changesets as a slice-friendly vector.
    #[must_use]
    pub fn changesets(&self) -> Vec<&Changeset> {
        self.by_rev.values().collect()
    }
}

fn logical_rev(change: &Change, period: TemporalPeriod) -> String {
    match period {
        TemporalPeriod::None => change.rev.clone(),
        TemporalPeriod::Day => format!("day:{}:{}", change.date, change.author),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_merge_combines_same_author_date() {
        let changes = vec![
            Change::new("a", "Ada", "2024-01-01", "x.rs", None, None),
            Change::new("b", "Ada", "2024-01-01", "y.rs", None, None),
        ];
        let index = ChangesetIndex::build(&changes, TemporalPeriod::Day);
        assert_eq!(index.by_rev.len(), 1);
        assert_eq!(
            index.by_rev.values().next().map(|c| c.entities.len()),
            Some(2)
        );
    }
}
