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
pub mod combobox;
pub mod compose;
pub mod controls;
pub mod counter;
pub mod icon_button;
pub mod labeled;
pub mod list_item;
pub mod native;
pub mod nav;
pub mod navigation;
pub mod primitives;
pub mod progress_linear;
pub mod ripple;
pub mod surface;
pub mod text_input;
pub mod svg_path;
pub mod tree_view;
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
pub use combobox::{
    ComboPick, Combobox, Commit, commit, ComboboxStyle, SharedPick, matching, panel_placement, panel_rect,
    panel_request, select, sync_query_to_selection, visible_options,
};
pub use counter::{Counter, CounterState, CounterStyle, SharedCounter, Step, clamp_value, step_value};
pub use compose::{OverlayHost, popup_placement_rect, 
    Accordion, AccordionIndicator, AccordionStyle, AnimatedContent, BottomSheet, Conditional,
    Crossfade, Dialog, Grid, List, Opacity, Overlay, Popup, PopupPlacement, SelectGroup,
    SelectGroupStyle,
    Scroll, ScrollAxes, SelectionMode, SharedTransitionLayout, Stack, Tabs,
};
pub use controls::{
    Button, Checkbox, InteractionState, Pressable, Progress, Radio, Select, SelectPopup,
    SelectStyle, Slider, Switch, SwitchPalette, TextField, popup_height,
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
pub use list_item::{ListItemRow, ListItemStyle, SlotBuilder};
pub use native::NativeSurface;
pub use nav::{NavHandle, NavHost};
pub use navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail};
pub use primitives::{
    Container, Divider, Icon, Image, ImageFit, RuleAlignment, SemanticsHint, Spacer, Text,
};
pub use progress_linear::{
    FillPlacement, INDETERMINATE_WIDTH_PCT, Indeterminate, LinearProgress, LinearProgressStyle,
    PercentRounding, SLIDING_TRAVEL_PCT, fill_percent,
};
pub use ripple::{Phase, Ripple, ripple};
/// A decorated box with children: v1's `Container::new(child).style(..)`.
pub use surface::Surface;
pub use text_input::{SharedBuffer, SharedFlag, read_flag, shared_flag, write_flag, TextInput, TextInputStyle, clamp_to_char_boundary};
/// SVG path data -> `Path`, for the icon sets the kits ship as `d="..."`.
pub use svg_path::parse_path_data;
pub use tree_view::{
    RowRef, SharedTree, TreeNode, TreeView, TreeViewStyle, node_at, toggle_path, visible_rows,
};
/// An icon drawn from an outline, for kits with no SVG pipeline in 2.0.
pub use vector_icon::VectorIcon;
