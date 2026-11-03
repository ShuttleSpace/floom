
use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest,
    OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point, Rect, SemanticsContext,
    SemanticsRole, UpdateContext, Widget, WidgetAny,
};
use floom_types::{CursorStyle, FontWeight, PointerButton, PointerPhase, Shadow};
use floom_widgets::primitives::Revision;
use floom_widgets::text_width_at;

use crate::color_picker::with_alpha;
use crate::OverlayHost;

/// The glyph the decrease stepper shows.
const DECREASE: &str = "\u{2212}";
/// The glyph the increase stepper shows.
const INCREASE: &str = "+";
/// The done button's label.
const DONE: &str = "Done";

/// A wall-clock time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeValue {
    /// The hour, 0..=23.
    pub hour: u8,
    /// The minute, 0..=59.
    pub minute: u8,
}

impl Default for TimeValue {
    fn default() -> Self {
        Self { hour: 0, minute: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimePickerStyle {
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

impl Default for TimePickerStyle {
    fn default() -> Self {
        Self {
            surface: Color::rgb(0, 0, 0),
            on_surface: Color::rgb(0, 0, 0),
            on_surface_variant: Color::rgb(0, 0, 0),
            outline: Color::rgb(0, 0, 0),
            primary: Color::rgb(0, 0, 0),
            height: 0.0,
            border_radius: 0.0,
            min_width: 0.0,
            panel_width: 0.0,
            cell_height: 0.0,
            padding_horizontal: 0.0,
            border_width: 0.0,
            disabled_border_alpha: 0,
            panel_padding: 0.0,
            panel_border_alpha: 0,
            panel_shadow: Shadow {
                color: Color::rgb(0, 0, 0),
                blur: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 0.0,
            },
            panel_gap: 0.0,
            row_gap: 0.0,
            stepper_size: 0.0,
            stepper_radius: 0.0,
            stepper_fill_alpha: 0,
            stepper_font_size: 0.0,
            value_font_size: 0.0,
            value_width: 0.0,
            done_height: 0.0,
            done_padding_horizontal: 0.0,
            text_size: 0.0,
        }
    }
}

/// The picker's state: the chosen time, and whether the panel is open.
///
/// v1 held two `RwSignal`s -- the value and `open`. 2.0's slots are shared
/// between threads and an `RwSignal` is `Rc`-based, so a value that changes
/// after mount cannot be a signal; it is a mutex-guarded cell instead, the
/// same shape the engine's `SharedFlag` and `SharedBuffer` have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePickerState {
    /// The chosen time. `None` means the placeholder is showing.
    pub value: Option<TimeValue>,
    /// Whether the panel is open.
    pub open: bool,
}

/// The shared cell the picker and the tree it built read and write.
pub type SharedTimeState = Arc<Mutex<TimePickerState>>;

/// The state as it stands, treating a poisoned cell as "nothing chosen".
pub fn read_state(state: &SharedTimeState) -> TimePickerState {
    match state.lock() {
        Ok(guard) => *guard,
        Err(_) => TimePickerState {
            value: None,
            open: false,
        },
    }
}

/// Writes the state, doing nothing if the cell is poisoned.
pub fn write_state(state: &SharedTimeState, edit: impl FnOnce(&mut TimePickerState)) {
    if let Ok(mut guard) = state.lock() {
        edit(&mut guard);
    }
}

pub struct TimePicker {
    style: TimePickerStyle,
    state: SharedTimeState,
    on_change: Option<Arc<dyn Fn(TimeValue) + Send + Sync>>,
    disabled: bool,
    placeholder: String,
    /// The overlay bookkeeping: the panel is a host overlay, with an in-tree
    /// fallback for a host that has none.
    surface: OverlayHost,
    /// The time the field last painted, so a change is a change.
    painted: Option<TimeValue>,
    revision: Revision,
}

impl TimePicker {
    pub fn new() -> Self {
        Self {
            style: TimePickerStyle::default(),
            state: Arc::new(Mutex::new(TimePickerState {
                value: None,
                open: false,
            })),
            on_change: None,
            disabled: false,
            placeholder: String::new(),
            surface: OverlayHost::new(false),
            painted: None,
            revision: Revision::default(),
        }
    }

    /// The picker's appearance.
    pub fn style(mut self, style: TimePickerStyle) -> Self {
        self.style = style;
        self
    }

    /// The shared cell the chosen time and the panel's open state live in.
    pub fn state(mut self, state: SharedTimeState) -> Self {
        self.state = state;
        self
    }

    /// Called with the chosen time when it is committed.
    pub fn on_change(mut self, callback: impl Fn(TimeValue) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Whether the picker answers the pointer.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The text shown when nothing is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

impl TimePicker {
    /// The text the field shows: the chosen time, or the placeholder.
    fn label(&self) -> String {
        match read_state(&self.state).value {
            Some(tv) => format!("{:02}:{:02}", tv.hour, tv.minute),
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

    /// The panel, as a child slot -- the fallback for a host with no overlays.
    fn panel_child(&self) -> Option<Child> {
        let style = self.style;
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        Some(Child::new(
            ChildKey::index(0),
            LayoutRequest::new()
                .width(style.panel_width)
                .height(panel_height(&style) as f32),
            move || {
                Box::new(TimePickerPanel {
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

impl Widget for TimePicker {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = read_state(&self.state);
        if now.value != self.painted {
            self.painted = now.value;
            self.revision.bump();
        }
        // The panel is up exactly while the shared state says so.
        self.surface.set_present(now.open);
        let style = self.style;
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                // Never modal: a panel is dismissed by "Done", by clicking
                // away, or by the field closing it, and the host's outside-press
                // rule only makes sense for a panel that does not block.
                false,
                panel_rect(bounds, style),
                move |handle| {
                    Box::new(TimePickerPanel {
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

    /// A click on the field opens the panel, and a second one closes it.
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
        write_state(&self.state, |guard| guard.open = !guard.open);
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
        // The panel is out of the flow either way: with a host it is on the
        // host's overlay stack, and without one it is the in-tree fallback.
        self.surface
            .fallback_children(self.panel_child(), ChildKey::index(0))
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
        cx.fill_rounded_rect(bounds, style.border_radius, self.border_color());
        cx.fill_rounded_rect(
            shrink(bounds, inset),
            (style.border_radius - style.border_width).max(0.0),
            style.surface,
        );
        let (label, ink) = match now.value {
            Some(tv) => (format!("{:02}:{:02}", tv.hour, tv.minute), style.on_surface),
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
            ink,
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
            // The panel is gone -- whether the user pressed "Done" or clicked
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

impl WidgetAny for TimePicker {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The field, as a button that opens the panel.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Button,
            self.label(),
        ))
    }
}

/// The panel the picker opens: an hour row, a minute row and a "Done" button.
struct TimePickerPanel {
    style: TimePickerStyle,
    state: SharedTimeState,
    on_change: Option<Arc<dyn Fn(TimeValue) + Send + Sync>>,
    /// The overlay this panel is the content of, so "Done" can close it.
    /// `None` in the in-tree fallback, where there is no overlay to close.
    handle: Option<OverlayHandle>,
    revision: Revision,
}

/// Which of the panel's four steppers a click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stepper {
    HourDown,
    HourUp,
    MinuteDown,
    MinuteUp,
}

/// What a click landed on in the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelHit {
    Stepper(Stepper),
    Done,
}

impl TimePickerPanel {
    /// What a point is over, if anything.
    fn hit_at(&self, layout: &PanelLayout, point: Point) -> Option<PanelHit> {
        if layout.done.contains(point) {
            return Some(PanelHit::Done);
        }
        if layout.hour_row.contains(point) {
            if layout.hour_down.contains(point) {
                return Some(PanelHit::Stepper(Stepper::HourDown));
            }
            if layout.hour_up.contains(point) {
                return Some(PanelHit::Stepper(Stepper::HourUp));
            }
            return None;
        }
        if layout.minute_row.contains(point) {
            if layout.minute_down.contains(point) {
                return Some(PanelHit::Stepper(Stepper::MinuteDown));
            }
            if layout.minute_up.contains(point) {
                return Some(PanelHit::Stepper(Stepper::MinuteUp));
            }
        }
        None
    }

    /// Sets the time and tells the caller, in v1's order.
    fn set_time(&mut self, hour: u8, minute: u8) {
        let value = TimeValue { hour, minute };
        write_state(&self.state, |guard| guard.value = Some(value));
        if let Some(callback) = &self.on_change {
            callback(value);
        }
    }

    /// Steps the hour or the minute, wrapping the way v1 did: the hour wraps
    /// over 24 and the minute over 60.
    fn step(&mut self, stepper: Stepper) {
        let current = read_state(&self.state)
            .value
            .unwrap_or(TimeValue { hour: 0, minute: 0 });
        match stepper {
            Stepper::HourDown => {
                let hour = if current.hour == 0 {
                    23
                } else {
                    current.hour - 1
                };
                self.set_time(hour, current.minute);
            }
            Stepper::HourUp => {
                self.set_time((current.hour + 1) % 24, current.minute);
            }
            Stepper::MinuteDown => {
                let minute = if current.minute == 0 {
                    59
                } else {
                    current.minute - 1
                };
                self.set_time(current.hour, minute);
            }
            Stepper::MinuteUp => {
                self.set_time(current.hour, (current.minute + 1) % 60);
            }
        }
    }

    /// Closes the panel, as "Done" did.
    fn done(&mut self, cx: &mut dyn EventContext) {
        write_state(&self.state, |guard| guard.open = false);
        if let Some(handle) = self.handle {
            cx.close_overlay(handle);
        }
    }
}

impl WidgetAny for TimePickerPanel {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The panel names the time it shows.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let value = read_state(&self.state)
            .value
            .unwrap_or(TimeValue { hour: 0, minute: 0 });
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Group,
            format!("{:02}:{:02}", value.hour, value.minute),
        ))
    }
}

impl Widget for TimePickerPanel {
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
        let layout = panel_layout(&self.style, cx.bounds());
        match self.hit_at(&layout, pointer.position) {
            Some(PanelHit::Stepper(stepper)) => {
                self.step(stepper);
                true
            }
            Some(PanelHit::Done) => {
                self.done(cx);
                true
            }
            None => false,
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let style = self.style;
        Some(
            LayoutRequest::new()
                .width(style.panel_width)
                .height(panel_height(&style) as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        let now = read_state(&self.state);
        let value = now.value.unwrap_or(TimeValue { hour: 0, minute: 0 });

        // The panel: its shadow behind it, its border, and its fill.
        cx.draw_shadow(bounds, style.border_radius, &style.panel_shadow);
        let inset = f64::from(style.border_width);
        cx.fill_rounded_rect(
            bounds,
            style.border_radius,
            with_alpha(style.outline, style.panel_border_alpha),
        );
        cx.fill_rounded_rect(
            shrink(bounds, inset),
            (style.border_radius - style.border_width).max(0.0),
            style.surface,
        );

        let layout = panel_layout(&style, bounds);

        // The hour row: the steppers and the number.
        self.paint_stepper(cx, &layout.hour_down, DECREASE);
        self.paint_stepper(cx, &layout.hour_up, INCREASE);
        cx.fill_text(
            Point::new(
                layout.hour_value.x0
                    + (layout.hour_value.width()
                        - f64::from(text_width_at(
                            &format!("{:02}", value.hour),
                            style.value_font_size,
                        )))
                        / 2.0,
                layout.hour_value.y0
                    + (layout.hour_value.height() - f64::from(style.value_font_size)) / 2.0,
            ),
            &format!("{:02}", value.hour),
            style.value_font_size,
            FontWeight::Medium,
            style.on_surface,
        );

        // The minute row.
        self.paint_stepper(cx, &layout.minute_down, DECREASE);
        self.paint_stepper(cx, &layout.minute_up, INCREASE);
        cx.fill_text(
            Point::new(
                layout.minute_value.x0
                    + (layout.minute_value.width()
                        - f64::from(text_width_at(
                            &format!("{:02}", value.minute),
                            style.value_font_size,
                        )))
                        / 2.0,
                layout.minute_value.y0
                    + (layout.minute_value.height() - f64::from(style.value_font_size)) / 2.0,
            ),
            &format!("{:02}", value.minute),
            style.value_font_size,
            FontWeight::Medium,
            style.on_surface,
        );

        // The "Done" button.
        let done_width = f64::from(text_width_at(DONE, style.text_size))
            + f64::from(style.done_padding_horizontal) * 2.0;
        cx.fill_text(
            Point::new(
                layout.done.x0 + (layout.done.width() - done_width) / 2.0,
                layout.done.y0 + (layout.done.height() - f64::from(style.text_size)) / 2.0,
            ),
            DONE,
            style.text_size,
            FontWeight::Normal,
            style.primary,
        );
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Pointer
    }
}

impl TimePickerPanel {
    /// One stepper: v1's filled circle with its sign in the middle.
    fn paint_stepper(&self, cx: &mut dyn PaintContext, box_: &Rect, sign: &str) {
        let style = self.style;
        cx.fill_rounded_rect(
            *box_,
            style.stepper_radius,
            with_alpha(style.on_surface, style.stepper_fill_alpha),
        );
        let sign_width = f64::from(text_width_at(sign, style.stepper_font_size));
        cx.fill_text(
            Point::new(
                box_.x0 + (box_.width() - sign_width) / 2.0,
                box_.y0 + (box_.height() - f64::from(style.stepper_font_size)) / 2.0,
            ),
            sign,
            style.stepper_font_size,
            FontWeight::Normal,
            style.on_surface,
        );
    }
}

/// Where everything in the panel sits.
///
/// Computed once, from the style and the panel's box, so the paint and the hit
/// test cannot disagree about where a stepper is.
struct PanelLayout {
    /// The hour row's box.
    hour_row: Rect,
    /// The hour stepper that steps down.
    hour_down: Rect,
    /// The hour stepper that steps up.
    hour_up: Rect,
    /// The hour number's box.
    hour_value: Rect,
    /// The minute row's box.
    minute_row: Rect,
    /// The minute stepper that steps down.
    minute_down: Rect,
    /// The minute stepper that steps up.
    minute_up: Rect,
    /// The minute number's box.
    minute_value: Rect,
    /// The "Done" button's box.
    done: Rect,
}

fn panel_layout(style: &TimePickerStyle, bounds: Rect) -> PanelLayout {
    let padding = f64::from(style.panel_padding);
    let inner = Rect {
        x0: bounds.x0 + padding,
        y0: bounds.y0 + padding,
        x1: bounds.x1 - padding,
        y1: bounds.y1 - padding,
    };
    let row_height = f64::from(style.cell_height);
    let gap = f64::from(style.panel_gap);
    let stepper = f64::from(style.stepper_size);
    let value_width = f64::from(style.value_width);
    let row_gap = f64::from(style.row_gap);
    // The row is the two steppers and the number, and v1 centred it.
    let row_width = stepper * 2.0 + value_width + row_gap * 2.0;
    let row_x0 = inner.x0 + (inner.width() - row_width) / 2.0;

    let hour_row = Rect {
        x0: inner.x0,
        y0: inner.y0,
        x1: inner.x1,
        y1: inner.y0 + row_height,
    };
    let minute_row = Rect {
        x0: inner.x0,
        y0: hour_row.y1 + gap,
        x1: inner.x1,
        y1: hour_row.y1 + gap + row_height,
    };
    let done = Rect {
        x0: inner.x0,
        y0: minute_row.y1 + gap,
        x1: inner.x1,
        y1: minute_row.y1 + gap + f64::from(style.done_height),
    };
    let value = Rect {
        x0: row_x0 + stepper + row_gap,
        y0: hour_row.y0,
        x1: row_x0 + stepper + row_gap + value_width,
        y1: hour_row.y1,
    };
    PanelLayout {
        hour_row,
        hour_down: Rect {
            x0: row_x0,
            y0: hour_row.y0,
            x1: row_x0 + stepper,
            y1: hour_row.y1,
        },
        hour_up: Rect {
            x0: row_x0 + stepper + row_gap + value_width + row_gap,
            y0: hour_row.y0,
            x1: row_x0 + stepper + row_gap + value_width + row_gap + stepper,
            y1: hour_row.y1,
        },
        hour_value: value,
        minute_row,
        minute_down: Rect {
            x0: row_x0,
            y0: minute_row.y0,
            x1: row_x0 + stepper,
            y1: minute_row.y1,
        },
        minute_up: Rect {
            x0: row_x0 + stepper + row_gap + value_width + row_gap,
            y0: minute_row.y0,
            x1: row_x0 + stepper + row_gap + value_width + row_gap + stepper,
            y1: minute_row.y1,
        },
        minute_value: Rect {
            x0: value.x0,
            y0: minute_row.y0,
            x1: value.x1,
            y1: minute_row.y1,
        },
        done,
    }
}

/// The panel's height: its padding, its two rows and its button, with v1's
/// gaps between them.
fn panel_height(style: &TimePickerStyle) -> f64 {
    2.0 * f64::from(style.panel_padding)
        + 2.0 * f64::from(style.cell_height)
        + 2.0 * f64::from(style.panel_gap)
        + f64::from(style.done_height)
}

/// The rect the panel asks the host for: under the field, as wide as v1's
/// panel.
///
/// The host places the content against this rect and clips it, so the width is
/// the panel's own and not the field's -- which is why the port asks for a
/// rect of its own rather than using the engine's `PopupPlacement::Below`,
/// whose rect is as wide as the control that owns it.
fn panel_rect(bounds: Rect, style: TimePickerStyle) -> Rect {
    Rect {
        x0: bounds.x0,
        y0: bounds.y1,
        x1: bounds.x0 + f64::from(style.panel_width),
        y1: bounds.y1 + panel_height(&style),
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
