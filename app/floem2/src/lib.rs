//! The Floem 2.0 public facade: one crate, one prelude, one place to start.
//!
//! The 2.0 stack is layered (types → core → runtime → widgets → app …), and
//! a beginner should not have to know that. This crate re-exports the layers
//! under their own names (`floem2::widgets`, `floem2::core`, …) and curates
//! the beginner-facing surface into [`prelude`]. Everything beyond the
//! prelude stays reachable but explicit: custom-widget contracts in
//! [`advanced`], full crates as top-level re-exports.
//!
//! The v1 crate (`floem`, the legacy monolith) is a different, compatibility
//! world: its reactive `View` API is preserved through the current major
//! version at `floem::prelude` / `floem::compat::v1_prelude`, and it is
//! deliberately *not* importable here.
//!
//! # The beginner surface
//!
//! A complete hello app uses nothing but the prelude:
//!
//! ```
//! use floem2::prelude::*;
//!
//! let mut host = WidgetHost::new();
//! host.set_size(Size::new(200.0, 100.0));
//!
//! let root = Stack::new(Axis::Vertical).child(Child::new(
//!     ChildKey::index(0),
//!     LayoutRequest::new(),
//!     || Box::new(Text::new("hello floem 2.0")),
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
//! use floem2::prelude::*;
//! let _ = create_signal(1);
//! ```
//!
//! Neither is the v1 `View` trait family:
//!
//! ```compile_fail
//! use floem2::prelude::*;
//! fn take_view<V: IntoView>(_: V) {}
//! take_view(|| "still the v1 API".to_string());
//! ```
//!
//! And the mount point only accepts Floem 2.0 widgets. Passing a real v1
//! view (from the legacy crate) fails to compile, and the failure carries
//! the migration diagnostic:
//!
//! ```compile_fail
//! use floem2::prelude::*;
//! let v1_label = floem::views::label(|| "v1 label".to_string());
//! let mut host = WidgetHost::new();
//! host.mount_root(LayoutStyle::default(), v1_label);
//! ```

#![forbid(unsafe_code)]

// The layered stack, re-exported under its own names. Anything the prelude
// deliberately leaves out stays reachable through these explicit paths — that
// is the "advanced APIs under explicit modules" rule.
pub use floem_app;
pub use floem_core;
pub use floem_layout;
pub use floem_scene;
pub use floem_types;
pub use floem_widgets;

// Feature bundles (Phase 29.3): each adds exactly its own backend.
#[cfg(feature = "devtools")]
pub use floem_devtools;
#[cfg(feature = "mobile")]
pub use floem_platform_android;
#[cfg(feature = "desktop")]
pub use floem_platform_desktop;
#[cfg(feature = "embedded")]
pub use floem_platform_embedded;
#[cfg(feature = "mobile")]
pub use floem_platform_harmony;
#[cfg(feature = "mobile")]
pub use floem_platform_ios;
#[cfg(feature = "web")]
pub use floem_platform_web;
#[cfg(feature = "web")]
pub use floem_renderer_web::WebPresenter;
#[cfg(feature = "renderer-software")]
pub use floem_renderer_software;
#[cfg(feature = "renderer-gpu")]
pub use floem_renderer_vello;
#[cfg(feature = "tui")]
pub use floem_platform_tui::{TuiApp, TuiPlatform};
#[cfg(feature = "tui")]
pub use floem_renderer_tui::TuiPresenter;

/// Run a Floem app in a terminal — the character-grid backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (terminal
/// setup, the stdin reader, SIGWINCH, the frame loop, quit on Ctrl-C/Ctrl-D)
/// is handled by the backend. Requires `--features tui`.
///
/// ```
/// use floem2::prelude::*;
/// # #[cfg(feature = "tui")]
/// # fn doc() -> Result<(), String> {
/// let mut host = floem2::WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello terminal"));
/// // floem2::run_tui(|host| { /* mount your tree */ });
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "tui")]
pub fn run_tui<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floem_app::WidgetHost) + 'static,
{
    use floem_app::{AppRunner, WidgetHost};
    use floem_renderer_tui::TuiPresenter;

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
                    floem_renderer_tui::TerminalSize::new(size.cols, size.rows),
                    std::io::stdout(),
                );
                self.runner = Some(AppRunner::new(host, presenter));
            }
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    floem_platform_tui::run(ClosureApp {
        build: Some(build),
        runner: None,
    })
}

