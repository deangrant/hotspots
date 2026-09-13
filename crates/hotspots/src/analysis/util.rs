//! Shared helpers for metric engines.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{Error, Result};
use crate::model::Change;
use crate::options::Options;

/// Converts a collection length to `u64`.
pub fn count_as_u64(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// Counts distinct revisions per entity.
pub fn entity_revisions(changes: &[Change]) -> BTreeMap<String, u64> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for change in changes {
        map.entry(change.entity.clone()).or_default().insert(change.rev.clone());
    }
    map.into_iter()
        .map(|(entity, revs)| (entity, count_as_u64(revs.len())))
        .collect()
}

/// Counts distinct authors per entity.
pub fn entity_authors(changes: &[Change]) -> BTreeMap<String, BTreeSet<String>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for change in changes {
        map.entry(change.entity.clone()).or_default().insert(change.author.clone());
    }
    map
}

/// Ensures every change carries line-churn fields.
///
/// # Errors
///
/// Returns an error when any row lacks numstat data.
pub fn require_churn(changes: &[Change]) -> Result<()> {
    if changes.iter().any(|c| c.added.is_none() || c.deleted.is_none()) {
        return Err(Error::msg(
            "analysis requires numstat line counts; use `-c git2` or a numstat log",
        ));
    }
    Ok(())
}

/// Drops entities below the configured revision threshold.
#[must_use]
pub const fn meets_min_revs(revs: u64, opts: &Options) -> bool {
    revs >= opts.min_revs
}

/// Parses `YYYY-MM-DD` into year, month, and day parts.
///
/// # Errors
///
/// Returns an error when the date is not `YYYY-MM-DD` with numeric parts.
pub fn parse_date(date: &str) -> Result<(i64, i64, i64)> {
    let mut parts = date.split('-');
    let year = parse_part(parts.next(), date)?;
    let month = parse_part(parts.next(), date)?;
    let day = parse_part(parts.next(), date)?;
    if parts.next().is_some() {
        return Err(Error::msg(format!("invalid date `{date}`")));
    }
    Ok((year, month, day))
}

fn parse_part(part: Option<&str>, date: &str) -> Result<i64> {
    let raw = part.ok_or_else(|| Error::msg(format!("invalid date `{date}`")))?;
    raw.parse::<i64>().map_err(|_| Error::msg(format!("invalid date `{date}`")))
}

/// Approximate day ordinal for age differences (proleptic Gregorian).
///
/// # Errors
///
/// Returns an error when the date string cannot be parsed.
pub fn date_ordinal(date: &str) -> Result<i64> {
    let (y, m, d) = parse_date(date)?;
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

/// Formats an integer as a decimal string.
pub fn fmt_u64(value: u64) -> String {
    value.to_string()
}

/// Formats a floating percentage with two fraction digits.
pub fn fmt_pct(value: f64) -> String {
    format!("{value:.2}")
}

/// Returns `(left, right)` ordered lexicographically for stable pair keys.
#[must_use]
pub fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_owned(), right.to_owned())
    } else {
        (right.to_owned(), left.to_owned())
    }
}

/// Computes `100 * numerator / denominator` for display percentages.
#[must_use]
pub fn percent(numerator: u64, denominator: u64) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_churn_rejects_missing_counts() {
        let missing = [Change::new("1", "Ada", "2024-01-01", "a.rs", None, Some(1))];
        assert!(require_churn(&missing).is_err());
        let ok = [Change::new(
            "1",
            "Ada",
            "2024-01-01",
            "a.rs",
            Some(1),
            Some(0),
        )];
        assert!(require_churn(&ok).is_ok());
    }

    #[test]
    fn parse_date_and_ordinal_edges() {
        assert!(parse_date("2024-01-02").is_ok());
        assert!(parse_date("bad").is_err());
        assert!(parse_date("2024").is_err());
        assert!(parse_date("2024-01-02-extra").is_err());
        assert!(parse_date("2024-xx-01").is_err());
        assert!(date_ordinal("2024-01-02").is_ok());
        assert!(date_ordinal("2024-03-01").is_ok());
    }

    #[test]
    fn percent_and_fmt_helpers() {
        assert!((percent(0, 0) - 0.0).abs() < f64::EPSILON);
        assert!(percent(1, 2) > 0.0);
        assert_eq!(fmt_u64(9), "9");
        assert_eq!(fmt_pct(12.345), "12.35");
    }

    #[test]
    fn ordered_pair_and_thresholds() {
        assert_eq!(ordered_pair("b", "a").0, "a");
        assert_eq!(ordered_pair("a", "b").0, "a");
        assert_eq!(count_as_u64(3), 3);
        let opts = Options {
            min_revs: 2,
            ..Options::default()
        };
        assert!(!meets_min_revs(1, &opts));
        assert!(meets_min_revs(2, &opts));
    }

    #[test]
    fn entity_maps_count_distinct() {
        let changes = [
            Change::new("1", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("2", "Ada", "2024-01-02", "a.rs", None, None),
            Change::new("3", "Bea", "2024-01-03", "a.rs", None, None),
        ];
        assert_eq!(entity_revisions(&changes).get("a.rs").copied(), Some(3));
        assert_eq!(
            entity_authors(&changes).get("a.rs").map(BTreeSet::len),
            Some(2)
        );
    }
}
