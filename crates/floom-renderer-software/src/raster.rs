//! Scanline rasterisation of vector paths, and the renderer's clip stack.
//!
//! ## Why a scanline filler and not supersampling
//!
//! A box filter over the pixel grid is trivial to write and quadratic in the
//! shape's area: a chart line spanning a 1200×800 window would cost 960 000
//! distance tests *per frame*, and the whole point of A9's budget is that
//! self-drawn content must not eat the frame. A scanline filler costs
//! `rows × edges crossing that row` instead, which for a stroke — the shape
//! that actually shows up in a chart — is a small constant per row.
//!
//! Anti-aliasing comes from two places: four sub-scanlines per pixel row
//! vertically, and *exact* horizontal coverage at each span end. The horizontal
//! axis is therefore analytically anti-aliased and the vertical axis is
//! supersampled fourfold, which is the standard trade and lands the same
//! perceptual result as the analytic SDF used for rounded rects.
//!
//! ## Winding
//!
//! Fill uses the non-zero rule. That matters beyond matching other toolkits:
//! [`floem_types::Stroke::outline`] emits a *union* of overlapping pieces and
//! relies on non-zero to keep them from cancelling, so the two halves of this
//! module are load-bearing for each other.

use floem_types::{ClipRegion, Point, Rect};

/// Sub-scanlines per pixel row. Four is the point where a diagonal edge's
/// staircase stops being visible at 1× and the cost is still linear.
const SUBSAMPLES: usize = 4;

/// Device-pixel clip rectangle: `(x0, y0, x1, y1)`, exclusive right/bottom.
pub type Scissor = (u32, u32, u32, u32);

/// One device-space edge, always stored top-down so the winding direction is
/// carried by `dir` instead of by the point order.
#[derive(Debug, Clone, Copy)]
struct Edge {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    dir: i32,
}

impl Edge {
    fn new(a: Point, b: Point) -> Option<Self> {
        if a.y == b.y {
            // Horizontal edges contribute no crossing to a scanline; including
            // them would double-count at their ends.
            return None;
        }
        let (top, bottom, dir) = if a.y < b.y { (a, b, 1) } else { (b, a, -1) };
        Some(Self {
            x0: top.x,
            y0: top.y,
            x1: bottom.x,
            y1: bottom.y,
            dir,
        })
    }

    /// The edge's x at scanline `y` (only valid for `y0 <= y <= y1`).
    fn x_at(&self, y: f64) -> f64 {
        self.x0 + (y - self.y0) / (self.y1 - self.y0) * (self.x1 - self.x0)
    }
}

