# hotspots

Command-line tool that mines **exported Git history logs** and prints
maintenance metrics. By default it ranks files by a composite **risk** score
(0–100 within the current log) from revisions, churn, SOC, and ownership
fragmentation, shown as an **aligned terminal table**.

Metrics are **indicators**, not blame or defect probability. High scores mean
“investigate,” not “this will fail.”

## Build and run

```bash
cargo build -p hotspots-cli --release
cargo run -p hotspots-cli -- -l logfile.log -c git2 -n 1
```

Omitting `-a` runs the default **risk** analysis. Omitting `--format` prints a
terminal table. Use `--format json` for a JSON array of objects.

## Generate a Git log

Preferred format (`-c git2`):

```bash
git log --all --numstat --date=short --pretty=format:'--%h--%ad--%aN' \
  --no-renames --after=YYYY-MM-DD > logfile.log
```

Legacy format (`-c git`):

```bash
git log --pretty=format:'[%h] %aN %ad %s' --date=short --numstat \
  --after=YYYY-MM-DD > logfile.log
```

Prefer `--no-renames` so paths stay comparable across commits (rename numstat
lines are rejected). Limit history with `--after` so recent maintenance
questions are not drowned by old data. Logs are capped at one million change
rows.

## File vs function grain

`--grain file` (default) treats each path in the numstat log as one entity.

`--grain function` expands Rust (`*.rs`) changes to `path::symbol` entities
(function names and `Type::method`) using a Git work tree (`--repo`) plus `syn`.
Revisions in the log must exist in `--repo`. Attribution uses **per-commit**
symbol tables (`git show REV:PATH`) and **zero-context hunk overlap**
(`git show`/`diff-tree -U0`), not a HEAD-only map applied to numstat totals.
Deleted or renamed-away paths load symbols from the parent blob (`REV^:PATH`);
missing on both sides still fails closed.

Non-Rust paths are **dropped** under function grain (a stderr note reports the
count). Changes whose hunks miss all `fn`/`impl` symbols (typical crate roots
or `mod`/`use`-only edits) are also dropped with a stderr count. Use
`--include` to restrict the log to Rust trees when mixed languages are present.
Failures (missing repo, git errors, unparsable `.rs`) abort the run; there is
no silent fallback to file grain.

```bash
hotspots -l logfile.log -c git2 --grain function --repo . -n 1 -r 20
```

## Examples

```bash
# Default: relative risk ranking as a terminal table
cargo run -p hotspots-cli -- -l logfile.log -c git2 -n 1 -r 20

# Same results as JSON for scripting
cargo run -p hotspots-cli -- -l logfile.log -c git2 -n 1 -r 20 --format json

# Function-level risk for Rust symbols
cargo run -p hotspots-cli -- -l logfile.log -c git2 \
  --grain function --repo . -n 1 -r 20

# Authors per entity
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a authors -n 5

# Logical coupling
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a coupling

# Drop vendor noise
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a revisions \
  --exclude vendor --exclude node_modules

# Summary
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a summary
```

See `hotspots --help` for the full flag and analysis list.

## Workspace

- `crates/hotspots` — library (parsers, filters, metrics, text/JSON writers; std-oriented)
- `crates/hotspots-rs` — Rust function-grain resolver (`git` CLI + `syn`)
- `crates/hotspots-cli` — thin CLI composition root

## License

[MIT](LICENSE) © 2026 Dean Grant
