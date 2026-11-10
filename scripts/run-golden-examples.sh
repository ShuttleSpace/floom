#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Run the Golden Examples (docs/2.0/EXAMPLES.md) and fail on any regression.
#
# `cargo build --workspace` only *compiles* the examples; their assertions are
# the whole point of a golden example, and nothing would run them. This script
# does. It is named by docs/2.0/RELEASE-POLICY.md as a release gate, and
# crates/app/floom/tests/docs_parity.rs asserts that the list below is exactly the
# set of headless examples the index advertises — so neither can drift.
#
# Usage:  bash scripts/run-golden-examples.sh
#
# Device-/target-gated examples are deliberately absent: they need OS providers
# or hardware that CI does not have, and `floom-window`/`floom-probe` need a
# display.

set -euo pipefail

# Keep the list alphabetical and in sync with docs/2.0/EXAMPLES.md.
EXAMPLES=(
  # Matches the headless column of docs/2.0/EXAMPLES.md exactly.
  # `crates/app/floom/tests/docs_parity.rs` asserts the two lists are equal, so a
  # rename on either side fails here rather than at run time.
  floom-native-plugin
  floom-showcase
)

failures=0

run_example() {
  local example=$1
  shift
  printf '=== %s ===\n' "$example"
  if output=$(cargo run -q -p "$example" -- "$@" 2>&1); then
    printf '%s\n' "$output"
  else
    printf '%s\n' "$output"
    printf 'FAILED: %s\n' "$example" >&2
    failures=$((failures + 1))
  fi
}

for example in "${EXAMPLES[@]}"; do
  case "$example" in
    floom-showcase)
      # The same binary opens the window with no arguments; the golden run
      # pumps a headless host so CI does not need a display.
      run_example "$example" --headless
      ;;
    *)
      run_example "$example"
      ;;
  esac
done

if [ "$failures" -ne 0 ]; then
  printf '\n%d golden example(s) failed\n' "$failures" >&2
  exit 1
fi

printf '\nall %d golden examples passed\n' "${#EXAMPLES[@]}"
