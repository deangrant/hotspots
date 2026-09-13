//! Broad unit coverage for analyses, pipeline, and parsers.

use std::io::Cursor;

use crate::analysis::{self, Table};
use crate::index::ChangesetIndex;
use crate::model::Change;
use crate::options::{Options, TemporalPeriod};
use crate::parse::{GitLegacyParser, GitNumstatParser, VcsParser, parser_for};
use crate::pipeline::analyze_log;
use crate::write_table;

fn opts() -> Options {
    Options {
        min_revs: 1,
        min_shared_revs: 1,
        min_coupling: 1,
        ..Options::default()
    }
}

fn change(
    rev: &str,
    author: &str,
    date: &str,
    entity: &str,
    added: Option<u64>,
    deleted: Option<u64>,
) -> Change {
    Change::new(rev, author, date, entity, added, deleted)
}

fn sample_changes() -> Vec<Change> {
    vec![
        change("1", "Ada", "2024-01-01", "a.rs", Some(3), Some(1)),
        change("1", "Ada", "2024-01-01", "b.rs", Some(2), Some(0)),
        change("2", "Bea", "2024-01-02", "a.rs", Some(1), Some(1)),
        change("3", "Ada", "2024-01-03", "a.rs", Some(4), Some(0)),
        change("3", "Ada", "2024-01-03", "b.rs", Some(1), Some(0)),
    ]
}

#[test]
fn all_named_analyses_run() {
    let changes = sample_changes();
    let o = opts();
    for name in analysis::analysis_names() {
        let result = analysis::run(name, &changes, &o);
        assert!(result.is_ok(), "{name}: {:?}", result.err());
    }
    assert!(analysis::run("nope", &changes, &o).is_err());
}

#[test]
fn churn_requires_numstat() {
    let changes = vec![change("1", "Ada", "2024-01-01", "a.rs", None, None)];
    let o = opts();
    assert!(analysis::run("abs-churn", &changes, &o).is_err());
    assert!(analysis::run("author-churn", &changes, &o).is_err());
    assert!(analysis::run("entity-churn", &changes, &o).is_err());
    assert!(analysis::run("entity-ownership", &changes, &o).is_err());
    assert!(analysis::run("main-dev", &changes, &o).is_err());
}

#[test]
fn age_with_and_without_reference() {
    let changes = sample_changes();
    let mut o = opts();
    assert!(analysis::run("age", &changes, &o).is_ok());
    o.age_time_now = Some(String::from("2024-02-01"));
    assert!(analysis::run("age", &changes, &o).is_ok());
    o.age_time_now = None;
    assert!(analysis::run("age", &[], &o).is_err());
}

#[test]
fn identity_and_hotspots_variants() {
    let mut o = opts();
    o.rows = Some(2);
    let with_churn = analysis::run("hotspots", &sample_changes(), &o);
    assert!(with_churn.is_ok());
    let no_churn = vec![change("1", "Ada", "2024-01-01", "a.rs", None, None)];
    assert!(analysis::run("hotspots", &no_churn, &o).is_ok());
    let identity = analysis::run("identity", &no_churn, &o);
    assert!(identity.is_ok());
    assert_eq!(identity.unwrap_or_default().rows[0][4], "-");
}

#[test]
fn summary_authors_revisions() {
    let o = opts();
    let changes = sample_changes();
    assert!(analysis::run("summary", &changes, &o).is_ok());
    assert!(analysis::run("authors", &changes, &o).is_ok());
    assert!(analysis::run("revisions", &changes, &o).is_ok());
}

#[test]
fn effort_fragmentation_communication_soc() {
    let o = opts();
    let changes = sample_changes();
    assert!(analysis::run("entity-effort", &changes, &o).is_ok());
    assert!(analysis::run("main-dev-by-revs", &changes, &o).is_ok());
    assert!(analysis::run("fragmentation", &changes, &o).is_ok());
    assert!(analysis::run("communication", &changes, &o).is_ok());
    assert!(analysis::run("soc", &changes, &o).is_ok());
}

#[test]
fn parsers_select_and_reject() {
    assert!(parser_for("git").is_ok());
    assert!(parser_for("git2").is_ok());
    assert!(parser_for("svn").is_err());
    assert!(parser_for("hg").is_err());
    assert!(parser_for("p4").is_err());
    assert!(parser_for("tfs").is_err());
    assert!(parser_for("zzz").is_err());
}

