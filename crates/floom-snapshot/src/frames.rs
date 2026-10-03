//! Per-frame pixel goldens: a digest that tolerates noise and still bites.
//!
//! [`crate::PixelSnapshot`] hashes the whole RGBA buffer, which is exactly right
//! for a *still* — and exactly wrong for an animation frame. A hash is all-or-
//! nothing: one pixel one LSB off on another machine (a different libm, a
//! different AA tie-break) fails the suite with a message that says nothing
//! about *what* moved, and the only repair is `UPDATE_SNAPSHOTS=1`, which
//! rewrites the golden and thereby also rewrites the regression you were trying
//! to catch. That is how a visual-regression suite rots: not by failing, but by
//! being re-baselined until it passes.
//!
//! So a frame is digested instead of hashed:
//!
//! * the buffer is divided into a `grid × grid` lattice of equal blocks and each
//!   block is **averaged with alpha weights** (premultiply, average, divide back
//!   out — averaging straight RGB across a half-transparent block would pull the
//!   block toward whatever colour the transparent pixels happen to carry);
//! * two frames are compared **cell by cell with a tolerance**, and a failure
//!   reports how many cells moved, by how much, and where.
//!
//! The pair of tests that give the tolerance its meaning are
//! `a_one_lsb_shift_everywhere_stays_inside_the_tolerance` and
//! `moving_the_picture_far_outside_the_tolerance_is_caught`: the digest must
//! shrug at the first and scream at the second. Without both, "tolerance" is
//! just a number that makes the suite green.
//!
//! The second thing this file owns is the failure a hash could never have
//! caught: **a golden frame sequence that never moves.** If every frame digests
//! the same, the suite has pinned a still and called it an animation — the
//! frames are compared against a file, the file matches, and an animation that
//! stopped animating passes forever. [`GoldenFrames::frozen`] is that check, and
//! `assert` refuses to write such a golden in the first place.

use crate::PixelSnapshot;

/// Default digest resolution. 12×12 over a ~100×50 fixture is a block of about
/// 8×4 pixels: enough averaging to absorb a one-level dither, small enough that
/// a card sliding half its width moves several cells.
pub const DEFAULT_GRID: usize = 12;

/// Default per-channel tolerance, in 0–255 levels.
///
/// Sized against the two things it has to straddle: a systematic one-level
/// shift (any cell moves by 1, well inside) and the faintest change a real
/// regression produces. The smallest one in practice is a shadow: dropping its
/// blur moves the blocks it touches by 6–20 levels, three to ten times the
/// tolerance. Two is not a guess at "close enough"; it is a tenth of the
/// smallest signal that matters.
pub const DEFAULT_TOL: u8 = 2;

/// One frame, reduced to a lattice of alpha-weighted block averages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDigest {
    pub width: u32,
    pub height: u32,
    /// Lattice resolution (the digest holds `grid * grid` cells).
    pub grid: usize,
    /// Row-major straight (non-premultiplied) RGBA, one entry per block.
    pub cells: Vec<[u8; 4]>,
}

