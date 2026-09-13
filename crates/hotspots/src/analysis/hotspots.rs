//! History hotspots ranked by revisions and optional churn.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{entity_revisions, fmt_u64, meets_min_revs};
use crate::model::Change;
use crate::options::Options;

/// Ranks entities by revision activity and total line churn when present.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let revs = entity_revisions(changes);
    let churn = entity_churn_totals(changes);
    let mut rows: Vec<(String, u64, u64)> = revs
        .into_iter()
        .filter(|(_, n)| meets_min_revs(*n, opts))
        .map(|(entity, n_revs)| {
            let total = churn.get(&entity).copied().unwrap_or(0);
            (entity, n_revs, total)
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "n-revs", "churn"]);
    for (entity, n_revs, churn) in rows {
        table.push_row([entity, fmt_u64(n_revs), fmt_u64(churn)]);
    }
    table.limit(opts.rows)
}

fn entity_churn_totals(changes: &[Change]) -> BTreeMap<String, u64> {
    let mut map = BTreeMap::new();
    for change in changes {
        let added = change.added.unwrap_or(0);
        let deleted = change.deleted.unwrap_or(0);
        *map.entry(change.entity.clone()).or_insert(0) += added + deleted;
    }
    map
}
