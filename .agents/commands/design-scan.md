# Design scan

Read and follow:

- [`.agents/skills/rust-style-guide/SKILL.md`](../skills/rust-style-guide/SKILL.md)
- [`.agents/skills/rust-solid-design/SKILL.md`](../skills/rust-solid-design/SKILL.md)

When scanning product code (parsers, analyses, grain, CLI), also read
[`.agents/skills/hotspots-domain/SKILL.md`](../skills/hotspots-domain/SKILL.md).

Scan the workspace (or the paths the user names) and emit **one** structured
report. Prefer this checklist over freeform multi-agent prose dumps.

## Report schema

Group every item under exactly one heading:

### Must-fix

Blocking style or SOLID violations.

### Nice-to-have

Improvements that are valid but not required for merge.

### Keep-as-is

Intentional patterns that look like smells but should not change (e.g. CC≤8
splits kept under the complexity / CRAP strict budget).

## Hard checks (must-fix if violated)

- `.rs` files ≤ **500** physical lines (`wc -l`)
- Physical lines ≤ **100** characters ([`rustfmt.toml`](../../rustfmt.toml)
  `max_width`)
- No `#[allow(...)]` on code — use `#[expect(..., reason = "...")]` (fixture
  *strings* mentioning `#[allow]` are fine)
- Soft warn (nice-to-have): production `.rs` files approaching **~450** lines

## Quality-gate checks

- Do **not** propose relaxing Clippy, llvm-cov `--fail-under-lines 100`, dry-rs
  `--fail-on-findings`, CRAP `--threshold strict` (8), or tool pin `.rev` files
  solely to hide failures (see [gate-contract](../rules/gate-contract/)).
- New or unexplained `// dry-rs:ignore` / `// dry-rs:ignore-file` without a
  reason → nice-to-have (or must-fix if it clearly silences a real clone that
  should be deduped).
- Intentional CC≤8 helper splits for Clippy → keep-as-is (do not re-merge for
  DRY alone).

## Output rules

- Cite concrete paths and symbols.
- Do not invent traits or speculative refactors in must-fix.
- Do not propose relaxing clippy thresholds or other verify gates.
