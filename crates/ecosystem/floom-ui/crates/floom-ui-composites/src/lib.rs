//! Composite controls that belong to a design system rather than to the engine.
//!
//! The layering runs: `floom-widgets` holds primitives (boxes, text, icons,
//! pressable surfaces), this crate holds the assemblies a designer would
//! recognise — a tab bar, a tree, a counter — and `floom-ui`'s
//! per-design-system crates supply the tokens.
//!
//! Everything here used to live in `floom-widgets`. None of it is a primitive:
//! each one encodes a choice about what a control *is* rather than how a box
//! behaves, and the engine is the wrong layer for that. See `Cargo.toml` for
//! why the dependency cannot run the other way.
//!
//! The first to move is [`NavHost`], because it is the cleanest case: nothing
//! in `floom-widgets` referred to it, it depends on no other widget, and it had
//! one consumer outside the crate. The rest followed once the layering held.

mod any;
pub mod badge;
pub mod chip;
pub mod accordion;
pub mod bottom_sheet;
pub mod dialog;
pub mod focus;
pub mod state;
pub mod overlay;
pub mod overlay_bookkeeping;
pub mod popup;
pub mod tabs;
pub mod combobox;
pub mod counter;
pub mod icon_button;
pub mod labeled;
pub mod list_item;
pub mod nav;
pub mod navigation;
pub mod select;
pub mod slider;
pub mod radio;
pub mod tree_view;
pub mod text_field;
pub mod checkbox;

pub use accordion::{
    Accordion, AccordionIndicator, AccordionStyle, ContentPlacement, SelectGroup, SelectGroupStyle,
    SelectionMode,
};
pub use bottom_sheet::BottomSheet;
pub use focus::FocusRing;
pub use state::{LayerShape, StateLayer};
pub use dialog::Dialog;
pub use overlay::Overlay;
pub use overlay_bookkeeping::OverlayHost;
pub use popup::{Popup, PopupPlacement, popup_placement_rect};
pub use tabs::Tabs;

pub use badge::{Badge};
pub use chip::{Chip};
pub use combobox::{ComboPick, Combobox, ComboboxStyle, Commit, SharedPick};
pub use combobox::{commit, matching, panel_placement, panel_rect, panel_request, select, sync_query_to_selection, visible_options};
pub use counter::{Counter, CounterState, CounterStyle, SharedCounter, Step, clamp_value, step_value};
pub use icon_button::{IconButton, IconButtonIcon, IconButtonPaint, IconButtonStyle, IconButtonVariant};
pub use labeled::{Labeled};
pub use list_item::{ListItemRow, ListItemStyle, SlotBuilder};
pub use nav::{NavHandle, NavHost};
pub use select::{IndexCallback, Select, SelectPopup, SelectStyle, popup_height};
pub use slider::{Slider, SliderStyle};
pub use radio::{Radio, RadioStyle};
pub use navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail};
pub use tree_view::{RowRef, SharedTree, TreeNode, TreeView, TreeViewStyle};
pub use tree_view::{node_at, toggle_path, visible_rows};
pub use text_field::{TextCallback, TextField, TextFieldStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
