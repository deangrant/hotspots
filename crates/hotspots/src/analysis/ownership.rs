//! Line-based ownership and main-developer metrics.
//!
//! `main-dev` ranks by added lines and reports ownership as share of additions.
//! Entities with no additions are omitted; use `main-dev-by-revs` for delete-only.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{
    entity_revisions, fmt_pct, fmt_u64, meets_min_revs, percent, require_churn,
};
use crate::error::Result;
use crate::model::Change;
use crate::options::Options;

type EntityAuthorChurn = BTreeMap<String, BTreeMap<String, (u64, u64)>>;

/// Reports added and deleted lines per entity and author.
pub fn entity_ownership(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let revs = entity_revisions(changes);
    let map = collect_churn(changes);
    let mut table = Table::with_headers(["entity", "author", "added", "deleted"]);
    let mut rows = flatten_churn(map);
    rows.retain(|(entity, _, _, _)| meets_min_revs(revs.get(entity).copied().unwrap_or(0), opts));
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.cmp(&a.2)).then(a.1.cmp(&b.1)));
    for (entity, author, added, deleted) in rows {
        table.push_row([entity, author, fmt_u64(added), fmt_u64(deleted)]);
    }
    Ok(table.limit(opts.rows))
}

/// Identifies the main developer by added lines for each entity.
pub fn main_dev(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let rows = main_dev_rows(changes, opts);
    Ok(build_main_dev_table(rows, opts.rows))
}

fn main_dev_rows(changes: &[Change], opts: &Options) -> Vec<(String, String, u64, f64)> {
    let revs = entity_revisions(changes);
    let map = collect_churn(changes);
    let mut rows = Vec::new();
    for (entity, authors) in map {
        if let Some(row) = main_dev_row(entity, authors, &revs, opts) {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

fn main_dev_row(
    entity: String,
    authors: BTreeMap<String, (u64, u64)>,
    revs: &BTreeMap<String, u64>,
    opts: &Options,
) -> Option<(String, String, u64, f64)> {
    if !meets_min_revs(revs.get(&entity).copied().unwrap_or(0), opts) {
        return None;
    }
    let total_added: u64 = authors.values().map(|(a, _)| *a).sum();
    if total_added == 0 {
        return None;
    }
    let (author, added) = leading_author_by_added(authors)?;
    Some((entity, author, added, percent(added, total_added)))
}

fn build_main_dev_table(rows: Vec<(String, String, u64, f64)>, limit: Option<usize>) -> Table {
    let mut table = Table::with_headers(["entity", "main-dev", "added", "ownership"]);
    for (entity, author, added, ownership) in rows {
        table.push_row([entity, author, fmt_u64(added), fmt_pct(ownership)]);
    }
    table.limit(limit)
}

fn collect_churn(changes: &[Change]) -> EntityAuthorChurn {
    let mut map: EntityAuthorChurn = BTreeMap::new();
    for change in changes {
        let entry = map
            .entry(change.entity.clone())
            .or_default()
            .entry(change.author.clone())
            .or_insert((0, 0));
        entry.0 += change.added.unwrap_or(0);
        entry.1 += change.deleted.unwrap_or(0);
    }
    map
}

fn leading_author_by_added(authors: BTreeMap<String, (u64, u64)>) -> Option<(String, u64)> {
    authors
        .into_iter()
        .map(|(author, (added, _))| (author, added))
        .max_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)))
}

fn flatten_churn(map: EntityAuthorChurn) -> Vec<(String, String, u64, u64)> {
    let mut rows = Vec::new();
    for (entity, authors) in map {
        for (author, (added, deleted)) in authors {
            rows.push((entity.clone(), author, added, deleted));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_author_empty_and_tie_break() {
        assert!(leading_author_by_added(BTreeMap::new()).is_none());
        let mut authors = BTreeMap::new();
        authors.insert(String::from("Bea"), (5, 0));
        authors.insert(String::from("Ada"), (5, 1));
        // Equal added counts: lexicographically larger author wins.
        assert_eq!(
            leading_author_by_added(authors),
            Some((String::from("Bea"), 5))
        );
    }

    #[test]
    fn ownership_respects_min_revs() {
        let changes = [
            Change::new("1", "Ada", "2024-01-01", "a.rs", Some(3), Some(0)),
            Change::new("2", "Ada", "2024-01-02", "a.rs", Some(1), Some(0)),
            Change::new("3", "Ada", "2024-01-03", "b.rs", Some(9), Some(0)),
        ];
        let opts = Options {
            min_revs: 2,
            ..Options::default()
        };
        let table = entity_ownership(&changes, &opts);
        assert!(table.is_ok_and(|t| t.rows.iter().all(|row| row[0] == "a.rs")));
        let mains = main_dev(&changes, &opts);
        assert!(mains.is_ok_and(|t| t.rows.len() == 1 && t.rows[0][0] == "a.rs"));
    }

    #[test]
    fn main_dev_omits_delete_only_and_ranks_by_added() {
        let changes = [
            Change::new("1", "Ada", "2024-01-01", "gone.rs", Some(0), Some(10)),
            Change::new("2", "Bea", "2024-01-01", "mixed.rs", Some(1), Some(50)),
            Change::new("3", "Ada", "2024-01-02", "mixed.rs", Some(5), Some(0)),
        ];
        let opts = Options {
            min_revs: 1,
            ..Options::default()
        };
        let mains = main_dev(&changes, &opts);
        assert!(mains.is_ok_and(|t| {
            t.rows.len() == 1
                && t.rows[0][0] == "mixed.rs"
                && t.rows[0][1] == "Ada"
                && t.rows[0][2] == "5"
        }));
    }
}
