#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Mutation campaign for the S6 slice (B5 AnimatedContent — the content-keyed
# generalisation of Crossfade). For each mutation: back up, mutate
# (scripts/mutate.py), run the targeted test, record whether it FAILED, restore,
# and verify the restore byte-exact. A mutation the targeted test does not catch
# is a test with no teeth.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s6_mutations.txt
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

COMPOSE=crates/widgets/floom-widgets/src/compose.rs

# 1. a switch stops spawning the arriving side — the departure is never answered,
#    so the scene holds one mounted child mid-switch instead of two.
mutate_one "$COMPOSE" \
"        // A value change: flip the resting entry to exit and spawn the new one.
        if self.value != self.last_value {
            self.last_value = self.value.clone();
            if let Some(active_id) = self.active_id {
                if let Some(slot) = self.entries.iter_mut().find(|s| s.id == active_id) {
                    slot.leaf.inner.when_signal.store(false, Ordering::Relaxed);
                }
            }
            if let Some(new_id) = self.spawn_entry() {
                self.active_id = Some(new_id);
                cx.mark_children_dirty(id);
            }
        }" \
"        // A value change: flip the resting entry to exit. (mutation: arriving side not spawned)
        if self.value != self.last_value {
            self.last_value = self.value.clone();
            if let Some(active_id) = self.active_id {
                if let Some(slot) = self.entries.iter_mut().find(|s| s.id == active_id) {
                    slot.leaf.inner.when_signal.store(false, Ordering::Relaxed);
                }
            }
        }" \
  "cargo test -q -p floom-app --test motion_host an_animated_content_keeps_both_sides_live_while_it_switches" \
  "switch-spawns-the-arriving-side"

# 2. a switch stops flipping the resting side to exit — the departure never
#    leaves, so an instant swap still shows two mounted children.
mutate_one "$COMPOSE" \
"                    slot.leaf.inner.when_signal.store(false, Ordering::Relaxed);" \
"                    slot.leaf.inner.when_signal.store(true, Ordering::Relaxed);" \
  "cargo test -q -p floom-app --test motion_host an_animated_content_swaps_instantly_without_a_transition" \
  "switch-flips-the-resting-side-to-exit"

# 3. the arriving side stops entering — a freshly mounted side is created hidden
#    and never shown, so a returning value never reappears (and never replays).
mutate_one "$COMPOSE" \
"        let leaf = AnimatedContentLeaf::new(child, self.transition, true);" \
"        let leaf = AnimatedContentLeaf::new(child, self.transition, false);" \
  "cargo test -q -p floom-app --test motion_host an_animated_content_replays_enter_when_a_value_returns" \
  "arriving-side-enters"

# A restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — so leave the touched file newer than its last
# artifact. Then *re-run the suite before trusting any count*.
touch "$COMPOSE"

echo "=== campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."
