//! Command-line argument parsing for the hotspots binary.

use hotspots::{Options, TemporalPeriod, analysis_names};

/// Parsed CLI invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Path to the VCS log file.
    pub log: String,
    /// VCS parser name (`git` or `git2`).
    pub vcs: String,
    /// Analysis options.
    pub options: Options,
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
        help: true,
    }
}

fn parse_required_args(argv: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
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
    parsed.options.min_revs = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
}

fn set_min_shared_revs(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.min_shared_revs = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
}

fn set_min_coupling(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.min_coupling = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
}

fn set_max_coupling(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.max_coupling = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
}

fn set_max_changeset_size(
    argv: &[String],
    index: usize,
    flag: &str,
    parsed: &mut Args,
) -> Result<usize, String> {
    parsed.options.max_changeset_size = parse_int(require_value(argv, index, flag)?, flag)?;
    Ok(index + 2)
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
    if parsed.log.is_empty() {
        return Err(String::from("missing required `-l/--log`"));
    }
    if parsed.vcs.is_empty() {
        return Err(String::from("missing required `-c/--vcs`"));
    }
    Ok(())
}

/// Help text printed for `-h/--help`.
#[must_use]
pub fn help_text() -> String {
    let analyses = analysis_names().join(", ");
    format!(
        "\
hotspots — mine Git history logs for maintenance metrics

Usage:
  hotspots -l <logfile> -c <git|git2> [options]

Options:
  -l, --log PATH                 VCS log file (required)
  -c, --vcs git|git2             Log format (required)
  -a, --analysis NAME            Analysis to run (default: risk)
  -r, --rows N                   Max output rows
  -n, --min-revs N               Min revisions per entity (default: 5)
  -m, --min-shared-revs N        Min shared revisions for coupling (default: 5)
  -i, --min-coupling N           Min coupling degree percent (default: 30)
  -x, --max-coupling N           Max coupling degree percent (default: 100)
  -s, --max-changeset-size N     Max changeset size for coupling (default: 30)
  -d, --age-time-now YYYY-MM-DD  Reference date for age analysis
  -t, --temporal-period day      Merge same-day commits per author
  -g, --group FILE               Layer map (`prefix => layer` lines)
      --exclude PREFIX           Drop matching paths (repeatable)
      --include PREFIX           Keep only matching paths (repeatable)
  -h, --help                     Show this help

Output is a JSON array of objects on stdout.

Analyses:
  {analyses}

Generate a preferred Git log:
  git log --all --numstat --date=short --pretty=format:'--%h--%ad--%aN' \\
    --no-renames --after=YYYY-MM-DD > logfile.log
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn rejects_missing_required_and_bad_values() {
        assert!(parse_args(&argv(&["-c", "git2"])).is_err());
        assert!(parse_args(&argv(&["-l", "x"])).is_err());
        assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-n", "nope"])).is_err());
        assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "-t", "week"])).is_err());
        assert!(parse_args(&argv(&["-l", "x", "-c", "git2", "--bogus"])).is_err());
        assert!(parse_args(&argv(&["-l"])).is_err());
    }
}
