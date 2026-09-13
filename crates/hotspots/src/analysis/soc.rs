//! Sum-of-coupling scores per entity.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{count_as_u64, fmt_u64, meets_min_revs};
use crate::index::{Changeset, ChangesetIndex};
use crate::model::Change;
use crate::options::Options;

/// Sums `(changeset_size - 1)` across changesets containing each entity.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let index = ChangesetIndex::build(changes, opts.temporal_period);
    let revs = index.entity_revisions();
    let scores = accumulate_scores(&index, opts.max_changeset_size);
    let mut rows = filter_scores(scores, &revs, opts);
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    build_soc_table(rows, opts.rows)
}

fn accumulate_scores(index: &ChangesetIndex, max_changeset_size: usize) -> BTreeMap<String, u64> {
    let mut scores: BTreeMap<String, u64> = BTreeMap::new();
    for changeset in index.changesets() {
        add_changeset_score(&mut scores, changeset, max_changeset_size);
    }
    scores
}

/// SOC scores for every entity that appears in eligible changesets.
pub fn scores_by_entity(changes: &[Change], opts: &Options) -> BTreeMap<String, u64> {
    let index = ChangesetIndex::build(changes, opts.temporal_period);
    accumulate_scores(&index, opts.max_changeset_size)
}

fn add_changeset_score(
    scores: &mut BTreeMap<String, u64>,
    changeset: &Changeset,
    max_changeset_size: usize,
) {
    if changeset.entities.len() > max_changeset_size {
        return;
    }
    let size = count_as_u64(changeset.entities.len());
    if size == 0 {
        return;
    }
    let add = size - 1;
    for entity in &changeset.entities {
        *scores.entry(entity.clone()).or_insert(0) += add;
    }
}

fn filter_scores(
    scores: BTreeMap<String, u64>,
    revs: &BTreeMap<String, u64>,
    opts: &Options,
) -> Vec<(String, u64)> {
    scores
        .into_iter()
        .filter(|(entity, _)| {
            let n = revs.get(entity).copied().unwrap_or(0);
            meets_min_revs(n, opts)
        })
        .collect()
}

fn build_soc_table(rows: Vec<(String, u64)>, limit: Option<usize>) -> Table {
    let mut table = Table::with_headers(["entity", "soc"]);
    for (entity, soc) in rows {
        table.push_row([entity, fmt_u64(soc)]);
    }
    table.limit(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn skips_empty_and_oversized_changesets() {
        let empty = Changeset {
            rev: String::from("r"),
            author: String::from("a"),
            date: String::from("2024-01-01"),
            entities: BTreeSet::new(),
        };
        let mut scores = BTreeMap::new();
        add_changeset_score(&mut scores, &empty, 30);
        assert!(scores.is_empty());

        let mut entities = BTreeSet::new();
        entities.insert(String::from("a.rs"));
        entities.insert(String::from("b.rs"));
        let big = Changeset {
            rev: String::from("r2"),
            author: String::from("a"),
            date: String::from("2024-01-01"),
            entities,
        };
        add_changeset_score(&mut scores, &big, 1);
        assert!(scores.is_empty());
        add_changeset_score(&mut scores, &big, 30);
        assert_eq!(scores.get("a.rs"), Some(&1));
    }

    fn same_day_pair_changes() -> Vec<Change> {
        vec![
            Change::new("1", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("1", "Ada", "2024-01-01", "b.rs", None, None),
            Change::new("2", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("2", "Ada", "2024-01-01", "b.rs", None, None),
        ]
    }

    #[test]
    fn day_period_min_revs_uses_logical_revisions() {
        let mut opts = Options {
            min_revs: 2,
            ..Options::default()
        };
        opts.temporal_period = crate::options::TemporalPeriod::Day;
        assert!(run(&same_day_pair_changes(), &opts).rows.is_empty());
        opts.min_revs = 1;
        assert!(!run(&same_day_pair_changes(), &opts).rows.is_empty());
    }

    fn changeset_entities(n: usize) -> Changeset {
        let entities = (0..n).map(|i| format!("f{i}.rs")).collect();
        Changeset {
            rev: String::from("r"),
            author: String::from("Ada"),
            date: String::from("2024-01-01"),
            entities,
        }
    }

    #[test]
    fn accepts_limit_sized_changeset_and_skips_limit_plus_one() {
        let limit = crate::options::MAX_CHANGESET_SIZE_LIMIT;
        let mut scores = BTreeMap::new();
        add_changeset_score(&mut scores, &changeset_entities(limit), limit);
        assert_eq!(scores.len(), limit);
        assert_eq!(scores.get("f0.rs"), Some(&(count_as_u64(limit) - 1)));
        scores.clear();
        add_changeset_score(&mut scores, &changeset_entities(limit + 1), limit);
        assert!(scores.is_empty());
    }
}
