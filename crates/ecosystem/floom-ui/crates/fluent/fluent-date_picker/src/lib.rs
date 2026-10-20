//! Fluent date picker, ported to the Floom 2.0 widget API.
//!
//! A date picker is a field that shows the chosen date, and a calendar that
//! opens under it: a header with the month and the two arrows that walk the
//! months, a row of weekday initials, and a six-week grid of days.
//!
//! ## What the port uses
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

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest, OverlayHandle,
    OverlayKind, OverlayRequest, PaintContext, Point, Rect, SemanticsContext, SemanticsRole,
    UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{
    Color as PaintColor, CursorStyle, FontWeight, PointerButton, PointerPhase, Shadow,
};
use floom_ui_composites::OverlayHost;
use floom_widgets::primitives::Revision;
use floom_widgets::text_width_at;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::date_picker::{DatePickerProps, DateValue};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The month the calendar shows when nothing is chosen: August 2026.
///
/// A v1 literal, and a strange one -- a picker with no date opens on a fixed
/// month rather than on the current one. Reproduced, because it is what v1
/// showed.
const DEFAULT_VIEW_MONTH: (i32, u32) = (2026, 8);

/// The month names, as v1 wrote them.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The weekday initials, as v1 wrote them.
const WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

/// The arrows that walk the months, as v1 wrote them.
const PREV_ARROW: &str = "\u{2039}";
const NEXT_ARROW: &str = "\u{203a}";

/// The number of week rows the grid holds.
const WEEK_ROWS: usize = 6;

/// The number of days in a week.
const WEEK_DAYS: usize = 7;

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Weekday of the 1st of the given month: 0 = Sunday … 6 = Saturday.
fn weekday_of_first(y: i32, m: u32) -> u32 {
    let mut days: i64 = 0;
    let mut yy = 1970;
    while yy < y {
        days += if is_leap(yy) { 366 } else { 365 };
        yy += 1;
    }
    let mut mm = 1u32;
    while mm < m {
        days += days_in_month(y, mm) as i64;
        mm += 1;
    }
    (((4 + days) % 7 + 7) % 7) as u32
}

/// Day number shown in the fixed cell `k` (0..42) for the displayed month, or
/// `None` for a leading/trailing blank cell.
fn day_at(k: usize, y: i32, m: u32) -> Option<u32> {
    let offset = weekday_of_first(y, m) as i32;
    let n = k as i32 - offset + 1;
    if n >= 1 && n <= days_in_month(y, m) as i32 {
        Some(n as u32)
    } else {
        None
    }
}

fn prev_month(y: i32, m: u32) -> (i32, u32) {
    if m == 1 {
        (y - 1, 12)
    } else {
        (y, m - 1)
    }
}

fn next_month(y: i32, m: u32) -> (i32, u32) {
    if m == 12 {
        (y + 1, 1)
    } else {
        (y, m + 1)
    }
}

/// The picker's state: the chosen date, whether the calendar is open, and the
/// month the calendar shows.
///
/// v1 held three `RwSignal`s -- the value, `open` and the displayed month.
/// 2.0's slots are shared between threads and an `RwSignal` is `Rc`-based, so
/// a value that changes after mount cannot be a signal; it is a
/// mutex-guarded cell instead, the same shape the engine's `SharedFlag` and
/// `SharedBuffer` have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatePickerState {
    /// The chosen date. `None` means the placeholder is showing.
    pub value: Option<DateValue>,
    /// Whether the calendar is open.
    pub open: bool,
    /// The month the calendar shows, as `(year, month)`.
    pub view_month: (i32, u32),
}

/// The shared cell the picker and the tree it built read and write.
pub type SharedDateState = Arc<Mutex<DatePickerState>>;

/// The state as it stands, treating a poisoned cell as "nothing chosen".
fn read_state(state: &SharedDateState) -> DatePickerState {
    match state.lock() {
        Ok(guard) => *guard,
        Err(_) => DatePickerState {
            value: None,
            open: false,
            view_month: DEFAULT_VIEW_MONTH,
        },
    }
}

