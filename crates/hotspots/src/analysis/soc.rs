//! Sum-of-coupling scores per entity.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{count_as_u64, entity_revisions, fmt_u64, meets_min_revs};
use crate::index::ChangesetIndex;
use crate::model::Change;
use crate::options::Options;

/// Sums `(changeset_size - 1)` across changesets containing each entity.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let index = ChangesetIndex::build(changes, opts.temporal_period);
    let revs = entity_revisions(changes);
    let mut scores: BTreeMap<String, u64> = BTreeMap::new();
    for changeset in index.changesets() {
        if changeset.entities.len() > opts.max_changeset_size {
            continue;
        }
        let size = count_as_u64(changeset.entities.len());
        if size == 0 {
            continue;
        }
        let add = size - 1;
        for entity in &changeset.entities {
            *scores.entry(entity.clone()).or_insert(0) += add;
        }
    }
    let mut rows: Vec<(String, u64)> = scores
        .into_iter()
        .filter(|(entity, _)| {
            let n = revs.get(entity).copied().unwrap_or(0);
            meets_min_revs(n, opts)
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "soc"]);
    for (entity, soc) in rows {
        table.push_row([entity, fmt_u64(soc)]);
    }
    table.limit(opts.rows)
}
