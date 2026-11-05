// SPDX-License-Identifier: MIT
//! Tier-0 primitive widgets for Floom.
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

pub mod canvas;
pub mod circular_progress;
pub mod compose;
pub mod controls;
pub mod forms;
pub mod focus;
pub mod gesture;
pub mod native;
pub mod navigation;
pub mod overlay;
pub mod primitives;
pub mod progress_linear;
pub mod ripple;
pub mod surface;
pub mod svg_path;
pub mod testkit;
pub mod text_input;
pub mod vector_icon;
mod widget_any;

/// `WidgetAny` lives in `floom-core` (the type-erasure contract); re-exported
/// here so existing importers keep working.
pub use floom_core::WidgetAny;
/// The accessibility bridge, shared with the sibling data-widget crate.
pub use widget_any::{to_core_role, to_core_state};

/// Self-drawn surfaces: a closure-driven drawing board, a WGSL shader box and
/// a loading spinner.
pub use canvas::{Canvas, ShaderBox, Spinner};
pub use circular_progress::{CircularProgress, CircularProgressStyle};
pub use compose::{
    AnimatedContent, Conditional, Crossfade, Grid, List, Opacity, Scroll, ScrollAxes,
    SharedTransitionLayout, Stack,
};
pub use controls::{Button, InteractionState, Pressable, Progress, Switch, SwitchPalette};
/// The engine's one rule for what a disabled control looks like, and the
/// interaction ranking it sits in.
///
/// Exported beside [`paint_icon`] and [`text_width_at`] for the same reason:
/// each encodes a convention rather than an implementation, and a widget above
/// this crate re-deriving one is how two controls end up disagreeing about it.
/// `Radio` in `floom-ui-composites` is the caller — its style leaves
/// `disabled_color` as `None` precisely so that "no colour named" falls through
/// to this half-alpha dim rather than to a dim of its own devising.
pub use controls::{disabled_tint, resolved_color};
/// Intrinsic width of a string at an arbitrary font size.
///
/// Public because widgets above this crate measure their own labels at a size
/// the engine does not choose. `floom-ui-composites`' `Select` is the caller;
/// a kit cannot re-derive it without re-deriving the empty-string rule too.
pub use controls::{paint_icon, text_width_at};
/// The layout vocabulary a widget author needs alongside [`Stack`] and
/// [`Surface::border_edges`]: the main axis a stack runs on, how its children
/// sit on each axis, and which of a box's four edges an outline is drawn on.
///
/// Re-exported rather than left to `floom-core` because `Axis` used to be a
/// *second*, identical enum in `primitives`, and the root re-export named that
/// one — so `use floom_widgets::*;` followed by `Stack::new(Axis::Horizontal)`
/// did not compile. One axis type, exported from here, cannot drift back.
pub use floom_core::{Align, Axis, Edges};
pub use native::NativeSurface;
pub use primitives::{
    Container, Divider, Icon, Image, ImageFit, Revision, RuleAlignment, SemanticsHint, Spacer,
    Text, emit_semantics, stroke_rounded_outline,
};
pub use progress_linear::{
    FillPlacement, INDETERMINATE_WIDTH_PCT, Indeterminate, LinearProgress, LinearProgressStyle,
    PercentRounding, SLIDING_TRAVEL_PCT, fill_percent,
};
pub use ripple::{Phase, Ripple, ripple};
/// A decorated box with children.
pub use surface::Surface;
/// SVG path data -> `Path`, for the icon sets the kits ship as `d="..."`.
pub use svg_path::parse_path_data;
pub use text_input::{
    SharedBuffer, SharedFlag, SharedList, SharedMap, TextInput, TextInputStyle,
    clamp_to_char_boundary, read_flag, read_list, shared_flag, shared_list, shared_map, write_flag,
    write_list,
};
/// An icon drawn from an outline, for kits with no SVG pipeline.
pub use vector_icon::VectorIcon;

/// Overlay tree with z-order hit priority, focus scopes, and enter/exit animation.
pub use overlay::{CompositorProp, OverlayEntry, OverlayManager, OverlayHandle, OverlayKind, sheet_bounds};

/// Focus scopes and keyboard traversal (Tab / Shift-Tab).
pub use focus::{FocusDirection, FocusManager, FocusScopeId};

/// Form state and validation — widget-free, closure-based field bindings.
pub use forms::{FieldBinding, FormController, Rule, Validation, ValidationTicket};
pub use forms::rules;

/// Gesture arena: Tab/Pinch/Long-press recognizers the widgets drive.
pub use gesture::{GestureArena, GestureKind, PointerSample, Recognizer, RecognizerId, RecognizerState};

/// Typed routing and navigation for Floom 2.0.
pub use navigation::{Navigator, Page, Router, Route};
pub use navigation::navigation_containers::{StackNav, TabNav};
pub use navigation::navigation_url::Url;