/// Writes the state, doing nothing if the cell is poisoned.
fn write_state(state: &SharedDateState, edit: impl FnOnce(&mut DatePickerState)) {
    if let Ok(mut guard) = state.lock() {
        edit(&mut guard);
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
            .unwrap_or(DEFAULT_VIEW_MONTH);
        Self {
            state: Arc::new(Mutex::new(DatePickerState {
                value: props.value,
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
        self.props.value = Some(value);
        write_state(&self.state, |guard| {
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
        self.props.on_change = Some(Box::new(move |value: DateValue| shared(value)));
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
        read_state(&self.state).value
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
    pub fn view(self) -> FluentDatePickerWidget {
        let style = self.resolved_style();
        FluentDatePickerWidget {
            style,
            state: self.state,
            on_change: self.on_change,
            disabled: self.props.disabled,
            placeholder: self.props.placeholder,
            surface: OverlayHost::new(false),
            painted: None,
            revision: Revision::default(),
        }
    }
}

impl Default for FluentDatePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// The mounted picker: the field, and the calendar it opens.
pub struct FluentDatePickerWidget {
    style: FluentDatePickerStyle,
    state: SharedDateState,
    on_change: Option<Arc<dyn Fn(DateValue) + Send + Sync>>,
    disabled: bool,
    placeholder: String,
    /// The overlay bookkeeping: the calendar is a host overlay, with an in-tree
    /// fallback for a host that has none.
    surface: OverlayHost,
    /// The date the field last painted, so a change is a change.
    painted: Option<DateValue>,
    revision: Revision,
}

impl FluentDatePickerWidget {
    /// The text the field shows: the chosen date, or the placeholder.
    fn label(&self) -> String {
        match read_state(&self.state).value {
            Some(d) => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
            None => self.placeholder.clone(),
        }
    }

    /// The field's border colour: v1's three cases, in v1's order.
    fn border_color(&self) -> Color {
        let style = self.style;
        if self.disabled {
            with_alpha(style.outline, style.disabled_border_alpha)
        } else if read_state(&self.state).open {
            style.primary
        } else {
            style.outline
        }
    }

    /// The calendar, as a child slot -- the fallback for a host with no
    /// overlays.
    fn calendar_child(&self) -> Option<Child> {
        let style = self.style;
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        Some(Child::new(
            ChildKey::index(0),
            LayoutRequest::new()
                .width(style.panel_width)
                .height(calendar_height(&style) as f32),
            move || {
                Box::new(FluentDatePickerCalendar {
                    style,
                    state: state.clone(),
                    on_change: on_change.clone(),
                    handle: None,
                    revision: Revision::default(),
                }) as Box<dyn WidgetAny>
            },
        ))
    }
}

impl Widget for FluentDatePickerWidget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = read_state(&self.state);
        if now.value != self.painted {
            self.painted = now.value;
            self.revision.bump();
        }
        // The calendar is up exactly while the shared state says so.
        self.surface.set_present(now.open);
        let style = self.style;
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                // Never modal: a calendar is dismissed by choosing a day, by
                // clicking away, or by the field closing it, and the host's
                // outside-press rule only makes sense for a panel that does not
                // block.
                false,
                calendar_rect(bounds, style),
                move |handle| {
                    Box::new(FluentDatePickerCalendar {
                        style,
                        state: state.clone(),
                        on_change: on_change.clone(),
                        handle: Some(handle),
                        revision: Revision::default(),
                    }) as Box<dyn WidgetAny>
                },
            ))
        });
        self.revision.sync(cx, id);
    }

    /// A click on the field opens the calendar, and a second one closes it.
    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        if self.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        if pointer.phase != PointerPhase::Up || pointer.button != Some(PointerButton::Primary) {
            return false;
        }
        write_state(&self.state, |guard| {
            if guard.open {
                guard.open = false;
            } else {
                // Opening the calendar returns to the chosen date's month,
                // which is what v1 did.
                if let Some(d) = guard.value {
                    guard.view_month = (d.year, d.month);
                }
                guard.open = true;
            }
        });
        true
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let style = self.style;
        Some(
            LayoutRequest::new()
                .height(style.height)
                .min_width(style.min_width)
                .padding_horiz(f64::from(style.padding_horizontal)),
        )
    }

    fn children(&self) -> Vec<Child> {
        // The calendar is out of the flow either way: with a host it is on the
        // host's overlay stack, and without one it is the in-tree fallback.
        self.surface
            .fallback_children(self.calendar_child(), ChildKey::index(0))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        let now = read_state(&self.state);
        let inset = f64::from(style.border_width);
        // The border, then the fill inside it: `PaintContext` has a rounded
        // rect fill and no stroke.
        cx.fill_rounded_rect(bounds, style.border_radius, to_paint(self.border_color()));
        cx.fill_rounded_rect(
            shrink(bounds, inset),
            (style.border_radius - style.border_width).max(0.0),
            to_paint(style.surface),
        );
        let (label, ink) = match now.value {
            Some(d) => (
                format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
                style.on_surface,
            ),
            None => (self.placeholder.clone(), style.on_surface_variant),
        };
        cx.fill_text(
            Point::new(
                bounds.x0 + f64::from(style.padding_horizontal),
                bounds.y0 + (bounds.height() - f64::from(style.text_size)) / 2.0,
            ),
            &label,
            style.text_size,
            FontWeight::Normal,
            to_paint(ink),
        );
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            // The calendar is gone -- whether the user chose a day or clicked
            // away -- so the field stops saying it is open.
            write_state(&self.state, |guard| guard.open = false);
            cx.mark_dirty(id);
        }
    }

    fn cursor(&self) -> CursorStyle {
        // A disabled control gets the platform arrow: a pointer over a thing
        // that cannot be clicked is a promise the control cannot keep.
        if self.disabled {
            CursorStyle::Default
        } else {
            CursorStyle::Pointer
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        // The engine's own vocabulary: the role, the name and the box. The
        // accessibility-flavoured helper lives in `floom-widgets` and would
        // need a second vocabulary brought in for three lines.
        cx.set_role(SemanticsRole::Button);
        cx.set_label(&self.label());
        let bounds = cx.bounds();
        cx.set_bounds(bounds);
    }
}

