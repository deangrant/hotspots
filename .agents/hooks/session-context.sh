#!/usr/bin/env bash
# sessionStart: inject short workspace context (fail-open).
set -eu

cat >/dev/null || true

printf '%s\n' '{
  "additional_context": "hotspots: /verify runs check.sh then llvm-cov (see verify-gates). /design-scan uses rust-style-guide + rust-solid-design. Read hotspots-domain before changing parsers, analyses, grain, or CLI."
}'
exit 0
