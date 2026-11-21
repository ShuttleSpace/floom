// SPDX-License-Identifier: MIT
//! The Floom 2.0 public facade: one crate, one prelude, one place to start.
//!
//! The 2.0 stack is layered (types → core → runtime → widgets → app …), and
//! a beginner should not have to know that. This crate re-exports the layers
//! under their own names (`floom::widgets`, `floom::core`, …) and curates
//! the beginner-facing surface into [`prelude`]. Everything beyond the
//! prelude stays reachable but explicit: custom-widget contracts in
//! [`advanced`], full crates as top-level re-exports.
//!
//! # The beginner surface
//!
//! A complete hello app uses nothing but the prelude:
//!
//! ```
//! use floom::prelude::*;
//!
//! let mut host = WidgetHost::new();
//! host.set_size(Size::new(200.0, 100.0));
//!
//! let root = Stack::new(Axis::Vertical).child(Child::new(
//!     ChildKey::index(0),
//!     LayoutRequest::new(),
//!     || Box::new(Text::new("hello floom 2.0")),
//! ));
//! host.mount_root(LayoutStyle::default(), root);
//!
//! let stats = host.frame();
//! assert!(stats.painted > 0);
//! ```
//!
//! # Unsupported patterns (compile-fail)
//!
//! The prelude is curated: it does not re-export the raw signal constructor,
//! which lives in `floom::reactive`:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! let _ = create_signal(1);
//! ```
//!
//! It also does not expose a `View` trait family for building trees:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! fn take_view<V: IntoView>(_: V) {}
//! take_view(|| "not a widget".to_string());
//! ```
//!
//! And the mount point only accepts widgets. A value that is not a widget
//! fails to compile, and the failure carries the diagnostic that names the
//! missing trait:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! let label = || "not a widget".to_string();
//! let mut host = WidgetHost::new();
//! host.mount_root(LayoutStyle::default(), label);
//! ```
//!
//! The last example asserts something real - a closure is not a widget - so
//! the `compile_fail` is meaningful rather than a vacuous missing-crate pass.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// The layered stack, re-exported under its own names. Anything the prelude
// deliberately leaves out stays reachable through these explicit paths — that
// is the "advanced APIs under explicit modules" rule.
pub use floom_app;
pub use floom_core;
pub use floom_layout;
pub use floom_scene;
pub use floom_types;
pub use floom_widgets;

// Short names for the whole root workspace: every library crate is reachable as
// `floom::<name>` (drop the `floom_` prefix). The full-name re-exports above and
// the curated `prelude` stay intact — these are additional aliases, so all 73
// existing `floom::floom_xxx` callers keep working unchanged. `platform` and
// `devtools` are already hand-written modules, so their crates are merged into
// those modules with a glob re-export instead (see platform.rs / devtools.rs).
pub use floom_animation as animation;
pub use floom_app as app;
pub use floom_core as core;
pub use floom_layout as layout;
pub use floom_scene as scene;
pub use floom_signal as signal;
pub use floom_types as types;
pub use floom_widgets as widgets;

pub use floom_accessibility as accessibility;
pub use floom_dsl as dsl;
pub use floom_editor as editor;
pub use floom_inspector as inspector;
pub use floom_native as native;
pub use floom_native_ffi as native_ffi;
pub use floom_renderer as renderer;
pub use floom_runtime_core as runtime_core;
pub use floom_style as style;
pub use floom_text as text;
pub use floom_tsx as tsx;
// `virtual` is a reserved keyword, so the short name is a raw identifier:
// callers write `floom::r#virtual`. The full `floom::floom_virtual` still works.
pub use floom_virtual as r#virtual;
pub use floom_widgets_data as widgets_data;

// Optional crates: aliased under the same feature that links them. An alias for
// an unlinked optional crate would not compile, so `#[cfg]` here mirrors the
// `dep:` entries in Cargo.toml. `--all-features` exposes every one.
#[cfg(feature = "devtools")]
pub use floom_hot_reload as hot_reload;
#[cfg(feature = "mobile")]
pub use floom_platform_android as platform_android;
#[cfg(feature = "desktop")]
pub use floom_platform_desktop as platform_desktop;
#[cfg(feature = "embedded")]
pub use floom_platform_embedded as platform_embedded;
#[cfg(feature = "harmony")]
pub use floom_platform_harmony as platform_harmony;
#[cfg(feature = "ios")]
pub use floom_platform_ios as platform_ios;
#[cfg(feature = "tui")]
pub use floom_platform_tui as platform_tui;
#[cfg(feature = "web")]
pub use floom_platform_web as platform_web;
#[cfg(feature = "renderer-software")]
pub use floom_renderer_software as renderer_software;
#[cfg(feature = "tui")]
pub use floom_renderer_tui as renderer_tui;
#[cfg(feature = "renderer-gpu")]
pub use floom_renderer_vello as renderer_vello;
#[cfg(feature = "web")]
pub use floom_renderer_web as renderer_web;

