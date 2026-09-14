# Contributor Guidance

Guidance for AI agents and humans working in this repository.

**New task?** Glance → route → skill → change → verify.

This file is the entry point for repository conventions. Keep detailed architecture,
implementation guidance, and task-specific instructions in the referenced files rather
than duplicating them here.

Canonical agent assets live under [`.agents/`](.agents/); [`.cursor/rules`](.cursor/rules),
[`.cursor/commands`](.cursor/commands), and [`.cursor/hooks.json`](.cursor/hooks.json)
symlink or point there for editor integration.

## Repository at a glance

hotspots is a virtual Cargo workspace that mines **exported Git history logs** and
prints **maintenance metrics**. By default it ranks entities with a composite **risk**
score (0–100 within the current log) from revisions, churn, SOC, and ownership
fragmentation, shown as an aligned terminal table. Metrics are **indicators**, not
blame or defect probability.

| Package | Path | Role |
| ------- | ---- | ---- |
| Core | `crates/hotspots` | Parsers, filters, grain orchestration, analyses, text/JSON writers (no `git`) |
| Rust grain | `crates/hotspots-rs` | `SymbolResolver` via `git` CLI + `syn` |
| CLI | `crates/hotspots-cli` | Argv, process I/O, help text, and resolver wiring |

**Hard invariants (never violate):**

- Log format contracts stay explicit (`-c git` / `git2`). Do not invent silent format guessing.
- Function grain fails closed. No silent fallback to file grain on missing repo, git errors, or unparsable `.rs`.
- Metrics are indicators (investigate), not blame or defect probability.
- `hotspots` does not invoke `git`. Git + `syn` stay in `hotspots-rs` behind `SymbolResolver`.
- Change streams cap at one million rows.
- Workspace lint `unsafe_code` is **forbid**.
- No `#[allow]`. Suppressions must be `#[expect(..., reason = "...")]`.
- Workspace members are `hotspots`, `hotspots-cli`, and `hotspots-rs`.
- Keep functions under [`clippy.toml`](clippy.toml) thresholds (cognitive 8, type 200, function 50 lines).
- Keep `.rs` files at or under 500 lines.

**Runtime:** Rust toolchain `1.94.0`. Full local `/verify` is `./scripts/check.sh`,
`./scripts/dry-gate.sh`, llvm-cov `--fail-under-lines 100`, then
`./scripts/crap-gate.sh` (see [verify-gates](.agents/skills/verify-gates/SKILL.md)).

## Instruction Precedence

When instructions conflict, apply the most specific applicable instruction:

1. Repository-level `AGENTS.md`
2. Applicable files under `.agents/rules/`
3. Applicable files under `.agents/skills/`
4. Relevant documentation under `.agents/docs/`
5. Existing local implementation conventions

More specific guidance takes precedence over general guidance.

Rules define repository constraints and invariants. Skills provide task-specific
implementation guidance. Documentation provides architectural and product context.

There is no always-on gate-contract rule. [rust-complexity-budget](.agents/rules/rust-complexity-budget/)
is glob-scoped to `**/*.rs`. [hotspots-domain](.agents/rules/hotspots-domain/) is glob-scoped
to product crates. [ai-slop-mitigation](.agents/rules/ai-slop-mitigation/) is opt-in.
This file tells you **when to load** skills and docs; rules state **what you must not do**.
If a skill suggests something a rule forbids, the rule wins.

## Operating Principles

- Make the smallest change that correctly solves the task.
- Preserve existing crate boundaries and public contracts unless the task requires changing them.
- Prefer existing workspace crates, utilities, and patterns before introducing new abstractions or dependencies.
- Do not refactor unrelated code while completing a task.
- Do not weaken, bypass, or remove repository rules to make a change easier.
- Keep changes focused, reviewable, and consistent with the surrounding code.
- Prefer concrete diffs over speculative refactors. Match existing style; skip filler.
- Treat CLI flags, help text, and [README.md](README.md) as significant UX contracts; inspect them before changing user-facing semantics.
- Keep functions under [`clippy.toml`](clippy.toml) thresholds instead of silencing complexity.
- Keep `.rs` files at or under 500 lines.

## Common mistakes

- Silent format guessing for `-c git` / `git2` instead of an explicit VCS flag.
- Silent fallback from function grain to file grain when `--repo`, git, or `syn` fails.
- Putting `git` or `syn` into `hotspots` instead of `hotspots-rs` behind `SymbolResolver`.
- HEAD-only symbol maps instead of per-commit tables plus zero-context (`-U0`) hunk overlap.
- Re-merging CC≤5 dispatch splits that were extracted for Clippy, then fighting complexity budgets.
- Relaxing Clippy thresholds, the llvm-cov 100% line gate, the dry-rs
  `--fail-on-findings` gate, or the CRAP `--threshold strict` gate solely to hide
  failures.
- Claiming `/verify` or CI passed without running `./scripts/check.sh`,
  `./scripts/dry-gate.sh`, llvm-cov, and `./scripts/crap-gate.sh`.

## Workflow

Before changing code:

1. Inspect the repository status and relevant diff.
2. Identify the crate(s), files, and architectural area affected.
3. Read the applicable rules.
4. Read the matching skill(s) before modifying that area.
5. Check [ARCHITECTURE.md](.agents/docs/ARCHITECTURE.md) when the change crosses crate or pipeline boundaries.
6. Make the smallest appropriate change.
7. Run the narrowest relevant tests and checks first.
8. Run `/verify` before considering the change complete when practical.
   Procedure: [verify-gates](.agents/skills/verify-gates/SKILL.md).
