//! Help text for the hotspots CLI.

use hotspots::analysis_names;

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
  -n, --min-revs N               Min revisions per entity for ranking analyses (default: 5)
  -m, --min-shared-revs N        Min shared revisions for coupling (default: 5)
  -i, --min-coupling N           Min coupling degree percent (default: 30)
  -x, --max-coupling N           Max coupling degree percent (default: 100)
  -s, --max-changeset-size N     Max changeset size for coupling (default: 30, max: 200)
  -d, --age-time-now YYYY-MM-DD  Reference date for age analysis
  -t, --temporal-period day      Merge same-day commits per author (coupling, SOC, risk)
  -g, --group FILE               Layer map (`prefix => layer`; first match wins)
      --exclude PREFIX           Drop matching paths (repeatable)
      --include PREFIX           Keep only matching nonempty path prefixes (repeatable)
      --format text|json         Output format (default: text)
      --grain file|function      Entity grain (default: file)
      --repo PATH                Git work tree (required for function grain)
  -h, --help                     Show this help

Default output is an aligned terminal table. Use `--format json` for scripting.

Function grain expands `*.rs` changes to `path::symbol` via git + syn using
`--repo`. Non-Rust paths are dropped. Revisions in the log must exist in
`--repo`. Accuracy uses per-commit symbol tables and zero-context hunk overlap
(not HEAD-only maps). Restrict with `--include` when the log mixes languages.

Layer maps try rules in file order (first match wins; put specific prefixes
first). Unmatched paths keep their names. `main-dev` ownership is share of
added lines and omits entities with no additions (`main-dev-by-revs` for
delete-only).

Analyses:
  {analyses}

Generate a preferred Git log:
  git log --all --numstat --date=short --pretty=format:'--%h--%ad--%aN' \\
    --no-renames --after=YYYY-MM-DD > logfile.log
"
    )
}
