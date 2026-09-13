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

type FlagHandler = fn(&[String], usize, &str, &mut Args) -> Result<usize, String>;

const FLAGS: &[(&[&str], FlagHandler)] = &[
    (&["-l", "--log"], set_log),
    (&["-c", "--vcs"], set_vcs),
    (&["-a", "--analysis"], set_analysis),
    (&["-r", "--rows"], set_rows),
    (&["-n", "--min-revs"], set_min_revs),
    (&["-m", "--min-shared-revs"], set_min_shared_revs),
    (&["-i", "--min-coupling"], set_min_coupling),
    (&["-x", "--max-coupling"], set_max_coupling),
    (&["-s", "--max-changeset-size"], set_max_changeset_size),
    (&["-d", "--age-time-now"], set_age_time_now),
    (&["-t", "--temporal-period"], set_temporal_period),
    (&["-g", "--group"], set_group),
    (&["--exclude"], set_exclude),
    (&["--include"], set_include),
    (&["--format"], set_format),
    (&["--grain"], set_grain),
    (&["--repo"], set_repo),
];

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

fn parse_flag(
    flag: &str,
    argv: &[String],
    index: usize,
    parsed: &mut Args,
) -> Result<usize, String> {
    for &(names, handler) in FLAGS {
        if names.contains(&flag) {
            return handler(argv, index, flag, parsed);
        }
    }
    Err(format!("unknown argument `{flag}`"))
}

fn set_log(argv: &[String], index: usize, flag: &str, parsed: &mut Args) -> Result<usize, String> {
    assign_string(argv, index, flag, &mut parsed.log)
}

fn set_vcs(argv: &[String], index: usize, flag: &str, parsed: &mut Args) -> Result<usize, String> {
    assign_string(argv, index, flag, &mut parsed.vcs)
}

fn set_analysis(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_string(argv, index, flag, &mut parsed.options.analysis)
}

fn set_rows(argv: &[String], index: usize, flag: &str, parsed: &mut Args) -> Result<usize, String> {
    parsed.options.rows = Some(parse_int(require_value(argv, index, flag)?, flag)?);
    Ok(index + 2)
}

fn set_min_revs(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_int(argv, index, flag, &mut parsed.options.min_revs)
}

fn set_min_shared_revs(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_int(argv, index, flag, &mut parsed.options.min_shared_revs)
}

fn set_min_coupling(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_int(argv, index, flag, &mut parsed.options.min_coupling)
}

fn set_max_coupling(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_int(argv, index, flag, &mut parsed.options.max_coupling)
}

fn set_max_changeset_size(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_int(argv, index, flag, &mut parsed.options.max_changeset_size)
}

fn set_age_time_now(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_optional(argv, index, flag, &mut parsed.options.age_time_now)
}

fn set_temporal_period(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.temporal_period = parse_temporal(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn set_group(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    assign_optional(argv, index, flag, &mut parsed.options.group_file)
}

fn set_exclude(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    push_string(argv, index, flag, &mut parsed.options.exclude)
}

fn set_include(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    push_string(argv, index, flag, &mut parsed.options.include)
}

fn set_format(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.format = OutputFormat::parse(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn set_grain(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.grain = Grain::parse(require_value(argv, index, flag)?)?;
    Ok(index + 2)
}

fn set_repo(argv: &[String], index: usize, flag: &str, parsed: &mut Args) -> Result<usize, String> {
    assign_optional(argv, index, flag, &mut parsed.options.repo)
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
    require_log(parsed)?;
    require_vcs(parsed)?;
    require_repo_for_function(parsed)
}

fn validate_options(parsed: &Args) -> Result<(), String> {
    require_known_analysis(parsed)?;
    require_coupling_bounds(parsed)?;
    require_changeset_limit(parsed)?;
    require_age_time_now(parsed)
}

fn require_log(parsed: &Args) -> Result<(), String> {
    if parsed.log.is_empty() {
        return Err(String::from("missing required `-l/--log`"));
    }
    Ok(())
}

fn require_vcs(parsed: &Args) -> Result<(), String> {
    if parsed.vcs.is_empty() {
        return Err(String::from("missing required `-c/--vcs`"));
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
