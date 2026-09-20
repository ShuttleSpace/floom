#!/usr/bin/env bash
# Mutation campaign for the S3 (subtree transforms) slice.
# For each mutation: back up, mutate (scripts/mutate.py), run the targeted
# test, record whether it FAILED, restore, and verify the restore byte-exact.
set -uo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:$PATH"

RESULTS=/tmp/s3_mutations.txt
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

GEOM=foundation/floem-types/src/geometry.rs
PATHS=foundation/floem-types/src/path.rs
SWLIB=render/floem-renderer-software/src/lib.rs
APPLIB=app/floem-app/src/lib.rs
SKIA=render/floem-renderer-skia/src/lib.rs
VELLO=render/floem-renderer-vello/src/lib.rs

# 1. scale_around drops its pivot (scales about the origin instead)
mutate_one "$GEOM" \
  "    pub fn scale_around(sx: f64, sy: f64, origin: Point) -> Self {
        Self::translate(origin.x, origin.y)
            .multiply(Self::scale(sx, sy))
            .multiply(Self::translate(-origin.x, -origin.y))
    }" \
  "    pub fn scale_around(sx: f64, sy: f64, _origin: Point) -> Self {
        Self::scale(sx, sy)
    }" \
  "cargo test -q -p floem-types" \
  "scale-around-pivot"

# 2. rotate_around drops its pivot
mutate_one "$GEOM" \
  "    pub fn rotate_around(radians: f64, origin: Point) -> Self {
        Self::translate(origin.x, origin.y)
            .multiply(Self::rotate(radians))
            .multiply(Self::translate(-origin.x, -origin.y))
    }" \
  "    pub fn rotate_around(radians: f64, _origin: Point) -> Self {
        Self::rotate(radians)
    }" \
  "cargo test -q -p floem-types" \
  "rotate-around-pivot"

# 3. multiply drops the translation carry
mutate_one "$GEOM" \
  "            e: self.a * other.e + self.c * other.f + self.e," \
  "            e: self.a * other.e + self.c * other.f," \
  "cargo test -q -p floem-types" \
  "multiply-translation-carry"

# 4. Path::transform passes line vertices through unmapped
mutate_one "$PATHS" \
  "                PathSeg::LineTo(to) => PathSeg::LineTo(map(to))," \
  "                PathSeg::LineTo(to) => PathSeg::LineTo(to)," \
  "cargo test -q -p floem-types" \
  "path-transform-maps-vertices"

# 5. ClipRegion::map lets rect regions through unmapped
mutate_one "$PATHS" \
  "            Self::Rect(bounds) => Self::Rect(t.map_rect(*bounds))," \
  "            Self::Rect(bounds) => Self::Rect(*bounds)," \
  "cargo test -q -p floem-types" \
  "clip-region-map-rect"

# 6. software: path points drawn untransformed inside a scope
mutate_one "$SWLIB" \
  "                    let map_polylines = |polylines: Vec<Polyline>| -> Vec<Polyline> {
                        if transformed {" \
  "                    let map_polylines = |polylines: Vec<Polyline>| -> Vec<Polyline> {
                        if false {" \
  "cargo test -q -p floem-renderer-software --test transforms" \
  "sw-path-points-mapped"

# 7. software: rotated rects take the axis-aligned fast path
mutate_one "$SWLIB" \
  "                SceneNode::Rect {
                    bounds,
                    fill,
                    radius: 0.0,
                } if !transformed => {" \
  "                SceneNode::Rect {
                    bounds,
                    fill,
                    radius: 0.0,
                } if true => {" \
  "cargo test -q -p floem-renderer-software --test transforms" \
  "sw-rect-transformed-arm"

# 8. software: clip regions pushed unmapped inside a scope
mutate_one "$SWLIB" \
  "                    let region = region.map(self.transform);" \
  "                    let region = (*region).clone();" \
  "cargo test -q -p floem-renderer-software --test transforms" \
  "sw-clip-region-mapped"

# 9. host: a scale composite stops producing a scope
mutate_one "$APPLIB" \
  "            Some(CompositorProp::Scale(s)) => {
                Some(floem_types::Transform::scale_around(s, s, center))
            }" \
  "            Some(CompositorProp::Scale(_)) => None," \
  "cargo test -q -p floem-app --test transforms" \
  "host-scale-scope-emitted"

# 10. host: mid-life bracket minting skips the subtree reseed
mutate_one "$APPLIB" \
  "                self.reseed_subtree_scene_nodes(index);
                let id = self.scene.insert(node);
                self.nodes[index].group_push = Some(id);" \
  "                let id = self.scene.insert(node);
                self.nodes[index].group_push = Some(id);" \
  "cargo test -q -p floem-app --test transforms" \
  "host-reseed-on-midlife-bracket"

# 11. skia: PushTransform records an identity matrix
mutate_one "$SKIA" \
  "                SceneNode::GroupPush { transform } => {
                    self.commands.push(SkiaCommand::PushTransform {
                        a: transform.a," \
  "                SceneNode::GroupPush { transform } => {
                    self.commands.push(SkiaCommand::PushTransform {
                        a: 1.0," \
  "cargo test -q -p floem-renderer-skia" \
  "skia-push-transform-carries-matrix"

# 12. vello headless: PushTransform records an identity transform
mutate_one "$VELLO" \
  "                    self.commands.push(VelloCommand::PushTransform {
                        transform: *transform,
                    });" \
  "                    self.commands.push(VelloCommand::PushTransform {
                        transform: floem_types::Transform::IDENTITY,
                    });" \
  "cargo test -q -p floem-renderer-vello --lib" \
  "vello-push-transform-carries-matrix"

echo "=== campaign results ==="
cat "$RESULTS"
