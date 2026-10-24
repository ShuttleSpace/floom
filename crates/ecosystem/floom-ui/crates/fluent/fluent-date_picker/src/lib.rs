//! Fluent date picker, ported to the Floom 2.0 widget API.
//!
//! A date picker is a field that shows the chosen date, and a calendar that
//! opens under it: a header with the month and the two arrows that walk the
//! months, a row of weekday initials, and a six-week grid of days.
//!
//! ## What the port uses
//!
//! The engine's `DatePicker`: the field that shows the date and the calendar
//! that picks one. The three floem-ui date-picker kits each used to carry
//! their own copy of both, along with the calendar's date arithmetic and the
//! layout that puts the days in the grid.
//!
//! The engine's overlay door. v1 put the calendar in an `Overlay` and placed
//! it by hand: a `LayoutChangedListener` read the field's *window* origin and
//! height and wrote them into `inset_top` and `inset_left`, so the panel's
//! position was the picker's own arithmetic -- and it was arithmetic about the
//! window, not about the field, so it was wrong the moment the page scrolled.
//! 2.0's host owns the overlay layers. The picker asks for a popup rect
//! derived from its own box, and the host puts the calendar there, paints it
//! above the tree and hit-tests it first. So the listener is gone, and so are
//! the window-origin insets.
//!
//! ## What this kit answers
//!
//! * the field's height, radius and minimum width, the calendar's width and
//!   the day cell's size: the tokens;
//! * the field's fill: `surface`; its border: `outline`, `primary` while the
//!   calendar is open, and `outline` at 97 while the picker is disabled; the
//!   date's ink: `on_surface`, the placeholder's: `on_surface_variant`;
//! * the calendar's fill: `surface`; its border: `outline` at 120; its
//!   shadow: black at 24, blurred 12 and spread 1;
//! * the chosen day's fill: `primary`; its ink: `on_primary`.
//!
//! ## The numbers v1 wrote as literals
//!
//! The field's horizontal padding (12), the border's thickness (1), the
//! calendar's padding (8), the gap between the calendar's three parts (6), the
//! gap between the grid's rows (2), the weekday row's size (11), the arrows'
//! size (18) and their padding (4).
//!
//! ## The token that never showed
//!
//! `DatePickerTokens::padding_horizontal` (16) is not in v1's markup: the
//! field padded itself by 12, a literal. The port keeps v1's 12 and leaves the
//! token unread, and the style reports the 12 under the name the paint uses.
//!
//! ## The differences this port names rather than reproduces
//!
//! * **The hovered day cell.** v1 filled a day with `primary` at 16 while the
//!   pointer was over it. 2.0's `Pressable` does not publish its interaction
//!   state to the widget that owns it, so there is no way to know which cell
//!   the pointer is over. The port's cells paint their selected fill or
//!   nothing. Named rather than faked with a fill that is always on.
//! * **The calendar opens on the frame after the click.** The calendar is a
//!   host overlay, and 2.0's overlay door is state-driven: the click writes
//!   "open" into the shared state, and the next update asks the host for the
//!   panel. v1's panel was up in the same frame.
//! * **The field and the calendar paint themselves.** 2.0 keeps a mounted
//!   child's widget when its key survives a rebuild, so a `Text` child would
//!   keep the date it was built with. The port's field and calendar read the
//!   shared state when they paint.
//!
//! ## The theme source
//!
//! v1's Fluent date picker took a *snapshot* of its own design system --
//! `MultiTheme::new_for(DesignSystem::Fluent)` -- so a change of the
//! app-wide theme does not reach it. The port keeps that.

