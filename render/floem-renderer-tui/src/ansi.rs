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

use crate::TuiGrid;

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
}
