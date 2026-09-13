//! Entity age relative to a reference date.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{date_ordinal, entity_revisions, fmt_u64, meets_min_revs};
use crate::error::{Error, Result};
use crate::model::Change;
use crate::options::Options;

/// Reports days since each entity's most recent change.
pub fn run(changes: &[Change], opts: &Options) -> Result<Table> {
    let latest = latest_dates(changes);
    let reference = reference_date(changes, opts)?;
    let ref_ord = date_ordinal(&reference)?;
    let rev_counts = entity_revisions(changes);
    let mut rows = Vec::new();
    for (entity, date) in latest {
        let n_revs = rev_counts.get(&entity).copied().unwrap_or(0);
        if !meets_min_revs(n_revs, opts) {
            continue;
        }
        let age = ref_ord - date_ordinal(&date)?;
        let age_days = u64::try_from(age.max(0)).unwrap_or(0);
        rows.push((entity, age_days));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "age-days"]);
    for (entity, age) in rows {
        table.push_row([entity, fmt_u64(age)]);
    }
    Ok(table.limit(opts.rows))
}

fn latest_dates(changes: &[Change]) -> BTreeMap<String, String> {
    let mut latest = BTreeMap::new();
    for change in changes {
        latest
            .entry(change.entity.clone())
            .and_modify(|current: &mut String| {
                if change.date.as_str() > current.as_str() {
                    current.clone_from(&change.date);
                }
            })
            .or_insert_with(|| change.date.clone());
    }
    latest
}

fn reference_date(changes: &[Change], opts: &Options) -> Result<String> {
    if let Some(date) = &opts.age_time_now {
        return Ok(date.clone());
    }
    changes
        .iter()
        .map(|c| c.date.as_str())
        .max()
        .map(str::to_owned)
        .ok_or_else(|| Error::msg("no dates available for age analysis"))
}
