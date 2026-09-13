//! Author communication via shared entities.

use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::table::Table;
use crate::analysis::util::{
    entity_authors, entity_revisions, fmt_u64, meets_min_revs, ordered_pair,
};
use crate::model::Change;
use crate::options::Options;

/// Counts how many entities pairs of authors both touched.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let shared = shared_pair_counts(changes, opts);
    build_communication_table(shared, opts.rows)
}

fn shared_pair_counts(changes: &[Change], opts: &Options) -> BTreeMap<(String, String), u64> {
    let revs = entity_revisions(changes);
    let authors_by_entity = entity_authors(changes);
    let mut shared = BTreeMap::new();
    for (entity, authors) in &authors_by_entity {
        if !meets_min_revs(revs.get(entity).copied().unwrap_or(0), opts) {
            continue;
        }
        accumulate_author_pairs(authors, &mut shared);
    }
    shared
}

fn accumulate_author_pairs(
    authors: &BTreeSet<String>,
    shared: &mut BTreeMap<(String, String), u64>,
) {
    let list: Vec<&String> = authors.iter().collect();
    for i in 0..list.len() {
        for j in (i + 1)..list.len() {
            let pair = ordered_pair(list[i], list[j]);
            *shared.entry(pair).or_insert(0) += 1;
        }
    }
}

fn build_communication_table(
    shared: BTreeMap<(String, String), u64>,
    rows: Option<usize>,
) -> Table {
    let mut ranked: Vec<(String, String, u64)> =
        shared.into_iter().map(|((a, b), count)| (a, b, count)).collect();
    ranked.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let mut table = Table::with_headers(["author", "peer", "shared"]);
    for (author, peer, count) in ranked {
        table.push_row([author, peer, fmt_u64(count)]);
    }
    table.limit(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_authors_on_shared_entity() {
        let changes = vec![
            Change::new("1", "Ada", "2024-01-01", "a.rs", None, None),
            Change::new("2", "Bea", "2024-01-02", "a.rs", None, None),
            Change::new("3", "Ada", "2024-01-03", "b.rs", None, None),
            Change::new("4", "Cara", "2024-01-04", "a.rs", None, None),
        ];
        let opts = Options {
            min_revs: 1,
            rows: Some(1),
            ..Options::default()
        };
        let table = run(&changes, &opts);
        assert_eq!(table.rows.len(), 1);
    }

    #[test]
    fn solo_author_yields_no_pairs() {
        let changes = vec![Change::new("1", "Ada", "2024-01-01", "a.rs", None, None)];
        let opts = Options {
            min_revs: 1,
            ..Options::default()
        };
        assert!(run(&changes, &opts).rows.is_empty());
    }

    #[test]
    fn ignores_entities_below_min_revs() {
        let changes = vec![
            Change::new("1", "Ada", "2024-01-01", "hot.rs", None, None),
            Change::new("2", "Bea", "2024-01-02", "hot.rs", None, None),
            Change::new("3", "Ada", "2024-01-03", "cold.rs", None, None),
            Change::new("4", "Bea", "2024-01-04", "cold.rs", None, None),
            Change::new("5", "Ada", "2024-01-05", "hot.rs", None, None),
        ];
        let opts = Options {
            min_revs: 3,
            ..Options::default()
        };
        let table = run(&changes, &opts);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][2], "1");
    }
}
