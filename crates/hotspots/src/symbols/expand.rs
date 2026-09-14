//! Expand file-level changes into symbol-level changes using hunk overlap.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::model::Change;
use crate::symbols::types::{ExpandStats, FileDiff, Hunk, SymbolFact, entity_id};

/// Expands each change using the matching [`FileDiff`] keyed by `(rev, path)`.
///
/// Rust paths whose hunks are empty or overlap no symbols are dropped and
/// counted in [`ExpandStats::dropped_no_overlap`].
///
/// # Errors
///
/// Returns an error when a Rust path lacks a matching diff.
pub fn expand_with_diffs(
    changes: &[Change],
    diffs: &BTreeMap<(String, String), FileDiff>,
) -> Result<(Vec<Change>, ExpandStats)> {
    let mut out = Vec::new();
    let mut stats = ExpandStats::default();
    for change in changes {
        expand_one_change(change, diffs, &mut out, &mut stats)?;
    }
    Ok((out, stats))
}

fn expand_one_change(
    change: &Change,
    diffs: &BTreeMap<(String, String), FileDiff>,
    out: &mut Vec<Change>,
    stats: &mut ExpandStats,
) -> Result<()> {
    if !crate::symbols::is_rust_path(&change.entity) {
        stats.dropped_non_rust = stats.dropped_non_rust.saturating_add(1);
        return Ok(());
    }
    let diff = require_diff(change, diffs)?;
    let expanded = expand_file_change(change, diff);
    if expanded.is_empty() {
        stats.dropped_no_overlap = stats.dropped_no_overlap.saturating_add(1);
    }
    out.extend(expanded);
    Ok(())
}

fn require_diff<'a>(
    change: &Change,
    diffs: &'a BTreeMap<(String, String), FileDiff>,
) -> Result<&'a FileDiff> {
    let key = (change.rev.clone(), change.entity.clone());
    diffs.get(&key).ok_or_else(|| {
        Error::msg(format!(
            "missing symbol diff for `{}` at `{}`",
            change.entity, change.rev
        ))
    })
}

/// Expands one file-level change into zero or more symbol-level changes.
///
/// Returns an empty list when hunks exist but no symbol receives churn.
#[must_use]
pub fn expand_file_change(change: &Change, diff: &FileDiff) -> Vec<Change> {
    if diff.hunks.is_empty() {
        return Vec::new();
    }
    let mut totals: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for hunk in &diff.hunks {
        attribute_hunk(hunk, &diff.symbols_new, &diff.symbols_old, &mut totals);
    }
    totals
        .into_iter()
        .map(|(name, (added, deleted))| {
            Change::new(
                change.rev.clone(),
                change.author.clone(),
                change.date.clone(),
                entity_id(&change.entity, &name),
                Some(added),
                Some(deleted),
            )
        })
        .collect()
}

fn attribute_hunk(
    hunk: &Hunk,
    symbols_new: &[SymbolFact],
    symbols_old: &[SymbolFact],
    totals: &mut BTreeMap<String, (u64, u64)>,
) {
    let added_lines = line_span(hunk.new_start, hunk.new_count);
    let deleted_lines = line_span(hunk.old_start, hunk.old_count);
    add_overlap_counts(symbols_new, &added_lines, true, totals);
    add_overlap_counts(symbols_old, &deleted_lines, false, totals);
}

fn line_span(start: u32, count: u32) -> Vec<u32> {
    if start == 0 || count == 0 {
        return Vec::new();
    }
    (start..start.saturating_add(count)).collect()
}

fn add_overlap_counts(
    symbols: &[SymbolFact],
    lines: &[u32],
    is_added: bool,
    totals: &mut BTreeMap<String, (u64, u64)>,
) {
    for &line in lines {
        if let Some(symbol) = covering_symbol(symbols, line) {
            let entry = totals.entry(symbol.name.clone()).or_insert((0, 0));
            if is_added {
                entry.0 = entry.0.saturating_add(1);
            } else {
                entry.1 = entry.1.saturating_add(1);
            }
        }
    }
}

