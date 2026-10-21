//! Cupertino time picker, ported to the Floom 2.0 widget API.
//!
//! A time picker is a field that shows the chosen time, and a panel that opens
//! under it: an hour row and a minute row, each a pair of steppers around the
//! number, and a "Done" button that closes the panel.
//!
//! ## What the port uses
//!
//! The engine's `TimePicker`: the field that shows the time and the panel
//! that picks one. The three floem-ui time-picker kits each used to carry
//! their own copy of both.
//!
//! The engine's overlay door. v1 put the panel in an `Overlay` and placed it
//! by hand: a `LayoutChangedListener` read the field's *window* origin and
//! height and wrote them into `inset_top` and `inset_left`, so the panel's
//! position was the picker's own arithmetic -- arithmetic about the window
//! rather than about the field, and so wrong the moment the page scrolled.
//! 2.0's host owns the overlay layers. The picker asks for a popup rect
//! derived from its own box, and the host puts the panel there, paints it
//! above the tree and hit-tests it first. So the listener is gone, and so are
//! the window-origin insets.
//!
//! ## What this kit answers
//!
//! * the field's height, radius and minimum width, the panel's width and the
//!   row's height: the tokens;
//! * the field's fill: `surface`; its border: `outline`, `primary` while the
//!   panel is open and `outline` at 97 while the picker is disabled; the
//!   time's ink: `on_surface`, the placeholder's: `on_surface_variant`;
//! * the panel's fill: `surface`; its border: `outline` at 120; its shadow:
//!   black at 24, blurred 12 and spread 1;
//! * the stepper's fill: `on_surface` at 8; the "Done" button's ink:
//!   `primary`.
//!
//! ## The numbers v1 wrote as literals
//!
//! The field's horizontal padding (12), the border's thickness (1), the
//! panel's padding (12), the gap between the panel's three parts (8), the gap
//! inside a row (8), the stepper's size (32) and radius (16), the stepper's
//! text size (18), the number's size (24) and width (48), and the "Done"
//! button's height (32) and padding (12).
//!
//! ## The token that never showed
//!
//! `TimePickerTokens::padding_horizontal` (16) is not in v1's markup: the
//! field padded itself by 12, a literal. The port keeps v1's 12 and leaves the
//! token unread, and the style reports the 12 under the name the paint uses.
//!
//! ## The differences this port names rather than reproduces
//!
//! * **The calendar opens on the frame after the click.** The panel is a host
//!   overlay, and 2.0's overlay door is state-driven: the click writes "open"
//!   into the shared state, and the next update asks the host for the panel.
//!   v1's panel was up in the same frame.
//! * **The field and the panel paint themselves.** 2.0 keeps a mounted
//!   child's widget when its key survives a rebuild, so a `Text` child would
//!   keep the time it was built with. The port's field and panel read the
//!   shared state when they paint.
//! * **The pointer cursor over a disabled picker.** v1 asked for the pointer
//!   whatever the state; the engine's rule is that a control which cannot be
//!   clicked shows the platform arrow, and the port follows the engine.
//!
//! ## The theme source
//!
//! v1's Cupertino time picker took a *snapshot* of its own design system --
//! `MultiTheme::new_for(DesignSystem::Cupertino)` -- so a change of the
//! app-wide theme does not reach it. The port keeps that.

//! ## What v1 wrote that this port does not carry
//!
//! * **`props().on_change` is a `Box<dyn Fn(TimeValue)>`.** The kit fills it
//!   with a wrapper around its own `Arc<dyn Fn(TimeValue) + Send + Sync>`:
//!   the closure a 2.0 tree is built from has to be `Send + Sync`, and v1's
//!   box is not.
//! * **The accessors.** v1's Cupertino time picker had no `props()`, `theme()`
//!   or `with_theme()`. The port gives all three kits the same three, because
//!   a kit that cannot be given a theme cannot be tested against one.
//!
//! ## The shape that belongs to the engine
//!
//! The picker widget -- a field, a shared state, and a panel that opens under
//! it -- is the same shape in all three kits, so it belongs in
//! `floom-ui-composites` beside `Combobox` and `Menu`. The engine could not be
//! edited while this port was written, so the shape is written here, once per
//! kit, and named here.

