//! Command-line argument parsing for the hotspots binary.

use hotspots::{
    Grain, MAX_CHANGESET_SIZE_LIMIT, Options, OutputFormat, TemporalPeriod, analysis_names, date,
};

/// Parsed CLI invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Path to the VCS log file.
    pub log: String,
    /// VCS parser name (`git` or `git2`).
    pub vcs: String,
    /// Analysis options.
    pub options: Options,
    /// stdout encoding.
    pub format: OutputFormat,
    /// When true, print help and exit successfully.
    pub help: bool,
}

/// Parses process arguments.
///
/// # Errors
///
/// Returns a human-readable message when flags are missing or invalid.
pub fn parse_args(argv: &[String]) -> Result<Args, String> {
    if wants_help(argv) {
        return Ok(help_args());
    }
    parse_required_args(argv)
}

fn wants_help(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "-h" || a == "--help")
}

fn help_args() -> Args {
    Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
        format: OutputFormat::Text,
        help: true,
    }
}

fn parse_required_args(argv: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
        format: OutputFormat::Text,
        help: false,
    };
    let mut index = 1;
    while index < argv.len() {
        index = parse_flag(argv[index].as_str(), argv, index, &mut parsed)?;
    }
    validate(&parsed)?;
    Ok(parsed)
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum Flag {
    Log = 0,
    Vcs = 1,
    Analysis = 2,
    Rows = 3,
    MinRevs = 4,
    MinSharedRevs = 5,
    MinCoupling = 6,
    MaxCoupling = 7,
    MaxChangesetSize = 8,
    AgeTimeNow = 9,
    TemporalPeriod = 10,
    Group = 11,
    Exclude = 12,
    Include = 13,
    Format = 14,
    Grain = 15,
    Repo = 16,
}

const FLAG_NAMES: &[(&[&str], Flag)] = &[
    (&["-l", "--log"], Flag::Log),
    (&["-c", "--vcs"], Flag::Vcs),
    (&["-a", "--analysis"], Flag::Analysis),
    (&["-r", "--rows"], Flag::Rows),
    (&["-n", "--min-revs"], Flag::MinRevs),
    (&["-m", "--min-shared-revs"], Flag::MinSharedRevs),
    (&["-i", "--min-coupling"], Flag::MinCoupling),
    (&["-x", "--max-coupling"], Flag::MaxCoupling),
    (&["-s", "--max-changeset-size"], Flag::MaxChangesetSize),
    (&["-d", "--age-time-now"], Flag::AgeTimeNow),
    (&["-t", "--temporal-period"], Flag::TemporalPeriod),
    (&["-g", "--group"], Flag::Group),
    (&["--exclude"], Flag::Exclude),
    (&["--include"], Flag::Include),
    (&["--format"], Flag::Format),
    (&["--grain"], Flag::Grain),
    (&["--repo"], Flag::Repo),
];

fn parse_flag(
    flag: &str,
    argv: &[String],
    index: usize,
    parsed: &mut Args,
) -> Result<usize, String> {
    let Some(kind) = lookup_flag(flag) else {
        return Err(format!("unknown argument `{flag}`"));
    };
    apply_flag(kind, argv, index, flag, parsed)
}

fn lookup_flag(flag: &str) -> Option<Flag> {
    FLAG_NAMES
        .iter()
        .find(|(names, _)| names.contains(&flag))
        .map(|(_, kind)| *kind)
}

fn apply_flag(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match (kind as usize) / 6 {
        0 => apply_low(kind, argv, index, flag, parsed),
        1 => apply_mid(kind, argv, index, flag, parsed),
        _ => apply_high(kind, argv, index, flag, parsed),
    }
}

fn apply_low(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    if (kind as usize) < 3 {
        apply_slot_0_2(kind, argv, index, flag, parsed)
    } else {
        apply_slot_3_5(kind, argv, index, flag, parsed)
    }
}

fn apply_mid(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    if (kind as usize) >= 9 {
        return apply_slot_9_11(kind, argv, index, flag, parsed);
    }
    apply_slot_6_8(kind, argv, index, flag, parsed)
}

fn apply_high(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    let use_top = (kind as usize) >= 15;
    if use_top {
        apply_slot_15_17(kind, argv, index, flag, parsed)
    } else {
        apply_slot_12_14(kind, argv, index, flag, parsed)
    }
}

