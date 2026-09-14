//! Unit tests for CLI argument parsing.

use hotspots::{Grain, Options, OutputFormat, TemporalPeriod};

use super::flags::{
    Flag, apply_slot_0_2, apply_slot_3_5, apply_slot_6_8, apply_slot_9_11, apply_slot_12_14,
    apply_slot_15_17,
};
use super::{help_args, parse_args};
use crate::help::help_text;

fn argv(args: &[&str]) -> Vec<String> {
    std::iter::once("hotspots")
        .chain(args.iter().copied())
        .map(String::from)
        .collect()
}

#[test]
fn help_short_circuits() {
    let parsed = parse_args(&argv(&["-h"]));
    assert!(parsed.as_ref().is_ok_and(|p| p.help));
    assert!(!help_text().is_empty());
}

#[test]
fn parses_required_fields() {
    let parsed = parse_args(&argv(&[
        "-l", "log.txt", "-c", "git2", "-a", "summary", "-n", "1",
    ]));
    assert!(parsed.as_ref().is_ok_and(|p| {
        p.log == "log.txt"
            && p.vcs == "git2"
            && p.options.analysis == "summary"
            && p.options.min_revs == 1
    }));
}

#[test]
fn parses_optional_numeric_flags() {
    let parsed = parse_args(&argv(&[
        "-l", "log.txt", "-c", "git2", "-r", "10", "-m", "2", "-i", "20", "-x", "80", "-s", "15",
    ]));
    assert!(parsed.as_ref().is_ok_and(|p| {
        p.options.rows == Some(10)
            && p.options.min_shared_revs == 2
            && p.options.min_coupling == 20
            && p.options.max_coupling == 80
            && p.options.max_changeset_size == 15
    }));
}

#[test]
fn parses_optional_path_flags() {
    let parsed = parse_args(&argv(&[
        "-l",
        "log.txt",
        "-c",
        "git2",
        "-d",
        "2024-01-01",
        "-t",
        "day",
        "-g",
        "groups.txt",
        "--exclude",
        "vendor",
        "--include",
        "src",
    ]));
    assert!(parsed.as_ref().is_ok_and(|p| {
        p.options.temporal_period == TemporalPeriod::Day
            && p.options.exclude == vec![String::from("vendor")]
            && p.options.include == vec![String::from("src")]
            && p.options.group_file.as_deref() == Some("groups.txt")
    }));
}

#[test]
fn defaults_analysis_to_risk() {
    let parsed = parse_args(&argv(&["-l", "log.txt", "-c", "git2"]));
    assert!(parsed.as_ref().is_ok_and(|p| p.options.analysis == "risk"));
    assert_eq!(Options::default().analysis, "risk");
}

#[test]
fn format_defaults_to_text_and_accepts_json() {
    let defaulted = parse_args(&argv(&["-l", "log.txt", "-c", "git2"]));
    assert!(defaulted.as_ref().is_ok_and(|p| p.format == OutputFormat::Text));
    let json = parse_args(&argv(&["-l", "log.txt", "-c", "git2", "--format", "json"]));
    assert!(json.as_ref().is_ok_and(|p| p.format == OutputFormat::Json));
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--format", "csv"])).is_err());
}

#[test]
fn rejects_missing_required_and_bad_values() {
    assert!(parse_args(&argv(&["-c", "git2"])).is_err());
    assert!(parse_args(&argv(&["-l", "x"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-n", "nope"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-t", "week"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--bogus"])).is_err());
    assert!(parse_args(&argv(&["-l"])).is_err());
}

#[test]
fn rejects_invalid_optional_integers() {
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-m", "nope"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-i", "nope"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-x", "nope"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-s", "nope"])).is_err());
}

#[test]
fn slot_helpers_reject_mismatched_kinds() {
    let mut parsed = help_args();
    let argv = argv(&["hotspots"]);
    assert!(apply_slot_0_2(Flag::Rows, &argv, 0, "-r", &mut parsed).is_err());
    assert!(apply_slot_3_5(Flag::Log, &argv, 0, "-l", &mut parsed).is_err());
    assert!(apply_slot_6_8(Flag::Group, &argv, 0, "-g", &mut parsed).is_err());
    assert!(apply_slot_9_11(Flag::Exclude, &argv, 0, "--exclude", &mut parsed).is_err());
    assert!(apply_slot_12_14(Flag::Repo, &argv, 0, "--repo", &mut parsed).is_err());
    assert!(apply_slot_15_17(Flag::Log, &argv, 0, "-l", &mut parsed).is_err());
}

#[test]
fn rejects_unknown_analysis_and_coupling_bounds() {
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-a", "nope"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-i", "80", "-x", "20"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-s", "201"])).is_err());
}

#[test]
fn rejects_invalid_age_time_now() {
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-d", "not-a-date"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-d", "2024-13-40"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-d", "2024-01-02"])).is_ok());
}

#[test]
fn rejects_empty_include_prefixes() {
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--include", ""])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--include", "/"])).is_err());
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--include", "   "])).is_err());
}

#[test]
fn grain_defaults_to_file_and_requires_repo_for_function() {
    let file = parse_args(&argv(&["-l", "log.txt", "-c", "git2"]));
    assert!(file.as_ref().is_ok_and(|p| p.options.grain == Grain::File));
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--grain", "function"])).is_err());
    let ok = parse_args(&argv(&[
        "-l", "x", "-c", "git2", "--grain", "function", "--repo", ".",
    ]));
    assert!(ok.as_ref().is_ok_and(|p| {
        p.options.grain == Grain::Function && p.options.repo.as_deref() == Some(".")
    }));
    assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--grain", "layer"])).is_err());
}

#[test]
fn rejects_unknown_vcs_at_parse_time() {
    let err = parse_args(&argv(&["-l", "x", "-c", "svn"]));
    assert!(err.as_ref().is_err_and(|e| e.contains("unknown vcs")));
    assert!(parse_args(&argv(&["-l", "x", "-c", "hg"])).is_err());
}

#[test]
fn rejects_duplicate_scalar_flags() {
    assert!(
        parse_args(&argv(&["-l", "a", "-c", "git", "-c", "git2"]))
            .is_err_and(|e| { e.contains("duplicate") })
    );
    assert!(
        parse_args(&argv(&[
            "-l", "a", "-c", "git2", "-a", "risk", "-a", "summary"
        ]))
        .is_err_and(|e| e.contains("duplicate"))
    );
}

#[test]
fn allows_repeated_exclude_and_include() {
    let parsed = parse_args(&argv(&[
        "-l",
        "log.txt",
        "-c",
        "git2",
        "--exclude",
        "vendor",
        "--exclude",
        "target",
        "--include",
        "src",
        "--include",
        "crates",
    ]));
    assert!(parsed.is_ok_and(|p| {
        p.options.exclude == vec![String::from("vendor"), String::from("target")]
            && p.options.include == vec![String::from("src"), String::from("crates")]
    }));
}