use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_ui_composites::TimePicker;
pub use floom_ui_composites::{SharedTimeState, TimePickerState, TimePickerStyle, TimeValue};
use floom_types::{
    Color as PaintColor, Shadow,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::time_picker::{TimePickerProps, TimeValue as ThemeTimeValue};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;

const INCREASE: &str = "+";


/// The theme's time value -> the engine's.
fn to_engine_time(value: ThemeTimeValue) -> floom_ui_composites::TimeValue {
    floom_ui_composites::TimeValue {
        hour: value.hour,
        minute: value.minute,
    }
}

/// The engine's time value -> the theme's.
fn to_theme_time(value: floom_ui_composites::TimeValue) -> ThemeTimeValue {
    ThemeTimeValue {
        hour: value.hour,
        minute: value.minute,
    }
}

/// The time picker's appearance, in the terms v1 used.
///
/// The first block is v1's own list. The second block is what v1's paint used
/// but v1's style never reported -- the padding, the border's thickness, the
/// panel's own padding and shadow, the gaps, and the sizes that were literals
/// in the markup. A style that does not say what the widget draws is not the
/// widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTimePickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub height: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub panel_width: f32,
    pub cell_height: f32,
    // --- what v1's paint used and v1's style did not report ---
    /// The field's horizontal inner padding. v1's `.padding_horiz(12.0)`.
    pub padding_horizontal: f32,
    /// The border's thickness. v1's `.border(1.0)`.
    pub border_width: f32,
    /// The field's border alpha while the picker is disabled.
    /// v1's `with_alpha(outline, 97)`.
    pub disabled_border_alpha: u8,
    /// The panel's padding. v1's `.padding(12.0)`.
    pub panel_padding: f32,
    /// The panel's border alpha. v1's `with_alpha(outline, 120)`.
    pub panel_border_alpha: u8,
    /// The panel's shadow. v1's blur 12, spread 1, black at 24.
    pub panel_shadow: Shadow,
    /// The gap between the panel's three parts. v1's `.gap(8.0)`.
    pub panel_gap: f32,
    /// The gap inside a row, between the stepper and the number.
    /// v1's `.gap(8.0)`.
    pub row_gap: f32,
    /// The stepper's size. v1's `.size(32.0)`.
    pub stepper_size: f32,
    /// The stepper's radius. v1's `.border_radius(16.0)`.
    pub stepper_radius: f32,
    /// The stepper's fill alpha. v1's `with_alpha(on_surface, 8)`.
    pub stepper_fill_alpha: u8,
    /// The stepper's text size. v1's `.font_size(18.0)`.
    pub stepper_font_size: f32,
    /// The number's size. v1's `.font_size(24.0)`.
    pub value_font_size: f32,
    /// The number's width. v1's `.width(48.0)`.
    pub value_width: f32,
    /// The "Done" button's height. v1's `.height(32.0)`.
    pub done_height: f32,
    /// The "Done" button's horizontal padding. v1's `.padding_horiz(12.0)`.
    pub done_padding_horizontal: f32,
    /// The field's label size: the theme's body-medium.
    pub text_size: f32,
}

/// Cupertino's time picker.
pub struct CupertinoTimePicker {
    props: TimePickerProps,
    theme: RwSignal<MultiTheme>,
    /// The picker's state, shared with the tree it builds.
    state: SharedTimeState,
    /// The callback, held as a `Send + Sync` handle because the tree's slots
    /// have to be.
    on_change: Option<Arc<dyn Fn(TimeValue) + Send + Sync>>,
}

