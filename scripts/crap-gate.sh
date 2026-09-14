#!/usr/bin/env bash
# Run crap-rs from crap-score source (no cargo install / crates.io).
# Expects lcov.info in the workspace root (from cargo llvm-cov).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

rev_file="${root}/scripts/crap-score.rev"
pinned_rev="$(tr -d '[:space:]' <"${rev_file}")"
repo_url="https://github.com/deangrant/crap-score.git"

if [ ! -f lcov.info ]; then
    echo "crap-gate: missing lcov.info (run cargo llvm-cov first)" >&2
    exit 1
fi

resolve_crap_score_dir() {
    if [ -n "${CRAP_SCORE_DIR:-}" ]; then
        printf '%s\n' "${CRAP_SCORE_DIR}"
        return
    fi
    if [ -f "${root}/../rust-crap/Cargo.toml" ]; then
        printf '%s\n' "${root}/../rust-crap"
        return
    fi
    if [ -f "${root}/../crap-score/Cargo.toml" ]; then
        printf '%s\n' "${root}/../crap-score"
        return
    fi

    local dest="${root}/.tools/crap-score"
    if [ ! -f "${dest}/Cargo.toml" ]; then
        mkdir -p "${root}/.tools"
        git clone --depth 1 "${repo_url}" "${dest}"
    fi
    git -C "${dest}" fetch --depth 1 origin "${pinned_rev}"
    git -C "${dest}" checkout --detach "${pinned_rev}"
    printf '%s\n' "${dest}"
}

crap_dir="$(resolve_crap_score_dir)"
if [ ! -f "${crap_dir}/Cargo.toml" ]; then
    echo "crap-gate: no Cargo.toml under ${crap_dir}" >&2
    exit 1
fi

cargo run --manifest-path "${crap_dir}/Cargo.toml" -p crap-rs --locked -- \
    --path . \
    --coverage lcov.info \
    --all-features \
    --threshold strict \
    --fail-above
