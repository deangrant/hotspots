//! Absolute, author, and entity churn metrics.

use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::table::Table;
use crate::analysis::util::{count_as_u64, fmt_u64, meets_min_revs, require_churn};
use crate::error::Result;
use crate::model::Change;
use crate::options::Options;

/// Totals added and deleted lines per calendar date.
pub fn abs_churn(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let mut by_date: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for change in changes {
        let entry = by_date.entry(change.date.clone()).or_insert((0, 0));
        entry.0 += change.added.unwrap_or(0);
        entry.1 += change.deleted.unwrap_or(0);
    }
    let mut table = Table::with_headers(["date", "added", "deleted"]);
    for (date, (added, deleted)) in by_date {
        table.push_row([date, fmt_u64(added), fmt_u64(deleted)]);
    }
    Ok(table.limit(opts.rows))
}

/// Totals churn by author.
pub fn author_churn(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let mut totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for change in changes {
        let entry = totals.entry(change.author.clone()).or_insert((0, 0));
        entry.0 += change.added.unwrap_or(0);
        entry.1 += change.deleted.unwrap_or(0);
    }
    let mut rows: Vec<(String, u64, u64)> = totals
        .into_iter()
        .map(|(author, (added, deleted))| (author, added, deleted))
        .collect();
    rows.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["author", "added", "deleted"]);
    for (author, added, deleted) in rows {
        table.push_row([author, fmt_u64(added), fmt_u64(deleted)]);
    }
    Ok(table.limit(opts.rows))
}

/// Totals churn and revisions by entity.
pub fn entity_churn(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let mut totals: BTreeMap<String, (u64, u64, BTreeSet<String>)> = BTreeMap::new();
    for change in changes {
        let entry = totals.entry(change.entity.clone()).or_insert_with(|| (0, 0, BTreeSet::new()));
        entry.0 += change.added.unwrap_or(0);
        entry.1 += change.deleted.unwrap_or(0);
        entry.2.insert(change.rev.clone());
    }
    let mut rows: Vec<(String, u64, u64, u64)> = totals
        .into_iter()
        .filter_map(|(entity, (added, deleted, revs))| {
            let n_revs = count_as_u64(revs.len());
            if !meets_min_revs(n_revs, opts) {
                return None;
            }
            Some((entity, added, deleted, n_revs))
        })
        .collect();
    rows.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "added", "deleted", "n-revs"]);
    for (entity, added, deleted, n_revs) in rows {
        table.push_row([entity, fmt_u64(added), fmt_u64(deleted), fmt_u64(n_revs)]);
    }
    Ok(table.limit(opts.rows))
}