impl WidgetAny for FluentDatePickerWidget {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The field, as a button that opens a calendar.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Button,
            self.label(),
        ))
    }
}

/// The calendar the picker opens.
struct FluentDatePickerCalendar {
    style: FluentDatePickerStyle,
    state: SharedDateState,
    on_change: Option<Arc<dyn Fn(DateValue) + Send + Sync>>,
    /// The overlay this calendar is the content of, so a chosen day can close
    /// it. `None` in the in-tree fallback, where there is no overlay to close.
    handle: Option<OverlayHandle>,
    revision: Revision,
}

impl FluentDatePickerCalendar {
    /// The day under a point, if any.
    fn day_at_point(&self, layout: &CalendarLayout, point: Point) -> Option<u32> {
        let (year, month) = read_state(&self.state).view_month;
        for row in 0..WEEK_ROWS {
            for column in 0..WEEK_DAYS {
                let cell = cell_rect(layout, &self.style, row, column);
                if !cell.contains(point) {
                    continue;
                }
                return day_at(row * WEEK_DAYS + column, year, month);
            }
        }
        None
    }

    /// The arrow under a point: the previous one, the next one, or neither.
    fn arrow_at_point(&self, layout: &CalendarLayout, point: Point) -> Option<Arrow> {
        if prev_arrow_rect(layout, &self.style).contains(point) {
            return Some(Arrow::Previous);
        }
        if next_arrow_rect(layout, &self.style).contains(point) {
            return Some(Arrow::Next);
        }
        None
    }

    /// Chooses a day: the value is set, the calendar closes, and the caller is
    /// told -- in v1's order.
    fn choose(&mut self, cx: &mut dyn EventContext, day: DateValue) {
        write_state(&self.state, |guard| {
            guard.value = Some(day);
            guard.open = false;
        });
        if let Some(callback) = &self.on_change {
            callback(day);
        }
        if let Some(handle) = self.handle {
            cx.close_overlay(handle);
        }
    }
}

impl WidgetAny for FluentDatePickerCalendar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The calendar names the month it shows.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let (year, month) = read_state(&self.state).view_month;
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Group,
            format!("{} {}", MONTHS[(month as usize).clamp(1, 12) - 1], year),
        ))
    }
}

