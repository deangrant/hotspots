//! Author communication via shared entities.

use std::collections::BTreeMap;

use crate::analysis::table::Table;
use crate::analysis::util::{entity_authors, fmt_u64, ordered_pair};
use crate::model::Change;
use crate::options::Options;

/// Counts how many entities pairs of authors both touched.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let authors_by_entity = entity_authors(changes);
    let mut shared: BTreeMap<(String, String), u64> = BTreeMap::new();
    for authors in authors_by_entity.values() {
        let list: Vec<&String> = authors.iter().collect();
        for i in 0..list.len() {
            for j in (i + 1)..list.len() {
                let pair = ordered_pair(list[i], list[j]);
                *shared.entry(pair).or_insert(0) += 1;
            }
        }
    }
    let mut rows: Vec<(String, String, u64)> =
        shared.into_iter().map(|((a, b), count)| (a, b, count)).collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let mut table = Table::with_headers(["author", "peer", "shared"]);
    for (author, peer, count) in rows {
        table.push_row([author, peer, fmt_u64(count)]);
    }
    table.limit(opts.rows)
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
}
