#!/usr/bin/env bash
# Mutation campaign for the S7 slice (A4's back half: general filters).
# For each mutation: back up, mutate (scripts/mutate.py), run the targeted
# test, record whether it FAILED, restore, and verify the restore byte-exact.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s7_mutations.txt
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
  elif echo "$out" | grep -qE "test result: ok|every claim asserted"; then
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

SOFTWARE=crates/floom-renderer-software/src/lib.rs
HOST=crates/floom-app/src/lib.rs
CONTAINER=crates/floom-widgets/src/primitives.rs
SW_TESTS="cargo test -q -p floom-renderer-software --test filters"
HOST_TESTS="cargo test -q -p floom-app --test filters"

# 1. the layer is composited without ever being blurred — a filter that copies
mutate_one "$SOFTWARE" \
  "        blur_window(&mut bytes, w, h, &layer.filter, self.scale);" \
  "        let _ = (&mut bytes, w, h, &layer.filter, self.scale);" \
  "$SW_TESTS" \
  "the-layer-is-really-blurred"

# 2. the blurred layer is never written back — a filter that erases
mutate_one "$SOFTWARE" \
  "                self.blend_pixel(x, y, color, 1.0);" \
  "                self.blend_pixel(x, y, color, 0.0);" \
  "$SW_TESTS" \
  "the-filtered-layer-reaches-the-surface"

# 3. the backdrop is read but never blurred — frost that just copies
mutate_one "$SOFTWARE" \
  "        blur_window(&mut bytes, w, h, &filter, scale);" \
  "        let _ = (&mut bytes, w, h, &filter, scale);" \
  "$SW_TESTS" \
  "the-backdrop-is-really-blurred"

# 4. the blurred backdrop is not written back — a no-op node
mutate_one "$SOFTWARE" \
  "                self.pixels[dst..dst + 4].copy_from_slice(&bytes[src..src + 4]);" \
  "                let _ = (dst, src, &bytes);" \
  "$SW_TESTS" \
  "the-blurred-backdrop-replaces-the-sharp-one"

# 5. the blur runs on straight channels instead of premultiplied ones: the
#    transparent pixels' colour is averaged in as ink and every edge darkens.
mutate_one "$SOFTWARE" \
  "            fields[c][i] = f32::from(bytes[i * 4 + c]) / 255.0 * a;" \
  "            fields[c][i] = f32::from(bytes[i * 4 + c]) / 255.0;" \
  "$SW_TESTS" \
  "the-blur-is-premultiplied"

# 6. both axes get the horizontal radius — the two collapse into one
mutate_one "$SOFTWARE" \
  "    let (rx, ry) = (kernel(radius_x), kernel(radius_y));" \
  "    let (rx, ry) = (kernel(radius_x), kernel(radius_x));" \
  "$SW_TESTS" \
  "the-two-axes-keep-their-own-kernel"

# 7. a filter declared after the first paint is not reseeded — the marker
#    lands after the picture it has to wrap
mutate_one "$HOST" \
  "        match self.nodes[index].filter_push {
            Some(id) => self.scene.upsert(id, node),
            None => {
                self.reseed_subtree_scene_nodes(index);" \
  "        match self.nodes[index].filter_push {
            Some(id) => self.scene.upsert(id, node),
            None => {
                let _ = index;" \
  "$HOST_TESTS" \
  "a-mid-life-filter-is-reseeded"

# 8. the scope is never closed — the pop marker is not written
mutate_one "$HOST" \
  "        self.paint_children(index, stats, fold_down);
        if self.nodes[index].filter_push.is_some() {
            self.write_filter_pop(index);
        }" \
  "        self.paint_children(index, stats, fold_down);
        let _ = index;" \
  "$HOST_TESTS" \
  "the-scope-is-closed"

# 9. the widget's declaration is never read — no control can ask for a filter
mutate_one "$HOST" \
  "        // space the content was laid out in, not in the rotated one).
        // An identity filter is \"no filter\": writing markers for it would
        // make every backend pay for a layer to do nothing, and a \`FilterPop\`
        // with no matching visual effect is one more thing that can go wrong.
        match self.nodes[index].widget.filter().filter(|f| !f.is_identity()) {" \
  "        // space the content was laid out in, not in the rotated one).
        match None::<floom_types::Filter> {" \
  "$HOST_TESTS" \
  "the-widgets-declaration-is-read"

# 10. the control never emits the backdrop op — the frosted panel is plain
mutate_one "$CONTAINER" \
  "            cx.backdrop_filter(bounds, &backdrop);" \
  "            let _ = (&backdrop, bounds);" \
  "$HOST_TESTS" \
  "the-control-emits-the-backdrop-op"

# 11. the golden example's own claims: run it against a filter that copies
mutate_one "$SOFTWARE" \
  "        blur_window(&mut bytes, w, h, &layer.filter, self.scale);" \
  "        let _ = (&mut bytes, w, h, &layer.filter, self.scale);" \
  "cargo run -q -p floom2-style" \
  "example-filter-acts-have-teeth"

# 12. ...and against a backdrop that does not replace what it read
mutate_one "$SOFTWARE" \
  "                self.pixels[dst..dst + 4].copy_from_slice(&bytes[src..src + 4]);" \
  "                let _ = (dst, src, &bytes);" \
  "cargo run -q -p floom2-style" \
  "example-frost-act-has-teeth"

# A restore-by-copy can land inside the same second as the mutated build, and
# cargo fingerprints on mtime — so leave every file this campaign touched newer
# than its last artifact. Then *re-run the suite before trusting any count*: a
# stale binary passing (or failing) is indistinguishable from a real result.
touch "$SOFTWARE" "$HOST" "$CONTAINER"

echo "=== campaign results ==="
cat "$RESULTS"
echo
echo "NOTE: verify with a clean full run afterwards; see the touch above."
