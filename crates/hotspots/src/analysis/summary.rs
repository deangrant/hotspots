//! Summary statistics over a change stream.

use std::collections::BTreeSet;

use crate::analysis::table::Table;
use crate::analysis::util::{count_as_u64, fmt_u64};
use crate::model::Change;
use crate::options::Options;

/// Counts commits, entities, change rows, and authors.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let mut revs = BTreeSet::new();
    let mut entities = BTreeSet::new();
    let mut authors = BTreeSet::new();
    for change in changes {
        revs.insert(change.rev.as_str());
        entities.insert(change.entity.as_str());
        authors.insert(change.author.as_str());
    }
    let mut table = Table::with_headers(["statistic", "value"]);
    push_stat(&mut table, "number-of-commits", count_as_u64(revs.len()));
    push_stat(
        &mut table,
        "number-of-entities",
        count_as_u64(entities.len()),
    );
    push_stat(
        &mut table,
        "number-of-entities-changed",
        count_as_u64(changes.len()),
    );
    push_stat(&mut table, "number-of-authors", count_as_u64(authors.len()));
    table.limit(opts.rows)
}

fn push_stat(table: &mut Table, name: &str, value: u64) {
    table.push_row([String::from(name), fmt_u64(value)]);
}