fn apply_slot_0_2(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::Log => assign_string(argv, index, flag, &mut parsed.log),
        Flag::Vcs => assign_string(argv, index, flag, &mut parsed.vcs),
        Flag::Analysis => assign_string(argv, index, flag, &mut parsed.options.analysis),
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_slot_3_5(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::Rows => apply_rows(argv, index, flag, parsed),
        Flag::MinRevs => assign_int(argv, index, flag, &mut parsed.options.min_revs),
        Flag::MinSharedRevs => assign_int(argv, index, flag, &mut parsed.options.min_shared_revs),
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_rows(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.rows = Some(parse_int(require_value(argv, index, flag)?, flag)?);
    Ok(index + 2)
}

fn apply_slot_6_8(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::MinCoupling => assign_int(argv, index, flag, &mut parsed.options.min_coupling),
        Flag::MaxCoupling => assign_int(argv, index, flag, &mut parsed.options.max_coupling),
        Flag::MaxChangesetSize => {
            assign_int(argv, index, flag, &mut parsed.options.max_changeset_size)
        }
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_slot_9_11(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::AgeTimeNow => assign_optional(argv, index, flag, &mut parsed.options.age_time_now),
        Flag::TemporalPeriod => apply_temporal(argv, index, flag, parsed),
        Flag::Group => assign_optional(argv, index, flag, &mut parsed.options.group_file),
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_temporal(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.temporal_period = parse_temporal(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn apply_slot_12_14(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::Exclude => push_string(argv, index, flag, &mut parsed.options.exclude),
        Flag::Include => push_string(argv, index, flag, &mut parsed.options.include),
        Flag::Format => apply_format(argv, index, flag, parsed),
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_format(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.format = OutputFormat::parse(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn apply_slot_15_17(
    kind: Flag,
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    match kind {
        Flag::Grain => apply_grain(argv, index, flag, parsed),
        Flag::Repo => assign_optional(argv, index, flag, &mut parsed.options.repo),
        _ => Err(format!("unhandled argument `{flag}`")),
    }
}

fn apply_grain(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.grain = Grain::parse(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn assign_string(
    argv: &[String],
    index: usize,
    flag: &str,
    target: &mut String,
) -> Result<usize, String> {
    require_value(argv, index, flag)?.clone_into(target);
    Ok(index + 2)
}

fn assign_optional(
    argv: &[String],
    index: usize,
    flag: &str,
    target: &mut Option<String>,
) -> Result<usize, String> {
    *target = Some(require_value(argv, index, flag)?.to_owned());
    Ok(index + 2)
}

fn assign_int<T: std::str::FromStr>(
    argv: &[String],
    index: usize,
    flag: &str,
    target: &mut T,
) -> Result<usize, String> {
    *target = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
}

fn push_string(
    argv: &[String],
    index: usize,
    flag: &str,
    target: &mut Vec<String>,
) -> Result<usize, String> {
    target.push(require_value(argv, index, flag)?.to_owned());
    Ok(index + 2)
}

fn require_value<'a>(argv: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    argv.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for `{flag}`"))
}

fn parse_int<T: std::str::FromStr>(raw: &str, flag: &str) -> Result<T, String> {
    raw.parse().map_err(|_| format!("invalid integer for `{flag}`: {raw}"))
}

fn parse_temporal(raw: &str) -> Result<TemporalPeriod, String> {
    match raw {
        "day" => Ok(TemporalPeriod::Day),
        other => Err(format!("invalid temporal period `{other}`; expected `day`")),
    }
}

fn validate(parsed: &Args) -> Result<(), String> {
    validate_required(parsed)?;
    validate_options(parsed)
}

fn validate_required(parsed: &Args) -> Result<(), String> {
    require_non_empty(&parsed.log, "-l/--log")?;
    require_non_empty(&parsed.vcs, "-c/--vcs")?;
    require_repo_for_function(parsed)
}

fn validate_options(parsed: &Args) -> Result<(), String> {
    require_known_analysis(parsed)?;
    require_coupling_bounds(parsed)?;
    require_changeset_limit(parsed)?;
    require_age_time_now(parsed)
}

fn require_non_empty(value: &str, flag: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("missing required `{flag}`"));
    }
    Ok(())
}

fn require_repo_for_function(parsed: &Args) -> Result<(), String> {
    if parsed.options.grain == Grain::Function && parsed.options.repo.is_none() {
        return Err(String::from("`--grain function` requires `--repo <path>`"));
    }
    Ok(())
}

fn require_known_analysis(parsed: &Args) -> Result<(), String> {
    if analysis_names().contains(&parsed.options.analysis.as_str()) {
        return Ok(());
    }
    Err(format!(
        "unknown analysis `{}`; expected one of: {}",
        parsed.options.analysis,
        analysis_names().join(", ")
    ))
}

fn require_coupling_bounds(parsed: &Args) -> Result<(), String> {
    if parsed.options.min_coupling > parsed.options.max_coupling {
        return Err(String::from(
            "`--min-coupling` must be less than or equal to `--max-coupling`",
        ));
    }
    Ok(())
}

fn require_changeset_limit(parsed: &Args) -> Result<(), String> {
    if parsed.options.max_changeset_size > MAX_CHANGESET_SIZE_LIMIT {
        return Err(format!(
            "`--max-changeset-size` must be at most {MAX_CHANGESET_SIZE_LIMIT}"
        ));
    }
    Ok(())
}

fn require_age_time_now(parsed: &Args) -> Result<(), String> {
    let Some(raw) = parsed.options.age_time_now.as_deref() else {
        return Ok(());
    };
    date::parse_date(raw)
        .map(|_| ())
        .map_err(|_| format!("invalid `--age-time-now` `{raw}`; expected `YYYY-MM-DD`"))
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "-l", "log.txt", "-c", "git2", "-r", "10", "-m", "2", "-i", "20", "-x", "80", "-s",
            "15",
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
        assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-d", "2024-01-02"])).is_ok());
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
}
