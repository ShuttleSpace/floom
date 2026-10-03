#!/usr/bin/env bash
# Run the Golden Examples (docs/2.0/EXAMPLES.md) and fail on any regression.
#
# `cargo build --workspace` only *compiles* the examples; their assertions are
# the whole point of a golden example, and nothing would run them. This script
# does. It is named by docs/2.0/RELEASE-POLICY.md as a release gate, and
# crates/floom2/tests/docs_parity.rs asserts that the list below is exactly the
# set of headless examples the index advertises — so neither can drift.
#
# Usage:  bash scripts/run-golden-examples.sh
#
# Device-/target-gated examples (webview, video, camera, harmony, web,
# embedded) are deliberately absent: they need OS providers or hardware that
# CI does not have, and `floom2-window` needs a display.

set -euo pipefail

# Keep the list alphabetical and in sync with docs/2.0/EXAMPLES.md.
EXAMPLES=(
  floom2-calculator
  floom2-capabilities
  floom2-chart
  floom2-counter
  floom2-datagrid
  floom2-demo
  floom2-editor
  floom2-forms
  floom2-hello
  floom2-list
  floom2-motion
  floom2-native-plugin
  floom2-navigation
  floom2-progress-ring
  floom2-shader
  floom2-style
  floom2-transform
  floom2-virtual-list
)

failures=0

for example in "${EXAMPLES[@]}"; do
  printf '=== %s ===\n' "$example"
  if output=$(cargo run -q -p "$example" 2>&1); then
    printf '%s\n' "$output"
  else
    printf '%s\n' "$output"
    printf 'FAILED: %s\n' "$example" >&2
    failures=$((failures + 1))
  fi
done

if [ "$failures" -ne 0 ]; then
  printf '\n%d golden example(s) failed\n' "$failures" >&2
  exit 1
fi

printf '\nall %d golden examples passed\n' "${#EXAMPLES[@]}"
