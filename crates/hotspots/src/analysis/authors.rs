//! Authors-per-entity analysis.

use crate::analysis::table::Table;
use crate::analysis::util::{
    count_as_u64, entity_authors, entity_revisions, fmt_u64, meets_min_revs,
};
use crate::model::Change;
use crate::options::Options;

/// Reports distinct authors and revisions for each entity.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let revs = entity_revisions(changes);
    let authors = entity_authors(changes);
    let mut rows: Vec<(String, u64, u64)> = authors
        .into_iter()
        .filter_map(|(entity, authors)| {
            let n_revs = revs.get(&entity).copied().unwrap_or(0);
            if !meets_min_revs(n_revs, opts) {
                return None;
            }
            Some((entity, count_as_u64(authors.len()), n_revs))
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "n-authors", "n-revs"]);
    for (entity, n_authors, n_revs) in rows {
        table.push_row([entity, fmt_u64(n_authors), fmt_u64(n_revs)]);
    }
    table.limit(opts.rows)
}
