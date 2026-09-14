#!/usr/bin/env bash
# Run dry-rs from dry-score source (no cargo install / crates.io).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

rev_file="${root}/scripts/dry-score.rev"
pinned_rev="$(tr -d '[:space:]' <"${rev_file}")"
repo_url="https://github.com/deangrant/dry-score.git"

resolve_dry_score_dir() {
    if [ -n "${DRY_SCORE_DIR:-}" ]; then
        printf '%s\n' "${DRY_SCORE_DIR}"
        return
    fi
    if [ -f "${root}/../dry-score/Cargo.toml" ]; then
        printf '%s\n' "${root}/../dry-score"
        return
    fi

    local dest="${root}/.tools/dry-score"
    if [ ! -f "${dest}/Cargo.toml" ]; then
        mkdir -p "${root}/.tools"
        git clone --depth 1 "${repo_url}" "${dest}"
    fi
    git -C "${dest}" fetch --depth 1 origin "${pinned_rev}"
    git -C "${dest}" checkout --detach "${pinned_rev}"
    printf '%s\n' "${dest}"
}

dry_dir="$(resolve_dry_score_dir)"
if [ ! -f "${dry_dir}/Cargo.toml" ]; then
    echo "dry-gate: no Cargo.toml under ${dry_dir}" >&2
    exit 1
fi

cargo run --manifest-path "${dry_dir}/Cargo.toml" -p dry-rs --locked -- \
    . \
    --exclude target,.git,fixtures,tests,.tools \
    --fail-on-findings