impl Widget for FluentDatePickerCalendar {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        if pointer.phase != PointerPhase::Up || pointer.button != Some(PointerButton::Primary) {
            return false;
        }
        let bounds = cx.bounds();
        let layout = calendar_layout(&self.style, bounds);
        if let Some(arrow) = self.arrow_at_point(&layout, pointer.position) {
            let (year, month) = read_state(&self.state).view_month;
            let next = match arrow {
                Arrow::Previous => prev_month(year, month),
                Arrow::Next => next_month(year, month),
            };
            write_state(&self.state, |guard| guard.view_month = next);
            self.revision.bump();
            return true;
        }
        if let Some(day) = self.day_at_point(&layout, pointer.position) {
            let (year, month) = read_state(&self.state).view_month;
            self.choose(cx, DateValue { year, month, day });
            return true;
        }
        false
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let style = self.style;
        Some(
            LayoutRequest::new()
                .width(style.panel_width)
                .height(calendar_height(&style) as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        let now = read_state(&self.state);
        let (year, month) = now.view_month;

        // The panel: its shadow behind it, its border, and its fill.
        cx.draw_shadow(bounds, style.border_radius, &style.panel_shadow);
        let inset = f64::from(style.border_width);
        cx.fill_rounded_rect(
            bounds,
            style.border_radius,
            to_paint(with_alpha(style.outline, style.panel_border_alpha)),
        );
        cx.fill_rounded_rect(
            shrink(bounds, inset),
            (style.border_radius - style.border_width).max(0.0),
            to_paint(style.surface),
        );

        let layout = calendar_layout(&style, bounds);

        // The header: the previous arrow, the month, the next arrow.
        let title = format!("{} {}", MONTHS[(month as usize).clamp(1, 12) - 1], year);
        let title_width = f64::from(text_width_at(&title, style.header_font_size));
        cx.fill_text(
            Point::new(
                layout.inner.x0 + (layout.inner.width() - title_width) / 2.0,
                layout.header.y0
                    + (layout.header.height() - f64::from(style.header_font_size)) / 2.0,
            ),
            &title,
            style.header_font_size,
            FontWeight::Medium,
            to_paint(style.on_surface),
        );
        let arrow_size = f64::from(style.arrow_font_size);
        let arrow_y = layout.header.y0 + (layout.header.height() - arrow_size) / 2.0;
        let prev = prev_arrow_rect(&layout, &style);
        let next = next_arrow_rect(&layout, &style);
        cx.fill_text(
            Point::new(prev.x0 + f64::from(style.arrow_padding), arrow_y),
            PREV_ARROW,
            style.arrow_font_size,
            FontWeight::Normal,
            to_paint(style.on_surface),
        );
        cx.fill_text(
            Point::new(
                next.x1
                    - f64::from(style.arrow_padding)
                    - f64::from(text_width_at(NEXT_ARROW, style.arrow_font_size)),
                arrow_y,
            ),
            NEXT_ARROW,
            style.arrow_font_size,
            FontWeight::Normal,
            to_paint(style.on_surface),
        );

        // The weekday row: the initials, spread across the panel.
        for (index, letter) in WEEKDAYS.iter().enumerate() {
            let letter_width = f64::from(text_width_at(letter, style.weekday_font_size));
            let step = (layout.weekday.width() - letter_width) / (WEEK_DAYS - 1) as f64;
            cx.fill_text(
                Point::new(
                    layout.weekday.x0 + step * index as f64,
                    layout.weekday.y0
                        + (layout.weekday.height() - f64::from(style.weekday_font_size)) / 2.0,
                ),
                letter,
                style.weekday_font_size,
                FontWeight::Normal,
                to_paint(style.on_surface_variant),
            );
        }

        // The grid: six weeks of seven fixed cells. A cell with no day in it
        // is left blank, as v1 hid it.
        for row in 0..WEEK_ROWS {
            for column in 0..WEEK_DAYS {
                let Some(day) = day_at(row * WEEK_DAYS + column, year, month) else {
                    continue;
                };
                let cell = cell_rect(&layout, &style, row, column);
                let chosen = now.value == Some(DateValue { year, month, day });
                if chosen {
                    // The chosen day is a filled circle: v1's radius was half
                    // the cell's size.
                    cx.fill_rounded_rect(cell, style.cell_size / 2.0, to_paint(style.primary));
                }
                let ink = if chosen {
                    style.on_primary
                } else {
                    style.on_surface
                };
                let number = day.to_string();
                let number_width = f64::from(text_width_at(&number, style.text_size));
                cx.fill_text(
                    Point::new(
                        cell.x0 + (cell.width() - number_width) / 2.0,
                        cell.y0 + (cell.height() - f64::from(style.text_size)) / 2.0,
                    ),
                    &number,
                    style.text_size,
                    FontWeight::Normal,
                    to_paint(ink),
                );
            }
        }
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Pointer
    }
}

/// Which arrow of the calendar's header a click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrow {
    Previous,
    Next,
}

/// Where everything in the calendar sits.
///
/// Computed once, from the style and the panel's box, so the paint and the hit
/// test cannot disagree about where a day is.
struct CalendarLayout {
    /// The panel's box less its padding.
    inner: Rect,
    /// The header's box.
    header: Rect,
    /// The weekday row's box.
    weekday: Rect,
    /// The grid's box.
    grid: Rect,
    /// The gap between two cells in a row.
    cell_gap: f64,
    /// The gap between two rows of cells.
    row_gap: f64,
}

