#!/usr/bin/env bash
# Mutation campaign for the A5 custom-shader slice. Each mutation back up,
# mutate (scripts/mutate.py), run the targeted test, record whether it FAILED,
# restore, and verify the restore byte-exact. A mutation the targeted test does
# not catch is a test with no teeth.
#
# The teeth here are the *degradation-is-never-silent* contract of A5:
#   - the software rasteriser has no shader runtime and must count the skip;
#   - the skia adapter records the request rather than dropping it;
#   - vello keeps the shader's WGSL *and* its scalar params in the node
#     fingerprint and readback key, so a change re-encodes instead of serving a
#     stale fragment, and actually runs the program (shaders_drawn, not a skip).
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/a5_mutations.txt
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

SOFT=render/floem-renderer-software/src/lib.rs
VELLO=render/floem-renderer-vello/src/gpu.rs

# 1. software backend: dropping the skip counter makes the degradation silent.
mutate_one "$SOFT" \
"                SceneNode::Shader { .. } => {
                    self.shaders_skipped += 1;
                }" \
"                SceneNode::Shader { .. } => {}" \
  "cargo test -q -p floem-renderer-software software_skips_and_counts_shader_nodes" \
  "software-counts-shader-skip"

# 2. vello fingerprint: dropping the WGSL from the hash means two different
#    programs encode identically and the cache serves a stale fragment.
mutate_one "$VELLO" \
"            hasher.write(shader.wgsl.as_bytes());
            for p in shader.params.iter().copied() {
                hasher.write_u32(p.to_bits());
            }" \
"            for p in shader.params.iter().copied() {
                hasher.write_u32(p.to_bits());
            }" \
  "cargo test -q -p floem-renderer-vello --features gpu a_shaders_source_and_params_are_part_of_its_fingerprint" \
  "vello-fingerprint-includes-wgsl"

# 3. vello readback key: dropping the height means a resize reuses the brush
#    from the previous size.
mutate_one "$VELLO" \
"    hasher.write_u32(pw);
    hasher.write_u32(ph);" \
"    hasher.write_u32(pw);" \
  "cargo test -q -p floem-renderer-vello --features gpu a_shader_cache_key_separates_source_from_size" \
  "vello-cache-key-includes-height"

# 4. vello encode: trading the draw for a skip means a shader is silently
#    degraded back to transparency on the only backend that can run it.
mutate_one "$VELLO" \
"                    if let Some(skey) =
                        self.run_shader_to_image(target, bounds, pw, ph, shader, &pipeline)
                    {
                        used_images.insert(skey);
                        stats.image_keys.push(skey);
                        stats.shader_drawn += 1;
                    } else {
                        stats.shader_skips += 1;
                    }" \
"                    if let Some(skey) =
                        self.run_shader_to_image(target, bounds, pw, ph, shader, &pipeline)
                    {
                        used_images.insert(skey);
                        stats.image_keys.push(skey);
                        stats.shader_skips += 1;
                    } else {
                        stats.shader_skips += 1;
                    }" \
  "cargo test -q -p floem-renderer-vello --features gpu shader_nodes_encode_through_the_gpu_backend" \
  "vello-shader-actually-awaited-draw"

# 5. the widget door: a host that swallows `draw_shader` leaves the whole
#    widget surface drawing nothing, while every renderer-side test stays green
#    (they build their scenes by hand). This is the mutation that says whether
#    A5 is reachable from an app at all.
HOST=app/floem-app/src/lib.rs
mutate_one "$HOST" \
"        self.push(floem_scene::SceneNode::shader(self.bounds, shader.clone()));" \
"        let _ = shader;" \
  "cargo test -q -p floem-app --test shader_box a_shader_box_carries_its_program_over_its_own_box" \
  "host-carries-the-shader-request"

# 6. the box: a shader that claims the window instead of the element paints
#    over everything around it. The renderer cannot tell — both are valid
#    rects — so only the host-side assertion can.
mutate_one "$HOST" \
"        self.push(floem_scene::SceneNode::shader(self.bounds, shader.clone()));" \
"        let mut box_bounds = self.bounds;
        box_bounds.x1 = box_bounds.x0 + 300.0;
        box_bounds.y1 = box_bounds.y0 + 200.0;
        self.push(floem_scene::SceneNode::shader(box_bounds, shader.clone()));" \
  "cargo test -q -p floem-app --test shader_box a_shader_box_carries_its_program_over_its_own_box" \
  "host-uses-the-laid-out-box-not-the-window"

# 7/8. The in-place setters: a builder is cloned into the tree, so a setter
#      that swaps the value without bumping the revision paints the *old*
#      program forever — the silent no-op this widget is most likely to ship.
CANVAS=widgets/floem-widgets/src/canvas.rs
mutate_one "$CANVAS" \
"    pub fn set_shader(&mut self, shader: Shader) {
        self.shader = shader;
        self.revision.bump();
    }" \
"    pub fn set_shader(&mut self, shader: Shader) {
        let _ = shader;
        self.revision.bump();
    }" \
  "cargo test -q -p floem-app --test shader_box replacing_the_program_on_the_mounted_box_repaints_the_new_source" \
  "set_shader-asks-for-a-frame"

mutate_one "$CANVAS" \
"    pub fn set_params(&mut self, params: impl Into<Vec<f32>>) {
        self.shader.params = params.into();
        self.revision.bump();
    }" \
"    pub fn set_params(&mut self, params: impl Into<Vec<f32>>) {
        let _ = params;
        self.revision.bump();
    }" \
  "cargo test -q -p floem-app --test shader_box changing_only_the_scalars_repaints_them" \
  "set_params-asks-for-a-frame"

# Restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — leave the touched files newer than their last
# artifact. Then *re-run the suite before trusting any count*.
touch "$SOFT" "$VELLO" "$HOST" "$CANVAS"

echo "=== A5 campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."
echo "NOTE: the two GPU teeth (2/3/4) need a usable adapter — run them on a"
echo "      machine with one, or they skip and report SURVIVED with no latch."