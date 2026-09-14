#!/usr/bin/env bash
# Same ordered gates as .agents/skills/verify-gates/SKILL.md.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

./scripts/check.sh

./scripts/dry-gate.sh

cargo llvm-cov --workspace --all-features \
  --lcov --output-path lcov.info \
  --fail-under-lines 100

./scripts/crap-gate.sh
