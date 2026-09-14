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
use floem_platform::{Platform, WindowConfig};
use floem_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;
use floem_widgets::{Button, Image, Stack, Text};

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
    /// Frames seen, published into the label so every frame carries a small
    /// damage region — that is what exercises damage-driven skipping and the
    /// presenter's partial (damage-region only) rasterisation.
    frames: u32,
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
        // Keep the loop awake: the frame counter gives every frame a small
        // damage region, which is what exercises damage-driven skipping and
        // partial rasterisation. Without this the loop would block after the
        // first frame (nothing else changes on its own).
        if let Some(window) = self.runner.primary_window() {
            platform.request_frame(window);
        }
    }
}

impl WindowApp {
    /// Rewrites the label: on every frame (the frame counter) or when the
    /// click counter moved since the last sync.
    fn sync_label(&mut self) {
        self.frames = self.frames.wrapping_add(1);
        // Low-frequency diagnostics: on a 480x320 window at 2x this prints a
        // rasterisation of ~19k pixels instead of the full 614k, with three
        // of four node fragments reused — partial rasterisation and
        // incremental encoding doing their job.
        if self.frames % 300 == 0 {
            let stats = self.runner.presenter().stats();
            eprintln!(
                "frame {}: rasters {} (partial {}), skips {}, fragments reused {} encoded {}",
                self.frames,
                stats.rasterized_pixels,
                stats.partial_rasterizations,
                stats.present_skips,
                stats.fragments_reused,
                stats.fragments_encoded,
            );
        }
        let clicked = self.clicked.load(Ordering::SeqCst);
        // The label is rewritten every frame on purpose: the frame counter
        // gives each frame a small damage region, which is exactly the case
        // damage-driven skipping and partial rasterisation exist for.
        if self.synced == clicked && self.frames == 1 {
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
            text.set_content(format!(
                "clicked {clicked} time{plural} · frame {}",
                self.frames
            ));
        }
        self.synced = clicked;
    }
}

fn main() {
    let mut host = WidgetHost::new();

    // A synthetic 64×32 gradient, exercised through the *full* image pipeline:
    // premultiplied RGBA8 → PNG encode → decode (`register_image_decoded` →
    // premultiply → registry). Pixels are fully opaque, so the premultiply is
    // a no-op and the on-screen gradient is byte-identical to the source.
    let mut pixels = Vec::with_capacity(64 * 32 * 4);
    for y in 0..32u8 {
        for x in 0..64u8 {
            pixels.extend_from_slice(&[x * 4, y * 8, 128, 255]);
        }
    }
    let png = {
        let (width, height) = (64u32, 32u32);
        let img = image::RgbaImage::from_raw(width, height, pixels.clone())
            .expect("gradient buffer matches its extent");
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)
            .expect("PNG encode");
        buf.into_inner()
    };
    let picture = host
        .register_image_decoded(&png)
        .expect("self-encoded PNG decodes");

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
        // The label needs no explicit extent: `Text::layout_request` reports
        // its measured intrinsic size and the host applies it as the default.
        .push_layout(LayoutRequest::new(), || {
            Box::new(Text::new("clicked 0 times").size(20.0))
        })
        .push_layout(LayoutRequest::new().width(128.0).height(64.0), move || {
            Box::new(Image::new(picture).intrinsic_size(Size::new(64.0, 32.0)))
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
        frames: 0,
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
