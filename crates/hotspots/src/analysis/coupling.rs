//! Logical coupling between entities that change together.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{entity_revisions, fmt_u64, meets_min_revs, ordered_pair};
use crate::index::ChangesetIndex;
use crate::model::Change;
use crate::options::Options;

/// Ranks entity pairs by shared-commit coupling degree.
///
/// Degree is `100 * shared_revs / max(revs_a, revs_b)`.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let index = ChangesetIndex::build(changes, opts.temporal_period);
    let revs = entity_revisions(changes);
    let shared = pair_shared_counts(&index, opts.max_changeset_size);
    let mut rows = Vec::new();
    for ((left, right), shared_revs) in shared {
        if shared_revs < opts.min_shared_revs {
            continue;
        }
        let revs_a = revs.get(&left).copied().unwrap_or(0);
        let revs_b = revs.get(&right).copied().unwrap_or(0);
        if !meets_min_revs(revs_a, opts) || !meets_min_revs(revs_b, opts) {
            continue;
        }
        let Some(degree) = coupling_degree(shared_revs, revs_a, revs_b) else {
            continue;
        };
        if degree < opts.min_coupling || degree > opts.max_coupling {
            continue;
        }
        let average = u64::midpoint(revs_a, revs_b);
        rows.push((left, right, degree, average));
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "coupled", "degree", "average-revs"]);
    for (left, right, degree, average) in rows {
        table.push_row([left, right, fmt_u64(degree), fmt_u64(average)]);
    }
    table.limit(opts.rows)
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
        if changeset.entities.len() > max_changeset_size {
            continue;
        }
        let entities: Vec<&String> = changeset.entities.iter().collect();
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let (a, b) = ordered_pair(entities[i], entities[j]);
                *shared.entry((a, b)).or_insert(0) += 1;
            }
        }
    }
    shared
}