// L8 tooling that reaches the software renderer; gated behind `full` so the
// default build stays headless (tests/feature_graph.rs asserts it).
#[cfg(feature = "full")]
pub use floom_accuracy as accuracy;
#[cfg(feature = "full")]
pub use floom_snapshot as snapshot;
#[cfg(feature = "full")]
pub use floom_testkit as testkit;
#[cfg(feature = "full")]
pub use floom_ui_events_winit as ui_events_winit;

// Feature bundles (Phase 29.3): each adds exactly its own backend.
#[cfg(feature = "devtools")]
pub mod devtools;
/// The one platform door: a single trait, a single entry, one degradation policy.
pub mod platform;
#[cfg(feature = "devtools")]
pub use floom_devtools;
#[cfg(feature = "embedded")]
pub use floom_platform::embedded as floom_platform_embedded;
#[cfg(feature = "mobile")]
pub use floom_platform::harmony as floom_platform_harmony;
#[cfg(feature = "mobile")]
pub use floom_platform::ios as floom_platform_ios;
#[cfg(feature = "mobile")]
pub use floom_platform_android;
#[cfg(feature = "desktop")]
pub use floom_platform_desktop;
#[cfg(feature = "tui")]
pub use floom_platform_tui::{TuiApp, TuiPlatform};
#[cfg(feature = "web")]
pub use floom_platform_web;
#[cfg(feature = "renderer-software")]
pub use floom_renderer_software;
#[cfg(feature = "tui")]
pub use floom_renderer_tui::TuiPresenter;
#[cfg(feature = "renderer-gpu")]
pub use floom_renderer_vello;
#[cfg(feature = "web")]
pub use floom_renderer_web::WebPresenter;

/// Re-exported so an app crate can reach it through `floom::` only.
#[cfg(all(feature = "harmony-winit", target_env = "ohos"))]
pub use floom_winit_ohos::export_ohos_winit_app;

/// The iOS entry point, in one line.
///
/// An app crate's `src/lib.rs` (the staticlib the Xcode host links) writes:
///
/// ```ignore
/// #[cfg(target_os = "ios")]
/// floom::ios_app!(build);
/// ```
///
/// and the Swift host calls `floom_ios_app_start(width, height, scale)` from
/// `viewDidAppear`, then `floom_ios_app_pump()` from a display link and
/// `floom_ios_app_touch(x, y, phase)` from the touches. `build` is the same
/// tree every other target gets. Width and height are pixels of the bound
/// `CGBitmapContext`; `scale` is `UIScreen.main.scale`.
#[cfg(all(feature = "ios", target_os = "ios"))]
#[macro_export]
macro_rules! ios_app {
    ($build:expr) => {
        /// Build the tree and paint the first frame. Returns false when no
        /// drawable is bound yet.
        #[unsafe(no_mangle)]
        pub extern "C" fn floom_ios_app_start(width: f64, height: f64, scale: f64) -> bool {
            let size = $crate::types::Size::new(width, height);
            $crate::platform::ios_host::start(
                $crate::platform::AppRequest {
                    title: env!("CARGO_PKG_NAME").to_string(),
                    size,
                    resizable: true,
                },
                Box::new($build),
                scale,
            )
        }

        /// Paint one frame. The host calls this once per display-link tick.
        #[unsafe(no_mangle)]
        pub extern "C" fn floom_ios_app_pump() -> bool {
            $crate::platform::ios_host::pump()
        }

        /// Forward one touch in pixels: 0 down, 1 move, 2 up, 3 cancel.
        #[unsafe(no_mangle)]
        pub extern "C" fn floom_ios_app_touch(x: f64, y: f64, phase: u32) -> bool {
            $crate::platform::ios_host::touch(x, y, phase)
        }
    };
}

