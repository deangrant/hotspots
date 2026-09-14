---
name: verify-gates
description: >-
  Run hotspots local gates: check.sh, llvm-cov, then CRAP. Use for /verify,
  stop-hook follow-ups, or when the user asks to run the pipeline.
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

2. Line coverage LCOV (needs `cargo-llvm-cov` and `llvm-tools-preview`):

   ```bash
   cargo llvm-cov --workspace --all-features \
     --lcov --output-path lcov.info \
     --fail-under-lines 100
   ```

3. CRAP gate (`crap-rs` from [crap-score](https://github.com/deangrant/crap-score)
   source via [`scripts/crap-gate.sh`](../../../scripts/crap-gate.sh); no
   crates.io install):

   ```bash
   ./scripts/crap-gate.sh
   ```

   Resolves the tool tree from `CRAP_SCORE_DIR`, else sibling `../rust-crap` or
   `../crap-score`, else a shallow clone of the pin in
   [`scripts/crap-score.rev`](../../../scripts/crap-score.rev) under `.tools/`.
   Fails when any function score is strictly above `--threshold strict` (8).

CI enforces the same coverage and CRAP gates via
[`.github/workflows/coverage.yml`](../../../.github/workflows/coverage.yml).
