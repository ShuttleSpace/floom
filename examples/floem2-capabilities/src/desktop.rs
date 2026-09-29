//! A real OS window: winit (via `floem-platform-desktop`) drives the frames and
//! the vello GPU presenter puts pixels on screen.
//!
//! The only thing this adds over [`crate::headless`] is the loop: the same
//! [`crate::ui::gallery_root`] is mounted, and `pump` is called from both
//! `events` and `about_to_wait`. `request_frame` is deliberately *not* called —
//! the runner already asks for a frame when the tree is dirty, so an idle
//! gallery parks instead of spinning a core.
//!
//! On this target every OS-backed row in the Readings tab will read `—`. That
//! is the point, not a bug: a Mac has no `Vibrator`, no `BatteryManager` and no
//! `UIDevice`, and the gallery says so in the same typeface as the rows that
//! answer. The same binary on Android fills the table in.

use floem_app::{AppRunner, WidgetHost};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
use floem_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;

use crate::ui;

/// Window size wide enough for the two-column capability table.
const WINDOW: Size = Size::new(900.0, 820.0);

struct CapabilitiesApp {
    runner: AppRunner<VelloGpuPresenter>,
}

impl WinitApp for CapabilitiesApp {
    fn events(&mut self, platform: &mut WinitPlatform) {
        // Input updates the tree and requests redraws; `RedrawRequested` runs a
        // frame and presents it through the GPU presenter.
        self.runner.pump(platform);
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        // Drain once more so redraw requests queued during `events` are honoured
        // before the loop blocks. No `request_frame` here: it would make every
        // wake-up produce another redraw and the loop would never park.
        self.runner.pump(platform);
    }
}

/// Open the gallery in a window and run until it closes.
pub fn launch() -> Result<(), String> {
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    host.mount_root(LayoutStyle::default(), ui::gallery_root());

    let presenter = VelloGpuPresenter::new().ok_or("no usable GPU adapter")?;
    println!("GPU adapter: {}", presenter.adapter_info());

    let app = CapabilitiesApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floem2 — platform capabilities".into(),
        size: WINDOW,
        resizable: true,
    }];
    run(app, windows)
}
