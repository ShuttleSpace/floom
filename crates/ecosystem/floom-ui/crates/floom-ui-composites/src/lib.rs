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
pub mod shared;
pub mod builders;
pub mod badge;
pub mod chip;
pub mod confirm_edit;
pub mod accordion;
pub mod bottom_navigation;
pub mod bottom_sheet;
pub mod dialog;
pub mod focus;
pub mod state;
pub mod overlay;
pub mod overlay_bookkeeping;
pub mod parallax;
pub mod popup;
pub mod tabs;
pub mod tabs_bar;
pub mod color_picker;
pub mod combobox;
pub mod counter;
pub mod date_picker;
pub mod icon_button;
pub mod infinite_scroller;
pub mod labeled;
pub mod list_item;
pub mod hover;
pub mod menu;
pub mod nav;
pub mod navigation;
pub mod select;
pub mod slider;
pub mod stepper;
pub mod radio;
pub mod range_slider;
pub mod tree_view;
pub mod text_field;
pub mod validation_messages;
pub mod virtual_scroller;
pub mod window_viewer;
pub mod time_picker;
pub mod tooltip;
pub mod checkbox;

pub use accordion::{
    Accordion, AccordionIndicator, AccordionStyle, ContentPlacement, SelectGroup, SelectGroupStyle,
    SelectionMode,
};
pub use bottom_navigation::{BottomNavigationBar, BottomNavigationStyle, IconBuilder};
pub use bottom_sheet::BottomSheet;
pub use focus::FocusRing;
pub use state::{LayerShape, StateLayer};
pub use dialog::Dialog;
pub use overlay::Overlay;
pub use overlay_bookkeeping::OverlayHost;
pub use parallax::{ParallaxLayer, SharedScroll, read_scroll, shared_scroll, write_scroll};
pub use popup::{Popup, PopupPlacement, popup_placement_rect};
pub use tabs::Tabs;
pub use tabs_bar::{TabsBar, TabsBarStyle};

pub use badge::{Badge};
pub use chip::{Chip};
pub use confirm_edit::{ConfirmEdit, ConfirmEditGlyphs, ConfirmEditStyle};
pub use color_picker::{ColorPicker, ColorPickerStyle, SharedColor, SharedHex, color_to_hex, hex_to_color, with_alpha};
pub use combobox::{ComboPick, Combobox, ComboboxStyle, Commit, SharedPick};
pub use combobox::{commit, matching, panel_placement, panel_rect, panel_request, select, sync_query_to_selection, visible_options};
pub use counter::{Counter, CounterState, CounterStyle, SharedCounter, Step, clamp_value, step_value};
pub use date_picker::{
    DatePicker, DatePickerState, DatePickerStyle, DateValue, DEFAULT_VIEW_MONTH,
    SharedDateState, read_date_state, write_date_state,
};
pub use icon_button::{IconButton, IconButtonIcon, IconButtonPaint, IconButtonStyle, IconButtonVariant};
pub use infinite_scroller::{InfiniteScrollerView, SharedHeight, shared_height};
pub use labeled::{Labeled};
pub use hover::HoverBox;
pub use menu::{Menu, MenuItem, MenuStyle};
pub use list_item::{ListItemRow, ListItemStyle, SlotBuilder};
pub use nav::{NavHandle, NavHost};
pub use select::{IndexCallback, Select, SelectPopup, SelectStyle, popup_height};
pub use slider::{Slider, SliderStyle};
pub use stepper::{Stepper, StepperStyle, StepperTick};
pub use range_slider::{RangeSlider, RangeSliderStyle};
pub use radio::{Radio, RadioStyle};
pub use navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail};
pub use tree_view::{RowRef, SharedTree, TreeNode, TreeView, TreeViewStyle};
pub use tree_view::{node_at, toggle_path, visible_rows};
pub use time_picker::{
    SharedTimeState, TimePicker, TimePickerState, TimePickerStyle, TimeValue, read_state,
    write_state,
};
pub use tooltip::{Tooltip, TooltipStyle};
pub use validation_messages::{ValidationMessagesList, ValidationMessagesStyle};
pub use virtual_scroller::VirtualScroller;
pub use builders::WidgetBuilder;
pub use shared::{SharedNumber, read_number, shared_number, write_number};
pub use window_viewer::{WindowViewer, WindowViewerStyle};
pub use text_field::{TextCallback, TextField, TextFieldStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
