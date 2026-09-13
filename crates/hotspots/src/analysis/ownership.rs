//! Line-based ownership and main-developer metrics.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{fmt_pct, fmt_u64, require_churn};
use crate::error::Result;
use crate::model::Change;
use crate::options::Options;

type EntityAuthorChurn = BTreeMap<String, BTreeMap<String, (u64, u64)>>;

/// Reports added and deleted lines per entity and author.
pub fn entity_ownership(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let map = collect_churn(changes);
    let mut table = Table::with_headers(["entity", "author", "added", "deleted"]);
    let mut rows = flatten_churn(map);
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.cmp(&a.2)).then(a.1.cmp(&b.1)));
    for (entity, author, added, deleted) in rows {
        table.push_row([entity, author, fmt_u64(added), fmt_u64(deleted)]);
    }
    Ok(table.limit(opts.rows))
}

/// Identifies the main developer by added lines for each entity.
pub fn main_dev(changes: &[Change], opts: &Options) -> Result<Table> {
    require_churn(changes)?;
    let map = collect_churn(changes);
    let mut rows = Vec::new();
    for (entity, authors) in map {
        let total_added: u64 = authors.values().map(|(a, _)| *a).sum();
        let Some((author, added, _)) = authors
            .into_iter()
            .map(|(author, (added, deleted))| (author, added, deleted))
            .max_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)))
        else {
            continue;
        };
        let ownership = percent(added, total_added);
        rows.push((entity, author, added, ownership));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut table = Table::with_headers(["entity", "main-dev", "added", "ownership"]);
    for (entity, author, added, ownership) in rows {
        table.push_row([entity, author, fmt_u64(added), fmt_pct(ownership)]);
    }
    Ok(table.limit(opts.rows))
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

fn flatten_churn(map: EntityAuthorChurn) -> Vec<(String, String, u64, u64)> {
    let mut rows = Vec::new();
    for (entity, authors) in map {
        for (author, (added, deleted)) in authors {
            rows.push((entity.clone(), author, added, deleted));
        }
    }
    rows
}

fn percent(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "display-only ownership percentage"
    )]
    {
        (numerator as f64) * 100.0 / (denominator as f64)
    }
}