/// Per-pixel coverage of a set of closed contours, over a device-pixel region.
///
/// `region` is `(x0, y0, x1, y1)` in device integer pixels and must already be
/// intersected with both the buffer and whatever clip is in force — this
/// routine works over exactly the region it is given, so handing it the
/// shape's own bounding box is the caller's decision to make.
///
/// Coverage lands in `out` (row-major, `stride × height`), which is reused
/// across calls: a per-frame allocation the size of the shape is exactly the
/// kind of cost that makes a self-drawn widget expensive without looking it.
/// Returns the row stride, or `0` when the region is empty.
pub fn rasterize(contours: &[Vec<Point>], region: Scissor, out: &mut Vec<f32>) -> usize {
    let (rx0, ry0, rx1, ry1) = region;
    let width = (rx1.saturating_sub(rx0)) as usize;
    let height = (ry1.saturating_sub(ry0)) as usize;
    out.clear();
    if width == 0 || height == 0 {
        return 0;
    }
    out.resize(width * height, 0.0);

    let mut edges: Vec<Edge> = Vec::new();
    for contour in contours {
        if contour.len() < 3 {
            continue;
        }
        for i in 0..contour.len() {
            let a = contour[i];
            let b = contour[(i + 1) % contour.len()];
            if let Some(edge) = Edge::new(a, b) {
                edges.push(edge);
            }
        }
    }
    if edges.is_empty() {
        return width;
    }
    // Sorted by top, so the active list can be built by a single forward pass
    // as rows advance. Without this the per-row loop would be linear in the
    // *total* edge count, which is what makes a 10 000-point path quadratic.
    edges.sort_by(|a, b| a.y0.partial_cmp(&b.y0).unwrap_or(std::cmp::Ordering::Equal));

    let mut active: Vec<Edge> = Vec::new();
    let mut next = 0usize;
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    let mut row = vec![0.0f32; width];

    for y in ry0..ry1 {
        let row_top = y as f64;
        let row_bottom = row_top + 1.0;
        active.retain(|e| e.y1 > row_top);
        while next < edges.len() && edges[next].y0 < row_bottom {
            active.push(edges[next]);
            next += 1;
        }
        if active.is_empty() {
            continue;
        }
        row.iter_mut().for_each(|c| *c = 0.0);
        let weight = 1.0 / SUBSAMPLES as f64;
        for sub in 0..SUBSAMPLES {
            let sy = row_top + (sub as f64 + 0.5) / SUBSAMPLES as f64;
            crossings.clear();
            for edge in &active {
                if edge.y0 <= sy && edge.y1 > sy {
                    crossings.push((edge.x_at(sy), edge.dir));
                }
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut winding = 0i32;
            for pair in crossings.windows(2) {
                winding += pair[0].1;
                if winding == 0 {
                    continue;
                }
                accumulate_span(&mut row, rx0, rx1, pair[0].0, pair[1].0, weight);
            }
        }
        let base = (y - ry0) as usize * width;
        out[base..base + width].copy_from_slice(&row);
    }
    width
}

/// The bounding box of a contour set, in device pixels, outward-rounded.
///
/// Used to keep a rasterisation to the shape's own extent instead of the
/// frame's: a chart line's coverage buffer must be the line's size, not the
/// window's.
pub fn contour_bounds(contours: &[Vec<Point>]) -> Option<Scissor> {
    let mut bounds: Option<Rect> = None;
    for contour in contours {
        for p in contour {
            bounds = Some(match bounds {
                None => Rect {
                    x0: p.x,
                    y0: p.y,
                    x1: p.x,
                    y1: p.y,
                },
                Some(b) => Rect {
                    x0: b.x0.min(p.x),
                    y0: b.y0.min(p.y),
                    x1: b.x1.max(p.x),
                    y1: b.y1.max(p.y),
                },
            });
        }
    }
    bounds.map(|b| device_box(b, 1.0))
}

/// Intersects two device-pixel boxes.
pub fn intersect_scissor(a: Scissor, b: Scissor) -> Scissor {
    (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3))
}

/// True when the box admits no pixels at all.
pub fn scissor_is_empty(s: Scissor) -> bool {
    s.0 >= s.2 || s.1 >= s.3
}

/// Adds the horizontal coverage of `[from, to)` to one row.
///
/// The span's ends get their true fractional coverage and the pixels strictly
/// between get the sub-scanline's whole weight — this is the "exact in x" half
/// of the anti-aliasing, and it is why the filler does not need horizontal
/// supersampling.
fn accumulate_span(row: &mut [f32], rx0: u32, rx1: u32, from: f64, to: f64, weight: f64) {
    let lo = from.max(rx0 as f64);
    let hi = to.min(rx1 as f64);
    if hi <= lo {
        return;
    }
    let first = lo.floor() as i64;
    let last = (hi.ceil() as i64) - 1;
    let index = |p: i64| (p - rx0 as i64) as usize;
    if first == last {
        row[index(first)] += ((hi - lo) * weight) as f32;
        return;
    }
    row[index(first)] += ((first as f64 + 1.0 - lo) * weight) as f32;
    for p in (first + 1)..last {
        row[index(p)] += weight as f32;
    }
    row[index(last)] += ((hi - last as f64) * weight) as f32;
}

