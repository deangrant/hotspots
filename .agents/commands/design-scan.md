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

Intentional patterns that look like smells but should not change (e.g. CC-split
helpers kept under the complexity budget).

## Hard checks (must-fix if violated)

- `.rs` files ≤ **500** physical lines (`wc -l`)
- Physical lines ≤ **100** characters ([`rustfmt.toml`](../../rustfmt.toml)
  `max_width`)
- No `#[allow(...)]` on code — use `#[expect(..., reason = "...")]` (fixture
  *strings* mentioning `#[allow]` are fine)
- Soft warn (nice-to-have): production `.rs` files approaching **~450** lines

## Output rules

- Cite concrete paths and symbols.
- Do not invent traits or speculative refactors in must-fix.
- Do not propose relaxing clippy thresholds.
