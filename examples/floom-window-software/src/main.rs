//! One window, the software renderer, and a report of what it got.
//!
//! The other window examples need a GPU. This one needs only a window server,
//! which is what makes it possible to run "the stack opens a window and draws"
//! on a machine with no GPU — a CI runner under Xvfb, for instance. See
//! `scripts/linux-smoke.sh`.
//!
//! ```sh
//! xvfb-run -a cargo run -p floom-window-software
//! ```
//!
//! `FLOOM_SOFTWARE_EXIT_FRAMES` (default 8) bounds the run: the app closes the
//! loop itself, so the script never has to kill a window that is waiting for a
//! human.

use std::env;
use std::time::{Duration, Instant};

use floom_app::AppRunner;
use floom_core::{Axis, LayoutRequest};
use floom_platform::{Platform, Size, WindowConfig, WindowId};
use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floom_renderer_software::SoftwarePresenter;
use floom_widgets::{Button, Stack};

struct SoftwareApp {
    runner: AppRunner<SoftwarePresenter>,
    window: Option<WindowId>,
    frames: u32,
    exit_after: u32,
    /// The loop only hands out frames while the tree is unsettled, so a frame
    /// budget alone can wait forever on a still tree. This is the other half.
    deadline: Instant,
}

impl WinitApp for SoftwareApp {
    fn window_created(&mut self, platform: &mut WinitPlatform, window: WindowId) {
        self.window = Some(window);
        // The evidence line: a window exists and the OS gave it a surface of a
        // real size. The script asserts on it, so it cannot pass silently.
        if let Some(created) = platform.window(window) {
            let size = created.size();
            eprintln!(
                "floom-software: window surface {}x{} at scale {}",
                size.width,
                size.height,
                created.display_info().scale_factor.get()
            );
        }
    }

    fn events(&mut self, platform: &mut WinitPlatform) {
        let stats = self.runner.pump(platform);
        let _ = platform;
        let _ = platform;
        // The presenter's own pixels are the only proof that a frame was drawn
        // rather than a window opened over an empty buffer.
        let presenter = self.runner.presenter();
        if presenter.width() == 0 || presenter.height() == 0 {
            return;
        }
        let inked = presenter
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[3] > 0)
            .count();
        self.frames += 1;
        // The extra numbers separate "no events reached the tree" from "the tree
        // painted nothing" from "the presenter cleared the buffer" — a frame that
        // arrives empty has to say which.
        eprintln!(
            "floom-software: frame {} drew {}x{} with {inked} inked pixels \
             (events={} scene_ops={} host={:?})",
            self.frames,
            presenter.width(),
            presenter.height(),
            stats.events,
            self.runner.host().scene().len(),
            self.runner.host().size(),
        );
        if inked > 0 && self.frames >= 2 {
            // `run` has no "close the loop" hook for the app, and this binary is
            // driven by a script rather than a human: leaving once a frame has
            // ink is the whole contract.
            eprintln!(
                "floom-software: {} frames is the report; leaving",
                self.frames
            );
            std::process::exit(0);
        }
        if self.frames >= self.exit_after {
            eprintln!(
                "floom-software: {} frames and no ink; leaving without a drawing",
                self.frames
            );
            std::process::exit(0);
        }
    }

    fn about_to_wait(&mut self, _platform: &mut WinitPlatform) {
        if self.deadline.elapsed() > Duration::from_secs(10) {
            eprintln!(
                "floom-software: 10s and {} frames later, the loop is still waiting",
                self.frames
            );
            std::process::exit(0);
        }
    }
}

fn main() {
    let exit_after = env::var("FLOOM_SOFTWARE_EXIT_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8u32);

    let mut host = floom_app::WidgetHost::new();
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Increment"))
            }),
    );

    let app = SoftwareApp {
        runner: AppRunner::new(host, SoftwarePresenter::new()),
        window: None,
        frames: 0,
        exit_after,
        deadline: Instant::now(),
    };

    let windows = vec![WindowConfig {
        title: "floom — software window".into(),
        size: Size::new(320.0, 240.0),
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
    eprintln!("floom-software: the loop returned after {exit_after} frames");
}