/// A coverage mask for one path-shaped clip level.
struct Mask {
    x0: u32,
    y0: u32,
    width: usize,
    coverage: Vec<f32>,
}

/// One level of the clip stack.
struct Level {
    region: ClipRegion,
    /// Present only for path levels: a rounded rectangle's coverage is four
    /// distance evaluations per pixel and is cheaper to compute on demand than
    /// to materialise for a whole region.
    mask: Option<Mask>,
}

/// The renderer's clip stack, with the scissor and masks derived from it.
///
/// Kept as a stack rather than a folded intersection because no backend can
/// represent the intersection of two rounded rectangles, let alone of two
/// paths. Each level is evaluated in its own terms and the coverages multiply,
/// which is exact where folding would have to approximate.
pub struct ClipStack {
    levels: Vec<Level>,
}

impl Default for ClipStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipStack {
    pub fn new() -> Self {
        Self { levels: Vec::new() }
    }

    /// True when every level is an axis-aligned rectangle — i.e. per-pixel
    /// coverage is 1 everywhere the scissor admits.
    ///
    /// Worth asking because it is what keeps the blit fast paths fast: an
    /// opaque rect under a plain rect clip can still write its pixels directly,
    /// and only a curved or path clip forces the per-pixel blend.
    pub fn is_rect_only(&self) -> bool {
        self.levels
            .iter()
            .all(|level| matches!(level.region, ClipRegion::Rect(_)))
    }

    /// Empties the stack. Every walk starts here: a scene's marker stream is
    /// allowed to end with regions still pushed, so a renderer that carried the
    /// stack into the next walk would clip the new frame to the old one's shape.
    pub fn clear(&mut self) {
        self.levels.clear();
    }

    pub fn push(&mut self, region: ClipRegion, scale: f64) {
        let mask = match &region {
            ClipRegion::Path {
                bounds,
                origin,
                path,
            } => {
                let contours = path
                    .flatten(floem_types::DEFAULT_FLATTEN_TOLERANCE)
                    .into_iter()
                    .map(|poly| {
                        poly.points
                            .into_iter()
                            .map(|p| Point::new((p.x + origin.x) * scale, (p.y + origin.y) * scale))
                            .collect::<Vec<Point>>()
                    })
                    .collect::<Vec<_>>();
                let region = device_box(*bounds, scale);
                let mut coverage = Vec::new();
                let width = rasterize(&contours, region, &mut coverage);
                Some(Mask {
                    x0: region.0,
                    y0: region.1,
                    width,
                    coverage,
                })
            }
            _ => None,
        };
        self.levels.push(Level { region, mask });
    }

    pub fn pop(&mut self) {
        self.levels.pop();
    }

    /// The device-pixel box every level admits, intersected.
    ///
    /// Unclamped by the buffer: the caller intersects with the target extent,
    /// exactly as it already does for the frame's damage clip.
    pub fn scissor(&self, scale: f64) -> Option<Scissor> {
        let mut result: Option<Scissor> = None;
        for level in &self.levels {
            let box_ = device_box(level.region.bounds(), scale);
            result = Some(match result {
                None => box_,
                Some((x0, y0, x1, y1)) => (
                    x0.max(box_.0),
                    y0.max(box_.1),
                    x1.min(box_.2),
                    y1.min(box_.3),
                ),
            });
        }
        result
    }

