#!/usr/bin/env bash
# Mutation campaign for the T2 slice (the terminal platform layer + diff
# redraw). Same discipline as T1: back up, mutate (scripts/mutate.py), run the
# targeted test, record whether it went red, restore byte-exact. A mutation no
# test catches is a test with no teeth.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

RESULTS=/tmp/tui_t2_mutations.txt
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

UNIT="cargo test -q -p floem-platform-tui --lib"
DIFF="cargo test -q -p floem-renderer-tui --lib"
ANSI=render/floem-renderer-tui/src/ansi.rs
INPUT=platform/floem-platform-tui/src/input.rs

# 1. Diff: drop the leading reset that establishes a known colour baseline.
#    (The bare `out.push_str(RESET);` also appears in encode_with_stats, so the
#    unique anchor is the `let mut cur_fg` that immediately follows it here.)
mutate_one "$ANSI" \
"    out.push_str(RESET);
    let mut cur_fg: Option<Color> = None;" \
"    let mut cur_fg: Option<Color> = None;" \
"$DIFF" "ansi: diff_drops_leading_reset"

# 2. Diff: never skip unchanged cells, so an identical frame repaints all.
mutate_one "$ANSI" \
"            if nc == oc {" \
"            if false {" \
"$DIFF" "ansi: diff_skips_nothing"

# 3. Diff: forget to move the cursor to the changed cell.
mutate_one "$ANSI" \
"            out.push_str(&format!(\"\u{1b}[{};{}H\", row + 1, col + 1));" \
"            let _ = (row, col);" \
"$DIFF" "ansi: diff_omits_cursor_move"

# 4. Diff: miscount changed cells (always report zero).
mutate_one "$ANSI" \
"            changed += 1;" \
"            let _ = 1;" \
"$DIFF" "ansi: diff_undercounts_changes"

# 5. Parser: text bytes go through Key instead of ImeCommit, dropping CJK.
mutate_one "$INPUT" \
"                if mods.is_empty() {
                    Some(Outcome::Text(c))
                } else {" \
"                if mods.is_empty() {
                    Some(Outcome::Key(KeyCode::from_char(c), mods))
                } else {" \
"$UNIT" "input: ascii_text_not_committed"

# 6. Parser: Latin1/control bytes below 0x80 are mis-routed into the UTF-8 path.
mutate_one "$INPUT" \
"                if b < 0x80 {
                    if let Some(o) = Self::map_ascii(b, Modifiers::NONE) {
                        emit(out, window, o);
                    }
                } else {" \
"                if b < 0x80 {
                    self.utf8.push(b);
                    if let Some(ch) = decode_utf8(&mut self.utf8) {
                        out.push(PlatformEvent::ImeCommit { window, text: ch.to_string() });
                    }
                } else {" \
"$UNIT" "input: ascii_routed_into_utf8"

# 7. Parser: arrow-up is mis-decoded as arrow-down.
mutate_one "$INPUT" \
"        b'A' => KeyCode::ARROW_UP," \
"        b'A' => KeyCode::ARROW_DOWN," \
"$UNIT" "input: arrow_up_misdecoded"

echo "=== TUI T2 mutation campaign ==="
cat "$RESULTS"
