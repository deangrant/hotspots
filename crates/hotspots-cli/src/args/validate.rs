//! Post-parse validation for required flags and option bounds.

use hotspots::{Grain, MAX_CHANGESET_SIZE_LIMIT, analysis_names, date};

use crate::args::Args;

pub(super) fn validate(parsed: &Args) -> Result<(), String> {
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
