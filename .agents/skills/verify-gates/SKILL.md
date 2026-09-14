---
name: verify-gates
description: >-
  Run hotspots local gates: check.sh, dry-rs, llvm-cov, then CRAP. Use for
  /verify, stop-hook follow-ups, or when the user asks to run the pipeline.
---

# Verify gates

Canonical ordered procedure for `/verify`. Prefer this skill over ad-hoc
command lists. The stop hook runs the same steps via
[`.agents/hooks/run-verify.sh`](../../hooks/run-verify.sh).

## Procedure

From the workspace root, in order. Stop at the first failure; fix it; restart
from the failed step (or the top).

1. Pipeline (fmt, Clippy, deny, audit, test, rustdoc):

   ```bash
   ./scripts/check.sh
   ```

2. Structural clones (`dry-rs` from [dry-score](https://github.com/deangrant/dry-score)
   source via [`scripts/dry-gate.sh`](../../../scripts/dry-gate.sh); no
   crates.io install):

   ```bash
   ./scripts/dry-gate.sh
   ```

   Resolves the tool tree from `DRY_SCORE_DIR`, else sibling `../dry-score`,
   else a shallow clone of the pin in
   [`scripts/dry-score.rev`](../../../scripts/dry-score.rev) under `.tools/`.
   Fails when any finding is reported (`--fail-on-findings`).

3. Line coverage LCOV (needs `cargo-llvm-cov` and `llvm-tools-preview`):

   ```bash
   cargo llvm-cov --workspace --all-features \
     --lcov --output-path lcov.info \
     --fail-under-lines 100
   ```

4. CRAP gate (`crap-rs` from [crap-score](https://github.com/deangrant/crap-score)
   source via [`scripts/crap-gate.sh`](../../../scripts/crap-gate.sh); no
   crates.io install):

   ```bash
   ./scripts/crap-gate.sh
   ```

   Resolves the tool tree from `CRAP_SCORE_DIR`, else sibling `../rust-crap` or
   `../crap-score`, else a shallow clone of the pin in
   [`scripts/crap-score.rev`](../../../scripts/crap-score.rev) under `.tools/`.
   Fails when any function score is strictly above `--threshold strict` (8).

CI enforces dry via [`.github/workflows/dry.yml`](../../../.github/workflows/dry.yml)
and coverage/CRAP via
[`.github/workflows/coverage.yml`](../../../.github/workflows/coverage.yml).

## Tool pins and dry-rs suppressions

Pins live in one-line rev files:

| Pin | File | Used by |
| --- | ---- | ------- |
| dry-score | [`scripts/dry-score.rev`](../../../scripts/dry-score.rev) | [`dry-gate.sh`](../../../scripts/dry-gate.sh), [`.github/workflows/dry.yml`](../../../.github/workflows/dry.yml) |
| crap-score | [`scripts/crap-score.rev`](../../../scripts/crap-score.rev) | [`crap-gate.sh`](../../../scripts/crap-gate.sh), [`.github/workflows/coverage.yml`](../../../.github/workflows/coverage.yml) |

To bump a pin: set the file to the desired commit SHA of
[dry-score](https://github.com/deangrant/dry-score) or
[crap-score](https://github.com/deangrant/crap-score), then run the matching
gate locally. Do not unpin or float `main` in CI.

**dry-rs findings:** prefer fixing real clones (extract shared helpers). Use a
full-line `// dry-rs:ignore` / `// dry-rs:ignore. reason` (or file-level
`// dry-rs:ignore-file`) only when the parallel shape is intentional (for
example near-identical parse fixtures). Never silence findings by raising
thresholds or skipping the gate.
