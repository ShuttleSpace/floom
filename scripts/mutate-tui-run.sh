#!/usr/bin/env bash
# Mutation campaign for the TUI real-run-loop (crates/floom-platform-tui/src/run.rs
# + the resize/exit hooks in lib.rs). Same discipline as T1/T2: back up, mutate
# (scripts/mutate.py), run the targeted lib test, record whether it went red,
# restore byte-exact. A mutation no test catches is a test with no teeth.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

RESULTS=/tmp/tui_run_mutations.txt
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
  elif echo "$out" | grep -qE "test result: ok"; then
    echo "SURVIVED  $label" >> "$RESULTS"
  elif echo "$out" | grep -qE "error\[|could not compile"; then
    echo "COMPILE_ERROR  $label" >> "$RESULTS"
  else
    echo "NO_OUTPUT  $label" >> "$RESULTS"
  fi
  cp /tmp/mut_backup.rs "$file"
  cmp -s /tmp/mut_backup.rs "$file" || echo "RESTORE_FAILED  $label" >> "$RESULTS"
  touch "$file"
}

UNIT="cargo test -q -p floom-platform-tui --lib"
LIB=crates/floom-platform-tui/src/lib.rs
RUN=crates/floom-platform-tui/src/run.rs

# 1. resize: stop emitting WindowResized, so a terminal resize never reaches the
#    runner and the grid is never rebuilt.
mutate_one "$LIB" \
"        if let Some(window) = self.window {
            self.host.push_event(PlatformEvent::WindowResized {
                window,
                size: Size::new(size.cols as f64, size.rows as f64),
            });
        }" \
"        if let Some(window) = self.window {
            let _ = window;
        }" \
"$UNIT" "resize: no_window_resized_event"

# 2. strip_quit_bytes: Ctrl-C is no longer a quit — it would leak into the tree.
mutate_one "$RUN" \
"        if b == 0x03 || b == 0x04 {" \
"        if b == 0x04 {" \
"$UNIT" "strip: ctrl_c_not_a_quit"

# 3. strip_quit_bytes: Ctrl-D is no longer a quit.
mutate_one "$RUN" \
"        if b == 0x03 || b == 0x04 {" \
"        if b == 0x03 {" \
"$UNIT" "strip: ctrl_d_not_a_quit"

echo "=== TUI run-loop mutation campaign ==="
cat "$RESULTS"