/// Run a Floem app in a web browser — the Web backend's one-call entry.
///
/// `build` mounts the app's widget tree into the host; the rest (canvas setup,
/// DOM event listeners, the `requestAnimationFrame` loop) is handled by the
/// backend. Requires `--features web` and a `wasm32` target. The pixels are
/// rasterised on the CPU by `floem-renderer-web` and blitted to a `<canvas>`
/// (id `floem-canvas`, or the first `<canvas>` found).
///
/// ```
/// use floem2::prelude::*;
/// # #[cfg(feature = "web")]
/// # fn doc() -> Result<(), String> {
/// let mut host = floem2::WidgetHost::new();
/// host.mount_root(LayoutStyle::default(), Text::new("hello web"));
/// floem2::run_web(|_host: &mut floem2::WidgetHost| {});
/// # let _ = host;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "web")]
pub fn run_web<F>(build: F) -> Result<(), String>
where
    F: FnOnce(&mut floem_app::WidgetHost) + 'static,
{
    use floem_app::{AppRunner, WidgetHost};
    use floem_renderer_web::WebPresenter;

    struct ClosureApp<F> {
        build: Option<F>,
        runner: Option<AppRunner<WebPresenter>>,
    }

    impl<F: FnOnce(&mut WidgetHost)> floem_platform_web::WebApp for ClosureApp<F> {
        fn events(&mut self, platform: &mut floem_platform_web::WebPlatform) {
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

    floem_platform_web::run(
        ClosureApp {
            build: Some(build),
            runner: None,
        },
        "floem-canvas",
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
///   live in the `floem` crate and are rejected here (see the crate-level
///   compile-fail examples).
/// - **Advanced APIs stay explicit**: `WidgetAny`, the create/update/event
///   contexts, semantics roles — all under [`crate::advanced`].
///
/// One deliberate non-re-export: the widgets crate also defines a
/// `floem_widgets::Axis` (divider/image orientation) distinct from the
/// layout [`Axis`] here. The layout `Axis` is what `Stack`/`List`/`Grid`
/// take, so it wins the glob; the orientation enum stays at its explicit
/// path `floem2::widgets::Axis`.
pub mod prelude {
    // --- Semantic widgets (floem-widgets) -------------------------------
    pub use floem_widgets::NativeSurface;
    pub use floem_widgets::{
        AnimatedContent, BottomSheet, Button, Canvas, Checkbox, Conditional, Container, Crossfade,
        Dialog, Divider, Grid, Icon, Image, ImageFit, InteractionState, Labeled, List,
        NavDestination, NavHandle, NavHost, NavigationBar, NavigationDrawer, NavigationRail,
        Opacity, Overlay, Popup, Progress, Radio, Select, ShaderBox, SharedTransitionLayout,
        Slider, Spacer, Spinner, Stack, Switch, Tabs, Text, TextField,
    };

    // --- App shell (floem-app) -------------------------------------------
    pub use floem_app::{AppRunner, FrameStats, NodeId, RunStats, WidgetHost};

    // --- Runtime primitives (floem-core) ---------------------------------
    pub use floem_core::{Align, Axis, Child, ChildKey, Event, LayoutRequest, Point, Rect, Size};

    // --- Layout style + input value types --------------------------------
    pub use floem_layout::LayoutStyle;
    pub use floem_types::{PointerEvent, ScrollEvent};

    // --- Motion vocabulary ------------------------------------------------
    // `WidgetHost::animate` is one call, but it takes this vocabulary, and an
    // app that cannot spell `Tween` or `Easing` cannot animate anything. It
    // rides in the prelude for the same reason `LayoutStyle` does: it is a
    // value an app *writes*, not a contract it implements.
    pub use floem_animation::{AnimId, AnimKind, Tween};
    pub use floem_types::{
        CompositorProp, Easing, PresenceChannel, PresenceTransition, RepeatMode, Transition,
    };

    // --- Self-drawn value types -------------------------------------------
    // A `Canvas` painter writes `Path`s and `Stroke`s in app colors; without
    // these the canvas is unnameable from the beginner surface. Same rule as
    // the motion vocabulary: values an app writes, not contracts it implements.
    pub use floem_types::{Brush, Color, Filter, LineCap, LineJoin, Path, Shader, Shadow, Stroke};
    // A presence transition is the other half of the motion vocabulary an app
    // writes: `Transition` answers "this value changed", `PresenceTransition`
    // answers "this subtree appeared or disappeared". Both belong on the
    // beginner surface for the same reason `Transition` does — so they ride in
    // the motion-vocabulary `pub use` above rather than in a second one.

    // A `Canvas::repaint_on` ping is also a value an app writes: bump it and
    // the canvas repaints on the next frame, paint-only.
    pub use floem_legacy_reactive::{RwSignal, SignalGet, SignalUpdate};
}

/// Explicit, opt-in surface for everything the prelude keeps out: the
/// custom-widget contracts, type erasure, and semantics vocabulary. Reaching
/// for these is a deliberate act — that is the point.
pub mod advanced {
    // Custom-widget authoring contracts (L1).
    pub use floem_core::{
        CreateContext, ElementId, EventContext, PaintContext, ResourceId, SemanticsHint,
        SemanticsRole, UpdateContext, Widget, WidgetAny,
    };
    // Layout engine seam for custom measure logic.
    pub use floem_layout::{LayoutEngine, MeasureFn};
    // The declarative motion a widget returns from `Widget::animation`: only
    // nameable by someone implementing that contract, which is what keeps it
    // out of the prelude.
    pub use floem_types::{AnimationSpec, RepeatSpec};
    // The accessibility bridge shared by widget crates.
    pub use floem_widgets::{to_core_role, to_core_state};
}
