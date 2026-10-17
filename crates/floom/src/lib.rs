//! The Floom 2.0 public facade: one crate, one prelude, one place to start.
//!
//! The 2.0 stack is layered (types → core → runtime → widgets → app …), and
//! a beginner should not have to know that. This crate re-exports the layers
//! under their own names (`floom::widgets`, `floom::core`, …) and curates
//! the beginner-facing surface into [`prelude`]. Everything beyond the
//! prelude stays reachable but explicit: custom-widget contracts in
//! [`advanced`], full crates as top-level re-exports.
//!
//! The v1 crate (`floom`, the legacy monolith) is a different, compatibility
//! world: its reactive `View` API is preserved through the current major
//! version at `floom::prelude` / `floom::compat::v1_prelude`, and it is
//! deliberately *not* importable here.
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
//! The v1 reactive graph is not part of the 2.0 prelude — a v1 signal
//! constructor does not resolve through it:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! let _ = create_signal(1);
//! ```
//!
//! Neither is the v1 `View` trait family:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! fn take_view<V: IntoView>(_: V) {}
//! take_view(|| "still the v1 API".to_string());
//! ```
//!
//! And the mount point only accepts Floom 2.0 widgets. A value that is not a
//! Floom 2.0 widget fails to compile, and the failure carries the migration
//! diagnostic:
//!
//! ```compile_fail
//! use floom::prelude::*;
//! let v1_label = || "v1 label".to_string();
//! let mut host = WidgetHost::new();
//! host.mount_root(LayoutStyle::default(), v1_label);
//! ```
//!
//! The original form of that doctest linked the v1 monolith and handed a real
//! `floom::views::label` to `mount_root`. Stage D deleted that crate, which
//! turned the doctest into a vacuous pass: `compile_fail` is satisfied by a
//! missing crate just as happily as by a rejected type, so the test would
//! have gone on passing while asserting nothing. The substitution above keeps
//! the assertion real - a closure is not a widget - with no v1 dependency.

#![forbid(unsafe_code)]

// The layered stack, re-exported under its own names. Anything the prelude
// deliberately leaves out stays reachable through these explicit paths — that
// is the "advanced APIs under explicit modules" rule.
pub use floom_app;
pub use floom_core;
pub use floom_layout;
pub use floom_scene;
pub use floom_types;
pub use floom_widgets;

// Feature bundles (Phase 29.3): each adds exactly its own backend.
#[cfg(feature = "devtools")]
pub use floom_devtools;
#[cfg(feature = "mobile")]
pub use floom_platform_android;
#[cfg(feature = "desktop")]
pub use floom_platform_desktop;
#[cfg(feature = "embedded")]
pub use floom_platform_embedded;
#[cfg(feature = "mobile")]
pub use floom_platform_harmony;
#[cfg(feature = "mobile")]
pub use floom_platform_ios;
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

