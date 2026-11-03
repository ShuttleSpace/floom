//! Terminal backend tests: a real widget tree reaching real characters.
//!
//! The unit tests inside `floom-renderer-tui` prove the grid and the encoder in
//! isolation; this suite proves the **pipeline** does, using the same
//! `WidgetHost` the rest of the app harness drives. That distinction matters
//! because the interesting failure modes live at the seam: text that shapes
//! but carries no string, or glyph positions in units this backend assumed
//! were something else, both look fine from either side alone.
//!
//! The layering direction is deliberate — `floom-app` may depend *down* onto a
//! renderer backend, so the end-to-end assertion lives here rather than pulling
//! widgets into an L4 crate.

use floom_app::WidgetHost;
use floom_core::Size;
use floom_layout::LayoutStyle;
use floom_renderer::Presenter;
use floom_renderer_tui::{TerminalSize, TuiPresenter};
use floom_types::{SurfaceId, WindowId};
use floom_widgets::Text;

fn surface(size: Size) -> floom_renderer::AttachedSurface {
    floom_renderer::AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, 1.0, None)
}

fn host_with_label(content: &str) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 40.0));
    host.mount_root(LayoutStyle::default(), Text::new(content).size(14.0));
    host.frame();
    host
}

/// The characters of the busiest row, left to right, trimmed of padding.
fn busiest_row(grid: &floom_renderer_tui::TuiGrid) -> String {
    let best = (0..grid.rows())
        .map(|row| {
            (0..grid.cols())
                .filter_map(|col| grid.cell(col, row))
                .filter(|c| c.ch != ' ')
                .map(|c| c.ch)
                .collect::<String>()
        })
        .max_by_key(|s| s.chars().count())
        .unwrap_or_default();
    best
}

#[test]
fn a_labels_characters_reach_the_terminal_grid() {
    let host = host_with_label("Hello");
    // 120 logical units across 120 columns, 40 rows: one unit per cell, plenty
    // of room for the baseline row the shaper reports.
    let mut presenter = TuiPresenter::new(TerminalSize::new(120, 40), Vec::new());
    assert!(presenter.attach(surface(Size::new(120.0, 40.0))));
    presenter.present(host.scene(), host.resources());

    let grid = presenter.last_grid().expect("the frame produced a grid");
    assert_eq!(grid.stats().text_runs, 1, "the label painted");
    assert_eq!(
        busiest_row(grid),
        "Hello",
        "the label's characters survive shaping and land in order"
    );
}

#[test]
fn the_same_host_frame_can_be_presented_twice() {
    let host = host_with_label("Hi");
    let mut presenter = TuiPresenter::new(TerminalSize::new(60, 20), Vec::new());
    presenter.attach(surface(Size::new(60.0, 20.0)));
    presenter.present(host.scene(), host.resources());
    presenter.present(host.scene(), host.resources());
    assert_eq!(presenter.frames_presented(), 2);
    assert_eq!(busiest_row(presenter.last_grid().unwrap()), "Hi");
}

#[test]
fn a_wider_terminal_puts_the_same_glyph_in_a_different_column() {
    // Proves the grid maps logical space rather than remembering pixels: twice
    // the columns means twice the cells per unit, so the same glyph sits twice
    // as far to the right.
    let host = host_with_label("Hi");
    let logical = Size::new(120.0, 40.0);

    let mut narrow = TuiPresenter::new(TerminalSize::new(60, 40), Vec::new());
    narrow.attach(surface(logical));
    narrow.present(host.scene(), host.resources());

    let mut wide = TuiPresenter::new(TerminalSize::new(120, 40), Vec::new());
    wide.attach(surface(logical));
    wide.present(host.scene(), host.resources());

    // The first glyph sits at column 0 in both grids (the label is left
    // aligned), so compare the *last* one: its advance is what spreads over
    // twice as many cells when there are twice as many columns.
    let col_of_last = |grid: &floom_renderer_tui::TuiGrid| {
        (0..grid.rows())
            .flat_map(|row| (0..grid.cols()).rev().map(move |col| (col, row)))
            .find_map(|(col, row)| grid.cell(col, row).filter(|c| c.ch != ' ').map(|_| col))
            .expect("no ink in the grid")
    };
    let narrow_col = col_of_last(narrow.last_grid().unwrap());
    let wide_col = col_of_last(wide.last_grid().unwrap());
    assert!(
        wide_col > narrow_col,
        "more columns means finer columns: {narrow_col} -> {wide_col}"
    );
}
