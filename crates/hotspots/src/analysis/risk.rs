//! Composite maintenance-risk score per entity.

use std::collections::BTreeMap;

use crate::analysis::effort;
use crate::analysis::hotspots;
use crate::analysis::soc;
use crate::analysis::table::Table;
use crate::analysis::util::{cmp_f64_desc, entity_revisions, fmt_pct, fmt_u64, meets_min_revs};
use crate::model::Change;
use crate::options::Options;

const W_REVS: f64 = 0.35;
const W_CHURN: f64 = 0.25;
const W_SOC: f64 = 0.20;
const W_FRAG: f64 = 0.20;

#[derive(Clone, Copy)]
struct Weights {
    revs: f64,
    churn: f64,
    soc: f64,
    frag: f64,
}

#[derive(Clone, Copy)]
struct Components {
    revs: u64,
    churn: u64,
    soc: u64,
    frag: f64,
}

/// Ranks entities by a 0–100 weighted maintenance-risk score.
pub fn run(changes: &[Change], opts: &Options) -> Table {
    let revs = entity_revisions(changes);
    let entities = filtered_entities(&revs, opts);
    if entities.is_empty() {
        return empty_table(opts.rows);
    }
    let has_churn = changes.iter().any(|c| c.added.is_some() && c.deleted.is_some());
    let weights = weights_for(has_churn);
    let churn = hotspots::churn_totals(changes);
    let soc_scores = soc::scores_by_entity(changes, opts);
    let frag_scores = effort::fragmentation_by_entity(changes);
    let maxima = maxima_for(&entities, &revs, &churn, &soc_scores, &frag_scores);
    let mut rows = score_rows(
        &entities,
        &revs,
        &churn,
        &soc_scores,
        &frag_scores,
        &maxima,
        &weights,
    );
    rows.sort_by(|a, b| cmp_f64_desc(a.1, b.1).then(a.0.cmp(&b.0)));
    build_table(rows, opts.rows)
}

fn filtered_entities(revs: &BTreeMap<String, u64>, opts: &Options) -> Vec<String> {
    revs.iter()
        .filter(|(_, n)| meets_min_revs(**n, opts))
        .map(|(entity, _)| entity.clone())
        .collect()
}

fn empty_table(limit: Option<usize>) -> Table {
    Table::with_headers(["entity", "risk", "revs", "churn", "soc", "fragmentation"]).limit(limit)
}

fn weights_for(has_churn: bool) -> Weights {
    if has_churn {
        return Weights {
            revs: W_REVS,
            churn: W_CHURN,
            soc: W_SOC,
            frag: W_FRAG,
        };
    }
    let scale = 1.0 / (W_REVS + W_SOC + W_FRAG);
    Weights {
        revs: W_REVS * scale,
        churn: 0.0,
        soc: W_SOC * scale,
        frag: W_FRAG * scale,
    }
}

fn maxima_for(
    entities: &[String],
    revs: &BTreeMap<String, u64>,
    churn: &BTreeMap<String, u64>,
    soc_scores: &BTreeMap<String, u64>,
    frag_scores: &BTreeMap<String, f64>,
) -> Components {
    let mut max_revs = 0_u64;
    let mut max_churn = 0_u64;
    let mut max_soc = 0_u64;
    let mut max_frag = 0.0_f64;
    for entity in entities {
        max_revs = max_revs.max(revs.get(entity).copied().unwrap_or(0));
        max_churn = max_churn.max(churn.get(entity).copied().unwrap_or(0));
        max_soc = max_soc.max(soc_scores.get(entity).copied().unwrap_or(0));
        max_frag = max_frag.max(frag_scores.get(entity).copied().unwrap_or(0.0));
    }
    Components {
        revs: max_revs,
        churn: max_churn,
        soc: max_soc,
        frag: max_frag,
    }
}

fn score_rows(
    entities: &[String],
    revs: &BTreeMap<String, u64>,
    churn: &BTreeMap<String, u64>,
    soc_scores: &BTreeMap<String, u64>,
    frag_scores: &BTreeMap<String, f64>,
    maxima: &Components,
    weights: &Weights,
) -> Vec<(String, f64, Components)> {
    entities
        .iter()
        .map(|entity| {
            let parts = Components {
                revs: revs.get(entity).copied().unwrap_or(0),
                churn: churn.get(entity).copied().unwrap_or(0),
                soc: soc_scores.get(entity).copied().unwrap_or(0),
                frag: frag_scores.get(entity).copied().unwrap_or(0.0),
            };
            let risk = weighted_risk(&parts, maxima, weights);
            (entity.clone(), risk, parts)
        })
        .collect()
}