/// Run a Floom app in a real OS window — the desktop backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (the winit event
/// loop, window realisation, input mapping, the GPU frame loop) is handled by
/// the backend. Requires `--features desktop,renderer-gpu`.
///
/// The window is created before the loop starts, so `title`/`size`/`resizable`
/// describe the first and only window. The presenter is
/// [`VelloGpuPresenter`](floom_renderer_vello::VelloGpuPresenter); if no GPU
/// adapter is usable the call fails with a message rather than panicking,
/// because a window that can never paint is not a degraded app — it is a
/// broken one.
///
/// ```
/// use floom::prelude::*;
/// # #[cfg(all(feature = "desktop", feature = "renderer-gpu"))]
/// # fn doc() -> Result<(), String> {
/// // floom::run_desktop("my app", Size::new(480.0, 320.0), |host| {
/// //     host.mount_root(LayoutStyle::default(), Text::new("hello window"));
/// // });
/// # Ok(())
/// # }
/// ```
#[cfg(all(feature = "desktop", feature = "renderer-gpu"))]
pub fn run_desktop<F>(title: &str, size: floom_types::Size, build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
    use floom_renderer_vello::VelloGpuPresenter;

    struct ClosureApp<F> {
        build: Option<F>,
        runner: Option<AppRunner<VelloGpuPresenter>>,
    }

    impl<F: FnOnce(&mut WidgetHost)> WinitApp for ClosureApp<F> {
        fn window_created(
            &mut self,
            _platform: &mut WinitPlatform,
            _window: floom_types::WindowId,
        ) {
            let Some(build) = self.build.take() else {
                return;
            };
            let mut host = WidgetHost::new();
            build(&mut host);
            // `WinitApp` has no error channel, so a machine with no usable
            // adapter is reported on stderr and the window is left unpainted
            // rather than panicking inside the event loop.
            let Some(presenter) = VelloGpuPresenter::new() else {
                eprintln!(
                    "floom::run_desktop: no usable GPU adapter; a desktop window needs one to paint"
                );
                return;
            };
            self.runner = Some(AppRunner::new(host, presenter));
        }

        fn events(&mut self, platform: &mut WinitPlatform) {
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }

        fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
            // Drain once more so redraws the runner requested during the last
            // `events` call are picked up before the loop blocks. Without this
            // a state change made during dispatch would wait for the next
            // external event to be noticed.
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
        ClosureApp {
            build: Some(build),
            runner: None,
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
/// let mut host = floom::WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello terminal"));
/// // floom::run_tui(|host| { /* mount your tree */ });
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tui")]
pub fn run_tui<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    use floom_app::{AppRunner, WidgetHost};
    use floom_renderer_tui::TuiPresenter;

    struct ClosureApp<F> {
        build: Option<F>,
        runner: Option<AppRunner<TuiPresenter<std::io::Stdout>>>,
    }

    impl<F: FnOnce(&mut WidgetHost)> TuiApp for ClosureApp<F> {
        fn events(&mut self, platform: &mut TuiPlatform) {
            if let Some(build) = self.build.take() {
                let mut host = WidgetHost::new();
                build(&mut host);
                let size = platform.size();
                let presenter = TuiPresenter::new(
                    floom_renderer_tui::TerminalSize::new(size.cols, size.rows),
                    std::io::stdout(),
                );
                self.runner = Some(AppRunner::new(host, presenter));
            }
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    floom_platform_tui::run(ClosureApp {
        build: Some(build),
        runner: None,
    })
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
#[cfg(all(feature = "mobile", feature = "renderer-gpu", target_os = "android"))]
pub fn run_android<F>(title: &str, size: floom_types::Size, build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform_android::{AndroidPlatform, AndroidRunner, run};
    use floom_renderer_vello::VelloGpuPresenter;

    struct ClosureApp<F> {
        build: Option<F>,
        runner: Option<AppRunner<VelloGpuPresenter>>,
    }

    impl<F: FnOnce(&mut WidgetHost)> AndroidRunner for ClosureApp<F> {
        fn events(&mut self, platform: &mut AndroidPlatform) {
            if let Some(build) = self.build.take() {
                let mut host = WidgetHost::new();
                build(&mut host);
                let Some(presenter) = VelloGpuPresenter::new() else {
                    eprintln!(
                        "floom::run_android: no usable GPU adapter; an Android surface needs one to paint"
                    );
                    return;
                };
                self.runner = Some(AppRunner::new(host, presenter));
            }
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }

        fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
            // Drain once more so redraws queued during the last `events` call are
            // honoured. Deliberately no `request_frame` here: that would make
            // every `about_to_wait` emit another `RedrawRequested`, so the loop
            // would never park (measured on the emulator: 2.7M redraws in 8s).
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    let windows = vec![floom_platform::WindowConfig {
        title: title.to_string(),
        size,
        resizable: false,
    }];
    run(
        ClosureApp {
            build: Some(build),
            runner: None,
        },
        windows,
    )
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
/// let mut host = floom::WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello web"));
/// floom::run_web(|_host: &mut floom::WidgetHost| {});
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "web")]
pub fn run_web<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floom_app::WidgetHost) + 'static,
{
    use floom_app::{AppRunner, WidgetHost};
    use floom_renderer_web::WebPresenter;

    struct ClosureApp<F> {
        build: Option<F>,
        runner: Option<AppRunner<WebPresenter>>,
    }

    impl<F: FnOnce(&mut WidgetHost)> floom_platform_web::WebApp for ClosureApp<F> {
        fn events(&mut self, platform: &mut floom_platform_web::WebPlatform) {
            if let Some(build) = self.build.take() {
                let mut host = WidgetHost::new();
                build(&mut host);
                let presenter = WebPresenter::new();
                self.runner = Some(AppRunner::new(host, presenter));
            }
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    floom_platform_web::run(
        ClosureApp {
            build: Some(build),
            runner: None,
        },
        "floom-canvas",
    )
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
/// - **No v1 API**: the legacy reactive graph and the v1 `View` lifecycle
///   live in the `floom` crate and are rejected here (see the crate-level
///   compile-fail examples).
/// - **Advanced APIs stay explicit**: `WidgetAny`, the create/update/event
///   contexts, semantics roles — all under [`crate::advanced`].
///
/// One deliberate non-re-export: the widgets crate also defines a
/// `floom_widgets::Axis` (divider/image orientation) distinct from the
/// layout [`Axis`] here. The layout `Axis` is what `Stack`/`List`/`Grid`
/// take, so it wins the glob; the orientation enum stays at its explicit
/// path `floom::widgets::Axis`.
pub mod prelude {
    // --- Semantic widgets (floom-widgets) -------------------------------
    pub use floom_widgets::NativeSurface;
    pub use floom_widgets::{
        AnimatedContent, Button, Canvas, Checkbox, Conditional, Container, Crossfade, Divider,
        Grid, Icon, Image, ImageFit, InteractionState, List, Opacity, Progress, ShaderBox,
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
    // the style's default is the engine's old disc written as a ring. `Progress`
    // and `Checkbox` stay: neither's appearance is contested, so neither had a
    // reason to move.

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
    // The signals are v2's own reactive primitives and they are genuinely useful --
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
/// This is v2's own reactive system, re-exported from [`floom_signal`].
///
/// ## The name
///
/// The crate was `floom-legacy-reactive`, on the assumption that "legacy" would
/// eventually be deleted and something else would take its place. Nothing
/// replaced it, so the name outlived its meaning and now mislabels the only
/// reactive implementation in the tree. It is `floom-signal`.
pub mod reactive {
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
