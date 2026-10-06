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
pub mod icon_button;
pub mod labeled;
pub mod native;
pub mod nav;
pub mod navigation;
pub mod primitives;
pub mod ripple;
pub mod surface;
pub mod svg_path;
pub mod testkit;
pub mod vector_icon;
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
    Accordion, AccordionIndicator, AccordionStyle, AnimatedContent, BottomSheet, Conditional,
    Crossfade, Dialog, Grid, List, Opacity, Overlay, Popup, SelectGroup, SelectGroupStyle,
    SelectionMode, SharedTransitionLayout, Stack, Tabs,
};
pub use controls::{
    Button, Checkbox, InteractionState, Pressable, Progress, Radio, Select, Slider, Switch,
    TextField,
};
/// The layout vocabulary a widget author needs alongside [`Stack`] and
/// [`Surface::border_edges`]: the main axis a stack runs on, how its children
/// sit on each axis, and which of a box's four edges an outline is drawn on.
///
/// Re-exported rather than left to `floom-core` because `Axis` used to be a
/// *second*, identical enum in `primitives`, and the root re-export named that
/// one — so `use floom_widgets::*;` followed by `Stack::new(Axis::Horizontal)`
/// did not compile. One axis type, exported from here, cannot drift back.
pub use floom_core::{Align, Axis, Edges};
pub use icon_button::{
    IconButton, IconButtonIcon, IconButtonPaint, IconButtonStyle, IconButtonVariant,
};
pub use labeled::Labeled;
pub use native::NativeSurface;
pub use nav::{NavHandle, NavHost};
pub use navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail};
pub use primitives::{Container, Divider, Icon, Image, ImageFit, SemanticsHint, Spacer, Text};
pub use ripple::{Phase, Ripple, ripple};
/// A decorated box with children: v1's `Container::new(child).style(..)`.
pub use surface::Surface;
/// SVG path data -> `Path`, for the icon sets the kits ship as `d="..."`.
pub use svg_path::parse_path_data;
/// An icon drawn from an outline, for kits with no SVG pipeline in 2.0.
pub use vector_icon::VectorIcon;
