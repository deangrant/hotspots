---
name: verify-gates
description: >-
  Run hotspots local gates: check.sh then llvm-cov. Use for /verify,
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

CI enforces the same coverage gate via
[`.github/workflows/coverage.yml`](../../../.github/workflows/coverage.yml).
