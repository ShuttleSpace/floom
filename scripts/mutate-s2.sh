#!/usr/bin/env bash
# Mutation campaign for the S2 (self-drawing) slice.
# For each mutation: back up, mutate (scripts/mutate.py), run the targeted
# test, record whether it FAILED, restore, and verify the restore byte-exact.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s2_mutations.txt
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
}

TYPES=crates/floom-types/src/path.rs
RASTER=crates/floom-renderer-software/src/raster.rs
SWLIB=crates/floom-renderer-software/src/lib.rs
APPLIB=crates/floom-app/src/lib.rs
CANVAS=crates/floom-widgets/src/canvas.rs
SKIA=crates/floom-renderer-skia-abi/src/lib.rs

# 1. stroke outline: drop contour orientation normalisation
mutate_one "$TYPES" \
  "    if signed_area(&points) < 0.0 {
        points.reverse();
    }" \
  "    if false {
        points.reverse();
    }" \
  "cargo test -q -p floom-types" \
  "stroke-winding-normalisation"

# 2. resolve_clip: rounded clip degrades to a square one
mutate_one "$TYPES" \
  "                    radius: *radius,
                }
            }
        }
        ClipShape::Path(path) => ClipRegion::Path {" \
  "                    radius: 0.0,
                }
            }
        }
        ClipShape::Path(path) => ClipRegion::Path {" \
  "cargo test -q -p floom-types" \
  "resolve-clip-rounded-radius"

# 3. rasteriser: winding stops accumulating across spans
mutate_one "$RASTER" \
  "                winding += pair[0].1;" \
  "                winding = pair[0].1;" \
  "cargo test -q -p floom-renderer-software" \
  "raster-winding-accumulation"

# 4. clip stack: rounded masks skipped (treated as rect-only)
mutate_one "$SWLIB" \
  "        if !self.clips.is_rect_only() {" \
  "        if false {" \
  "cargo test -q -p floom-renderer-software" \
  "clipstack-rounded-mask-applied"

# 5. clip stack: walk-start clear removed (leak into next walk)
mutate_one "$SWLIB" \
  "        self.clips.clear();" \
  "        ;" \
  "cargo test -q -p floom-renderer-software" \
  "clipstack-clear-at-walk-start"

# 6. host: clip pop never written (bracket left open)
mutate_one "$APPLIB" \
  "    fn write_clip_pop(&mut self, index: usize) {" \
  "    fn write_clip_pop(&mut self, index: usize) {
        if true {
            return;
        }" \
  "cargo run -q -p floom2-chart" \
  "host-clip-pop-written"

# 7. host: fill_path drops the node
mutate_one "$APPLIB" \
  "        self.push(floom_scene::SceneNode::filled_path(path.clone(), color));" \
  "        let _ = (path, color);" \
  "cargo run -q -p floom2-chart" \
  "host-fill-path-emits-node"

# 8. canvas: default semantics role stops being Image
mutate_one "$CANVAS" \
  "            role: SemanticsRole::Image," \
  "            role: SemanticsRole::None," \
  "cargo test -q -p floom-widgets --lib" \
  "canvas-default-role-image"

# 9. spinner: sweep ignores the animation value
mutate_one "$CANVAS" \
  "        let sweep = std::f64::consts::TAU * (0.15 + 0.75 * progress);" \
  "        let sweep = std::f64::consts::TAU * 0.5;" \
  "cargo test -q -p floom-widgets --lib" \
  "spinner-sweep-reads-animation-value"

# 10. skia: pop clip recorded as nothing
mutate_one "$SKIA" \
  "                SceneNode::ClipPop => self.commands.push(SkiaCommand::PopClip)," \
  "                SceneNode::ClipPop => {}," \
  "cargo test -q -p floom-renderer-skia-abi" \
  "skia-pop-clip-recorded"

echo "=== campaign results ==="
cat "$RESULTS"