impl FrameDigest {
    /// Reduce `snap` to a `grid × grid` lattice.
    ///
    /// Remainder pixels go to the last row/column of blocks rather than being
    /// dropped: a 100 px-wide frame on a grid of 12 would otherwise silently
    /// ignore its right-hand 4 px, which is exactly where a card's shadow sits.
    pub fn of(snap: &PixelSnapshot, grid: usize) -> Self {
        assert!(grid > 0, "a digest needs at least one cell");
        assert!(snap.width > 0 && snap.height > 0, "an empty frame digests to nothing");
        let w = snap.width as usize;
        let h = snap.height as usize;
        let mut cells = Vec::with_capacity(grid * grid);
        for gy in 0..grid {
            let y0 = gy * h / grid;
            let y1 = ((gy + 1) * h / grid).max(y0 + 1).min(h);
            for gx in 0..grid {
                let x0 = gx * w / grid;
                let x1 = ((gx + 1) * w / grid).max(x0 + 1).min(w);
                // Alpha-weighted sums: premultiply, accumulate, divide back.
                let (mut sr, mut sg, mut sb, mut sa) = (0u64, 0u64, 0u64, 0u64);
                let mut count = 0u64;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let i = (y * w + x) * 4;
                        let a = snap.rgba[i + 3] as u64;
                        sr += snap.rgba[i] as u64 * a;
                        sg += snap.rgba[i + 1] as u64 * a;
                        sb += snap.rgba[i + 2] as u64 * a;
                        sa += a;
                        count += 1;
                    }
                }
                let a = (sa / count) as u8;
                // A fully transparent block has no colour to report; forcing it
                // to zero keeps two empty regions equal instead of comparing
                // whatever stale values the renderer left behind.
                let rgb = if sa == 0 {
                    [0, 0, 0]
                } else {
                    [(sr / sa) as u8, (sg / sa) as u8, (sb / sa) as u8]
                };
                cells.push([rgb[0], rgb[1], rgb[2], a]);
            }
        }
        Self {
            width: snap.width,
            height: snap.height,
            grid,
            cells,
        }
    }

    /// The digest as stored: a header line, then one row of hex cells per line.
    pub fn text(&self) -> String {
        let mut out = format!("{}x{} grid {} tol {}\n", self.width, self.height, self.grid, DEFAULT_TOL);
        for row in 0..self.grid {
            let line: Vec<String> = (0..self.grid)
                .map(|col| {
                    let c = self.cells[row * self.grid + col];
                    format!("{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
                })
                .collect();
            out.push_str(&line.join(" "));
            out.push('\n');
        }
        out
    }

    /// Parse the [`FrameDigest::text`] form back. `None` if it is not one.
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let header = lines.next()?;
        // "96x48 grid 12 tol 2"
        let (dims, rest) = header.split_once(' ')?;
        let (w, h) = dims.split_once('x')?;
        let grid: usize = rest.split("grid ").nth(1)?.split_whitespace().next()?.parse().ok()?;
        let mut cells = Vec::with_capacity(grid * grid);
        for line in lines {
            for tok in line.split_whitespace() {
                if tok.len() != 8 {
                    return None;
                }
                let mut c = [0u8; 4];
                for k in 0..4 {
                    c[k] = u8::from_str_radix(&tok[k * 2..k * 2 + 2], 16).ok()?;
                }
                cells.push(c);
            }
        }
        if cells.len() != grid * grid {
            return None;
        }
        Some(Self {
            width: w.parse().ok()?,
            height: h.parse().ok()?,
            grid,
            cells,
        })
    }

    /// How far two digests are apart: the largest per-channel gap anywhere, and
    /// how many cells exceed `tol`.
    ///
    /// Compared cell by cell rather than through a hash so the answer can name
    /// a place: "17 of 144 cells moved, worst 43 at cell (7, 4)" tells you what
    /// broke, while "hash differs" tells you to re-baseline.
    pub fn diff(&self, other: &Self, tol: u8) -> FrameDiff {
        assert_eq!(
            (self.grid, self.width, self.height),
            (other.grid, other.width, other.height),
            "cannot compare digests of different shapes"
        );
        let mut worst = 0u8;
        let mut worst_at = 0usize;
        let mut over = 0usize;
        for (i, (a, b)) in self.cells.iter().zip(other.cells.iter()).enumerate() {
            let d = a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            if d > worst {
                worst = d;
                worst_at = i;
            }
            if d > tol {
                over += 1;
            }
        }
        FrameDiff {
            cells_over: over,
            worst,
            worst_cell: (worst_at % self.grid, worst_at / self.grid),
        }
    }
}

/// The outcome of comparing two [`FrameDigest`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDiff {
    /// Cells whose worst channel moved by more than the tolerance.
    pub cells_over: usize,
    /// The largest per-channel gap anywhere.
    pub worst: u8,
    /// `(col, row)` of that worst cell.
    pub worst_cell: (usize, usize),
}

impl FrameDiff {
    /// Did anything move beyond the tolerance?
    pub fn moved(&self) -> bool {
        self.cells_over > 0
    }
}

/// A named sequence of animation frames, asserted as one golden.
///
/// One file per sequence, not one per frame: the thing being pinned is the
/// *trajectory*, and a file per frame would let a single frame's golden drift
/// while the others hold.
pub struct GoldenFrames {
    grid: usize,
    tol: u8,
    frames: Vec<(String, FrameDigest)>,
}

impl Default for GoldenFrames {
    fn default() -> Self {
        Self {
            grid: DEFAULT_GRID,
            tol: DEFAULT_TOL,
            frames: Vec::new(),
        }
    }
}

impl GoldenFrames {
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the lattice resolution.
    pub fn grid(mut self, grid: usize) -> Self {
        self.grid = grid;
        self
    }

    /// Override the per-channel tolerance.
    pub fn tol(mut self, tol: u8) -> Self {
        self.tol = tol;
        self
    }

    /// Record one frame of the sequence.
    pub fn frame(&mut self, label: &str, snap: &PixelSnapshot) -> &mut Self {
        self.frames
            .push((label.to_string(), FrameDigest::of(snap, self.grid)));
        self
    }

    /// The recorded frames, in order.
    pub fn frames(&self) -> &[(String, FrameDigest)] {
        &self.frames
    }

