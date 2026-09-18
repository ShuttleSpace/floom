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
#[cfg(feature = "renderer-software")]
pub use floem_renderer_software;
#[cfg(feature = "renderer-gpu")]
pub use floem_renderer_vello;

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
        BottomSheet, Button, Checkbox, Conditional, Container, Dialog, Divider, Grid, Icon, Image,
        ImageFit, InteractionState, Labeled, List, NavDestination, NavHandle, NavHost,
        NavigationBar, NavigationDrawer, NavigationRail, Overlay, Popup, Progress, Radio, Select,
        Slider, Spacer, Stack, Switch, Tabs, Text, TextField,
    };

    // --- App shell (floem-app) -------------------------------------------
    pub use floem_app::{AppRunner, FrameStats, NodeId, RunStats, WidgetHost};

    // --- Runtime primitives (floem-core) ---------------------------------
    pub use floem_core::{Align, Axis, Child, ChildKey, Event, LayoutRequest, Point, Rect, Size};

    // --- Layout style + input value types --------------------------------
    pub use floem_layout::LayoutStyle;
    pub use floem_types::{PointerEvent, ScrollEvent};
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
    // The accessibility bridge shared by widget crates.
    pub use floem_widgets::{to_core_role, to_core_state};
}
