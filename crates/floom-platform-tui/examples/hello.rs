//! A minimal terminal app: mounts a label and runs the real event loop.
//!
//! Run with a real terminal:
//!
//! ```text
//! cargo run -p floom-platform-tui --example hello
//! ```
//!
//! Ctrl-C / Ctrl-D quits. Resize the window to watch the grid rebuild and the
//! frame re-lay out. Piped output (CI, tests) is detected and left untouched,
//! and stdin EOF exits cleanly, so the loop is also drivable headless.

use floom_app::{AppRunner, WidgetHost};
use floom_layout::LayoutStyle;
use floom_platform_tui::{TuiApp, TuiPlatform, run};
use floom_renderer_tui::TuiPresenter;
use floom_widgets::Text;

/// The app owns the runner; the loop owns the terminal.
struct HelloApp {
    runner: Option<AppRunner<TuiPresenter<std::io::Stdout>>>,
}

impl TuiApp for HelloApp {
    fn events(&mut self, platform: &mut TuiPlatform) {
        // Build the host + runner once, on the first frame hook.
        if self.runner.is_none() {
            let mut host = WidgetHost::new();
            host.mount_root(
                LayoutStyle::default(),
                Text::new("Hello from floom2 TUI!").size(14.0),
            );
            let size = platform.size();
            let presenter = TuiPresenter::new(
                floom_renderer_tui::TerminalSize::new(size.cols, size.rows),
                std::io::stdout(),
            );
            self.runner = Some(AppRunner::new(host, presenter));
        }
        if let Some(runner) = self.runner.as_mut() {
            runner.pump(platform);
        }
    }
}

fn main() {
    let app = HelloApp { runner: None };
    if let Err(e) = run(app) {
        eprintln!("tui run failed: {e}");
        std::process::exit(1);
    }
}