9. Review the final diff for unintended changes.

Do not read every skill or document by default. Load only the guidance relevant to
the task being performed.

## Task routing

Read the matching skill **before** editing that area. Load only what the task needs.

| If you are changing… | Read first |
| -------------------- | ---------- |
| Parsers, analyses, grain, CLI, help, or README UX | [`hotspots-domain`](.agents/skills/hotspots-domain/) |
| Local verify gates (`check.sh`, dry-rs, llvm-cov, CRAP) | [`verify-gates`](.agents/skills/verify-gates/) |
| Rust style, docs, naming, API conventions | [`rust-style-guide`](.agents/skills/rust-style-guide/) |
| Traits, modules, dependency direction | [`rust-solid-design`](.agents/skills/rust-solid-design/) |

Cross-crate or pipeline changes: read
[ARCHITECTURE.md](.agents/docs/ARCHITECTURE.md) and every affected skill.

## Repository Documentation

- [README.md](README.md) — product overview, log export, grain, examples, CI notes
- [`.agents/docs/ARCHITECTURE.md`](.agents/docs/ARCHITECTURE.md) — crate roles, analysis pipeline, module maps, and invariants
- [DeepWiki](https://deepwiki.com/deangrant/hotpsots) — indexed project wiki for additional architecture, API, and pipeline context
- [`Cargo.toml`](Cargo.toml) — virtual workspace members and maximum `[workspace.lints]`
- [`clippy.toml`](clippy.toml) — cognitive 8, type 200, function 50 lines
- [`rustfmt.toml`](rustfmt.toml) — `max_width` 100
- [`deny.toml`](deny.toml) — cargo-deny policy (`multiple-versions` deny)
- [`rust-toolchain.toml`](rust-toolchain.toml) — pinned toolchain and components
- [`.github/workflows/`](.github/workflows/) — lint, test, coverage, supply-chain

## Rules

Canonical repository rules live under [`.agents/rules/`](.agents/rules/). The directory
is loaded from [`.cursor/rules`](.cursor/rules).

### Glob-scoped

- [`.agents/rules/rust-complexity-budget/`](.agents/rules/rust-complexity-budget/) — clippy / file / CC budget (`**/*.rs`)
- [`.agents/rules/hotspots-domain/`](.agents/rules/hotspots-domain/) — log/grain/UX contracts (product crates)

### Opt-in

- [`.agents/rules/ai-slop-mitigation/`](.agents/rules/ai-slop-mitigation/) — concrete diffs, no filler

## Skills

Canonical skills live under [`.agents/skills/`](.agents/skills/).

Read the matching skill before changing the corresponding area. Skills are
task-specific guidance and should not be loaded unless relevant.

- [`.agents/skills/rust-style-guide/`](.agents/skills/rust-style-guide/) — formatting, docs, naming, API conventions
- [`.agents/skills/rust-solid-design/`](.agents/skills/rust-solid-design/) — SOLID in Rust: traits, modules, DI
- [`.agents/skills/verify-gates/`](.agents/skills/verify-gates/) — `check.sh`, dry-rs, llvm-cov 100%, CRAP strict
- [`.agents/skills/hotspots-domain/`](.agents/skills/hotspots-domain/) — log formats, grain, analyses, crate map

## Commands

Canonical slash commands live under [`.agents/commands/`](.agents/commands/).
The directory is symlinked from [`.cursor/commands`](.cursor/commands).

Prefer repository commands over manually recreating equivalent workflows.

- `/verify` — `./scripts/check.sh`, dry-rs, llvm-cov `--fail-under-lines 100`, then CRAP strict
- `/design-scan` — style + SOLID checklist with must-fix / nice-to-have / keep-as-is

## Hooks

Hook configuration lives in [`.cursor/hooks.json`](.cursor/hooks.json).

Hooks may auto-format after edit and may run verify on stop. They do not guarantee
correctness. Always run `/verify` explicitly before claiming done.

- `afterFileEdit` → [`.agents/hooks/rustfmt.sh`](.agents/hooks/rustfmt.sh) — formats edited `*.rs` with rustfmt (fail-open)
- `sessionStart` → [`.agents/hooks/session-context.sh`](.agents/hooks/session-context.sh) — injects short workspace context (fail-open)
- `stop` → [`.agents/hooks/verify-on-stop.sh`](.agents/hooks/verify-on-stop.sh) — runs [`.agents/hooks/run-verify.sh`](.agents/hooks/run-verify.sh) on relevant dirty trees (`loop_limit` 3, fail-open)

## Definition of done

A change is complete when:

1. Only intended files changed (review `git diff`).
2. Applicable rules and skills were followed for touched crates.
3. Narrow tests for touched paths passed (see table below).
4. `/verify` was run when the change is merge-ready, or you explicitly report what was skipped and why.
5. README or ARCHITECTURE were updated if behavior or crate contracts changed.

| Touched area | Minimum verification |
| ------------ | -------------------- |
| Any `.rs` / workspace code | `./scripts/check.sh` (or the equivalent failing step while iterating) |
| Merge-ready / `/verify` claim | `./scripts/check.sh`, `./scripts/dry-gate.sh`, llvm-cov `--fail-under-lines 100`, **and** `./scripts/crap-gate.sh` |

- Do not claim a check passed unless it was actually run and passed.
- If verification cannot be completed, clearly state what was not run and why.