/// The OpenHarmony entry point, in one line.
///
/// An app crate's `src/lib.rs` (the cdylib `floom package` builds into a HAP)
/// writes:
///
/// ```ignore
/// #[cfg(target_env = "ohos")]
/// floom::ohos_app!(build);
/// ```
///
/// and the C++ bridge creates the runtime with `ohos_winit_runtime_new` and
/// then feeds it the `ohos_winit_runtime_*` surface/touch/vsync symbols — touch,
/// surface lifecycle and vsync come from ArkUI, the loop and the tree from Rust.
/// The tree is the same `build` the app passes to `floom::run` on every other
/// target.
///
/// The window size is the surface's: ArkUI owns the XComponent's geometry, so
/// the placeholder request below is only a title and a resizability.
#[cfg(all(feature = "harmony-winit", target_env = "ohos"))]
#[macro_export]
macro_rules! ohos_app {
    ($build:expr) => {
        // The whole entry surface: `ohos_winit_runtime_new` creates the runtime
        // (and with it the worker thread), the rest of the `ohos_winit_runtime_*`
        // symbols feed it touch, surface and vsync events from the XComponent
        // callback. One symbol to call from the C++ bridge.
        $crate::export_ohos_winit_app!(move || {
            $crate::platform::ohos_winit::handler(
                $crate::platform::AppRequest {
                    title: ::std::string::String::from("Floom"),
                    size: $crate::floom_types::Size::new(1.0, 1.0),
                    resizable: true,
                },
                ::std::boxed::Box::new($build),
            )
        });
    };
}

/// Run a Floom app — the one unified entry point.
///
/// `build` mounts the app's widget tree into the host; the rest (the backend's
/// event loop, window realisation, input mapping, frame loop) is handled by the
/// platform backend that is compiled in. This is the single external API the
/// requirement asks for: callers write `floom::run(..)` and never name a
/// concrete backend (`WinitApp`/`TuiApp`/`WebApp`/…) or a `run_desktop`/
/// `run_android` split.
///
/// The compiled-in backend is selected by cargo features; on a desktop build
/// (`--features desktop,renderer-gpu`) this is the winit + vello GPU window. A
/// backend that is not compiled in is a hard error at this call, not a silent
/// no-op.
///
/// The window is created before the loop starts, so `title`/`size` describe the
/// first and only window. A backend that cannot paint reports the reason rather
/// than panicking inside its event loop, because a window that can never paint
/// is not a degraded app — it is a broken one.
///
/// For capability questions ("does this platform have a system clipboard?"),
/// use [`platform::require`]; it applies the same policy in one place instead
/// of every app inventing its own fallback.
///
/// ```
/// use floom::prelude::*;
/// # #[cfg(all(feature = "desktop", feature = "renderer-gpu"))]
/// # fn doc() -> Result<(), String> {
/// // floom::run("my app", Size::new(480.0, 320.0), |host| {
/// //     host.mount_root(LayoutStyle::default(), Text::new("hello window"));
/// // });
/// # Ok(())
/// # }
/// ```
pub fn run<F>(title: &str, size: floom_types::Size, build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    platform::current().run(
        platform::AppRequest {
            title: title.to_string(),
            size,
            resizable: true,
        },
        Box::new(build),
    )
}

/// Backward-compatible alias for [`run`].
///
/// Kept so generated projects (and any caller) that still name `run_desktop`
/// keep working; new code should call [`run`].
#[cfg(all(feature = "desktop", feature = "renderer-gpu"))]
#[deprecated(note = "use `floom::run`")]
pub fn run_desktop<F>(title: &str, size: floom_types::Size, build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    run(title, size, build)
}

