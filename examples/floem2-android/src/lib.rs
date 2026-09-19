//! Floem 2.0 on Android — the same stack as `floem2-window`, but driven by the
//! Android (winit) event loop instead of the desktop one.
//!
//! `native-activity` loads `libfloem2_android_example.so` and calls
//! [`android_main`]; that is why this crate is a `cdylib`. The GPU surface is
//! built from the Android `ANativeWindow*` by the renderer exactly the way the
//! desktop window is — the only Android-specific seam is the entry point and the
//! [`AndroidRunner`] callbacks that pump the app from winit's push loop.
//!
//! This is the floem2 analogue of `examples/android` (the v1 entry point) and
//! closes the last gap in `platform/floem-platform-android`: turning the
//! compiling L4 backend + `AppRunner` into something an Android process can
//! actually launch and render.
//!
//! The GPU presenter is behind the `real` feature (see `Cargo.toml`), so the
//! Android module is only compiled when both the Android target and that feature
//! are active — which is exactly the configuration the APK build uses.

#[cfg(all(target_os = "android", feature = "real"))]
mod android {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    use floem_app::{AppRunner, NodeId, WidgetHost};
    use floem_core::{Axis, Color, LayoutRequest};
    use floem_layout::LayoutStyle;
    use floem_platform::WindowConfig;
    use floem_platform_android::{run, set_android_app, AndroidPlatform, AndroidRunner};
    use floem_renderer_vello::VelloGpuPresenter;
    use floem_types::Size;
    use floem_widgets::{Button, Stack, Text};
    use winit::platform::android::activity::AndroidApp;

    /// The application: an [`AppRunner`] plus a tiny counter UI.
    ///
    /// The runner *is* the app once it is wired to the platform and presenter;
    /// the only extra state is the counter, updated inside the frame hook —
    /// the same place everything else in the tree is updated.
    struct AndroidWindowApp {
        runner: AppRunner<VelloGpuPresenter>,
        clicked: Arc<AtomicI32>,
        /// Resolved lazily once the first frame has mounted the tree.
        label: Option<NodeId>,
        /// Last counter value written to logcat, so a tap is observable in the
        /// log rather than only as pixels (see `sync_label`).
        logged: i32,
    }

    impl AndroidRunner for AndroidWindowApp {
        fn events(&mut self, platform: &mut AndroidPlatform) {
            // The frame hook: drain the platform queue. Input updates the widget
            // tree and requests redraws; `RedrawRequested` runs a frame and
            // presents it through the GPU presenter.
            self.runner.pump(platform);
            self.sync_label();
        }

        fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
            // Drain the queue once more so redraw requests the runner queued
            // during `events` are honoured. Do NOT request a frame here: that
            // makes every `about_to_wait` produce another `RedrawRequested`,
            // so the loop never parks and burns a core (measured: 2.7M redraws
            // in 8s on the emulator). The runner already requests a frame when
            // the tree is dirty, which is what drives the first frame too.
            self.runner.pump(platform);
            self.sync_label();
        }
    }

    impl AndroidWindowApp {
        /// Rewrites the label when the click counter moved since the last sync.
        fn sync_label(&mut self) {
            let clicked = self.clicked.load(Ordering::SeqCst);
            let node = match self.label {
                Some(node) => node,
                None => {
                    let Some(found) = find_label(&self.runner) else {
                        return;
                    };
                    self.label = Some(found);
                    // Evidence the tree is mounted and addressable, so a device
                    // run can tell "rendered" apart from "started". Stderr is
                    // deliberate: Android surfaces it in logcat as
                    // `RustStdoutStderr`, whatever the logger's level is.
                    eprintln!(
                        "floem2-android: widget tree mounted ({} nodes)",
                        self.runner.host().node_count()
                    );
                    found
                }
            };
            if let Some(text) = self.runner.host_mut().widget_mut::<Text>(node) {
                text.set_content(format!("clicked {clicked} times"));
            }
            if clicked != self.logged {
                // A tap reached the widget: pointer event -> L4 queue -> input
                // routing -> `on_click`. This is the observable tap evidence.
                self.logged = clicked;
                eprintln!("floem2-android: pointer -> button click, counter = {clicked}");
            }
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

    /// Android entry point invoked by the `native-activity` glue.
    #[no_mangle]
    pub fn android_main(app: AndroidApp) {
        // Hand the `AndroidApp` to the platform so `run` can build the winit
        // event loop that requires it.
        set_android_app(app);

        let mut host = WidgetHost::new();

        // A minimal counter: a text label and a button. The button only bumps a
        // shared atomic; the label is rewritten from the frame hook
        // (`sync_label`), which is how state reaches an immutable-text widget
        // without a reactive layer.
        let clicked = Arc::new(AtomicI32::new(0));
        let flag = clicked.clone();
        let root = Stack::new(Axis::Vertical)
            .gap(16.0)
            .push_layout(LayoutRequest::new(), || {
                // `Text::new` defaults to `Color::BLACK`, and nothing here paints
                // a background, so the default surface (black) would swallow the
                // label. Name the colour explicitly.
                Box::new(Text::new("clicked 0 times").size(20.0).color(Color::WHITE))
            })
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), move || {
                let flag = flag.clone();
                Box::new(Button::new("click me").on_click(move || {
                    flag.fetch_add(1, Ordering::SeqCst);
                }))
            });

        host.mount_root(LayoutStyle::default(), root);

        let presenter = match VelloGpuPresenter::new() {
            Some(p) => p,
            None => {
                eprintln!("no usable GPU adapter; this example needs a GPU presenter");
                return;
            }
        };
        eprintln!("GPU adapter: {}", presenter.adapter_info());

        let app = AndroidWindowApp {
            runner: AppRunner::new(host, presenter),
            clicked,
            label: None,
            logged: 0,
        };

        let windows = vec![WindowConfig {
            title: "floem2 — Android".into(),
            size: Size::new(360.0, 640.0),
            resizable: false,
        }];

        if let Err(err) = run(app, windows) {
            eprintln!("android loop failed: {err}");
        }
    }
}
