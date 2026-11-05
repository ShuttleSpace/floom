#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Mutation campaign for the S4 slice (brushes A3, shadows A4, implicit
# transitions B4, motion tokens B6).
# For each mutation: back up, mutate (scripts/mutate.py), run the targeted
# test, record whether it FAILED, restore, and verify the restore byte-exact.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s4_mutations.txt
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

BRUSH=crates/core/floom-types/src/brush.rs
MOTION=crates/core/floom-types/src/motion.rs
SWLIB=crates/render/floom-renderer-software/src/lib.rs
APPLIB=crates/app/floom-app/src/lib.rs
PRIMS=crates/widgets/floom-widgets/src/primitives.rs
CTRLS=crates/widgets/floom-widgets/src/controls.rs
VELLO=crates/render/floom-renderer-vello/src/lib.rs
MATERIAL=crates/floom-material/src/scale.rs

# 1. a linear gradient samples its first stop everywhere (the ramp collapses)
mutate_one "$BRUSH" \
  "                    (((p.x - from.x) * axis.x + (p.y - from.y) * axis.y) / length_sq)
                        .clamp(0.0, 1.0)" \
  "                    0.0" \
  "cargo test -q -p floom-types" \
  "linear-gradient-sampling"

# 2. a radial gradient samples its first stop everywhere
mutate_one "$BRUSH" \
  "                    (p.distance_to(*center) / radius).clamp(0.0, 1.0)" \
  "                    0.0" \
  "cargo test -q -p floom-types" \
  "radial-gradient-sampling"

# 3. stops stop clamping: a gradient wraps instead of holding its edge colours
mutate_one "$BRUSH" \
  "    if t >= last.0 {
        return last.1;
    }" \
  "    if t >= last.0 {
        return first.1;
    }" \
  "cargo test -q -p floom-types" \
  "stops-clamp-past-the-last"

# 4. a shadow's reach forgets the offset — damage would clip the silhouette
mutate_one "$BRUSH" \
  "        self.offset_x.abs().max(self.offset_y.abs()) + self.blur" \
  "        self.blur" \
  "cargo test -q -p floom-scene --tests" \
  "shadow-reach-includes-the-offset"

# 5. software: the shadow composites with no offset (blur in place)
mutate_one "$SWLIB" \
  "                let sx = local.x * scale - off_x - f64::from(ix0);
                let sy = local.y * scale - off_y - f64::from(iy0);" \
  "                let sx = local.x * scale - f64::from(ix0);
                let sy = local.y * scale - f64::from(iy0);" \
  "cargo test -q -p floom-renderer-software --test brush" \
  "sw-shadow-offset-applied"

# 6. software: no blur passes — a hard-edged silhouette pretending to be a shadow
mutate_one "$SWLIB" \
  "        box_blur(&mut alpha, w, h, br);
        box_blur(&mut alpha, w, h, br);
        box_blur(&mut alpha, w, h, br);" \
  "        let _ = br;" \
  "cargo test -q -p floom-renderer-software --test brush" \
  "sw-shadow-blur-passes"

# 7. software: gradients are sampled once at the shape instead of per pixel
mutate_one "$SWLIB" \
  "            return paint.sample(in_scope);" \
  "            return paint.sample(Point::new(0.0, 0.0));" \
  "cargo test -q -p floom-renderer-software --test brush" \
  "sw-gradient-sampled-per-pixel"

# 8. host: draw_shadow drops the request on the floor
mutate_one "$APPLIB" \
  "            shadow: Some(*shadow),
        });
    }" \
  "            shadow: None,
        });
    }" \
  "cargo test -q -p floom-app --test style" \
  "host-draw-shadow-carries-the-request"

# 9. host: a brush fill is written as one sampled colour (the gradient dies at
#    the widget door, which is exactly the A3 gap)
mutate_one "$APPLIB" \
  "    fn fill_rect_brush(&mut self, rect: Rect, brush: &floom_types::Brush) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: brush.clone()," \
  "    fn fill_rect_brush(&mut self, rect: Rect, brush: &floom_types::Brush) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: floom_types::Brush::Solid(brush.sample(rect.center()))," \
  "cargo test -q -p floom-app --test style" \
  "host-brush-fill-carries-the-brush"

# 10. Container forgets to cast the shadow it was given
mutate_one "$PRIMS" \
  "        if let Some(shadow) = self.shadow {
            cx.draw_shadow(bounds, self.radius, &shadow);
        }" \
  "        if let Some(_shadow) = self.shadow {
        }" \
  "cargo test -q -p floom-widgets --test primitives" \
  "container-casts-its-shadow"

# 11. Container collapses a gradient to its mid colour before painting it
mutate_one "$PRIMS" \
  "            None if self.radius > 0.0 => cx.fill_rounded_rect_brush(bounds, self.radius, fill),
            None => cx.fill_rect_brush(bounds, fill)," \
  "            None if self.radius > 0.0 => {
                let colour = fill.sample(bounds.center());
                cx.fill_rounded_rect(bounds, self.radius, colour)
            }
            None => cx.fill_rect(bounds, fill.sample(bounds.center()))," \
  "cargo test -q -p floom-widgets --test primitives" \
  "container-keeps-the-ramp"

# 12. Progress re-derives its ink from the flat colour, dropping the gradient
mutate_one "$CTRLS" \
  "            None => cx.fill_rect_brush(part, &ink)," \
  "            None => cx.fill_rect(part, base)," \
  "cargo test -q -p floom-widgets --test controls" \
  "progress-fills-with-its-gradient"

# 13. the transition key stops mattering, so "my target changed" replays nothing
mutate_one "$MOTION" \
  "    pub fn matches(&self, other: &Self) -> bool {
        self == other
    }" \
  "    pub fn matches(&self, other: &Self) -> bool {
        let mut a = *self;
        let mut b = *other;
        a.key = 0;
        b.key = 0;
        a == b
    }" \
  "cargo test -q -p floom-app --test motion_host" \
  "transition-key-restarts-the-tween"

# 14. arming a transition stops bumping the key it is identified by
mutate_one "$CTRLS" \
  "        self.transition_from = old_target;
        self.transition_rev = self.transition_rev.wrapping_add(1);" \
  "        self.transition_from = old_target;" \
  "cargo test -q -p floom-app --test motion_host" \
  "arm-transition-bumps-the-key"

# 15. vello drops the shadow at the adapter boundary (silently, by definition)
mutate_one "$VELLO" \
  "                        bounds: *bounds,
                        brush: fill.clone(),
                        radius: radius.max(0.0),
                        shadow: *shadow,
                    });" \
  "                        bounds: *bounds,
                        brush: fill.clone(),
                        radius: radius.max(0.0),
                        shadow: None,
                    });" \
  "cargo test -q -p floom-renderer-vello --lib" \
  "vello-carries-the-shadow-request"

# 16. the motion token's ms → s conversion drifts by a factor of ten
mutate_one "$MATERIAL" \
  "        Transition::new(duration_ms as f64 / 1000.0).with_easing(easing)" \
  "        Transition::new(duration_ms as f64 / 100.0).with_easing(easing)" \
  "cargo test -q -p floom-material" \
  "motion-token-conversion"

# 17. the golden example's own claims: run it against a collapsed ramp
mutate_one "$BRUSH" \
  "                    (((p.x - from.x) * axis.x + (p.y - from.y) * axis.y) / length_sq)
                        .clamp(0.0, 1.0)" \
  "                    0.0" \
  "cargo run -q -p floom2-style" \
  "example-gradient-act-has-teeth"

echo "=== campaign results ==="
cat "$RESULTS"
