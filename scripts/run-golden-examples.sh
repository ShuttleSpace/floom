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
  # Matches the headless column of docs/2.0/EXAMPLES.md exactly.
  # `crates/floom/tests/docs_parity.rs` asserts the two lists are equal, so a
  # rename on either side fails here rather than at run time.
  #
  # `floom2-calculator` was in this list until now and has no directory.
  #
  # The four `-demo` names carry a suffix to clear a collision with a crate of
  # the same name -- floom-editor, floom-forms, floem-navigation, floom-style
  # are all crates in this workspace.
  floom-capabilities
  floom-chart
  floom-counter
  floom-datagrid
  floom-demo
  floom-editor-demo
  floom-forms-demo
  floom-hello
  floom-list
  floom-motion
  floom-native-plugin
  floom-navigation-demo
  floom-progress-ring
  floom-shader
  floom-style-demo
  floom-transform
  floom-virtual-list
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