/// Run a Floom app in a window with the devtools panel already wired — the
/// one-call entry for the desktop backend *plus* the devtools door.
///
/// Exactly [`run_desktop`], with one difference: `attach(&mut host)` is called
/// before the runner starts, so F12 opens the panel from the first frame. The
/// `readout` closure is the panel's content — see
/// [`devtools::attach`](crate::devtools::attach).
///
/// This exists because the shortcut cannot be hooked from inside the tree: a
/// key event arrives at the platform loop first, and only the loop can offer it
/// to the panel before the focused control also sees it. So the one-line
/// integration has two halves — `attach`, which any embedder can call on its
/// own host, and a `WinitApp::key` implementation that forwards the shortcut.
/// `run_desktop_with_devtools` is the spelling that does both.
///
/// Requires `--features desktop,renderer-gpu,devtools`.
///
/// ```
/// use floom::prelude::*;
/// # #[cfg(all(feature = "desktop", feature = "renderer-gpu", feature = "devtools"))]
/// # fn doc() -> Result<(), String> {
/// // floom::run_desktop_with_devtools("my app", Size::new(480.0, 320.0), |host| {
/// //     host.mount_root(LayoutStyle::default(), Text::new("hello window"));
/// // }, |state| Vec::new());
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "desktop", feature = "renderer-gpu", feature = "devtools"))]
pub fn run_desktop_with_devtools<F, R>(
    title: &str,
    size: floom_types::Size,
    build: F,
    readout: R,
) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
    R: Fn(&devtools::DevtoolsState) -> Vec<devtools::ReadoutRow> + Send + Sync + 'static,
{
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
    use floom_renderer_vello::VelloGpuPresenter;

    struct DevtoolsApp<F, R> {
        build: Option<F>,
        readout: Option<R>,
        runner: Option<AppRunner<VelloGpuPresenter>>,
        devtools: Option<devtools::Devtools>,
        picker: Option<devtools::PickHandle>,
        /// Everything the devtools controls from outside the tree, as of the
        /// last frame hook.
        ///
        /// Compared rather than acted on unconditionally: the shortcut and the
        /// pick are answered in `key`/`pointer`, before the tree, and a consumed
        /// event produces no platform event at all — so nothing would wake the
        /// loop and the panel would change without ever being drawn. And a
        /// per-hook frame request would keep the redraw loop from ever settling,
        /// because retargeting the highlight goes through `widget_mut`, which
        /// marks the host dirty.
        outside_was: Option<OutsideState>,
    }

    /// The part of the devtools state a frame has to be requested for.
    ///
    /// Everything a reader of the panel would notice: whether it is up, which
    /// tab, which element, which nodes are collapsed, and which element the
    /// highlight box is around.
    type OutsideState = (
        bool,
        devtools::DevtoolsTab,
        Option<String>,
        std::collections::BTreeSet<String>,
        Option<(Option<String>, bool)>,
    );

    impl<F, R> DevtoolsApp<F, R> {
        /// Everything the devtools controls from outside the tree, as one
        /// comparable value.
        ///
        /// The panel's own state is a mutex the platform loop writes and the
        /// tree reads, so a change to it produces no platform event. Comparing
        /// this across frame hooks is what turns such a change into a frame.
        fn outside_state(
            devtools: Option<&devtools::Devtools>,
            picker: Option<&devtools::PickHandle>,
        ) -> OutsideState {
            let (visible, tab, selected, collapsed) = devtools.map_or_else(
                || {
                    (
                        false,
                        devtools::DevtoolsTab::default(),
                        None,
                        std::collections::BTreeSet::new(),
                    )
                },
                |devtools| {
                    let state = devtools.state();
                    let guard = state.lock().ok();
                    let visible = guard.as_ref().is_some_and(|s| s.visible());
                    let tab = guard.as_ref().map(|s| s.tab()).unwrap_or_default();
                    let selected = guard
                        .as_ref()
                        .and_then(|s| s.selected().map(str::to_string));
                    // The collapsed set is in here because clicking a chevron
                    // writes it from inside the tree: the row cannot mark the
                    // host dirty, so it requests a repaint of itself, and the
                    // *panel* is what has to rebuild. Comparing the set here
                    // makes that rebuild a frame this loop asks for rather than
                    // one it hopes the repaint channel will produce.
                    let collapsed = guard
                        .as_ref()
                        .map(|s| s.collapsed().clone())
                        .unwrap_or_default();
                    (visible, tab, selected, collapsed)
                },
            );
            let highlight =
                picker.map(|picker| (picker.highlight_target(), picker.highlight_is_live()));
            (visible, tab, selected, collapsed, highlight)
        }
    }

    impl<
        F: FnOnce(&mut WidgetHost),
        R: Fn(&devtools::DevtoolsState) -> Vec<devtools::ReadoutRow> + Send + Sync + 'static,
    > WinitApp for DevtoolsApp<F, R>
    {
        fn window_created(
            &mut self,
            _platform: &mut WinitPlatform,
            _window: floom_types::WindowId,
        ) {
            let Some(build) = self.build.take() else {
                return;
            };
            let Some(readout) = self.readout.take() else {
                return;
            };
            let mut host = WidgetHost::new();
            build(&mut host);
            // `attach_with_rows` mounts the Chrome panels *and* the caller's own
            // rows, and it wires the tree cell this loop refreshes below. The
            // alternative — a bare `attach` — would leave the panel showing the
            // tree as it was at startup, which is a stale answer.
            let devtools = devtools::attach_with_rows(&mut host, readout);
            // The picker is mounted alongside the panel: its highlight is a
            // passthrough overlay above the panel, so the box is visible over
            // everything and clickable through.
            let picker = devtools::attach_picker(&mut host, devtools.state());
            self.devtools = Some(devtools);
            self.picker = Some(picker);
            let Some(presenter) = VelloGpuPresenter::new() else {
                eprintln!(
                    "floom::run_desktop_with_devtools: no usable GPU adapter; a desktop window needs one to paint"
                );
                return;
            };
            self.runner = Some(AppRunner::new(host, presenter));
        }

        fn events(&mut self, platform: &mut WinitPlatform) {
            // Split the borrow up front: the panel and the host are two fields
            // of the same struct, and `sync` needs one of each.
            let Self {
                build: _,
                readout: _,
                devtools,
                picker,
                runner,
                outside_was,
            } = self;
            let Some(runner) = runner.as_mut() else {
                return;
            };
            // Refresh the panel's tree before the frame, so what it shows is the
            // app as it is now rather than as it was when the window opened.
            if let Some(devtools) = devtools.as_ref() {
                devtools.refresh_tree(runner.host());
            }
            // The panel's visibility reaches the overlay stack and the keyboard
            // here, not in the widget: a hidden drawer that stayed interactive
            // would eat every press down its column, and a panel that closed
            // while holding focus would strand the keyboard on a node with no
            // children. Called every frame — `sync` compares the visibility
            // itself and does nothing when it has not moved.
            if let Some(devtools) = devtools.as_mut() {
                devtools.sync(runner.host_mut());
            }
            // Everything that drives the devtools lives *outside* the tree: the
            // shortcut and the pick are answered in `key`/`pointer`, before the
            // tree, and a consumed event produces no platform event at all — so
            // nothing would wake the loop and the panel would change without
            // ever being drawn. The change is noticed here instead, and turned
            // into a frame request. Only on a change: requesting one per hook
            // would keep the redraw loop from ever settling.
            let now = Self::outside_state(devtools.as_ref(), picker.as_ref());
            if outside_was.as_ref() != Some(&now) {
                *outside_was = Some(now);
                // The highlight is retargeted before the frame so the box this
                // frame paints is around the element the pick just made, not
                // the one before it.
                if let Some(picker) = picker.as_ref() {
                    picker.sync_highlight(runner.host_mut());
                }
                runner.request_frame(platform);
            }
            runner.pump(platform);
        }

        // The shortcut, before the tree: a keystroke that opens the panel should
        // not also type into whatever had focus.
        fn key(&mut self, event: &floom_types::KeyEvent) -> bool {
            if let Some(picker) = self.picker.as_ref()
                && picker.handle_key(event)
            {
                return true;
            }
            self.devtools
                .as_mut()
                .is_some_and(|devtools| devtools.handle_key(event))
        }

        // The pick, before the tree: clicking a button to inspect it must not
        // also press it. The host owns the mapping from a point to a live-tree
        // key, because it owns the tree.
        fn pointer(&mut self, event: &floom_types::PointerEvent) -> bool {
            let (Some(picker), Some(runner)) = (self.picker.as_ref(), self.runner.as_ref()) else {
                return false;
            };
            let host = runner.host();
            let consumed = picker.pointer(event, |point| {
                host.element_at(point).map(floom_app::WidgetHost::node_key)
            });
            // A committed pick selects the element the readout is about. The
            // panel only learns that in `update`, so the next frame is requested
            // from `events` — which compares the state — rather than here, where
            // there is no platform to ask.
            picker.commit_selection();
            consumed
        }

        fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    let windows = vec![floom_platform::WindowConfig {
        title: title.to_string(),
        size,
        resizable: true,
    }];
    run(
        DevtoolsApp {
            build: Some(build),
            readout: Some(readout),
            runner: None,
            devtools: None,
            picker: None,
            outside_was: None,
        },
        windows,
    )
}

/// Run a Floom app in a terminal — the character-grid backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (terminal
/// setup, the stdin reader, SIGWINCH, the frame loop, quit on Ctrl-C/Ctrl-D)
/// is handled by the backend. Requires `--features tui`.
///
/// ```
/// use floom::prelude::*;
/// # #[cfg(feature = "tui")]
/// # fn doc() -> Result<(), String> {
/// let mut host = WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello terminal"));
/// // floom::run_tui(|host| { /* mount your tree */ });
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
/// Run a Floom app in a terminal — a thin alias for [`run`] on the TUI backend.
///
/// Kept only for callers that already name it; the unified entry [`run`] picks the
/// terminal backend automatically under `--features tui`.
#[cfg(feature = "tui")]
#[deprecated(note = "use `floom::run`")]
pub fn run_tui<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    run("floom", floom_types::Size::new(800.0, 600.0), build)
}

