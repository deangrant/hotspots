---
name: hotspots-domain
description: >-
  Git history log mining and maintenance metrics: log formats, grain, analyses,
  CLI flags, crate map. Use when changing parsers, analyses, function grain,
  CLI, or help/README UX.
---

# hotspots domain

Command-line tool that mines **exported Git history logs** and prints
maintenance metrics. Metrics are **indicators**, not blame or defect
probability — high scores mean “investigate,” not “this will fail.”

## Crates

| Crate | Role |
| --- | --- |
| [`crates/hotspots`](../../../crates/hotspots) | Library: parsers, filters, metrics, text/JSON writers |
| [`crates/hotspots-rs`](../../../crates/hotspots-rs) | Rust function-grain resolver (`git` CLI + `syn`) |
| [`crates/hotspots-cli`](../../../crates/hotspots-cli) | Thin CLI composition root |

## Log export

Preferred format (`-c git2`):

```bash
git log --all --numstat --date=short --pretty=format:'--%h--%ad--%aN' \
  --no-renames --after=YYYY-MM-DD > logfile.log
```

Legacy format (`-c git`):

```bash
git log --pretty=format:'[%h] %aN <%ad> %s' --date=short --numstat \
  --after=YYYY-MM-DD > logfile.log
```

- Prefer `--no-renames` so paths stay comparable (rename numstat lines are
  rejected).
- Limit with `--after` so recent questions are not drowned by old data.
- Logs are capped at one million change rows.
- `--include` prefixes must be nonempty after trim (empty/`/` are rejected).

## Grain

| `--grain` | Behavior |
| --- | --- |
| `file` (default) | Each path in the numstat log is one entity |
| `function` | Expands Rust `*.rs` changes to `path::symbol` via `--repo` + `syn` |

Function grain:

- Revisions in the log must exist in `--repo`.
- Attribution uses **per-commit** symbol tables and **zero-context** hunk
  overlap — not a HEAD-only map.
- Non-Rust paths are **dropped** (stderr count). Empty hunks and missed symbols
  also drop with a stderr count.
- Failures (missing repo, git errors, unparsable `.rs`) **abort**; there is no
  silent fallback to file grain.
- Restrict with `--include` when the log mixes languages.
- `git` binary from `PATH`, or `GIT_EXECUTABLE` when set.

## Analyses

Default (`-a` omitted): **`risk`** — composite 0–100 ranking within the current
log from revisions, churn, SOC, and ownership fragmentation.

Other names (see `analysis_names()` / `--help`): `abs-churn`, `age`,
`author-churn`, `authors`, `communication`, `coupling`, `entity-churn`,
`entity-effort`, `entity-ownership`, `fragmentation`, `hotspots`, `identity`,
`main-dev`, `main-dev-by-revs`, `revisions`, `soc`, `summary`.

Common flags: `-r/--rows`, `-n/--min-revs`, coupling mins/maxes, `--exclude` /
`--include`, `--format text|json`, `-g/--group` layer map.

## UX source of truth

Prefer [README.md](../../../README.md) and
[`crates/hotspots-cli/src/help.rs`](../../../crates/hotspots-cli/src/help.rs)
when changing user-facing flag or analysis semantics.