    /// How much of the device pixel at `(x, y)` survives the clip, in `0..=1`.
    ///
    /// Only the non-rectangular levels have anything to say here: a rectangle's
    /// contribution is already enforced by the scissor.
    pub fn coverage(&self, x: u32, y: u32, scale: f64) -> f32 {
        let mut coverage = 1.0f32;
        for level in &self.levels {
            match &level.region {
                ClipRegion::Rect(_) => {}
                ClipRegion::RoundedRect { bounds, radius } => {
                    coverage *= rounded_coverage(*bounds, *radius, scale, x, y);
                    if coverage <= 0.0 {
                        return 0.0;
                    }
                }
                ClipRegion::Path { .. } => {
                    let Some(mask) = &level.mask else { continue };
                    if x < mask.x0 || y < mask.y0 {
                        return 0.0;
                    }
                    let col = (x - mask.x0) as usize;
                    let row = (y - mask.y0) as usize;
                    // `width` is the mask's row stride; a lookup past the end
                    // means the pixel is outside the region entirely.
                    let index = row * mask.width + col;
                    if col >= mask.width || index >= mask.coverage.len() {
                        return 0.0;
                    }
                    coverage *= mask.coverage[index];
                    if coverage <= 0.0 {
                        return 0.0;
                    }
                }
            }
        }
        coverage
    }
}

/// A logical rectangle in device pixels, outward-rounded so no partly-covered
/// pixel is left outside the box.
pub fn device_box(bounds: Rect, scale: f64) -> Scissor {
    let x0 = (bounds.x0 * scale).floor().max(0.0) as u32;
    let y0 = (bounds.y0 * scale).floor().max(0.0) as u32;
    let x1 = (bounds.x1 * scale).ceil().max(0.0) as u32;
    let y1 = (bounds.y1 * scale).ceil().max(0.0) as u32;
    (x0.min(x1), y0.min(y1), x1, y1)
}

/// Analytic coverage of a device-space rounded rectangle at a pixel centre.
///
/// The same signed-distance ramp the fill path uses, so a clip's corner and a
/// rounded rect's corner blur identically — a clip that anti-aliased
/// differently from the shape it clips would leave a bright fringe.
pub fn rounded_coverage(bounds: Rect, radius: f64, scale: f64, x: u32, y: u32) -> f32 {
    let d = rounded_distance(bounds, radius, scale, x as f64 + 0.5, y as f64 + 0.5);
    (0.5 - d).clamp(0.0, 1.0) as f32
}

