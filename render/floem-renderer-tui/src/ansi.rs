//! Turning a [`TuiGrid`] into the bytes a terminal understands.
//!
//! The encoder is kept deliberately dumb about *where* the output goes — it
//! returns a string of rows joined by newlines, with no cursor motion and no
//! screen clearing. Framing belongs to whoever owns the terminal (see
//! [`TuiPresenter`]); mixing the two would make the encoding impossible to
//! assert on and would repaint the whole screen from inside a library that
//! cannot know the screen's state.
//!
//! What it *is* careful about is emitting no more escapes than necessary: a
//! run of cells sharing a colour writes the code once, so a mostly-empty row
//! costs a handful of bytes rather than ~20 per cell.
//!
//! [`TuiGrid`]: crate::TuiGrid
//! [`TuiPresenter`]: crate::TuiPresenter

use floem_types::Color;

use crate::{Cell, TuiGrid};

const RESET: &str = "\u{1b}[0m";
const DEFAULT_FG: &str = "\u{1b}[39m";
const DEFAULT_BG: &str = "\u{1b}[49m";

/// Encoded-row count and escape density, for framing decisions downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EncodeStats {
    pub rows: usize,
    pub escapes: usize,
}

/// Render `grid` as ANSI-coloured rows separated by `\n`.
///
/// Every row ends with a full reset so a partially-updated screen cannot leave
/// a colour bleeding into whatever draws next.
pub fn encode(grid: &TuiGrid) -> String {
    encode_with_stats(grid).0
}

/// [`encode`], plus how noisy it was. Tests assert on both: `"\u{1b}[0m"`-free
/// output is not by itself proof that colours are being reused.
pub fn encode_with_stats(grid: &TuiGrid) -> (String, EncodeStats) {
    let mut out = String::new();
    let mut stats = EncodeStats {
        rows: grid.rows(),
        escapes: 0,
    };
    let mut cur_fg: Option<Color> = None;
    let mut cur_bg: Option<Color> = None;

    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let Some(cell) = grid.cell(col, row) else {
                continue;
            };
            if cell.fg != cur_fg {
                out.push_str(&fg_code(cell.fg));
                stats.escapes += 1;
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                out.push_str(&bg_code(cell.bg));
                stats.escapes += 1;
                cur_bg = cell.bg;
            }
            out.push(cell.ch);
        }
        // End-of-row reset, then reset both tracked colours: a SGR sequence is
        // only skipped when it matches what the terminal is already showing,
        // and after a reset that is the terminal's default — not what we last
        // emitted.
        out.push_str(RESET);
        stats.escapes += 1;
        cur_fg = None;
        cur_bg = None;
        if row + 1 < grid.rows() {
            out.push('\n');
        }
    }

    (out, stats)
}

fn fg_code(color: Option<Color>) -> String {
    match color {
        Some(c) => truecolor(38, c),
        None => DEFAULT_FG.to_string(),
    }
}

fn bg_code(color: Option<Color>) -> String {
    match color {
        Some(c) => truecolor(48, c),
        None => DEFAULT_BG.to_string(),
    }
}

fn truecolor(channel: u8, color: Color) -> String {
    format!("\u{1b}[{};2;{};{};{}m", channel, color.r, color.g, color.b)
}

/// What a diff frame did, so a caller can tell "nothing changed" apart from
/// "the whole screen was repainted".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiffStats {
    /// Cells whose content differs from the previous frame.
    pub changed_cells: usize,
    /// ANSI escapes emitted (cursor moves + colour selections + leading reset).
    pub escapes: usize,
    /// `true` when there was no previous frame, so the whole screen was cleared
    /// and repainted rather than patched.
    pub full_repaint: bool,
}

/// Emit only the cells that differ from `old`, each positioned by an absolute
/// cursor move. Returns `(bytes, changed_cells, escapes)`.
///
/// The terminal keeps its colour state across a cursor move, so we carry the
/// tracked `cur_fg`/`cur_bg` through the diff exactly as [`encode`] would: only
/// transitions are emitted — a cell we jump over and do not touch keeps what was
/// already on screen. A leading reset establishes a known baseline so a skipped
/// cell cannot inherit a colour from a region we leapt past.
pub fn diff_encode(new: &TuiGrid, old: &TuiGrid) -> (String, usize, usize) {
    let rows = new.rows().min(old.rows());
    let cols = new.cols().min(old.cols());
    let mut out = String::new();
    out.push_str(RESET);
    let mut cur_fg: Option<Color> = None;
    let mut cur_bg: Option<Color> = None;
    let mut escapes = 1; // the leading reset
    let mut changed = 0;
    for row in 0..rows {
        for col in 0..cols {
            let nc = new.cell(col, row).unwrap();
            let oc = old.cell(col, row).unwrap();
            if nc == oc {
                continue;
            }
            changed += 1;
            out.push_str(&format!("\u{1b}[{};{}H", row + 1, col + 1));
            escapes += 1; // the cursor move
            let (chunk, f, b, e) = encode_cell(nc, &cur_fg, &cur_bg);
            out.push_str(&chunk);
            escapes += e;
            cur_fg = f;
            cur_bg = b;
        }
    }
    (out, changed, escapes)
}

