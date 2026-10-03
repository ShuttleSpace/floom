//! Tier-0 primitive widgets for Floom 2.0.
//!
//! The widget set mirrors the architecture's tier list: this crate holds the
//! Tier-0 primitives ([`primitives`]); interactive controls, navigation and
//! data widgets land in sibling crates as their phases complete.
//!
//! Primitives depend only on renderer-neutral contracts (`floom-core`),
//! platform-neutral value types (`floom-types`), the retained scene
//! (`floom-scene`) and the semantics schema (`floom-accessibility`). They never
//! reach a concrete renderer, windowing system, or GPU backend.

#![forbid(unsafe_code)]

pub mod badge;
pub mod canvas;
pub mod compose;
pub mod controls;
pub mod labeled;
pub mod native;
pub mod nav;
pub mod navigation;
pub mod primitives;
pub mod ripple;
pub mod testkit;
mod widget_any;

/// `WidgetAny` lives in `floom-core` (the type-erasure contract); re-exported
/// here so existing importers keep working.
pub use floom_core::WidgetAny;
/// The accessibility bridge, shared with the sibling data-widget crate.
pub use widget_any::{to_core_role, to_core_state};

/// Self-drawn surfaces: a closure-driven drawing board, a WGSL shader box and
/// a loading spinner.
pub use badge::Badge;
pub use canvas::{Canvas, ShaderBox, Spinner};
pub use compose::{
    AnimatedContent, BottomSheet, Conditional, Crossfade, Dialog, Grid, List, Opacity, Overlay,
    Popup, SharedTransitionLayout, Stack, Tabs,
};
pub use controls::{
    Button, Checkbox, InteractionState, Progress, Radio, Select, Slider, Switch, TextField,
};
pub use labeled::Labeled;
pub use native::NativeSurface;
pub use nav::{NavHandle, NavHost};
pub use navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail};
pub use ripple::{Phase, Ripple, ripple};
pub use primitives::{
    Axis, Container, Divider, Icon, Image, ImageFit, SemanticsHint, Spacer, Text,
};
