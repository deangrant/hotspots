//! Commit-based effort, main-dev-by-revs, and fragmentation.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::table::Table;
use crate::analysis::util::{count_as_u64, fmt_pct, fmt_u64, meets_min_revs};
use crate::model::Change;
use crate::options::Options;

type EntityAuthorRevs = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

/// Reports author revisions versus total revisions per entity.
pub fn entity_effort(changes: &[Change], opts: &Options) -> Table {
    let map = collect_revs(changes);
    let mut rows = Vec::new();
    for (entity, authors) in &map {
        let total = total_revs(authors);
        if !meets_min_revs(total, opts) {
            continue;
        }
        for (author, revs) in authors {
            rows.push((
                entity.clone(),
                author.clone(),
                count_as_u64(revs.len()),
                total,
            ));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(b.2.cmp(&a.2)).then(a.1.cmp(&b.1)));
    let mut table = Table::with_headers(["entity", "author", "author-revs", "total-revs"]);
    for (entity, author, author_revs, total) in rows {
        table.push_row([entity, author, fmt_u64(author_revs), fmt_u64(total)]);
    }
    table.limit(opts.rows)
}

/// Identifies the main developer by revision count for each entity.
pub fn main_dev_by_revs(changes: &[Change], opts: &Options) -> Table {
    let map = collect_revs(changes);
    let mut rows = Vec::new();
    for (entity, authors) in map {
        let total = total_revs(&authors);
        if !meets_min_revs(total, opts) {
            continue;
        }
        let Some((author, author_revs)) = authors
            .into_iter()
            .map(|(author, revs)| (author, count_as_u64(revs.len())))
            .max_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)))
        else {
            continue;
        };
        let ownership = percent(author_revs, total);
        rows.push((entity, author, author_revs, ownership));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut table = Table::with_headers(["entity", "main-dev", "author-revs", "ownership"]);
    for (entity, author, author_revs, ownership) in rows {
        table.push_row([entity, author, fmt_u64(author_revs), fmt_pct(ownership)]);
    }
    table.limit(opts.rows)
}

/// Measures ownership fragmentation as `1 - sum(share^2)`.
pub fn fragmentation(changes: &[Change], opts: &Options) -> Table {
    let map = collect_revs(changes);
    let mut rows = Vec::new();
    for (entity, authors) in map {
        let total = total_revs(&authors);
        if !meets_min_revs(total, opts) {
            continue;
        }
        let score = fragmentation_score(&authors, total);
        rows.push((entity, score));
    }
    rows.sort_by(|a, b| cmp_f64_desc(a.1, b.1).then(a.0.cmp(&b.0)));
    let mut table = Table::with_headers(["entity", "fragmentation"]);
    for (entity, score) in rows {
        table.push_row([entity, fmt_pct(score)]);
    }
    table.limit(opts.rows)
}

fn collect_revs(changes: &[Change]) -> EntityAuthorRevs {
    let mut map: EntityAuthorRevs = BTreeMap::new();
    for change in changes {
        map.entry(change.entity.clone())
            .or_default()
            .entry(change.author.clone())
            .or_default()
            .insert(change.rev.clone());
    }
    map
}

fn total_revs(authors: &BTreeMap<String, BTreeSet<String>>) -> u64 {
    let mut all = BTreeSet::new();
    for revs in authors.values() {
        all.extend(revs.iter().cloned());
    }
    count_as_u64(all.len())
}

fn fragmentation_score(authors: &BTreeMap<String, BTreeSet<String>>, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let mut sum_sq = 0.0;
    for revs in authors.values() {
        let share = percent(count_as_u64(revs.len()), total) / 100.0;
        sum_sq += share * share;
    }
    1.0 - sum_sq
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

fn cmp_f64_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).map_or(Ordering::Equal, |order| order)
}
