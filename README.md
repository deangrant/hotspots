# hotspots

Command-line tool that mines **exported Git history logs** and prints
**JSON** maintenance metrics: churn, ownership, age, coupling, communication,
and related code-health signals.

Metrics are **indicators**, not blame. High coupling can be intentional; treat
results as prompts for investigation.

## Build and run

```bash
cargo build -p hotspots-cli --release
cargo run -p hotspots-cli -- -l logfile.log -c git2 -a summary
```

The binary name is `hotspots`. Analysis results are written to stdout as a JSON
array of objects.

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