/// Run a Floom app on Android — the Android backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (the winit
/// event loop, surface realisation, input and soft-keyboard plumbing) is
/// handled by the backend. Requires `--features mobile,renderer-gpu`.
///
/// # The `android_main` seam
///
/// Unlike every other `run_*` entry, this one is not called from `main`. Android
/// hands the app an `AndroidApp` before any Rust entry point runs, and winit
/// insists on receiving it while building the event loop, so the glue must call
/// [`floom_platform_android::set_android_app`] **exactly once** before
/// [`run_android`] — this is the seam `run_android` cannot paper over for you.
/// In a `native-activity` app that looks like:
///
/// ```ignore
/// #[no_mangle]
/// fn android_main(app: AndroidApp) -> Result<(), String> {
///     floom_platform_android::set_android_app(app);
///     floom::run_android("my app", Size::new(360.0, 640.0), |host| {
///         host.mount_root(LayoutStyle::default(), Text::new("hello android"));
///     })
/// }
/// ```
///
/// [`run_android`]: fn.run_android.html
/// Run a Floom app on Android — a thin alias for [`run`] on the Android backend.
///
/// The `android_main` seam ([`floom_platform_android::set_android_app`] called once
/// before this) is unchanged; the unified entry [`run`] picks the Android backend
/// automatically under `--features mobile,renderer-gpu` on the `android` target.
#[cfg(all(feature = "mobile", feature = "renderer-gpu", target_os = "android"))]
#[deprecated(note = "use `floom::run`")]
pub fn run_android<F>(title: &str, size: floom_types::Size, build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    run(title, size, build)
}