fn weighted_risk(parts: &Components, maxima: &Components, weights: &Weights) -> f64 {
    let n_revs = norm_u64(parts.revs, maxima.revs);
    let n_churn = norm_u64(parts.churn, maxima.churn);
    let n_soc = norm_u64(parts.soc, maxima.soc);
    let n_frag = norm_f64(parts.frag, maxima.frag);
    let weighted = weights.revs.mul_add(
        n_revs,
        weights
            .churn
            .mul_add(n_churn, weights.soc.mul_add(n_soc, weights.frag * n_frag)),
    );
    100.0 * weighted
}

fn norm_u64(value: u64, max: u64) -> f64 {
    if max == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "display-only risk normalization within one log"
    )]
    {
        (value as f64) / (max as f64)
    }
}

fn norm_f64(value: f64, max: f64) -> f64 {
    if max <= 0.0 {
        return 0.0;
    }
    value / max
}

fn build_table(rows: Vec<(String, f64, Components)>, limit: Option<usize>) -> Table {
    let mut table =
        Table::with_headers(["entity", "risk", "revs", "churn", "soc", "fragmentation"]);
    for (entity, risk, parts) in rows {
        table.push_row([
            entity,
            fmt_pct(risk),
            fmt_u64(parts.revs),
            fmt_u64(parts.churn),
            fmt_u64(parts.soc),
            fmt_pct(parts.frag),
        ]);
    }
    table.limit(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(min_revs: u64) -> Options {
        Options {
            min_revs,
            ..Options::default()
        }
    }

    fn change(
        rev: &str,
        author: &str,
        entity: &str,
        added: Option<u64>,
        deleted: Option<u64>,
    ) -> Change {
        Change::new(rev, author, "2024-01-01", entity, added, deleted)
    }

    #[test]
    fn empty_input_yields_empty_table() {
        let table = run(&[], &opts(1));
        assert!(table.rows.is_empty());
        assert_eq!(table.headers[1], "risk");
    }

    #[test]
    fn min_revs_filters_entities() {
        let changes = vec![
            change("1", "Ada", "a.rs", Some(1), Some(0)),
            change("2", "Ada", "a.rs", Some(1), Some(0)),
            change("3", "Ada", "b.rs", Some(1), Some(0)),
        ];
        let table = run(&changes, &opts(2));
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0][0], "a.rs");
    }

    #[test]
    fn ranks_higher_activity_first() {
        let changes = vec![
            change("1", "Ada", "hot.rs", Some(10), Some(0)),
            change("2", "Ada", "hot.rs", Some(10), Some(0)),
            change("3", "Bea", "hot.rs", Some(10), Some(0)),
            change("1", "Ada", "cold.rs", Some(1), Some(0)),
            change("1", "Ada", "other.rs", Some(1), Some(0)),
        ];
        let table = run(&changes, &opts(1));
        assert_eq!(table.rows[0][0], "hot.rs");
        let hot: f64 = table.rows[0][1].parse().unwrap_or(0.0);
        let cold: f64 = table.rows[1][1].parse().unwrap_or(0.0);
        assert!(hot >= cold);
    }

    #[test]
    fn renormalizes_when_churn_absent() {
        let changes = vec![
            change("1", "Ada", "a.rs", None, None),
            change("2", "Bea", "a.rs", None, None),
            change("1", "Ada", "b.rs", None, None),
        ];
        let table = run(&changes, &opts(1));
        assert!(!table.rows.is_empty());
        assert_eq!(
            table.rows.iter().find(|r| r[0] == "a.rs").map(|r| r[3].as_str()),
            Some("0")
        );
        let weights = weights_for(false);
        assert!((weights.churn - 0.0).abs() < f64::EPSILON);
        assert!((weights.revs + weights.soc + weights.frag - 1.0).abs() < 1e-9);
    }

    #[test]
    fn normalize_helpers_cover_zero_max() {
        assert!((norm_u64(3, 0) - 0.0).abs() < f64::EPSILON);
        assert!((norm_u64(3, 6) - 0.5).abs() < f64::EPSILON);
        assert!((norm_f64(1.0, 0.0) - 0.0).abs() < f64::EPSILON);
        assert!((norm_f64(1.0, 2.0) - 0.5).abs() < f64::EPSILON);
    }
}
