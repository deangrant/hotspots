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

/// Parses process arguments.
///
/// # Errors
///
/// Returns a human-readable message when flags are missing or invalid.
pub fn parse_args(argv: &[String]) -> Result<Args, String> {
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        return Ok(Args {
            log: String::new(),
            vcs: String::new(),
            options: Options::default(),
            help: true,
        });
    }
    let mut parsed = Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
        help: false,
    };
    let mut index = 1;
    while index < argv.len() {
        let flag = argv[index].as_str();
        index = parse_flag(flag, argv, index, &mut parsed)?;
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
    match flag {
        "-l" | "--log" => assign_string(argv, index, flag, &mut parsed.log),
        "-c" | "--vcs" => assign_string(argv, index, flag, &mut parsed.vcs),
        "-a" | "--analysis" => assign_string(argv, index, flag, &mut parsed.options.analysis),
        "-r" | "--rows" => {
            parsed.options.rows = Some(parse_int(require_value(argv, index, flag)?, flag)?);
            Ok(index + 2)
        }
        "-n" | "--min-revs" => {
            parsed.options.min_revs = parse_int(require_value(argv, index, flag)?, flag)?;
            Ok(index + 2)
        }
        "-m" | "--min-shared-revs" => {
            parsed.options.min_shared_revs = parse_int(require_value(argv, index, flag)?, flag)?;
            Ok(index + 2)
        }
        "-i" | "--min-coupling" => {
            parsed.options.min_coupling = parse_int(require_value(argv, index, flag)?, flag)?;
            Ok(index + 2)
        }
        "-x" | "--max-coupling" => {
            parsed.options.max_coupling = parse_int(require_value(argv, index, flag)?, flag)?;
            Ok(index + 2)
        }
        "-s" | "--max-changeset-size" => {
            parsed.options.max_changeset_size = parse_int(require_value(argv, index, flag)?, flag)?;
            Ok(index + 2)
        }
        "-d" | "--age-time-now" => {
            assign_optional(argv, index, flag, &mut parsed.options.age_time_now)
        }
        "-t" | "--temporal-period" => {
            parsed.options.temporal_period = parse_temporal(require_value(argv, index, flag)?)?;
            Ok(index + 2)
        }
        "-g" | "--group" => assign_optional(argv, index, flag, &mut parsed.options.group_file),
        "--exclude" => push_string(argv, index, flag, &mut parsed.options.exclude),
        "--include" => push_string(argv, index, flag, &mut parsed.options.include),
        other => Err(format!("unknown argument `{other}`")),
    }
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
  -a, --analysis NAME            Analysis to run (default: authors)
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