/// Run a Floom app in a web browser — the Web backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (canvas setup,
/// DOM event listeners, the `requestAnimationFrame` loop) is handled by the
/// backend. Requires `--features web` and a `wasm32` target. The pixels are
/// rasterised on the CPU by `floom-renderer-web` and blitted to a `<canvas>`
/// (id `floom-canvas`, or the first `<canvas>` found).
///
/// ```
/// use floom::prelude::*;
/// # #[cfg(feature = "web")]
/// # fn doc() -> Result<(), String> {
/// let mut host = WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello web"));
/// floom::run_web(|_host: &mut WidgetHost| {});
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
/// Run a Floom app in a web browser — a thin alias for [`run`] on the Web backend.
///
/// Kept only for callers that already name it; the unified entry [`run`] picks the
/// Web backend automatically under `--features web` on `wasm32`.
#[cfg(feature = "web")]
#[deprecated(note = "use `floom::run`")]
pub fn run_web<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    run("floom", floom_types::Size::new(800.0, 600.0), build)
}

/// The beginner-facing stable surface: semantic widgets and runtime
/// primitives, nothing else.
///
/// Rules of the prelude (Phase 29.1):
///
/// - **Semantic widgets**: the things a screen is made of — `Stack`, `Text`,
///   `Button`, `List`, `Tabs`, … No type-erasure, no contexts, no scene.
/// - **Runtime primitives**: the value types those widgets take — geometry
///   (`Point`, `Size`, `Rect`), tree vocabulary (`Child`, `ChildKey`,
///   `LayoutRequest`), input events, and the app shell (`WidgetHost`,
///   `AppRunner`).
/// - **Curated, not comprehensive**: the prelude exposes widgets and the value
///   types an app writes; the reactive graph and the widget contracts live in
///   their own modules (`floom::reactive`, `floom::advanced`) and are reached
///   explicitly (see the crate-level compile-fail examples).
/// - **Advanced APIs stay explicit**: `WidgetAny`, the create/update/event
///   contexts, semantics roles — all under [`crate::advanced`].
///
/// One deliberate non-re-export: the widgets crate also defines a
/// `floom_widgets::Axis` (divider/image orientation) distinct from the
/// layout [`Axis`](floom_core::Axis) here. The layout `Axis` is what `Stack`/`List`/`Grid`
/// take, so it wins the glob; the orientation enum stays at its explicit
/// path `floom::widgets::Axis`.
pub mod prelude {
    // --- Semantic widgets (floom-widgets) -------------------------------
    pub use floom_widgets::NativeSurface;
    pub use floom_widgets::{
        AnimatedContent, Button, Canvas, Conditional, Container, Crossfade, Divider, Grid, Icon,
        Image, ImageFit, InteractionState, List, Opacity, Progress, ShaderBox,
        SharedTransitionLayout, Spacer, Spinner, Stack, Switch, Text,
    };

