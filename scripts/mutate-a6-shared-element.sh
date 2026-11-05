#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Mutation campaign for the A6 slice (shared elements, MOTION-GAP B5/A6).
#
# The shared-element net is only worth having if it actually *bites*: if the
# flight logic is broken and neither the scene-coordinate tests in motion_host
# nor the per-frame pixel golden in floom-snapshot go red, the net is decoration.
# So we break the widget on purpose and watch the right test fail. Four mutations
# hit the novel parts of SharedTransitionLayout: the guard that only animates
# while Flying, the translate that carries the source onto the destination's box,
# the finish→fold that reveals the destination's own content, and the
# phase→signal map that decides which box shows first.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/a6_mutations.txt
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

WIDGET=crates/widgets/floom-widgets/src/compose.rs
SCENE="cargo test -q -p floom-app --test motion_host"
GOLDEN="cargo test -q -p floom-snapshot --test golden_frames a_shared_element_flight_is_golden_frame_by_frame"

# 1. the guard is inverted: animation() now returns None *while* the flight is
#    Flying, so the flight never leaves the source box. The scene-coordinate
#    test that requires the ink to travel through the gap must fail.
mutate_one "$WIDGET" \
"        if self.phase != SharedPhase::Flying {
            return None;
        }" \
"        if self.phase == SharedPhase::Flying {
            return None;
        }" \
"$SCENE a_shared_element_flies_from_source_to_destination" \
"flying-guard-inverted"

# 2. the flight drives the wrong channel: the translate is replaced by an
#    opacity tween, so the ink never travels — every frame stays at the source
#    box. The scene test's "travels through the gap" assertion is the bite.
#    (Corrupting the translate *distance* instead is not caught: the finish→fold
#    reveals the destination at its own box whatever the translate hit, so the
#    landing assertion would read 40 either way.)
mutate_one "$WIDGET" \
"            (0.0, delta)
        } else {
            (0.0, -delta)
        };
        Some(
            floom_core::AnimationSpec::new(from, to, self.duration)
                .with_easing(self.easing)
                .with_key(self.rev)
                .with_prop(CompositorProp::TranslateY(0.0))," \
"            (0.0, delta)
        } else {
            (0.0, -delta)
        };
        Some(
            floom_core::AnimationSpec::new(from, to, self.duration)
                .with_easing(self.easing)
                .with_key(self.rev)
                .with_prop(CompositorProp::Opacity(0.0))," \
"$SCENE a_shared_element_flies_from_source_to_destination" \
"flight-drives-opacity-not-translate"

# 3. the finish never folds: animation_finished forgets to set the flag, so after
#    the flight the source stays shown and the destination is never revealed.
#    The pixel golden's landing frame must show the HOT destination, not the INK
#    source — a Sharp render that keeps the source fails the golden.
mutate_one "$WIDGET" \
"    fn animation_finished(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {
        self.finished = true;
    }" \
"    fn animation_finished(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {
        self.finished = false;
    }" \
"$GOLDEN" \
"finish-never-folds"

# 4. the construction defaults let both boxes show at rest: the destination's
#    signal starts true (as well as the source's), so the resting scene has two
#    texts. The scene test's at-rest "one text" guard is the bite.
mutate_one "$WIDGET" \
"            signal_source: Arc::new(AtomicBool::new(true)),
            signal_dest: Arc::new(AtomicBool::new(false))," \
"            signal_source: Arc::new(AtomicBool::new(true)),
            signal_dest: Arc::new(AtomicBool::new(true))," \
"$SCENE a_shared_element_flies_from_source_to_destination" \
"dest-shown-at-rest"

# A restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — so leave the touched files newer than their last
# artifact. Then *re-run the suite before trusting any count*.
touch "$WIDGET"

echo "=== campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."