#!/usr/bin/env bash
# Mutation campaign for the S5 slice (B5 presence transitions).
# For each mutation: back up, mutate (scripts/mutate.py), run the targeted
# test, record whether it FAILED, restore, and verify the restore byte-exact.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s5_mutations.txt
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
MOTION=foundation/floem-types/src/motion.rs

# 1. the exit arm stops keeping the child mounted — the departure is invisible
mutate_one "$COMPOSE" \
  "        Some(child) if self.phase != PresencePhase::Hidden => vec![child.clone()]," \
  "        Some(child) if self.phase != PresencePhase::Hidden && self.phase != PresencePhase::Exiting => vec![child.clone()]," \
  "cargo test -q -p floem-app --test motion_host presence" \
  "exit-keeps-the-child-mounted"

# 2. the host's "it finished" flag is never folded in — an exit never ends
mutate_one "$COMPOSE" \
  "        if self.finished {" \
  "        if false {" \
  "cargo test -q -p floem-app --test motion_host presence" \
  "finished-transition-is-folded-in"

# 3. animation_finished stops flagging completion (the event door is deaf)
mutate_one "$COMPOSE" \
  "        self.finished = true;" \
  "        self.finished = false;" \
  "cargo test -q -p floem-app --test motion_host presence" \
  "animation-finished-flags-the-end"

# 4. a re-appearance stops replaying its enter (the restart key is not bumped)
mutate_one "$COMPOSE" \
  "                match self.enter {
                    Some(transition) if !transition.is_instant() => {
                        self.phase = PresencePhase::Entering;
                        self.rev = self.rev.wrapping_add(1);
                    }" \
  "                match self.enter {
                    Some(transition) if !transition.is_instant() => {
                        self.phase = PresencePhase::Entering;
                    }" \
  "cargo test -q -p floem-app --test motion_host presence" \
  "reappearance-bumps-the-restart-key"

# 5. the presence leaf stops reading its signal — a crossfade never swaps
mutate_one "$COMPOSE" \
  "        let want = self.when_signal.load(Ordering::Relaxed);" \
  "        let want = true;" \
  "cargo test -q -p floem-app --test motion_host crossfade" \
  "presence-leaf-reads-its-signal"

# 6. the crossfade stops flipping one of its signals — that side never arrives
mutate_one "$COMPOSE" \
  "            self.signal_b.store(!self.show_a, Ordering::Relaxed);" \
  "            self.signal_b.store(false, Ordering::Relaxed);" \
  "cargo test -q -p floem-app --test motion_host crossfade" \
  "crossfade-flips-both-signals"

# 7. the endpoints stop being reversed on the way out — an exit fades *in*
mutate_one "$MOTION" \
  "        let (from, to) = if entering { (from, to) } else { (to, from) };" \
  "        let (from, to) = if entering { (from, to) } else { (from, to) };" \
  "cargo test -q -p floem-types --lib" \
  "exit-endpoints-are-reversed"

# 8. a zero-duration presence stops counting as instant
mutate_one "$MOTION" \
  "        self.duration_secs <= 0.0" \
  "        false" \
  "cargo test -q -p floem-types --lib" \
  "zero-duration-presence-is-instant"

# 9. the golden example's own claims: run it against an exit that unmounts now
mutate_one "$COMPOSE" \
  "        Some(child) if self.phase != PresencePhase::Hidden => vec![child.clone()]," \
  "        Some(child) if self.phase != PresencePhase::Hidden && self.phase != PresencePhase::Exiting => vec![child.clone()]," \
  "cargo run -q -p floem2-motion" \
  "example-presence-act-has-teeth"

echo "=== campaign results ==="
cat "$RESULTS"
