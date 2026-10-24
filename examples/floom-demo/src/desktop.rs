//! A real OS window: winit (via `floom-platform-desktop`) drives the frames and
//! the vello GPU presenter puts pixels on screen.
//!
//! The only thing this adds over [`crate::headless`] is the loop: the same
//! [`crate::demos::gallery_root`] is mounted, and `pump` is called from both
//! `events` and `about_to_wait`. `request_frame` is deliberately *not* called —
//! the runner already asks for a frame when the tree is dirty, so an idle
//! gallery parks instead of spinning a core.

use floom_app::{AppRunner, WidgetHost};
use floom_layout::LayoutStyle;
use floom_platform::WindowConfig;
use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floom_renderer_vello::VelloGpuPresenter;
use floom_types::Size;

use crate::demos;

/// The window's logical size, mirrored by the host's viewport.
const WINDOW: Size = Size::new(420.0, 760.0);

struct DemoApp {
    runner: AppRunner<VelloGpuPresenter>,
}

impl WinitApp for DemoApp {
    fn events(&mut self, platform: &mut WinitPlatform) {
        // The panel reads the tree through a cell; refresh it before the frame so
        // what it shows is the gallery as it is now.
        #[cfg(feature = "devtools")]
        self.devtools.refresh_tree(self.runner.host());
        // Input updates the tree and requests redraws; `RedrawRequested` runs a
        // frame and presents it through the GPU presenter.
        self.runner.pump(platform);
    }

    // The devtools shortcut is asked *before* the tree sees the key: a keystroke
    // that opens a panel should not also type into the control that had focus.
    // Returning true here drops the key, so the gallery never receives it.
    #[cfg(feature = "devtools")]
    fn key(&mut self, event: &floom_types::KeyEvent) -> bool {
        self.devtools.handle_key(event)
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

    // One line, and F12 opens the devtools panel over the gallery: the element
    // tree, the breadcrumb to the selected node, and that node's style or
    // layout. See `crates/floom/src/devtools.rs` for the door this is.
    #[cfg(feature = "devtools")]
    let mut devtools = Some(floom::devtools::attach_elements(&mut host));
    // The shortcut cannot be hooked from inside the tree -- the key event
    // arrives at the platform loop first -- so the loop hands it over here.
    #[cfg(not(feature = "devtools"))]
    let mut devtools: Option<()> = None;
    let _ = &mut devtools;

    let presenter = VelloGpuPresenter::new().unwrap_or_else(|| {
        eprintln!("no usable GPU adapter; floom-demo needs a GPU presenter");
        std::process::exit(1);
    });
    println!("GPU adapter: {}", presenter.adapter_info());

    let app = DemoApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floom — demo gallery".into(),
        size: WINDOW,
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