//! ## What v1 wrote that this port does not carry
//!
//! * **`props().on_change` is a `Box<dyn Fn(DateValue)>`.** The kit fills it
//!   with a wrapper around its own `Arc<dyn Fn(DateValue) + Send + Sync>`:
//!   the closure a 2.0 tree is built from has to be `Send + Sync`, and v1's
//!   box is not.
//! * **The accessors.** v1's Fluent date picker had no `props()`, `theme()`
//!   or `with_theme()`. The port gives all three kits the same three, because
//!   a kit that cannot be given a theme cannot be tested against one.
//!
//! ## The shape that belongs to the engine
//!
//! The picker widget -- a field, a shared state, and a calendar that opens
//! under it -- is the same shape in all three kits, so it belongs in
//! `floom-ui-composites` beside `Combobox` and `Menu`. The engine could not be
//! edited while this port was written, so the shape is written here, once per
//! kit, and named here.

use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Shadow};
use floom_ui_composites::DatePicker;
pub use floom_ui_composites::{DatePickerState, DatePickerStyle, DateValue, SharedDateState};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::date_picker::{DatePickerProps, DateValue as ThemeDateValue};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The month the calendar shows when nothing is chosen: August 2026.
///
/// A v1 literal, and a strange one -- a picker with no date opens on a fixed
/// month rather than on the current one. Reproduced, because it is what v1
/// showed.
/// The theme's date value -> the engine's.
fn to_engine_date(value: ThemeDateValue) -> floom_ui_composites::DateValue {
    floom_ui_composites::DateValue {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

/// The engine's date value -> the theme's.
fn to_theme_date(value: floom_ui_composites::DateValue) -> ThemeDateValue {
    ThemeDateValue {
        year: value.year,
        month: value.month,
        day: value.day,
    }
}

/// The date picker's appearance, in the terms v1 used.
///
/// The first block is v1's own list. The second block is what v1's paint used
/// but v1's style never reported -- the padding, the border's thickness, the
/// calendar's own padding and shadow, the gaps and the sizes that were
/// literals in the markup. A style that does not say what the widget draws is
/// not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentDatePickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub height: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub panel_width: f32,
    pub cell_size: f32,
    pub header_font_size: f32,
    // --- what v1's paint used and v1's style did not report ---
    /// The field's horizontal inner padding. v1's `.padding_horiz(12.0)`.
    pub padding_horizontal: f32,
    /// The border's thickness. v1's `.border(1.0)`.
    pub border_width: f32,
    /// The field's border alpha while the picker is disabled.
    /// v1's `with_alpha(outline, 97)`.
    pub disabled_border_alpha: u8,
    /// The calendar's padding. v1's `.padding(8.0)`.
    pub panel_padding: f32,
    /// The calendar's border alpha. v1's `with_alpha(outline, 120)`.
    pub panel_border_alpha: u8,
    /// The calendar's shadow. v1's blur 12, spread 1, black at 24.
    pub panel_shadow: Shadow,
    /// The gap between the calendar's header, weekday row and grid.
    /// v1's `.gap(6.0)`.
    pub panel_gap: f32,
    /// The gap between the grid's rows. v1's `.gap(2.0)`.
    pub grid_gap: f32,
    /// The weekday row's size. v1's `.font_size(11.0)`.
    pub weekday_font_size: f32,
    /// The arrows' size. v1's `.font_size(18.0)`.
    pub arrow_font_size: f32,
    /// The arrows' padding. v1's `.padding(4.0)`.
    pub arrow_padding: f32,
    /// The field's label size: the theme's body-medium.
    pub text_size: f32,
    /// The fill behind a hovered day. v1's `with_alpha(primary, 16)`.
    ///
    /// Reported because v1's style did not, and *not* painted: 2.0's
    /// `Pressable` does not publish its interaction state, so the port cannot
    /// know which cell the pointer is over. See the module note.
    pub hover_alpha: u8,
}

/// Fluent's date picker.
pub struct FluentDatePicker {
    props: DatePickerProps,
    theme: RwSignal<MultiTheme>,
    /// The picker's state, shared with the tree it builds.
    state: SharedDateState,
    /// The callback, held as a `Send + Sync` handle because the tree's slots
    /// have to be.
    on_change: Option<Arc<dyn Fn(DateValue) + Send + Sync>>,
}

impl FluentDatePicker {
    pub fn new() -> Self {
        let props = DatePickerProps::default();
        let view_month = props
            .value
            .map(|d| (d.year, d.month))
            .unwrap_or(floom_ui_composites::DEFAULT_VIEW_MONTH);
        Self {
            state: Arc::new(Mutex::new(DatePickerState {
                value: props.value.map(to_engine_date),
                open: false,
                view_month,
            })),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props,
            on_change: None,
        }
    }

    /// The date to show, and the month the calendar opens on.
    pub fn value(mut self, value: DateValue) -> Self {
        self.props.value = Some(to_theme_date(value));
        floom_ui_composites::write_date_state(&self.state, |guard| {
            guard.value = Some(value);
            guard.view_month = (value.year, value.month);
        });
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

    /// Called with the date the user chose.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside
    /// a slot closure the host may hand between threads, so the callback has
    /// to be both clonable and thread-safe to be rebuilt into the tree.
    pub fn on_change(mut self, callback: impl Fn(DateValue) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(DateValue) + Send + Sync> = Arc::new(callback);
        self.on_change = Some(shared.clone());
        // The props keep reporting that there is a callback, as v1's did. The
        // box is a wrapper, because the props' own type is not `Send + Sync`
        // and the tree cannot hold one.
        self.props.on_change = Some(Box::new(move |value: ThemeDateValue| {
            shared(to_engine_date(value))
        }));
        self
    }

    pub fn props(&self) -> &DatePickerProps {
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
    pub fn state(&self) -> SharedDateState {
        self.state.clone()
    }

    /// The date the picker holds, if any.
    pub fn selected(&self) -> Option<DateValue> {
        floom_ui_composites::read_date_state(&self.state).value
    }

    pub fn resolved_style(&self) -> FluentDatePickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::DatePicker(tokens) = theme.component(ComponentId::DatePicker) else {
            unreachable!("ComponentId::DatePicker must resolve to DatePickerTokens")
        };
        FluentDatePickerStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            on_primary: theme.colors().on_primary,
            height: tokens.height,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            panel_width: tokens.panel_width,
            cell_size: tokens.cell_size,
            header_font_size: tokens.header_font_size,
            padding_horizontal: 12.0,
            border_width: 1.0,
            disabled_border_alpha: 97,
            panel_padding: 8.0,
            panel_border_alpha: 120,
            panel_shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            panel_gap: 6.0,
            grid_gap: 2.0,
            weekday_font_size: 11.0,
            arrow_font_size: 18.0,
            arrow_padding: 4.0,
            text_size: theme.typography().body.medium.font_size,
            hover_alpha: 16,
        }
    }

    /// The picker, ready to mount.
    pub fn view(self) -> DatePicker {
        let style = self.resolved_style();
        let mut picker = DatePicker::new()
            .style(DatePickerStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                primary: to_paint(style.primary),
                on_primary: to_paint(style.on_primary),
                height: style.height,
                border_radius: style.border_radius,
                min_width: style.min_width,
                panel_width: style.panel_width,
                cell_size: style.cell_size,
                header_font_size: style.header_font_size,
                padding_horizontal: style.padding_horizontal,
                border_width: style.border_width,
                disabled_border_alpha: style.disabled_border_alpha,
                panel_padding: style.panel_padding,
                panel_border_alpha: style.panel_border_alpha,
                panel_shadow: style.panel_shadow,
                panel_gap: style.panel_gap,
                grid_gap: style.grid_gap,
                weekday_font_size: style.weekday_font_size,
                arrow_font_size: style.arrow_font_size,
                arrow_padding: style.arrow_padding,
                text_size: style.text_size,
                hover_alpha: style.hover_alpha,
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

impl Default for FluentDatePicker {
    fn default() -> Self {
        Self::new()
    }
}