    /// True when no frame differs from the first by more than the tolerance —
    /// i.e. the sequence pins a still and the animation never moved.
    ///
    /// This is the defect a hash-based golden cannot see: identical frames make
    /// an identical file, the file matches, and an animation that stopped
    /// animating is green forever.
    pub fn frozen(&self) -> bool {
        let Some((_, first)) = self.frames.first() else {
            return true;
        };
        self.frames
            .iter()
            .skip(1)
            .all(|(_, d)| !d.diff(first, self.tol).moved())
    }

    /// Compare (or, with `UPDATE_SNAPSHOTS=1`, write) the whole sequence.
    pub fn assert(&self, manifest_dir: &str, name: &str) {
        assert!(
            self.frames.len() >= 2,
            "a frame sequence needs at least two frames; one frame is a still, \
             and a still belongs in `PixelSnapshot::assert`"
        );
        assert!(
            !self.frozen(),
            "golden `{name}` is frozen: no frame differs from the first by more \
             than {tol} levels, so it pins a still and would pass even if the \
             animation stopped running entirely",
            tol = self.tol
        );

        let mut content = format!(
            "# per-frame pixel golden `{name}`: alpha-weighted {grid}x{grid} block \
             averages, tolerance {tol}\n",
            grid = self.grid,
            tol = self.tol
        );
        for (label, digest) in &self.frames {
            content.push_str(&format!("# frame {label}\n"));
            content.push_str(&digest.text());
        }

        if std::env::var("UPDATE_SNAPSHOTS")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            let path = crate::snapshot_path(manifest_dir, name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create snapshots dir");
            }
            std::fs::write(&path, &content).expect("write snapshot");
            return;
        }

        let path = crate::snapshot_path(manifest_dir, name);
        let prev = match std::fs::read_to_string(&path) {
            Ok(p) => p,
            Err(_) => panic!(
                "frame golden `{name}` missing at {}; run with UPDATE_SNAPSHOTS=1 to create it",
                path.display()
            ),
        };
        let expected = parse_sequence(&prev).unwrap_or_else(|| {
            panic!("frame golden `{name}` at {} is not a frame sequence", path.display())
        });

        assert_eq!(
            expected.len(),
            self.frames.len(),
            "frame golden `{name}` has {} frames on disk and {} now; the sequence \
             itself changed shape — regenerate with UPDATE_SNAPSHOTS=1",
            expected.len(),
            self.frames.len()
        );

        for ((label, got), (want_label, want)) in self.frames.iter().zip(expected.iter()) {
            assert_eq!(
                label, want_label,
                "frame golden `{name}`: frame labels no longer line up \
                 (`{want_label}` on disk, `{label}` now)"
            );
            let d = want.diff(got, self.tol);
            assert!(
                !d.moved(),
                "frame golden `{name}`: frame `{label}` moved — {} of {} cells over \
                 tolerance {tol}, worst {worst} at cell ({x}, {y})\n{disk}\n{now}",
                d.cells_over,
                self.grid * self.grid,
                tol = self.tol,
                worst = d.worst,
                x = d.worst_cell.0,
                y = d.worst_cell.1,
                disk = want.text(),
                now = got.text(),
            );
        }
    }
}

