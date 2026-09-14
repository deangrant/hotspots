//! Flag name table and per-flag value application.
//!
//! Keeps cyclomatic complexity low by routing through slot helpers.

use hotspots::{Grain, OutputFormat, TemporalPeriod};

use crate::args::Args;

#[derive(Clone, Copy)]
#[repr(usize)]
pub(super) enum Flag {
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

pub(super) fn parse_flag(
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

pub(super) fn apply_slot_0_2(
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

pub(super) fn apply_slot_3_5(
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

pub(super) fn apply_slot_6_8(
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

pub(super) fn apply_slot_9_11(
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

pub(super) fn apply_slot_12_14(
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

pub(super) fn apply_slot_15_17(
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
