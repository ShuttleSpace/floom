// SPDX-License-Identifier: MIT
//! Pixel-level evidence for the terminal backend.
//!
//! The terminal's picture is a **character grid**, not a framebuffer, so its
//! "pixel" is a cell: a character, a foreground and a background. The
//! snapshot here is that grid, rendered as text — one line per row, one
//! entry per cell — which is a picture a person can read in a diff.
//!
//! # Why not the ANSI bytes
//!
//! The presenter does emit ANSI escape sequences, and hashing those would be
//! simpler. It would also be worse: the bytes say *how* the terminal was
//! told to paint, not *what* was painted. A change in how the escapes are
//! emitted — a different but equivalent sequence — would move the hash
//! without changing a single character on the screen, and a golden that
//! moves for a change nobody can see is a golden people learn to re-bless.
//!
//! The grid says what is on the screen, so that is what is hashed.
//!
//! # What the reference is
//!
//! There is no second terminal backend to compare against, so the reference
//! is the **cell content itself**: the tests assert that a shape reaches the
//! cells it should and the cells it should not, and the golden pins the
//! whole picture.

use floom_renderer::{assert_snapshot, snapshots_dir};
use floom_renderer_tui::{Cell, TuiGrid, paint_scene};
use floom_scene::{Rect, Scene, SceneNode};
use floom_types::{Brush, Color, Point, Size};

/// A scene with a filled block and a smaller block beside it.
///
/// Two shapes, so a test can tell "the block is there" from "the block is in
/// the right place": the second one is offset, and a backend that filled the
/// whole grid would pass the first check and fail the second.
fn scene() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    // The grid is 16x6 cells over 160x60 logical units, so a cell is 10x10
    // logical units. The shapes are in logical coordinates and span whole
    // cells: the first covers columns 1..3, rows 0..1; the second covers
    // columns 6..8, rows 1..2.
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 10.0,
            y0: 0.0,
            x1: 30.0,
            y1: 20.0,
        },
        fill: Brush::Solid(Color::rgb(220, 40, 40)),
        radius: 0.0,
        shadow: None,
    });
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 60.0,
            y0: 10.0,
            x1: 80.0,
            y1: 30.0,
        },
        fill: Brush::Solid(Color::rgb(40, 120, 220)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    scene
}

/// The grid, painted.
fn grid() -> TuiGrid {
    let mut grid = TuiGrid::new(16, 6, Size::new(160.0, 60.0));
    paint_scene(&mut grid, &scene(), &floom_scene::ResourceRegistry::new());
    grid
}

/// The grid as text: one line per row, cells separated by a space.
///
/// The character and the two colours, so a golden shows not just *what* is
/// on the screen but in what colour. A blank cell is a space with no
/// colours, which is not the same as a space painted in the background
/// colour.
fn grid_text(g: &TuiGrid) -> String {
    let mut out = String::new();
    for row in 0..g.rows() {
        for col in 0..g.cols() {
            if col > 0 {
                out.push('|');
            }
            let Some(cell) = g.cell(col, row) else {
                out.push('?');
                continue;
            };
            out.push_str(&cell_text(cell));
        }
        out.push('\n');
    }
    out
}

/// One cell as text.
fn cell_text(cell: &Cell) -> String {
    let ch = cell.ch;
    match (cell.fg, cell.bg) {
        (None, None) => format!("{ch}"),
        (fg, bg) => {
            let fg = fg.map(|c| format!("{:?}", c)).unwrap_or_default();
            let bg = bg.map(|c| format!("{:?}", c)).unwrap_or_default();
            format!("{ch}[{fg}/{bg}]")
        }
    }
}

#[test]
fn a_filled_block_reaches_the_cells_it_should() {
    let g = grid();

    // The first block covers columns 1..3, rows 0..2.
    let filled = g.cell(1, 0).expect("a cell");
    assert_eq!(
        filled.bg,
        Some(Color::rgb(220, 40, 40)),
        "the block's colour"
    );
    assert_eq!(filled.ch, ' ', "a filled cell is still a space");

    // And the cells outside it are untouched.
    let outside = g.cell(4, 0).expect("a cell");
    assert_eq!(outside.bg, None, "the cell past the block is empty");
}

#[test]
fn the_second_block_is_in_its_own_place() {
    // The check that a backend filling the whole grid would fail.
    let g = grid();
    let second = g.cell(6, 1).expect("a cell");
    assert_eq!(
        second.bg,
        Some(Color::rgb(40, 120, 220)),
        "the second block is where it was asked to be"
    );
    assert_eq!(
        g.cell(1, 4).expect("a cell").bg,
        None,
        "and the grid is not simply filled"
    );
}

#[test]
fn the_grid_has_a_golden() {
    // The whole picture, as text a person can read in a diff.
    let g = grid();
    let text = grid_text(&g);
    assert_snapshot(snapshots_dir!(), "tui_two_blocks", &text);
}

#[test]
fn a_grid_of_a_different_size_is_a_different_picture() {
    // The size travels with the text, so a grid that changed shape is a
    // different picture rather than the same one truncated.
    let mut wide = TuiGrid::new(20, 6, Size::new(200.0, 60.0));
    paint_scene(&mut wide, &scene(), &floom_scene::ResourceRegistry::new());
    assert_ne!(
        grid_text(&grid()),
        grid_text(&wide),
        "a wider grid is a different picture"
    );
}

#[test]
fn a_point_is_in_the_scene() {
    // A guard on the fixture: if the scene stopped having shapes, the
    // probes above would be measuring nothing and would still pass.
    let scene = scene();
    let mut rects = 0;
    for (_id, node) in scene.iter() {
        if matches!(node, SceneNode::Rect { .. }) {
            rects += 1;
        }
    }
    assert_eq!(rects, 2, "the fixture has two shapes");
    let _ = Point::ZERO;
}