impl CupertinoTimePicker {
    pub fn new() -> Self {
        let props = TimePickerProps::default();
        Self {
            state: Arc::new(Mutex::new(TimePickerState {
                value: props.value.map(to_engine_time),
                open: false,
            })),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props,
            on_change: None,
        }
    }

    /// The time to show.
    pub fn value(mut self, value: TimeValue) -> Self {
        self.props.value = Some(to_theme_time(value));
        floom_ui_composites::write_state(&self.state, |guard| guard.value = Some(value));
        self
    }

    /// The text shown while nothing is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    /// Whether the picker answers at all.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the time the user chose.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside
    /// a slot closure the host may hand between threads, so the callback has
    /// to be both clonable and thread-safe to be rebuilt into the tree.
    pub fn on_change(mut self, callback: impl Fn(TimeValue) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(TimeValue) + Send + Sync> = Arc::new(callback);
        self.on_change = Some(shared.clone());
        // The props keep reporting that there is a callback, as v1's did. The
        // box is a wrapper, because the props' own type is not `Send + Sync`
        // and the tree cannot hold one.
        self.props.on_change = Some(Box::new(move |value: ThemeTimeValue| {
            shared(to_engine_time(value))
        }));
        self
    }

    pub fn props(&self) -> &TimePickerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The picker's shared state, for a caller that wants to read it.
    pub fn state(&self) -> SharedTimeState {
        self.state.clone()
    }

    /// The time the picker holds, if any.
    pub fn selected(&self) -> Option<TimeValue> {
        floom_ui_composites::read_state(&self.state).value
    }

    pub fn resolved_style(&self) -> CupertinoTimePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::TimePicker(tokens) = theme.component(ComponentId::TimePicker) else {
            unreachable!("ComponentId::TimePicker must resolve to TimePickerTokens")
        };
        CupertinoTimePickerStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            height: tokens.height,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            panel_width: tokens.panel_width,
            cell_height: tokens.cell_height,
            padding_horizontal: 12.0,
            border_width: 1.0,
            disabled_border_alpha: 97,
            panel_padding: 12.0,
            panel_border_alpha: 120,
            panel_shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            panel_gap: 8.0,
            row_gap: 8.0,
            stepper_size: 32.0,
            stepper_radius: 16.0,
            stepper_fill_alpha: 8,
            stepper_font_size: 18.0,
            value_font_size: 24.0,
            value_width: 48.0,
            done_height: 32.0,
            done_padding_horizontal: 12.0,
            text_size: theme.typography().body.medium.font_size,
        }
    }

    /// The picker, ready to mount.
    pub fn view(self) -> TimePicker {
        let style = self.resolved_style();
        let mut picker = TimePicker::new()
            .style(TimePickerStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                primary: to_paint(style.primary),
                height: style.height,
                border_radius: style.border_radius,
                min_width: style.min_width,
                panel_width: style.panel_width,
                cell_height: style.cell_height,
                padding_horizontal: style.padding_horizontal,
                border_width: style.border_width,
                disabled_border_alpha: style.disabled_border_alpha,
                panel_padding: style.panel_padding,
                panel_border_alpha: style.panel_border_alpha,
                panel_shadow: style.panel_shadow,
                panel_gap: style.panel_gap,
                row_gap: style.row_gap,
                stepper_size: style.stepper_size,
                stepper_radius: style.stepper_radius,
                stepper_fill_alpha: style.stepper_fill_alpha,
                stepper_font_size: style.stepper_font_size,
                value_font_size: style.value_font_size,
                value_width: style.value_width,
                done_height: style.done_height,
                done_padding_horizontal: style.done_padding_horizontal,
                text_size: style.text_size,
            })
            .state(self.state.clone())
            .disabled(self.props.disabled)
            .placeholder(self.props.placeholder.clone());
        if let Some(on_change) = self.on_change.clone() {
            picker = picker.on_change(move |value| on_change(value));
        }
        picker
    }
}

impl Default for CupertinoTimePicker {
    fn default() -> Self {
        Self::new()
    }
}

