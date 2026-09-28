//! A real OS window: winit (via `floem-platform-desktop`) drives the frames and
//! the vello GPU presenter puts pixels on screen.
//!
//! The only thing this adds over [`crate::headless`] is the loop: the same
//! [`crate::demos::gallery_root`] is mounted, and `pump` is called from both
//! `events` and `about_to_wait`. `request_frame` is deliberately *not* called —
//! the runner already asks for a frame when the tree is dirty, so an idle
//! gallery parks instead of spinning a core.

use floem_app::{AppRunner, WidgetHost};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
use floem_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;

use crate::demos;

/// The window's logical size, mirrored by the host's viewport.
const WINDOW: Size = Size::new(420.0, 760.0);

struct DemoApp {
    runner: AppRunner<VelloGpuPresenter>,
}

impl WinitApp for DemoApp {
    fn events(&mut self, platform: &mut WinitPlatform) {
        // Input updates the tree and requests redraws; `RedrawRequested` runs a
        // frame and presents it through the GPU presenter.
        self.runner.pump(platform);
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        // Drain once more so redraw requests queued during `events` are honoured
        // before the loop blocks.
        self.runner.pump(platform);
    }
}

pub fn launch() {
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    host.mount_root(LayoutStyle::default(), demos::gallery_root());

    let presenter = VelloGpuPresenter::new().unwrap_or_else(|| {
        eprintln!("no usable GPU adapter; floem2-demo needs a GPU presenter");
        std::process::exit(1);
    });
    println!("GPU adapter: {}", presenter.adapter_info());

    let app = DemoApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floem2 — demo gallery".into(),
        size: WINDOW,
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
