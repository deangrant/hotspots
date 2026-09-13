//! Logical coupling between entities that change together.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{entity_revisions, fmt_u64, meets_min_revs, ordered_pair};
use crate::index::ChangesetIndex;
use crate::model::Change;
use crate::options::Options;

type CouplingRow = (String, String, u64, u64);

/// Ranks entity pairs by shared-commit coupling degree.
///
/// Degree is `100 * shared_revs / max(revs_a, revs_b)`.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let index = ChangesetIndex::build(changes, opts.temporal_period);
    let revs = entity_revisions(changes);
    let shared = pair_shared_counts(&index, opts.max_changeset_size);
    let mut rows = collect_rows(shared, &revs, opts);
    rows.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)).then(a.0.cmp(&b.0)));
    build_table(rows, opts.rows)
}

fn collect_rows(
    shared: BTreeMap<(String, String), u64>,
    revs: &BTreeMap<String, u64>,
    opts: &Options,
) -> Vec<CouplingRow> {
    shared
        .into_iter()
        .filter_map(|(pair, shared_revs)| try_row(pair, shared_revs, revs, opts))
        .collect()
}

fn try_row(
    pair: (String, String),
    shared_revs: u64,
    revs: &BTreeMap<String, u64>,
    opts: &Options,
) -> Option<CouplingRow> {
    let (left, right) = pair;
    if !passes_revision_gates(shared_revs, &left, &right, revs, opts) {
        return None;
    }
    let revs_a = revs.get(&left).copied().unwrap_or(0);
    let revs_b = revs.get(&right).copied().unwrap_or(0);
    let degree = coupling_degree(shared_revs, revs_a, revs_b)?;
    if !degree_in_range(degree, opts) {
        return None;
    }
    Some((left, right, degree, u64::midpoint(revs_a, revs_b)))
}

fn passes_revision_gates(
    shared_revs: u64,
    left: &str,
    right: &str,
    revs: &BTreeMap<String, u64>,
    opts: &Options,
) -> bool {
    if shared_revs < opts.min_shared_revs {
        return false;
    }
    let revs_a = revs.get(left).copied().unwrap_or(0);
    let revs_b = revs.get(right).copied().unwrap_or(0);
    meets_min_revs(revs_a, opts) && meets_min_revs(revs_b, opts)
}

const fn degree_in_range(degree: u64, opts: &Options) -> bool {
    degree >= opts.min_coupling && degree <= opts.max_coupling
}

fn build_table(rows: Vec<CouplingRow>, limit: Option<usize>) -> Table {
    let mut table = Table::with_headers(["entity", "coupled", "degree", "average-revs"]);
    for (left, right, degree, average) in rows {
        table.push_row([left, right, fmt_u64(degree), fmt_u64(average)]);
    }
    table.limit(limit)
}

fn coupling_degree(shared: u64, revs_a: u64, revs_b: u64) -> Option<u64> {
    let denom = revs_a.max(revs_b);
    if denom == 0 {
        return None;
    }
    Some(100 * shared / denom)
}

fn pair_shared_counts(
    index: &ChangesetIndex,
    max_changeset_size: usize,
) -> BTreeMap<(String, String), u64> {
    let mut shared: BTreeMap<(String, String), u64> = BTreeMap::new();
    for changeset in index.changesets() {
        count_pairs_in_changeset(&mut shared, changeset, max_changeset_size);
    }
    shared
}

fn count_pairs_in_changeset(
    shared: &mut BTreeMap<(String, String), u64>,
    changeset: &crate::index::Changeset,
    max_changeset_size: usize,
) {
    if changeset.entities.len() > max_changeset_size {
        return;
    }
    let entities: Vec<&String> = changeset.entities.iter().collect();
    for i in 0..entities.len() {
        bump_pairs_with(shared, &entities, i);
    }
}

fn bump_pairs_with(shared: &mut BTreeMap<(String, String), u64>, entities: &[&String], i: usize) {
    for j in (i + 1)..entities.len() {
        let (a, b) = ordered_pair(entities[i], entities[j]);
        *shared.entry((a, b)).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options {
            min_revs: 1,
            min_shared_revs: 1,
            min_coupling: 1,
            max_coupling: 100,
            ..Options::default()
        }
    }

    fn pair_changes() -> Vec<Change> {
        vec![
            Change::new("1", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("1", "Ada", "2024-01-01", "b.rs", None, None),
            Change::new("2", "Ada", "2024-01-02", "a.rs", None, None),
            Change::new("2", "Ada", "2024-01-02", "b.rs", None, None),
        ]
    }

    #[test]
    fn shared_pairs_and_degree_zero_denom() {
        let table = run(&pair_changes(), &opts());
        assert!(!table.rows.is_empty());
        assert_eq!(coupling_degree(1, 0, 0), None);
        assert_eq!(coupling_degree(1, 4, 2), Some(25));
    }

    #[test]
    fn skips_oversized_and_filters_degree() {
        let changes = pair_changes();
        let mut o = opts();
        o.max_changeset_size = 1;
        assert!(run(&changes, &o).rows.is_empty());
        o.max_changeset_size = 30;
        o.min_coupling = 100;
        o.max_coupling = 100;
        let high = run(&changes, &o);
        assert!(!high.rows.is_empty());
        o.max_coupling = 1;
        o.min_coupling = 0;
        assert!(run(&changes, &o).rows.is_empty());
    }

    #[test]
    fn filters_when_entity_below_min_revs() {
        let changes = vec![
            Change::new("1", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("1", "Ada", "2024-01-01", "b.rs", None, None),
            Change::new("2", "Ada", "2024-01-02", "a.rs", None, None),
            Change::new("2", "Ada", "2024-01-02", "b.rs", None, None),
            Change::new("3", "Ada", "2024-01-03", "lonely.rs", None, None),
            Change::new("3", "Ada", "2024-01-03", "a.rs", None, None),
        ];
        let mut o = opts();
        o.min_revs = 2;
        o.min_shared_revs = 1;
        let table = run(&changes, &o);
        assert!(table.rows.iter().all(|row| row[0] != "lonely.rs" && row[1] != "lonely.rs"));
    }
}
