//! Entities ranked by revision count.

use crate::analysis::table::Table;
use crate::analysis::util::{entity_revisions, fmt_u64, meets_min_revs};
use crate::model::Change;
use crate::options::Options;

/// Lists entities ordered by number of revisions.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let mut rows: Vec<(String, u64)> = entity_revisions(changes)
        .into_iter()
        .filter(|(_, revs)| meets_min_revs(*revs, opts))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "n-revs"]);
    for (entity, n_revs) in rows {
        table.push_row([entity, fmt_u64(n_revs)]);
    }
    table.limit(opts.rows)
}
