#!/usr/bin/env bash
# Mutation campaign for the T1 slice (the terminal backend: scene -> character
# grid -> ANSI). Same discipline as the other campaigns: back up, mutate
# (scripts/mutate.py), run the targeted test, record whether it went red,
# restore, and verify the restore byte-exact. A mutation no test catches is a
# test with no teeth — and so is a mutation that silently failed to apply.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"

RESULTS=/tmp/tui_mutations.txt
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
  # Cargo fingerprints on mtime; a byte-identical restore can still leave the
  # mutated binary in place for the next run.
  touch "$file"
}

UNIT="cargo test -q -p floem-renderer-tui --lib"
E2E="cargo test -q -p floem-app --test tui_backend"
GRID=render/floem-renderer-tui/src/grid.rs
ANSI=render/floem-renderer-tui/src/ansi.rs

# 1. Every column is off by one. If nothing pins where a glyph lands, the whole
#    grid could shift a cell and still look plausible.
mutate_one "$GRID" \
"        let col = (x / self.cell_size.width) as usize;" \
"        let col = ((x + self.cell_size.width) / self.cell_size.width) as usize;" \
"$UNIT" "grid: col_of_off_by_one"

# 2. Text writes its colour to the background instead of the foreground. The
#    characters would still be readable, which is exactly why it needs a test.
mutate_one "$GRID" \
"        cell.fg = Some(composite(fg, cell.fg.unwrap_or(Color::TRANSPARENT)));" \
"        cell.bg = Some(composite(fg, cell.bg.unwrap_or(Color::TRANSPARENT)));" \
"$UNIT" "grid: put_writes_bg_instead_of_fg"

# 3. Alpha is ignored, so every translucent panel paints as if opaque.
mutate_one "$GRID" \
"    if src.a == u8::MAX {" \
"    if true {" \
"$UNIT" "grid: alpha_ignored_in_composite"

# 4. The glyph column is mirrored to the negative half-plane, so every glyph
#    falls outside the grid. End-to-end this must not pass as "nothing to draw".
mutate_one "$GRID" \
"                        let x = origin.x + run.origin.x + f64::from(glyph.x);" \
"                        let x = -(origin.x + run.origin.x + f64::from(glyph.x));" \
"$E2E" "grid: glyph_x_negated (e2e)"

# 5. A rect fills only its top-left cell.
mutate_one "$GRID" \
"        let x1 = ((rect.x1 / self.cell_size.width).ceil() as isize).max(0) as usize;" \
"        let x1 = ((rect.x0 / self.cell_size.width).ceil() as isize).max(0) as usize + 1;" \
"$UNIT" "grid: rect_fills_one_cell"

# 6. Foreground is emitted through the background SGR channel.
mutate_one "$ANSI" \
"        Some(c) => truecolor(38, c)," \
"        Some(c) => truecolor(48, c)," \
"$UNIT" "ansi: fg_uses_bg_channel"

# 7. Rows stop being reset, so a colour bleeds into the next row.
mutate_one "$ANSI" \
"        out.push_str(RESET);" \
"        let _ = RESET;" \
"$UNIT" "ansi: no_per_row_reset"

echo "=== TUI mutation campaign ==="
cat "$RESULTS"