/// Read a stored sequence back as `(label, digest)` pairs.
pub fn parse_sequence(text: &str) -> Option<Vec<(String, FrameDigest)>> {
    let mut out = Vec::new();
    let mut label: Option<String> = None;
    let mut body = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# frame ") {
            if let (Some(l), Some(d)) = (label.take(), FrameDigest::parse(&body)) {
                out.push((l, d));
            }
            label = Some(rest.trim().to_string());
            body.clear();
        } else if !line.starts_with('#') {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let (Some(l), Some(d)) = (label, FrameDigest::parse(&body)) {
        out.push((l, d));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PixelSnapshot, single_rect_scene};
    use floom_types::{Color, Size};

    fn solid(w: u32, h: u32, color: Color) -> PixelSnapshot {
        PixelSnapshot::of(
            &single_rect_scene(Size::new(w as f64, h as f64), color),
            None,
            Size::new(w as f64, h as f64),
            1.0,
        )
    }

    /// Every channel of every pixel shifted by `delta` (saturating).
    fn shifted(snap: &PixelSnapshot, delta: i16) -> PixelSnapshot {
        let mut out = snap.clone();
        for c in out.rgba.chunks_mut(4) {
            for v in c.iter_mut() {
                let x = *v as i16 + delta;
                *v = x.clamp(0, 255) as u8;
            }
        }
        out.hash = format!("{:016x}", crate::fnv1a(&out.rgba));
        out
    }

    #[test]
    fn a_digest_is_reproducible() {
        let a = FrameDigest::of(&solid(48, 24, Color::rgb(30, 90, 200)), 12);
        let b = FrameDigest::of(&solid(48, 24, Color::rgb(30, 90, 200)), 12);
        assert_eq!(a, b);
    }

    #[test]
    fn a_digest_round_trips_through_its_text_form() {
        let d = FrameDigest::of(&solid(48, 24, Color::rgb(30, 90, 200)), 12);
        let back = FrameDigest::parse(&d.text()).expect("the text form must parse back");
        assert_eq!(d, back);
    }

    #[test]
    fn a_one_lsb_shift_everywhere_stays_inside_the_tolerance() {
        // The tolerance exists for this: a machine whose rasteriser lands one
        // level off must not turn the suite red, or the golden gets
        // re-baselined and stops being a golden.
        let base = solid(48, 24, Color::rgb(30, 90, 200));
        let jittered = shifted(&base, 1);
        let d = FrameDigest::of(&base, 12).diff(&FrameDigest::of(&jittered, 12), DEFAULT_TOL);
        assert_eq!(
            d.worst, 1,
            "a uniform one-level shift must survive as a one-level difference"
        );
        assert!(!d.moved(), "and must not count as a regression: {d:?}");
    }

    #[test]
    fn a_shift_far_outside_the_tolerance_is_caught() {
        // The other half: tolerance must not swallow a real change. Half the
        // range is far more than needed — this is the digest refusing to be
        // numb, not a claim about the smallest visible delta.
        let base = solid(48, 24, Color::rgb(30, 90, 200));
        let other = shifted(&base, 64);
        let d = FrameDigest::of(&base, 12).diff(&FrameDigest::of(&other, 12), DEFAULT_TOL);
        assert!(d.moved() && d.worst >= 64, "a real change must be seen: {d:?}");
    }

    #[test]
    fn a_transparent_block_reports_no_colour() {
        // Without the premultiply/divide-back, a half-covered block averages
        // the *straight* RGB of its clear pixels — usually (0,0,0) — and a
        // region that is simply empty reports a colour that drifts with
        // whatever the renderer left behind.
        let mut scene = floom_scene::Scene::new();
        scene.begin_update();
        // 36 of the frame's 48 columns: on a 2x2 lattice the right-hand blocks
        // are half covered (24..36 of 24..48) and the bottom ones are clear.
        let _ = scene.insert(crate::rect(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(36.0, 24.0),
            ),
            Color::rgb(200, 40, 40),
        ));
        scene.finish_update();
        let snap = PixelSnapshot::of(&scene, None, Size::new(48.0, 48.0), 1.0);
        let d = FrameDigest::of(&snap, 2);
        // Top-right block: half is the rect's red, half is clear.
        let top_right = d.cells[1];
        assert_eq!(
            top_right,
            [200, 40, 40, 127],
            "a half-covered block keeps the colour and halves the alpha: {top_right:?}"
        );
        // Bottom-right block: entirely clear.
        let bottom_right = d.cells[3];
        assert_eq!(
            bottom_right,
            [0, 0, 0, 0],
            "an empty block must report nothing, not leftover channels: {bottom_right:?}"
        );
    }

    #[test]
    fn a_sequence_that_never_moves_is_frozen() {
        let snap = solid(48, 24, Color::rgb(30, 90, 200));
        let mut g = GoldenFrames::new();
        g.frame("t=0", &snap).frame("t=6", &snap);
        assert!(
            g.frozen(),
            "two identical frames are a still, and the golden must say so"
        );
    }

    #[test]
    fn a_sequence_that_moves_is_not_frozen() {
        let a = solid(48, 24, Color::rgb(30, 90, 200));
        let b = solid(48, 24, Color::rgb(200, 90, 30));
        let mut g = GoldenFrames::new();
        g.frame("t=0", &a).frame("t=6", &b);
        assert!(!g.frozen(), "a frame that changed is an animation");
    }

    #[test]
    fn a_stored_sequence_parses_back_with_its_labels() {
        let a = solid(48, 24, Color::rgb(30, 90, 200));
        let b = solid(48, 24, Color::rgb(200, 90, 30));
        let mut g = GoldenFrames::new();
        g.frame("t=0", &a).frame("t=6", &b);

        let mut content = String::from("# header\n");
        for (label, d) in g.frames() {
            content.push_str(&format!("# frame {label}\n"));
            content.push_str(&d.text());
        }
        let back = parse_sequence(&content).expect("a written sequence must parse");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].0, "t=0");
        assert_eq!(back[1].0, "t=6");
        assert_eq!(back[0].1, g.frames()[0].1);
    }
}