/// Inigo Quilez's rounded-box signed distance, in device pixels.
pub fn rounded_distance(bounds: Rect, radius: f64, scale: f64, px: f64, py: f64) -> f64 {
    let (x0, y0, x1, y1) = (
        bounds.x0 * scale,
        bounds.y0 * scale,
        bounds.x1 * scale,
        bounds.y1 * scale,
    );
    let radius = (radius * scale).clamp(0.0, ((x1 - x0) / 2.0).min((y1 - y0) / 2.0));
    let cx = (x0 + x1) / 2.0;
    let cy = (y0 + y1) / 2.0;
    let qx = (px - cx).abs() - ((x1 - x0) / 2.0 - radius);
    let qy = (py - cy).abs() - ((y1 - y0) / 2.0 - radius);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_square(origin: Point, size: f64) -> Vec<Vec<Point>> {
        vec![vec![
            origin,
            Point::new(origin.x + size, origin.y),
            Point::new(origin.x + size, origin.y + size),
            Point::new(origin.x, origin.y + size),
        ]]
    }

    /// An axis-aligned square at whole-pixel coordinates covers its interior
    /// fully and nothing outside it. This is the filler's control group: if the
    /// span accumulation were off by a half pixel, the edges would smear.
    #[test]
    fn an_aligned_square_fills_exactly_its_pixels() {
        let contours = unit_square(Point::new(2.0, 3.0), 4.0);
        let mut coverage = Vec::new();
        let stride = rasterize(&contours, (0, 0, 8, 8), &mut coverage);
        assert_eq!(stride, 8);
        let at = |x: usize, y: usize| coverage[y * 8 + x];
        for y in 3..7 {
            for x in 2..6 {
                assert!(
                    (at(x, y) - 1.0).abs() < 1e-6,
                    "inside ({x},{y}) = {}",
                    at(x, y)
                );
            }
        }
        for y in 0..8 {
            for x in 0..8 {
                if (2..6).contains(&x) && (3..7).contains(&y) {
                    continue;
                }
                assert_eq!(at(x, y), 0.0, "outside ({x},{y})");
            }
        }
    }

    /// A half-pixel offset must produce half coverage on the edge columns —
    /// that is the anti-aliasing, and a filler without it would report 0 or 1.
    #[test]
    fn a_half_pixel_offset_produces_half_coverage() {
        let contours = unit_square(Point::new(2.5, 3.0), 4.0);
        let mut coverage = Vec::new();
        rasterize(&contours, (0, 0, 8, 8), &mut coverage);
        let at = |x: usize, y: usize| coverage[y * 8 + x];
        assert!((at(2, 4) - 0.5).abs() < 1e-6, "left edge {}", at(2, 4));
        assert!((at(6, 4) - 0.5).abs() < 1e-6, "right edge {}", at(6, 4));
        assert!((at(4, 4) - 1.0).abs() < 1e-6);
    }

    /// Two concentric squares wound the same way must still fill solid: the
    /// non-zero rule counts winding, it does not xor.
    #[test]
    fn nested_same_winding_contours_stay_solid() {
        let mut contours = unit_square(Point::new(0.0, 0.0), 8.0);
        contours.extend(unit_square(Point::new(2.0, 2.0), 4.0));
        let mut coverage = Vec::new();
        rasterize(&contours, (0, 0, 8, 8), &mut coverage);
        assert!((coverage[4 * 8 + 4] - 1.0).abs() < 1e-6);
    }

    /// A clip stack's scissor is the intersection of its levels, and the
    /// coverage of a non-rectangular level falls off across its own corner.
    #[test]
    fn the_clip_scissor_intersects_and_a_rounded_level_softens_its_corner() {
        let mut stack = ClipStack::new();
        assert_eq!(stack.scissor(1.0), None);
        stack.push(
            ClipRegion::Rect(Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
            }),
            1.0,
        );
        stack.push(
            ClipRegion::RoundedRect {
                bounds: Rect {
                    x0: 5.0,
                    y0: 0.0,
                    x1: 20.0,
                    y1: 10.0,
                },
                radius: 4.0,
            },
            1.0,
        );
        assert_eq!(stack.scissor(1.0), Some((5, 0, 10, 10)));
        // Inside the rounded level's straight part: fully admitted.
        assert_eq!(stack.coverage(7, 5, 1.0), 1.0);
        // At the level's own top-left corner, outside the arc entirely.
        assert_eq!(stack.coverage(5, 0, 1.0), 0.0, "corner of the round level");
        // And the pixel the arc passes through is partly admitted — that is
        // the clip's anti-aliasing, and a hard-edged clip would report 0 or 1.
        let edge = stack.coverage(5, 1, 1.0);
        assert!(edge > 0.0 && edge < 1.0, "arc edge coverage was {edge}");

        stack.pop();
        assert_eq!(stack.scissor(1.0), Some((0, 0, 10, 10)));
        assert_eq!(stack.coverage(0, 0, 1.0), 1.0);
        stack.pop();
        assert_eq!(stack.scissor(1.0), None, "an empty stack clips nothing");
        assert_eq!(stack.coverage(0, 0, 1.0), 1.0);
    }

    /// A walked-away stack must not survive into the next walk.
    #[test]
    fn clearing_removes_every_level() {
        let mut stack = ClipStack::new();
        stack.push(
            ClipRegion::Rect(Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 4.0,
                y1: 4.0,
            }),
            1.0,
        );
        assert_eq!(stack.scissor(1.0), Some((0, 0, 4, 4)));
        stack.clear();
        assert_eq!(stack.coverage(0, 0, 1.0), 1.0);
        assert_eq!(stack.scissor(1.0), None);
    }
}
