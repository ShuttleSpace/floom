//! End-to-end test of the terminal *platform* backend driving a real widget tree.
//!
//! `tui_backend.rs` proves scene → grid through the renderer. This suite proves
//! the runtime seam that sits around it: a [`TuiPlatform`] feeding an
//! [`AppRunner`] that hosts a real widget tree, where creating a window, pumping
//! a redraw, and injecting keystrokes all flow through the same
//! `Platform`/`Presenter` wiring every other backend uses. If the platform
//! backend's events were shaped wrong, the runner would not turn them into a
//! frame or a dispatched key.

use floem_app::{AppRunner, WidgetHost};
use floem_core::Size;
use floem_layout::LayoutStyle;
use floem_platform::{Platform, WindowConfig};
use floem_renderer::Presenter;
use floem_renderer_tui::TuiPresenter;
use floem_widgets::Text;

use floem_platform_tui::{TerminalSize as PlatformSize, TuiPlatform};

fn host_with_label(content: &str) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 40.0));
    host.mount_root(LayoutStyle::default(), Text::new(content).size(14.0));
    host.frame();
    host
}

/// Every non-space character in the grid, row-major (top to bottom, then left
/// to right). The label may wrap across rows, so the assertion checks the whole
/// grid rather than guessing which row is "busiest".
fn grid_text(grid: &floem_renderer_tui::TuiGrid) -> String {
    (0..grid.rows())
        .flat_map(|row| {
            (0..grid.cols())
                .filter_map(move |col| grid.cell(col, row))
                .filter(|c| c.ch != ' ')
                .map(|c| c.ch)
        })
        .collect()
}

#[test]
fn a_window_is_created_and_a_frame_renders_the_label() {
    let host = host_with_label("HelloTUI");
    let presenter = TuiPresenter::new(floem_renderer_tui::TerminalSize::new(200, 40), Vec::new());
    let mut platform = TuiPlatform::with_override(Some(PlatformSize::new(200, 40)));
    let mut runner = AppRunner::new(host, presenter);

    // Creating the window queues a `WindowCreated` event for the next pump.
    let win = platform.create_window(WindowConfig::default());
    let s1 = runner.pump(&mut platform);
    assert_eq!(s1.windows_created, 1, "the platform produced one window");

    // The redraw that creation requested renders the first frame.
    let _s2 = runner.pump(&mut platform);
    assert!(
        runner.presenter().frames_presented() >= 1,
        "a frame was presented"
    );
    let grid = runner.presenter().last_grid().unwrap();
    let text = grid_text(grid);
    assert!(
        text.contains("Hello"),
        "the label's characters reached the terminal grid: {text:?}"
    );
    assert!(
        text.contains("TUI"),
        "the whole label (including the wrapped tail) reached the grid: {text:?}"
    );

    // The runner attached the presenter to the created window.
    assert_eq!(runner.primary_window(), Some(win));
}

#[test]
fn injected_text_is_dispatched_into_the_tree() {
    let host = host_with_label("Type:");
    let presenter = TuiPresenter::new(floem_renderer_tui::TerminalSize::new(80, 24), Vec::new());
    let mut platform = TuiPlatform::with_override(Some(PlatformSize::new(80, 24)));
    let mut runner = AppRunner::new(host, presenter);

    platform.create_window(WindowConfig::default());
    runner.pump(&mut platform); // window created
    runner.pump(&mut platform); // first frame rendered
    assert_eq!(runner.presenter().frames_presented(), 1);

    let before = runner.stats().input_dispatched;
    // Inject "ab" — two Latin characters committed as IME text.
    platform.inject_input(b"ab");
    runner.pump(&mut platform); // input events drained + dispatched

    let after = runner.stats().input_dispatched;
    assert!(
        after > before,
        "the injected keystrokes were dispatched into the widget tree: {before} -> {after}"
    );

    // A keypress requests a repaint; pump it so the frame loop settles.
    runner.pump(&mut platform);
    assert!(
        runner.presenter().frames_presented() >= 2,
        "input triggered a follow-up frame"
    );
}

#[test]
fn injected_cjk_text_is_also_committed() {
    // The whole point of the text-via-ImeCommit path: a non-Latin character has
    // no `KeyCode`, yet it must still reach the tree.
    let host = host_with_label("Input:");
    let presenter = TuiPresenter::new(floem_renderer_tui::TerminalSize::new(80, 24), Vec::new());
    let mut platform = TuiPlatform::with_override(Some(PlatformSize::new(80, 24)));
    let mut runner = AppRunner::new(host, presenter);

    platform.create_window(WindowConfig::default());
    runner.pump(&mut platform);
    runner.pump(&mut platform);

    let before = runner.stats().input_dispatched;
    platform.inject_input("你".as_bytes());
    runner.pump(&mut platform);
    let after = runner.stats().input_dispatched;
    assert!(
        after > before,
        "a CJK character was committed through the IME path: {before} -> {after}"
    );
}