    // --- Composite controls (floom-ui-composites, one layer up) ----------
    // `Labeled`, `NavHandle`, `NavHost`, `NavDestination`, `NavigationBar`,
    // `NavigationDrawer` and `NavigationRail` used to be `floom_widgets`
    // modules and used to be re-exported here. They are not primitives: a
    // navigation rail is a decision about what navigation looks like, and
    // that decision is not the engine's to make. They now live in
    // `floom-ui-composites`, which sits in the `floom-ui` workspace -- a
    // *separate* workspace the root one `exclude`s. So the prelude cannot name
    // them: a crate that ships in an app binary does not get to depend on a
    // satellite workspace to keep a promise it made before the layering was
    // right. Depend on `floom-ui-composites` directly for these seven.

    // The following five joined them: `Overlay`, `Dialog`, `Popup`, `BottomSheet`
    // and `Tabs`. A modal card, a tab strip and an anchored panel are the same
    // kind of decision, so the "why" is one note rather than five. What is left
    // in the engine is the layout vocabulary -- `Stack`, `List`, `Grid`,
    // `Conditional`, `Crossfade`, `AnimatedContent`, `Opacity`, `Scroll` -- which is
    // arithmetic rather than product.

    // `Slider` moved up with them, for the sharper version of the same reason:
    // what a slider *does* -- clamp, quantise, drag, the keyboard walk -- is a
    // primitive, but what it *looks like* is not, so its paint went from six
    // hard-coded colours and two magic numbers inside the engine to a
    // `SliderStyle` a kit fills in. Same edge, same answer: depend on
    // `floom-ui-composites` for it.

    // `Radio` moved up for the same reason, and it is the sharpest case of the
    // lot: the engine painted a *filled disc* with a smaller disc on top, while
    // all three kits paint a *stroked ring* with a filled dot. That is a
    // different shape rather than a different colour, so it could not be reached
    // by adding knobs -- the paint had to be rewritten around a `RadioStyle`, and
    // the style's default is the engine's old disc written as a ring.
    //
    // `Checkbox` moved up last, and it is the sibling case: the engine painted a
    // bare `#2B6CB0` fill with the character `U+2714` on it and no rim at all,
    // where all three kits paint a 2px rim around the box, a transparent
    // unchecked interior and a vector mark at `size * 0.7`. The disagreement is
    // wider than a radio's -- it is a shape (the rim), an absence (the
    // transparent fill) and two mechanisms (a state layer, and a focus ring on
    // two of the three) rather than one colour each -- so the paint had to be
    // rewritten around a `CheckboxStyle` whose default is the engine's old fill.
    // `Progress` and `Switch` stay: neither's appearance is contested, so
    // neither had a reason to move.

    // `TextField` moved up too, and it is the widest of the six: what a text
    // field *does* -- hold a string, keep a caret in it, own an IME composition
    // and render the preedit inline -- is primitive, but what it *looks like* is
    // four kinds of ink (a rim, a fill, a placeholder, a committed run and an
    // underlined preedit) plus a focus affordance, and the engine's paint had no
    // way to express one. Its `paint` now reads a `TextFieldStyle` a kit fills
    // in, whose default is the engine's old hard-coded fill and inks.

    // --- App shell (floom-app) -------------------------------------------
    pub use floom_app::{AppRunner, FrameStats, NodeId, RunStats, WidgetHost};

    // --- Runtime primitives (floom-core) ---------------------------------
    pub use floom_core::{
        Align, Axis, Child, ChildKey, Event, LayoutRequest, Length, Point, Rect, Size,
    };

    // --- Layout style + input value types --------------------------------
    pub use floom_layout::LayoutStyle;
    pub use floom_types::{PointerEvent, ScrollEvent};

    // --- Motion vocabulary ------------------------------------------------
    // `WidgetHost::animate` is one call, but it takes this vocabulary, and an
    // app that cannot spell `Tween` or `Easing` cannot animate anything. It
    // rides in the prelude for the same reason `LayoutStyle` does: it is a
    // value an app *writes*, not a contract it implements.
    pub use floom_animation::{AnimId, AnimKind, Tween};
    pub use floom_types::{
        CompositorProp, Easing, PresenceChannel, PresenceTransition, RepeatMode, Transition,
    };