fn covering_symbol(symbols: &[SymbolFact], line: u32) -> Option<&SymbolFact> {
    symbols
        .iter()
        .filter(|s| s.start_line <= line && line <= s.end_line)
        .min_by_key(|s| s.end_line.saturating_sub(s.start_line))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(name: &str, start: u32, end: u32) -> SymbolFact {
        SymbolFact {
            path: String::from("a.rs"),
            start_line: start,
            end_line: end,
            name: String::from(name),
        }
    }

    #[test]
    fn expands_overlapping_hunk_to_symbol() {
        let change = Change::new("abc", "Ada", "2024-01-01", "a.rs", Some(2), Some(0));
        let diff = FileDiff {
            rev: String::from("abc"),
            path: String::from("a.rs"),
            symbols_new: vec![fact("foo", 1, 10), fact("bar", 11, 20)],
            symbols_old: vec![],
            hunks: vec![Hunk {
                old_start: 0,
                old_count: 0,
                new_start: 2,
                new_count: 2,
            }],
        };
        let rows = expand_file_change(&change, &diff);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entity, "a.rs::foo");
        assert_eq!(rows[0].added, Some(2));
    }

    #[test]
    fn skips_when_hunks_miss_symbols() {
        let change = Change::new("abc", "Ada", "2024-01-01", "a.rs", Some(1), Some(0));
        let diff = FileDiff {
            rev: String::from("abc"),
            path: String::from("a.rs"),
            symbols_new: vec![fact("foo", 1, 5)],
            symbols_old: vec![],
            hunks: vec![Hunk {
                old_start: 0,
                old_count: 0,
                new_start: 40,
                new_count: 1,
            }],
        };
        let mut diffs = BTreeMap::new();
        diffs.insert((String::from("abc"), String::from("a.rs")), diff.clone());
        let expanded = expand_with_diffs(&[change], &diffs);
        assert!(
            expanded
                .is_ok_and(|(rows, stats)| { rows.is_empty() && stats.dropped_no_overlap == 1 })
        );
        assert!(
            expand_file_change(
                &Change::new("abc", "Ada", "2024-01-01", "a.rs", Some(1), Some(0)),
                &diff
            )
            .is_empty()
        );
    }

    #[test]
    fn drops_non_rust_in_batch() {
        let changes = vec![
            Change::new("1", "Ada", "2024-01-01", "a.rs", Some(1), Some(0)),
            Change::new("1", "Ada", "2024-01-01", "readme.md", Some(1), Some(0)),
        ];
        let mut diffs = BTreeMap::new();
        diffs.insert(
            (String::from("1"), String::from("a.rs")),
            FileDiff {
                rev: String::from("1"),
                path: String::from("a.rs"),
                symbols_new: vec![fact("foo", 1, 10)],
                symbols_old: vec![],
                hunks: vec![Hunk {
                    old_start: 0,
                    old_count: 0,
                    new_start: 1,
                    new_count: 1,
                }],
            },
        );
        let expanded = expand_with_diffs(&changes, &diffs);
        assert!(
            expanded
                .as_ref()
                .is_ok_and(|(rows, stats)| { stats.dropped_non_rust == 1 && rows.len() == 1 })
        );
    }

    #[test]
    fn missing_diff_errors() {
        let change = Change::new("1", "Ada", "2024-01-01", "a.rs", Some(1), Some(0));
        let expanded = expand_with_diffs(std::slice::from_ref(&change), &BTreeMap::new());
        assert!(expanded.is_err_and(|e| e.to_string().contains("missing symbol diff")));
    }

    #[test]
    fn empty_hunks_yield_no_rows() {
        let change = Change::new("1", "Ada", "2024-01-01", "a.rs", Some(1), Some(0));
        let diff = FileDiff {
            rev: String::from("1"),
            path: String::from("a.rs"),
            symbols_new: vec![fact("foo", 1, 10)],
            symbols_old: vec![],
            hunks: vec![],
        };
        assert!(expand_file_change(&change, &diff).is_empty());
        let mut diffs = BTreeMap::new();
        diffs.insert((String::from("1"), String::from("a.rs")), diff);
        assert!(
            expand_with_diffs(std::slice::from_ref(&change), &diffs)
                .is_ok_and(|(rows, stats)| rows.is_empty() && stats.dropped_no_overlap == 1)
        );
    }

    #[test]
    fn attributes_deletes_using_old_symbols() {
        let change = Change::new("abc", "Ada", "2024-01-01", "a.rs", Some(0), Some(2));
        let diff = FileDiff {
            rev: String::from("abc"),
            path: String::from("a.rs"),
            symbols_new: vec![fact("foo", 1, 10)],
            symbols_old: vec![fact("foo", 1, 10), fact("bar", 11, 20)],
            hunks: vec![Hunk {
                old_start: 12,
                old_count: 2,
                new_start: 0,
                new_count: 0,
            }],
        };
        let rows = expand_file_change(&change, &diff);
        assert!(rows.iter().any(|r| r.entity == "a.rs::bar" && r.deleted == Some(2)));
    }

    #[test]
    fn empty_old_symbols_do_not_attribute_deletes_to_new() {
        let change = Change::new("abc", "Ada", "2024-01-01", "a.rs", Some(0), Some(2));
        let diff = FileDiff {
            rev: String::from("abc"),
            path: String::from("a.rs"),
            symbols_new: vec![fact("foo", 1, 10)],
            symbols_old: vec![],
            hunks: vec![Hunk {
                old_start: 1,
                old_count: 2,
                new_start: 0,
                new_count: 0,
            }],
        };
        assert!(expand_file_change(&change, &diff).is_empty());
    }
}