fn calendar_layout(style: &FluentDatePickerStyle, bounds: Rect) -> CalendarLayout {
    let padding = f64::from(style.panel_padding);
    let inner = Rect {
        x0: bounds.x0 + padding,
        y0: bounds.y0 + padding,
        x1: bounds.x1 - padding,
        y1: bounds.y1 - padding,
    };
    let header_height =
        f64::from(style.arrow_font_size) * 1.2 + f64::from(style.arrow_padding) * 2.0;
    let weekday_height = f64::from(style.weekday_font_size) * 1.2;
    let header = Rect {
        x0: inner.x0,
        y0: inner.y0,
        x1: inner.x1,
        y1: inner.y0 + header_height,
    };
    let weekday = Rect {
        x0: inner.x0,
        y0: header.y1 + f64::from(style.panel_gap),
        x1: inner.x1,
        y1: header.y1 + f64::from(style.panel_gap) + weekday_height,
    };
    let grid_top = weekday.y1 + f64::from(style.panel_gap);
    let cell = f64::from(style.cell_size);
    let row_gap = f64::from(style.grid_gap);
    let cell_gap = (inner.width() - WEEK_DAYS as f64 * cell) / (WEEK_DAYS - 1) as f64;
    let grid = Rect {
        x0: inner.x0,
        y0: grid_top,
        x1: inner.x1,
        y1: grid_top + WEEK_ROWS as f64 * cell + (WEEK_ROWS - 1) as f64 * row_gap,
    };
    CalendarLayout {
        inner,
        header,
        weekday,
        grid,
        cell_gap,
        row_gap,
    }
}

/// One cell of the grid.
fn cell_rect(
    layout: &CalendarLayout,
    style: &FluentDatePickerStyle,
    row: usize,
    column: usize,
) -> Rect {
    let cell = f64::from(style.cell_size);
    let x0 = layout.grid.x0 + column as f64 * (cell + layout.cell_gap);
    let y0 = layout.grid.y0 + row as f64 * (cell + layout.row_gap);
    Rect {
        x0,
        y0,
        x1: x0 + cell,
        y1: y0 + cell,
    }
}

/// The previous arrow's box.
fn prev_arrow_rect(layout: &CalendarLayout, style: &FluentDatePickerStyle) -> Rect {
    let width = f64::from(style.arrow_font_size) + f64::from(style.arrow_padding) * 2.0;
    Rect {
        x0: layout.header.x0,
        y0: layout.header.y0,
        x1: layout.header.x0 + width,
        y1: layout.header.y1,
    }
}

/// The next arrow's box.
fn next_arrow_rect(layout: &CalendarLayout, style: &FluentDatePickerStyle) -> Rect {
    let width = f64::from(style.arrow_font_size) + f64::from(style.arrow_padding) * 2.0;
    Rect {
        x0: layout.header.x1 - width,
        y0: layout.header.y0,
        x1: layout.header.x1,
        y1: layout.header.y1,
    }
}

/// The calendar's height: its padding, its header, its weekday row and its
/// grid, with v1's gaps between them.
fn calendar_height(style: &FluentDatePickerStyle) -> f64 {
    let padding = f64::from(style.panel_padding);
    let header_height =
        f64::from(style.arrow_font_size) * 1.2 + f64::from(style.arrow_padding) * 2.0;
    let weekday_height = f64::from(style.weekday_font_size) * 1.2;
    let grid_height = WEEK_ROWS as f64 * f64::from(style.cell_size)
        + (WEEK_ROWS - 1) as f64 * f64::from(style.grid_gap);
    2.0 * padding
        + header_height
        + f64::from(style.panel_gap)
        + weekday_height
        + f64::from(style.panel_gap)
        + grid_height
}

/// The rect the calendar asks the host for: under the field, as wide as v1's
/// panel.
///
/// The host places the content against this rect and clips it, so the width is
/// the panel's own and not the field's -- which is why the port asks for a
/// rect of its own rather than using the engine's `PopupPlacement::Below`,
/// whose rect is as wide as the control that owns it.
fn calendar_rect(bounds: Rect, style: FluentDatePickerStyle) -> Rect {
    Rect {
        x0: bounds.x0,
        y0: bounds.y1,
        x1: bounds.x0 + f64::from(style.panel_width),
        y1: bounds.y1 + calendar_height(&style),
    }
}

/// A box pulled in by `inset` on every side.
fn shrink(bounds: Rect, inset: f64) -> Rect {
    Rect {
        x0: bounds.x0 + inset,
        y0: bounds.y0 + inset,
        x1: (bounds.x1 - inset).max(bounds.x0 + inset),
        y1: (bounds.y1 - inset).max(bounds.y0 + inset),
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