    // --- Self-drawn value types -------------------------------------------
    // A `Canvas` painter writes `Path`s and `Stroke`s in app colors; without
    // these the canvas is unnameable from the beginner surface. Same rule as
    // the motion vocabulary: values an app writes, not contracts it implements.
    pub use floom_types::{Brush, Color, Filter, LineCap, LineJoin, Path, Shader, Shadow, Stroke};
    // A presence transition is the other half of the motion vocabulary an app
    // writes: `Transition` answers "this value changed", `PresenceTransition`
    // answers "this subtree appeared or disappeared". Both belong on the
    // beginner surface for the same reason `Transition` does — so they ride in
    // the motion-vocabulary `pub use` above rather than in a second one.

    // NOTE: `RwSignal` used to live here. It does not any more, and the reason is
    // worth keeping.
    //
    // The signals are Floom's own reactive primitives and they are genuinely useful --
    // but they arrived in the prelude as an inheritance rather than a decision. A
    // beginner writing `Counter` should reach for `counter.value(0)` and not have to
    // know a signal exists; a beginner who *does* need mutable state should be told
    // where to look rather than handed it. Every other entry in this prelude is
    // something an app writes in the widget DSL; a signal is the machinery *under*
    // that, not part of it.
    //
    // So the reactive surface moved to `floom::reactive`, which is where a reader who
    // wants it will look. That module documents what it commits to.
}

/// Mutable state: signals, scopes, and the traits that read and write them.
///
/// ## Why this is not in the prelude
///
/// `RwSignal` was in the prelude until this module existed, because the reactive
/// runtime predates it and there was nowhere else to put it. That is an
/// inheritance, not a decision.
///
/// The argument against is not that signals are hard. It is that most apps
/// should not be holding one. A counter should be `counter.value(0)`; a text
/// field should be `text_field(initial)`; a canvas that repaints on a timer
/// should be `canvas.repaint_on(..)`. Every one of those hides a signal on the
/// app's behalf. An app that reaches past them has usually outgrown the widget,
/// and at that point the cost of one extra `use` is far below the cost of every
/// app having to name a concept it does not use.
///
/// ## What is here
///
/// `RwSignal`, `ReadSignal`, `WriteSignal`; the `SignalGet` / `SignalUpdate` /
/// `SignalWith` traits; `Scope` and the effect / memo / derived constructors.
/// This is Floom's reactive system, re-exported from [`floom_signal`].
///
/// ## The name
///
/// The crate was `floom-legacy-reactive`, on the assumption that "legacy" would
/// eventually be deleted and something else would take its place. Nothing
/// replaced it, so the name outlived its meaning and now mislabels the only
/// reactive implementation in the tree. It is `floom-signal`.
pub mod reactive {
    // Re-exports the full reactive surface, including functions floom-signal has
    // since deprecated in favour of the `RwSignal`/`Effect`/`Context`/`Scope`
    // constructors. Kept for backward compatibility; do not surface more
    // deprecated items than necessary.
    #[allow(deprecated)]
    pub use floom_signal::{
        Context, Effect, Memo, ReadSignal, RwSignal, Scope, SignalGet, SignalUpdate, SignalWith,
        Trigger, WriteSignal, batch, create_derived_rw_signal, create_effect,
        create_global_rw_signal, create_memo, create_rw_signal, create_signal, create_trigger,
        provide_context, untrack, use_context, with_scope,
    };
}

/// Explicit, opt-in surface for everything the prelude keeps out: the
/// custom-widget contracts, type erasure, and semantics vocabulary. Reaching
/// for these is a deliberate act — that is the point.
pub mod advanced {
    // Custom-widget authoring contracts (L1).
    pub use floom_core::{
        CreateContext, ElementId, EventContext, PaintContext, ResourceId, SemanticsHint,
        SemanticsRole, UpdateContext, Widget, WidgetAny,
    };
    // Layout engine seam for custom measure logic.
    pub use floom_layout::{LayoutEngine, MeasureFn};
    // The declarative motion a widget returns from `Widget::animation`: only
    // nameable by someone implementing that contract, which is what keeps it
    // out of the prelude.
    pub use floom_types::{AnimationSpec, RepeatSpec};
    // The accessibility bridge shared by widget crates.
    pub use floom_widgets::{to_core_role, to_core_state};
}
