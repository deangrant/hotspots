//! Entity age relative to a reference date.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{
    build_entity_u64_table, date_ordinal, entity_revisions, meets_min_revs,
};
use crate::error::{Error, Result};
use crate::model::Change;
use crate::options::Options;

/// Reports days since each entity's most recent change.
pub fn run(changes: &[Change], opts: &Options) -> Result<Table> {
    let latest = latest_dates(changes);
    let reference = reference_date(changes, opts)?;
    let ref_ord = date_ordinal(&reference)?;
    let rev_counts = entity_revisions(changes);
    let rows = collect_age_rows(latest, ref_ord, &rev_counts, opts)?;
    Ok(build_entity_u64_table(
        ["entity", "age-days"],
        rows,
        opts.rows,
    ))
}

fn collect_age_rows(
    latest: BTreeMap<String, String>,
    ref_ord: i64,
    rev_counts: &BTreeMap<String, u64>,
    opts: &Options,
) -> Result<Vec<(String, u64)>> {
    let mut rows = Vec::new();
    for (entity, date) in latest {
        if let Some(row) = age_row(entity, &date, ref_ord, rev_counts, opts)? {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(rows)
}

fn age_row(
    entity: String,
    date: &str,
    ref_ord: i64,
    rev_counts: &BTreeMap<String, u64>,
    opts: &Options,
) -> Result<Option<(String, u64)>> {
    let n_revs = rev_counts.get(&entity).copied().unwrap_or(0);
    if !meets_min_revs(n_revs, opts) {
        return Ok(None);
    }
    let age = ref_ord - date_ordinal(date)?;
    let age_days = u64::try_from(age.max(0)).unwrap_or(0);
    Ok(Some((entity, age_days)))
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
