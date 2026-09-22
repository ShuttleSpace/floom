#!/usr/bin/env bash
# Mutation campaign for the S8 slice (B4 opacity — the implicit transition
# extended from colour to the compositor opacity channel). For each mutation:
# back up, mutate (scripts/mutate.py), run the targeted test, record whether it
# FAILED, restore, and verify the restore byte-exact. A mutation the targeted
# test does not catch is a test with no teeth.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s8_mutations.txt
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

COMPOSE=widgets/floem-widgets/src/compose.rs

# 1. a transition that returns no longer starts from where it rested — `from` is
#    left at its initial value, so the 0.5 -> 1.0 leg snaps to full opacity
#    instead of fading back up.
mutate_one "$COMPOSE" \
"            // A transition starts from the previously rested value. Without a
            // transition the change still needs to reach the compositor, so the
            // key is bumped and the declaration re-synced in every case.
            if self.transition.is_some() {
                self.from = self.last_target;
            }
            self.rev = self.rev.wrapping_add(1);
            self.last_target = self.target;
            // Re-sync the declared animation on this node so the host restarts
            // (or parks the new instant value for) the compositor tween.
            cx.mark_dirty(id);" \
"            // A transition starts from the previously rested value. Without a
            // transition the change still needs to reach the compositor, so the
            // key is bumped and the declaration re-synced in every case.
            self.rev = self.rev.wrapping_add(1);
            self.last_target = self.target;
            // Re-sync the declared animation on this node so the host restarts
            // (or parks the new instant value for) the compositor tween.
            cx.mark_dirty(id);" \
  "cargo test -q -p floem-app --test motion_host an_opacity_transition_reverses_to_its_old_value_when_it_returns" \
  "transition-starts-from-rested-value"

# 2. the transition is ignored entirely — every change snaps, so the 1.0 -> 0.5
#    leg is instantly 0.5 and never animates.
mutate_one "$COMPOSE" \
"        let (from, to, duration) = match self.transition {
            Some(t) => (f64::from(self.from), f64::from(self.target), t.duration_secs),
            // No transition: an instant tween whose endpoints coincide parks
            // the static value on the compositor.
            None => (f64::from(self.target), f64::from(self.target), 0.0),
        };" \
"        let (from, to, duration) = (f64::from(self.target), f64::from(self.target), 0.0);" \
  "cargo test -q -p floem-app --test motion_host an_opacity_transition_fades_from_its_old_value_to_the_new_one" \
  "transition-is-observed"

# 3. the declared animation is never re-synced on the node, so a change of
#    target never reaches the compositor and the subtree stays at full opacity.
mutate_one "$COMPOSE" \
"            // Re-sync the declared animation on this node so the host restarts
            // (or parks the new instant value for) the compositor tween.
            cx.mark_dirty(id);" \
"            // Re-sync the declared animation on this node so the host restarts
            // (or parks the new instant value for) the compositor tween." \
  "cargo test -q -p floem-app --test motion_host an_opacity_transition_fades_from_its_old_value_to_the_new_one" \
  "declared-animation-is-re-synced"

# A restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — so leave the touched file newer than its last
# artifact. Then *re-run the suite before trusting any count*.
touch "$COMPOSE"

echo "=== campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."