#[test]
fn parse_errors_and_empty() {
    let orphan = GitNumstatParser.parse(&mut Cursor::new(b"1\t0\ta.rs\n"));
    assert!(orphan.is_err());
    let empty = GitNumstatParser.parse(&mut Cursor::new(b""));
    assert!(empty.is_ok());
    assert!(empty.unwrap_or_default().is_empty());
    let blank = GitNumstatParser.parse(&mut Cursor::new(b"\n\n"));
    assert!(blank.is_ok());
    assert!(GitNumstatParser.parse(&mut Cursor::new("--\n1\t0\ta.rs")).is_err());
    assert!(
        GitNumstatParser
            .parse(&mut Cursor::new("--abc--2024-01-01--Ada\nx\t0\ta.rs\n"))
            .is_err()
    );
    assert!(
        GitNumstatParser
            .parse(&mut Cursor::new("--abc--2024-01-01--Ada\n1\t2\n"))
            .is_err()
    );
}

#[test]
fn legacy_header_errors() {
    assert!(GitLegacyParser.parse(&mut Cursor::new("[rev missing date")).is_err());
    assert!(GitLegacyParser.parse(&mut Cursor::new("[] Ada 2024-01-01 x")).is_err());
    assert!(GitLegacyParser.parse(&mut Cursor::new("[abc] 2024-01-01 subject")).is_err());
    assert!(GitLegacyParser.parse(&mut Cursor::new("[abc missing close")).is_err());
}

#[test]
fn pipeline_with_and_without_group() {
    let log = "\
--a1--2024-01-01--Ada
2\t0\tsrc/a.rs
1\t0\tvendor/x.js
";
    let mut o = opts();
    o.exclude.push(String::from("vendor"));
    assert!(analyze_log(&mut Cursor::new(log.as_bytes()), "git2", &o).is_ok());

    let group =
        std::env::temp_dir().join(format!("hotspots-pipe-group-{}.txt", std::process::id()));
    assert!(std::fs::write(&group, "src => app\n").is_ok());
    o.group_file = Some(group.to_string_lossy().into_owned());
    o.analysis = String::from("summary");
    assert!(analyze_log(&mut Cursor::new(log.as_bytes()), "git2", &o).is_ok());
    let _ = std::fs::remove_file(group);
}

#[test]
fn index_periods_and_output_control() {
    let changes = sample_changes();
    assert!(!ChangesetIndex::build(&changes, TemporalPeriod::None).changesets().is_empty());
    assert!(!ChangesetIndex::build(&changes, TemporalPeriod::Day).changesets().is_empty());

    let mut table = Table::with_headers(["a"]);
    table.push_row(["\u{0001}"]);
    let mut buf = Vec::new();
    assert!(write_table(&mut buf, &table).is_ok());
    assert!(String::from_utf8(buf).unwrap_or_default().contains("\\u0001"));
}

#[test]
fn min_revs_filters_age_and_authors() {
    let changes = sample_changes();
    let mut o = opts();
    o.min_revs = 100;
    assert!(analysis::run("age", &changes, &o).is_ok());
    assert!(analysis::run("authors", &changes, &o).unwrap_or_default().rows.is_empty());
    assert!(analysis::run("entity-churn", &changes, &o).unwrap_or_default().rows.is_empty());
}

#[test]
fn min_revs_filters_effort_metrics() {
    let changes = sample_changes();
    let mut o = opts();
    o.min_revs = 100;
    assert!(analysis::run("entity-effort", &changes, &o).unwrap_or_default().rows.is_empty());
    assert!(
        analysis::run("main-dev-by-revs", &changes, &o)
            .unwrap_or_default()
            .rows
            .is_empty()
    );
    assert!(analysis::run("fragmentation", &changes, &o).unwrap_or_default().rows.is_empty());
}

#[test]
fn coupling_shared_rev_gate_and_soc_skip() {
    let changes = sample_changes();
    let mut o = opts();
    o.min_shared_revs = 100;
    assert!(analysis::run("coupling", &changes, &o).unwrap_or_default().rows.is_empty());
    o.min_shared_revs = 1;
    o.max_changeset_size = 0;
    assert!(analysis::run("soc", &changes, &o).unwrap_or_default().rows.is_empty());
}

#[test]
fn analyze_log_legacy_git() {
    let log = "[abc] Ada 2024-01-01 fix\n1\t0\ta.rs\n";
    let mut o = opts();
    o.analysis = String::from("summary");
    assert!(analyze_log(&mut Cursor::new(log.as_bytes()), "git", &o).is_ok());
}
