# hotspots

Command-line tool that mines **exported Git history logs** and prints
**JSON** maintenance metrics. By default it ranks files by a composite
**risk** score (0–100 within the current log) from revisions, churn, SOC, and
ownership fragmentation.

Metrics are **indicators**, not blame or defect probability. High scores mean
“investigate,” not “this will fail.”

## Build and run

```bash
cargo build -p hotspots-cli --release
cargo run -p hotspots-cli -- -l logfile.log -c git2 -n 1
```

Omitting `-a` runs the default **risk** analysis. Results are a JSON array of
objects on stdout.

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

Prefer `--no-renames` so paths stay comparable across commits. Limit history with
`--after` so recent maintenance questions are not drowned by old data.

## Examples

```bash
# Default: relative risk ranking (0–100 within this log)
cargo run -p hotspots-cli -- -l logfile.log -c git2 -n 1 -r 20

# Authors per entity
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a authors -n 5

# Logical coupling
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a coupling

# Drop vendor noise
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a revisions \
  --exclude vendor --exclude node_modules

# Summary as JSON on stdout
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a summary
```

See `hotspots --help` for the full flag and analysis list.

## Workspace

- `crates/hotspots` — library (parsers, filters, metrics, JSON writer)
- `crates/hotspots-cli` — thin CLI composition root

Dependencies are intentionally limited to the Rust standard library.

## License

[MIT](LICENSE) © 2026 Dean Grant