/// Encode one cell relative to the terminal's tracked colour state.
///
/// Returns the bytes written, the new tracked foreground/background, and the
/// number of SGR escapes emitted. Only the colour transitions from `cur_*` are
/// written, matching how [`encode`] reuses colours across a run of cells.
pub fn encode_cell(
    cell: &Cell,
    cur_fg: &Option<Color>,
    cur_bg: &Option<Color>,
) -> (String, Option<Color>, Option<Color>, usize) {
    let mut out = String::new();
    let mut escapes = 0;
    let mut f = *cur_fg;
    let mut b = *cur_bg;
    if cell.fg != f {
        out.push_str(&fg_code(cell.fg));
        f = cell.fg;
        escapes += 1;
    }
    if cell.bg != b {
        out.push_str(&bg_code(cell.bg));
        b = cell.bg;
        escapes += 1;
    }
    out.push(cell.ch);
    (out, f, b, escapes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::rect;
    use floem_scene::Size;

    fn grid(cols: usize, rows: usize) -> TuiGrid {
        TuiGrid::new(
            cols,
            rows,
            Size::new(cols as f64 * 10.0, rows as f64 * 10.0),
        )
    }

    #[test]
    fn an_untouched_grid_encodes_as_plain_spaces() {
        let g = grid(3, 2);
        assert_eq!(encode(&g), "   \u{1b}[0m\n   \u{1b}[0m");
    }

    #[test]
    fn colours_are_emitted_once_per_run_of_cells() {
        let mut g = grid(4, 1);
        let red = Color::rgb(10, 20, 30);
        g.fill_background(rect(0.0, 0.0, 40.0, 10.0), red);
        g.put(0, 0, 'X', Color::rgb(200, 200, 200));

        let (out, stats) = encode_with_stats(&g);
        assert!(out.starts_with("\u{1b}[38;2;200;200;200m"), "{out:?}");
        assert!(out.contains("\u{1b}[48;2;10;20;30m"), "{out:?}");
        // Two colour selections, the switch back to the default foreground for
        // the remaining cells, and the row reset — four escapes for four cells,
        // which is the point: nothing repeats per cell.
        assert_eq!(stats.escapes, 4, "no per-cell colour repetition: {out:?}");
        assert_eq!(out.matches("\u{1b}[48").count(), 1, "bg selected once");
        assert!(
            out.contains("\u{1b}[39m"),
            "fg released back to the terminal default: {out:?}"
        );
    }

    /// Everything outside CSI sequences, so assertions see glyphs and not the
    /// digits inside `\u{1b}[38;2;…m`.
    fn visible(out: &str) -> String {
        let mut stripped = String::with_capacity(out.len());
        let mut chars = out.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                stripped.push(c);
            }
        }
        stripped
    }

    #[test]
    fn every_row_carries_its_characters() {
        let mut g = grid(2, 2);
        g.put(0, 0, 'a', Color::BLACK);
        g.put(1, 0, 'b', Color::BLACK);
        g.put(0, 1, 'c', Color::BLACK);
        g.put(1, 1, 'd', Color::BLACK);
        let out = encode(&g);
        let text: String = visible(&out)
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        assert_eq!(text, "abcd");
    }

    #[test]
    fn rows_are_reset_independently() {
        let mut g = grid(1, 2);
        g.put(0, 0, 'A', Color::rgb(1, 1, 1));
        let out = encode(&g);
        let rows: Vec<&str> = out.split('\n').collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].ends_with(RESET));
        assert!(!rows[1].contains("\u{1b}[38"), "row 2 starts from default");
    }

    // --- diff encoding ------------------------------------------------------

    fn two_grids() -> (TuiGrid, TuiGrid) {
        let mut a = grid(4, 1);
        a.fill_background(rect(0.0, 0.0, 40.0, 10.0), Color::rgb(10, 20, 30));
        a.put(0, 0, 'X', Color::rgb(200, 200, 200));
        let mut b = a.clone();
        // Flip one cell: 'X' -> 'Y' with a different foreground.
        b.put(0, 0, 'Y', Color::rgb(11, 22, 33));
        (a, b)
    }

    #[test]
    fn identical_grids_diff_to_no_changes() {
        let g = grid(4, 2);
        let (out, changed, escapes) = diff_encode(&g, &g);
        assert_eq!(changed, 0);
        // Only the leading reset; no cursor moves, no content.
        assert_eq!(out, RESET);
        assert_eq!(escapes, 1);
    }

    #[test]
    fn diff_emits_only_the_changed_cell() {
        let (a, b) = two_grids();
        let (out, changed, _escapes) = diff_encode(&b, &a);
        assert_eq!(changed, 1, "exactly one cell changed");
        assert!(
            out.starts_with(RESET),
            "a diff starts from a known colour baseline"
        );
        assert!(out.contains("\u{1b}[1;1H"), "the changed cell is positioned");
        assert!(out.contains('Y'), "the new glyph is written");
        // The new foreground SGR for (11,22,33).
        assert!(out.contains("\u{1b}[38;2;11;22;33m"));
        assert!(
            !out.contains("\u{1b}[2J"),
            "a patch never clears the whole screen"
        );
    }

    #[test]
    fn diff_is_shorter_than_a_full_reencode() {
        let mut a = grid(80, 24);
        a.fill_background(rect(0.0, 0.0, 800.0, 240.0), Color::rgb(1, 2, 3));
        a.put(0, 0, 'X', Color::rgb(200, 200, 200));
        let mut b = a.clone();
        b.put(0, 0, 'Y', Color::rgb(200, 200, 200));
        let full = encode(&b);
        let (patch, changed, _) = diff_encode(&b, &a);
        assert_eq!(changed, 1);
        assert!(
            patch.len() < full.len(),
            "patching one cell must cost far less than re-encoding the screen: {} vs {}",
            patch.len(),
            full.len()
        );
    }
}
