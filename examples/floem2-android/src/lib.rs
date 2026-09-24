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
    use floem_platform::{Clipboard, Ime, WindowConfig};
    use floem_platform_android::{run, set_android_app, AndroidPlatform, AndroidRunner};
    use floem_platform_mobile::packages::keyboard::soft_input::soft_keyboard_state;
    use floem_renderer_vello::VelloGpuPresenter;
    use floem_types::{Size, WindowId};
    use floem_widgets::{Button, Stack, Text, TextField};
    use winit::platform::android::activity::AndroidApp;

    /// Text written to the clipboard by the one-shot service probe, and expected
    /// back verbatim. Distinctive on purpose: it has to be unmistakable in
    /// logcat, and it must not collide with anything a previous run left behind.
    const CLIPBOARD_PROBE: &str = "floem2-android clipboard probe 9f3c";

    /// How long to keep watching for the soft keyboard before saying so. A
    /// bounded wait keeps a keyboard that never appears from costing a JNI poll
    /// on every frame for the life of the app.
    const KEYBOARD_POLL_FRAMES: u32 = 600;

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
        /// The realised window, learned from `window_created`.
        window: Option<WindowId>,
        /// The text field, once the tree is mounted. Focused by the probe so a
        /// device run has somewhere for typed characters to land.
        field: Option<NodeId>,
        /// Last text/preedit printed, so a change is logged once rather than
        /// every frame (an idle app would otherwise repeat itself forever).
        last_text: String,
        last_preedit: String,
        /// The platform-service probe runs once, after the tree is mounted.
        probed: bool,
        /// Frames spent waiting for the soft keyboard, and whether it has been
        /// seen. Polling stops on the first sighting (or after
        /// [`KEYBOARD_POLL_FRAMES`]), so a settled app pays nothing.
        keyboard_frames: u32,
        keyboard_seen: bool,
    }

    impl AndroidRunner for AndroidWindowApp {
        fn window_created(&mut self, _platform: &mut AndroidPlatform, window: WindowId) {
            self.window = Some(window);
        }

        fn events(&mut self, platform: &mut AndroidPlatform) {
            // The frame hook: drain the platform queue. Input updates the widget
            // tree and requests redraws; `RedrawRequested` runs a frame and
            // presents it through the GPU presenter.
            self.runner.pump(platform);
            self.sync_label(platform);
            self.sync_text_field();
            self.poll_soft_keyboard();
        }

        fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
            // Drain the queue once more so redraw requests the runner queued
            // during `events` are honoured. Do NOT request a frame here: that
            // makes every `about_to_wait` produce another `RedrawRequested`,
            // so the loop never parks and burns a core (measured: 2.7M redraws
            // in 8s on the emulator). The runner already requests a frame when
            // the tree is dirty, which is what drives the first frame too.
            self.runner.pump(platform);
            self.sync_label(platform);
            self.probe_platform_services(platform);
        }
    }

    impl AndroidWindowApp {
        /// One-shot proof that the OS-service bridges are live.
        ///
        /// Both of these were inert stubs until now, and neither can be checked
        /// by reading code: the JNI bridge is only wired during startup, and the
        /// soft keyboard belongs to the platform. So exercise them for real and
        /// report the outcome — this is the evidence a device run reads.
        fn probe_platform_services(&mut self, platform: &mut AndroidPlatform) {
            if self.probed || self.label.is_none() {
                return;
            }
            self.probed = true;

            // Clipboard: write, then read back. A failed write means the JNI
            // bridge or the service lookup failed (the backend logs which one);
            // a successful write followed by a mismatched read means the
            // round-trip is broken rather than merely absent.
            let wrote = platform.set_text(CLIPBOARD_PROBE);
            let read_back = platform.get_text();
            eprintln!(
                "floem2-android: clipboard write={wrote} read_back={read_back:?} matched={}",
                read_back.as_deref() == Some(CLIPBOARD_PROBE)
            );

            // Soft keyboard: the request is observable from *outside* the
            // process (`adb shell dumpsys input_method`), which is the point —
            // it shows the request reached the system, not just our own state.
            if let Some(window) = self.window {
                platform.set_enabled(window, true);
                eprintln!("floem2-android: IME enable requested");
            }

            // Where the button is, so a device run can prove a tap lands on the
            // rect the app says it painted, rather than trusting a coordinate
            // worked out by hand.
            //
            // Both numbers are *window-relative*, which is what the platform's
            // input path uses: Android delivers motion events relative to the
            // window, so hit-testing is `window_px / scale`. `adb shell input
            // tap` takes *display* coordinates, which differ by the window's
            // origin — the status-bar inset, measured 128 px on the API 34
            // emulator. Add that offset (or measure the drawn rect from a
            // screenshot) before tapping; do not feed these numbers to `input
            // tap` directly.
            // Focus the text field so the characters the IME produces have a
            // destination. Without this a device run types into nothing, and a
            // silent no-op is indistinguishable from a broken bridge.
            if let Some(field) = find_text_field(&self.runner) {
                self.field = Some(field);
                let element = self.runner.host().node_id(field);
                self.runner.host_mut().set_focus(element);
                eprintln!("floem2-android: text field focused");
            }

            if let Some(button) = find_button(&self.runner) {
                let rect = self.runner.host().bounds(button);
                let scale = self.runner.host().scale();
                eprintln!(
                    "floem2-android: button logical=({:.1},{:.1} {:.1}x{:.1}) scale={scale} \
                     window_px=x{:.0}..{:.0} y{:.0}..{:.0}",
                    rect.x0,
                    rect.y0,
                    rect.width(),
                    rect.height(),
                    rect.x0 * scale,
                    rect.x1 * scale,
                    rect.y0 * scale,
                    rect.y1 * scale,
                );
            }
        }

        /// Rewrites the label when the click counter moved since the last sync.
        fn sync_label(&mut self, platform: &mut AndroidPlatform) {
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
                // Read the clipboard again here, well after startup. The probe
                // above races the system: it runs about 30 ms *before*
                // ActivityTaskManager reports the activity `Displayed`, and
                // `ClipboardService` denies reads from an app it does not yet
                // consider focused. A tap happens once the app is settled, so
                // this distinguishes "the read is broken" from "the first read
                // was too early" — and because it reads text written by the
                // startup probe, it also proves the value survived in the
                // system clipboard rather than only inside our process.
                let after_focus = platform.get_text();
                eprintln!(
                    "floem2-android: clipboard read after focus = {after_focus:?} (probe survived: {})",
                    after_focus.as_deref() == Some(CLIPBOARD_PROBE)
                );
            }
        }

        /// Report what the text field holds whenever it changes.
        ///
        /// This is the evidence the IME's characters actually arrived: the
        /// platform reads them out of the anchor, the runner turns each into a
        /// key-down, and the focused field consumes them like typing. Printed
        /// only on change, because the frame hook runs on every frame.
        fn sync_text_field(&mut self) {
            let Some(node) = self.field else { return };
            let Some(field) = self.runner.host().widget::<TextField>(node) else {
                return;
            };
            let text = field.text().to_string();
            let preedit = field.preedit().to_string();
            if text == self.last_text && preedit == self.last_preedit {
                return;
            }
            let composing = field.is_composing();
            eprintln!(
                "floem2-android: text field text={text:?} preedit={preedit:?} composing={composing}"
            );
            self.last_text = text;
            self.last_preedit = preedit;
        }

        /// Watch for the soft keyboard and report its live geometry once.
        ///
        /// Polls rather than listening, because there is nothing to listen to
        /// yet: `android-activity` raises `MainEvent::InsetsChanged` when the
        /// keyboard appears, and winit's Android backend drops it
        /// (`winit-android/src/event_loop.rs`, `TODO: handle Android InsetsChanged
        /// notification`). A `NativeActivity` window also reports
        /// `adjustResize` yet is *not* actually resized, so the keyboard
        /// produces no window event either. Until one of those is fixed, an app
        /// only notices the keyboard on a wake-up it gets for some other
        /// reason — which is why this is polled from the frame hook and capped:
        /// an idle app gets no frames, so nothing here is a steady-state cost.
        ///
        /// Measured on API 34: nothing was reported while the keyboard was up
        /// and the app idle, and a single unrelated key event was enough to
        /// produce `visible, height 336.4 logical px` (883 device px at 2.625,
        /// matching the drawn keyboard).
        fn poll_soft_keyboard(&mut self) {
            if self.keyboard_seen || self.keyboard_frames >= KEYBOARD_POLL_FRAMES {
                return;
            }
            self.keyboard_frames += 1;
            let state = soft_keyboard_state();
            if state.visible {
                self.keyboard_seen = true;
                eprintln!(
                    "floem2-android: soft keyboard visible, height {:.1} logical px",
                    state.height
                );
            } else if self.keyboard_frames == KEYBOARD_POLL_FRAMES {
                eprintln!(
                    "floem2-android: soft keyboard not observed within {KEYBOARD_POLL_FRAMES} frames"
                );
            }
        }
    }

    /// Finds the text field node.
    fn find_text_field(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
        let host = runner.host();
        for index in 0..host.node_count() {
            if let Some(node) = host.node_at(index) {
                if host.widget::<TextField>(node).is_some() {
                    return Some(node);
                }
            }
        }
        None
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

    /// Finds the button node. Needed only so the probe can report its rect;
    /// everything else reaches the button through events.
    fn find_button(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
        let host = runner.host();
        for index in 0..host.node_count() {
            if let Some(node) = host.node_at(index) {
                if host.widget::<Button>(node).is_some() {
                    return Some(node);
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
            })
            .push_layout(LayoutRequest::new().width(260.0).height(40.0), || {
                Box::new(TextField::new().placeholder("tap me, then type"))
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
            window: None,
            field: None,
            last_text: String::new(),
            last_preedit: String::new(),
            probed: false,
            keyboard_frames: 0,
            keyboard_seen: false,
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
