//! Floem 2.0 in a **real OS window**: winit event loop in, GPU pixels out.
//!
//! This is the first floem-2 binary that opens an actual window. The whole
//! stack is the one the architecture diagram promises:
//!
//! ```text
//! winit (floem-platform-desktop, feature `winit`)
//!   └─▶ AppRunner (floem-app)  ──▶ WidgetHost ──▶ Scene
//!                                        └─▶ VelloGpuPresenter (feature `gpu`)
//!                                              └─▶ wgpu swap-chain present
//! ```
//!
//! Run it with:
//!
//! ```sh
//! cargo run -p floem2-window --features real
//! ```
//!
//! Without the `real` feature the binary is not built at all
//! (`required-features`), so headless builds and CI never compile wgpu.

#![cfg(feature = "real")]

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use floem_app::{AppRunner, NodeId, WidgetHost};
use floem_core::{Axis, LayoutRequest};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
use floem_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;
use floem_widgets::{Button, Stack, Text};

/// The application: an [`AppRunner`] plus the counter's UI state.
///
/// The runner *is* the app once it is wired to a platform and a presenter; the
/// only extra state here is the counter/label pair, updated inside the frame
/// hook — the same place everything else in the tree is updated.
struct WindowApp {
    runner: AppRunner<VelloGpuPresenter>,
    clicked: Arc<AtomicI32>,
    /// Value already reflected in the label.
    synced: i32,
    /// Resolved lazily once the first frame has mounted the tree.
    label: Option<NodeId>,
}

impl WinitApp for WindowApp {
    fn events(&mut self, platform: &mut WinitPlatform) {
        // The frame hook: drain the platform queue. Input updates the widget
        // tree and requests redraws; `RedrawRequested` runs a frame and
        // presents it through the GPU presenter.
        self.runner.pump(platform);
        self.sync_label();
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        // Drain again so runner-initiated redraw requests (queued during the
        // last `events` call) are picked up before the loop blocks.
        self.runner.pump(platform);
        self.sync_label();
    }
}

impl WindowApp {
    /// Rewrites the label when the click counter moved since the last sync.
    fn sync_label(&mut self) {
        let clicked = self.clicked.load(Ordering::SeqCst);
        if self.synced == clicked {
            return;
        }
        // The composite children mount during the first frame's reconcile, so
        // the label node only exists after that; resolve it on first need.
        let node = match self.label {
            Some(node) => node,
            None => {
                let Some(found) = find_label(&self.runner) else {
                    return;
                };
                self.label = Some(found);
                found
            }
        };
        let plural = if clicked == 1 { "" } else { "s" };
        if let Some(text) = self.runner.host_mut().widget_mut::<Text>(node) {
            text.set_content(format!("clicked {clicked} time{plural}"));
        }
        self.synced = clicked;
    }
}

fn main() {
    let mut host = WidgetHost::new();

    // A minimal counter: a text label and a fixed-size button. The button only
    // bumps a shared atomic; the label text is rewritten from the frame hook
    // (`WindowApp::sync_label`), which is how state reaches an immutable-text
    // widget without a reactive layer.
    //
    // The button is constructed inside its builder closure (builders are
    // replayable `Fn`s, so each call builds a fresh widget); the shared flag
    // is cloned per construction.
    let clicked = Arc::new(AtomicI32::new(0));
    let flag = clicked.clone();
    let root = Stack::new(Axis::Vertical)
        .gap(16.0)
        // Text has no intrinsic size in the layout engine yet (no text
        // measurement), so children need an explicit extent to get non-zero
        // bounds — otherwise `Text::paint` skips and nothing reaches the GPU.
        .push_layout(LayoutRequest::new().height(28.0), || {
            Box::new(Text::new("clicked 0 times").size(20.0))
        })
        .push_layout(LayoutRequest::new().width(160.0).height(40.0), move || {
            let flag = flag.clone();
            Box::new(Button::new("click me").on_click(move || {
                flag.fetch_add(1, Ordering::SeqCst);
            }))
        });

    host.mount_root(LayoutStyle::default(), root);

    let presenter = VelloGpuPresenter::new().unwrap_or_else(|| {
        eprintln!("no usable GPU adapter; this example needs a GPU presenter");
        std::process::exit(1);
    });
    println!("GPU adapter: {}", presenter.adapter_info());

    let app = WindowApp {
        runner: AppRunner::new(host, presenter),
        clicked,
        synced: 0,
        label: None,
    };

    let windows = vec![WindowConfig {
        title: "floem2 — real window".into(),
        size: Size::new(480.0, 320.0),
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}

/// Finds the counter label node: the first `Text` whose content starts with
/// "clicked".
fn find_label(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
    let host = runner.host();
    for index in 0..host.node_count() {
        if let Some(node) = host.node_at(index) {
            if let Some(text) = host.widget::<Text>(node) {
                if text.content().starts_with("clicked") {
                    return Some(node);
                }
            }
        }
    }
    None
}
