#!/usr/bin/env bash
# Mutation campaign for the B9 slice (per-frame pixel goldens, MOTION-GAP B9).
#
# The golden net is only worth having if it actually *bites*: if a frame
# changes and the golden does not go red, the net is decoration. So we break
# the thing under test on purpose and watch the right test fail. Four mutations
# hit the novel comparison/freeze logic in crates/floom-snapshot/src/frames.rs
# (these are the parts a plain hash could never have caught), and one hits the
# pixel layer the net guards — disabling the software blur, which must fail the
# blurred-card golden. A mutation a test does not catch is a test with no teeth.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/b9_mutations.txt
: > "$RESULTS"

mutate_one() {
  local file="$1" old="$2" new="$3" cmd="$4" label="$5"
  cp "$file" /tmp/mut_backup.rs
  if ! scripts/mutate.py "$file" "$old" "$new" >/dev/null 2>&1; then
    echo "MUTATION_DID_NOT_LAND  $label" >> "$RESULTS"
    cp /tmp/mut_backup.rs "$file"
    return
  fi
  local out
  out=$(eval "$cmd" 2>&1)
  if echo "$out" | grep -qE "\.\.\. FAILED|panicked at"; then
    local first
    first=$(echo "$out" | grep -E "\.\.\. FAILED|panicked at" | head -1 | sed 's/^ *//')
    echo "KILLED  $label  [$first]" >> "$RESULTS"
  elif echo "$out" | grep -qE "test result: ok|asserted, not printed"; then
    echo "SURVIVED  $label" >> "$RESULTS"
  elif echo "$out" | grep -qE "error\[|could not compile"; then
    echo "COMPILE_ERROR  $label" >> "$RESULTS"
  else
    echo "NO_OUTPUT  $label" >> "$RESULTS"
  fi
  cp /tmp/mut_backup.rs "$file"
  cmp -s /tmp/mut_backup.rs "$file" || echo "RESTORE_FAILED  $label" >> "$RESULTS"
  # Cargo fingerprints on mtime; a byte-identical restore can still leave the
  # mutated binary in place for the next run.
  touch "$file"
}

FRAMES=crates/floom-snapshot/src/frames.rs
SW=crates/floom-renderer-software/src/lib.rs

# 1. frozen() lies and says every sequence moves — a still would pass as an
#    animation, so the test that asserts a moving sequence is NOT frozen must go.
mutate_one "$FRAMES" \
"            .all(|(_, d)| !d.diff(first, self.tol).moved())" \
"            .all(|(_, _)| true)" \
"cargo test -q -p floom-snapshot --lib a_sequence_that_moves_is_not_frozen" \
"frozen-lies-and-says-moving"

# 2. the other way: frozen() lies and says nothing moves — a still would pass as
#    a still (it should) AND an animation would too, so the test that asserts a
#    still IS frozen must go.
mutate_one "$FRAMES" \
"            .all(|(_, d)| !d.diff(first, self.tol).moved())" \
"            .all(|(_, _)| false)" \
"cargo test -q -p floom-snapshot --lib a_sequence_that_never_moves_is_frozen" \
"frozen-lies-and-says-still"

# 3. the tolerance stops being a tolerance: nothing is ever "over", so a change
#    far outside the budget is swallowed and the catch test must fail.
mutate_one "$FRAMES" \
"        if d > tol {" \
"        if d > 255 {" \
"cargo test -q -p floom-snapshot --lib a_shift_far_outside_the_tolerance_is_caught" \
"tolerance-swallowed"

# 4. the alpha-weighted averaging stops premultiplying: a half-covered block now
#    averages the clear pixels' (0,0,0) in and reports the wrong colour, so the
#    block test must fail.
mutate_one "$FRAMES" \
"                } else {
                    [(sr / sa) as u8, (sg / sa) as u8, (sb / sa) as u8]
                };" \
"                } else {
                    [(sr / count) as u8, (sg / count) as u8, (sb / count) as u8]
                };" \
"cargo test -q -p floom-snapshot --lib a_transparent_block_reports_no_colour" \
"premultiply-dropped"

# 5. the pixel layer the net guards: disable the software blur entirely. The
#    blurred-card golden was recorded WITH blur, so a sharp render of the same
#    fixture must miss the golden and the test must fail.
mutate_one "$SW" \
"    if w == 0 || h == 0 || (rx == 0 && ry == 0) {" \
"    if true {" \
"cargo test -q -p floom-snapshot --test golden_frames a_fading_blurred_card_is_golden_frame_by_frame" \
"blur-disabled-pixel-layer"

# A restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — so leave the touched files newer than their last
# artifact. Then *re-run the suite before trusting any count*.
touch "$FRAMES" "$SW"

echo "=== campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."
