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
//!
//! ## Idle harness (device case G1-4)
//!
//! The default loop calls `request_frame` from `about_to_wait` and rewrites a
//! frame counter into the label on every frame, so it is *always* awake — which
//! makes "with no events, no frames are requested" unfalsifiable with this
//! binary. To make G1-4 observable, run with `FLOEM2_WINDOW_IDLE=1`:
//!
//! ```sh
//! FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=30 cargo run -p floem2-window --features real
//! ```
//!
//! The keep-awake call is dropped and the label is never rewritten, so the tree
//! settles after the initial cascade. A watchdog thread prints `RunStats` after
//! `FLOEM2_WINDOW_IDLE_SECS` (default 30) and exits. **PASS:** `frames` and
//! `auto_redraws` freeze at a small number. **FAIL:** they keep climbing — the
//! runner is spinning on its own.

#![cfg(feature = "real")]

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use floem_app::{AppRunner, NodeId, RunStats, WidgetHost};
use floem_core::{Axis, LayoutRequest};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
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
    /// Device case G1-4 harness. `None` in the default path (the loop is kept
    /// awake on purpose); `Some` when `FLOEM2_WINDOW_IDLE=1` is set, in which
    /// case the keep-awake call is dropped and the label is never re-written so
    /// the tree can actually settle — only then can the "park when static"
    /// contract be observed.
    idle: Option<IdleProbe>,
}

/// Publishes [`RunStats`] from the event loop to a watchdog thread that prints
/// and exits after a fixed idle window, so device case G1-4 ("with no events,
/// no frames are requested") is falsifiable instead of assumed.
///
/// The reason this exists: the default path calls `request_frame` from
/// `about_to_wait` and rewrites the frame counter into the label every frame,
/// so the loop is *always* awake and the runner's own park-when-static contract
/// can never be seen. A case that the only available harness cannot fail is the
/// same defect class as a CI gate that cannot fail — it looks green while
/// proving nothing.
struct IdleProbe {
    shared: Arc<Mutex<IdleSnapshot>>,
    /// `about_to_wait` invocations: how many times the loop was about to block.
    /// On a correctly-parking loop this stops at a handful after the initial
    /// cascade; a spinning loop drives it into the thousands.
    waits: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct IdleSnapshot {
    frames: u64,
    redraws_requested: u64,
    auto_redraws: u64,
    events: u64,
    waits: u64,
}

impl IdleProbe {
    fn publish(&self, waits: u64, stats: RunStats) {
        let mut g = self.shared.lock().unwrap();
        g.frames = stats.frames;
        g.redraws_requested = stats.redraws_requested;
        g.auto_redraws = stats.auto_redraws;
        g.events = stats.events;
        g.waits = waits;
    }
}

impl std::fmt::Display for IdleSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "frames={} redraws_requested={} auto_redraws={} events={} about_to_wait={}",
            self.frames, self.redraws_requested, self.auto_redraws, self.events, self.waits
        )
    }
}

/// Builds the idle probe from `FLOEM2_WINDOW_IDLE=1` (and optional
/// `FLOEM2_WINDOW_IDLE_SECS`, default 30). Spawns the watchdog thread that
/// sleeps, prints the last published snapshot, and exits — the winit loop
/// cannot be stopped from another thread without an `EventLoopProxy`, and this
/// binary exists to be killed, so exiting here is what makes the number read.
fn idle_probe_from_env() -> Option<IdleProbe> {
    let on = env::var("FLOEM2_WINDOW_IDLE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !on {
        return None;
    }
    let secs = env::var("FLOEM2_WINDOW_IDLE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    let shared = Arc::new(Mutex::new(IdleSnapshot::default()));
    let watchdog = shared.clone();
    eprintln!("idle probe armed for {secs}s (device case G1-4)");
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(secs));
        let snap = *watchdog.lock().unwrap();
        eprintln!("idle probe after {secs}s: {snap}");
        std::process::exit(0);
    });
    Some(IdleProbe { shared, waits: 0 })
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
        match &mut self.idle {
            None => {
                // Keep the loop awake: the frame counter gives every frame a
                // small damage region, which is what exercises damage-driven
                // skipping and partial rasterisation. Without this the loop
                // would block after the first frame (nothing else changes on
                // its own).
                self.runner.request_frame(platform);
            }
            Some(probe) => {
                // Idle harness (G1-4): drop the keep-awake call and let the
                // runner decide. Publish and trace so the watchdog sees the
                // settle — and so a loop that refuses to park is visible.
                probe.waits += 1;
                probe.publish(probe.waits, self.runner.stats());
                if probe.waits <= 4 || probe.waits % 600 == 0 {
                    let snap = *probe.shared.lock().unwrap();
                    eprintln!("[idle] waits={} {}", probe.waits, snap);
                }
            }
        }
    }
}

impl WindowApp {
    /// Rewrites the label: on every frame (the frame counter) or when the
    /// click counter moved since the last sync.
    fn sync_label(&mut self) {
        // Idle harness (G1-4): never churn the label, or the tree never
        // settles and the "park when static" contract cannot be observed. The
        // watchdog reads `RunStats` from `about_to_wait`, not from here.
        if self.idle.is_some() {
            return;
        }
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
        idle: idle_probe_from_env(),
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
