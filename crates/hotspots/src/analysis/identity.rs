//! Identity dump of parsed change rows.

use crate::analysis::table::Table;
use crate::analysis::util::fmt_u64;
use crate::model::Change;
use crate::options::Options;

/// Emits one output row per parsed change for debugging.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let mut table = Table::with_headers(["entity", "author", "date", "rev", "added", "deleted"]);
    for change in changes {
        table.push_row([
            change.entity.clone(),
            change.author.clone(),
            change.date.clone(),
            change.rev.clone(),
            optional_count(change.added),
            optional_count(change.deleted),
        ]);
    }
    table.limit(opts.rows)
}

fn optional_count(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("-"), fmt_u64)
}
