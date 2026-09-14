//! Tier-1 interactive controls.
//!
//! Every control carries an [`InteractionState`] (hover / press / focus /
//! disable) and reports it through [`SemanticsHint::state`], so assistive
//! technology sees the same state the control paints. Mutation goes through
//! builders that bump the internal revision, keeping a single invalidation
//! model across all widgets.

use floem_accessibility::{SemanticsRole, SemanticsState};
use floem_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget,
};
use floem_types::{PointerButton, PointerPhase};

use crate::primitives::{Revision, SemanticsHint};

/// Font size every tier-1 control paints its label with. Shared between
/// `paint` and `layout_request` so the intrinsic size always matches what is
/// drawn.
const LABEL_FONT_SIZE: f32 = 14.0;
/// Horizontal padding between a Button's edge and its label (each side).
const BUTTON_PADDING: f32 = 8.0;
/// Vertical padding around a Button's label (each side).
const BUTTON_VERTICAL_PADDING: f32 = 4.0;
/// Gap between a CheckBox/Radio/Switch indicator and its label.
const INDICATOR_LABEL_GAP: f32 = 6.0;
/// The indicator square's side (CheckBox box / Radio dot diameter).
const INDICATOR_SIDE: f32 = 16.0;
/// Switch track width; its height is half of this.
const SWITCH_TRACK_WIDTH: f32 = 40.0;
/// Minimum intrinsic width of a TextField, so an empty input (no value, no
/// placeholder) is still visible and clickable instead of collapsing to the
/// 8px padding.
const TEXTFIELD_MIN_WIDTH: f32 = 120.0;
/// Horizontal padding between a Select's edge and its label (each side);
/// matches the offset `draw_centered_text` applies.
const SELECT_PADDING: f32 = 8.0;
/// Slider thumb diameter (and therefore the slider's intrinsic height — the
/// thumb is the tallest thing it paints).
const SLIDER_THUMB: f32 = 16.0;
/// Default track length for a Slider that was given no explicit width.
const SLIDER_TRACK_WIDTH: f32 = 160.0;
/// Progress bar height: the bar paints its whole box, so its intrinsic size
/// is a convention rather than a measurement.
const PROGRESS_HEIGHT: f32 = 8.0;
/// Default Progress bar length when no explicit width is given.
const PROGRESS_WIDTH: f32 = 160.0;

/// Intrinsic width of a label at the controls' shared font size.
///
/// Empty labels measure to zero, which keeps "no label" controls auto-sized
/// instead of requesting a phantom extent.
fn label_width(label: &str) -> f32 {
    if label.is_empty() {
        0.0
    } else {
        floem_text::measure_text(label, LABEL_FONT_SIZE, f64::INFINITY).width as f32
    }
}

/// Line height at the controls' shared font size, measured from a reference
/// glyph so it is well-defined even for empty strings (an empty text field
/// still needs a row tall enough to type into).
fn line_height() -> f32 {
    floem_text::measure_text(" ", LABEL_FONT_SIZE, f64::INFINITY).height as f32
}

/// Intrinsic height of a label at the controls' shared font size.
fn label_height(label: &str) -> f32 {
    if label.is_empty() {
        0.0
    } else {
        floem_text::measure_text(label, LABEL_FONT_SIZE, f64::INFINITY).height as f32
    }
}

/// Shared hover/press/focus/disable state for interactive controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
}

impl InteractionState {
    /// The subset that reaches assistive technology.
    pub fn to_semantics(self) -> SemanticsState {
        let mut state = SemanticsState::empty();
        if self.disabled {
            state |= SemanticsState::DISABLED;
        } else {
            state |= SemanticsState::ENABLED;
            state |= SemanticsState::FOCUSABLE;
        }
        if self.focused {
            state |= SemanticsState::FOCUSED;
        }
        state
    }

    pub fn set_hovered(&mut self, hovered: bool) -> bool {
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }
}

// --- colour helpers -------------------------------------------------------

/// Linear blend from `a` to `b`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), ch(a.a, b.a))
}

/// Dims a colour toward transparency to render the disabled appearance.
fn disabled_tint(c: Color) -> Color {
    Color::rgba(c.r, c.g, c.b, c.a / 2)
}

/// Resolves the base colour through the interaction state.
fn resolved_color(base: Color, pressed: Color, hovered: Color, state: InteractionState) -> Color {
    if state.disabled {
        return disabled_tint(base);
    }
    if state.pressed {
        return pressed;
    }
    if state.hovered {
        return hovered;
    }
    base
}

/// Darker/lighter variants of a base control colour.
fn emphasis(c: Color) -> (Color, Color) {
    (mix(c, Color::BLACK, 0.18), mix(c, Color::WHITE, 0.18))
}

// --- shared pointer tracking ---------------------------------------------

/// Outcome of feeding a pointer event into a control.
///
/// `Consumed` and `Activate` both stop the event from bubbling; only
/// `Activate` triggers the control's action, and only on the release that
/// completes a press begun inside the control.
enum Press {
    Ignored,
    Consumed,
    Activate,
}

/// Applies a pointer event to `state`, reporting how far it got.
fn track_pointer(state: &mut InteractionState, cx: &mut dyn EventContext, id: ElementId) -> Press {
    let Event::Pointer(pointer) = cx.event() else {
        return Press::Ignored;
    };
    if state.disabled {
        return Press::Ignored;
    }
    match pointer.phase {
        PointerPhase::Enter | PointerPhase::Move => {
            state.hovered = true;
            Press::Consumed
        }
        PointerPhase::Leave | PointerPhase::Cancel => {
            state.hovered = false;
            state.pressed = false;
            Press::Consumed
        }
        PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
            state.hovered = true;
            state.pressed = true;
            cx.request_focus(id);
            Press::Consumed
        }
        PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
            let activate = state.pressed;
            state.pressed = false;
            if activate {
                Press::Activate
            } else {
                Press::Ignored
            }
        }
        _ => Press::Ignored,
    }
}

/// True when a key event should activate the focused control.
fn is_activation_key(key: &KeyEvent, state: InteractionState) -> bool {
    if state.disabled || !state.focused || key.phase == KeyPhase::Up {
        return false;
    }
    matches!(key.key, Key::Named(NamedKey::Enter) | Key::Char(' '))
}

/// Draws `text` left-aligned and vertically centred inside `bounds`.
fn draw_centered_text(
    cx: &mut dyn PaintContext,
    bounds: Rect,
    text: &str,
    size: f32,
    color: Color,
) {
    if text.is_empty() || size <= 0.0 {
        return;
    }
    let origin = Point::new(
        bounds.x0 + f64::from(BUTTON_PADDING),
        bounds.y0 + (bounds.height() - f64::from(size)) / 2.0,
    );
    cx.fill_text(origin, text, size, color);
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// A UI callback.
///
/// Deliberately *not* `Send + Sync`: input dispatch happens on one thread, and
/// keeping the bound off lets callbacks capture `Rc`/`Cell` handles. Widgets
/// that carry callbacks can still live inside composites — construct the
/// widget inside the builder closure (builders are replayable `Fn`s) rather
/// than capturing a pre-built one.
pub type Callback = Box<dyn Fn()>;

/// A clickable action surface.
pub struct Button {
    label: String,
    on_click: Option<Callback>,
    base: Color,
    text_color: Color,
    radius: f32,
    state: InteractionState,
    revision: Revision,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            base: Color::rgb(0x2B, 0x6C, 0xB0),
            text_color: Color::WHITE,
            radius: 4.0,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn color(mut self, base: Color) -> Self {
        self.base = base;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(SemanticsRole::Button, self.label.clone())
            .with_state(self.state.to_semantics())
    }
}

impl Widget for Button {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the label's measured extent plus the padding paint
    /// applies. The host treats this as default sizing — an explicit mount
    /// style still pins the button, while unpinned ones shrink-wrap.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        Some(
            floem_core::LayoutRequest::new()
                .width(label_width(&self.label) + BUTTON_PADDING * 2.0)
                .height(label_height(&self.label) + BUTTON_VERTICAL_PADDING * 2.0),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Activate => {
                if let Some(callback) = &self.on_click {
                    callback();
                }
                self.revision.bump();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event() {
                    if is_activation_key(&key, self.state) {
                        if let Some(callback) = &self.on_click {
                            callback();
                        }
                        self.revision.bump();
                        return true;
                    }
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let (darker, lighter) = emphasis(self.base);
        let fill = resolved_color(self.base, darker, lighter, self.state);
        if self.radius > 0.0 {
            cx.fill_rounded_rect(bounds, self.radius, fill);
        } else {
            cx.fill_rect(bounds, fill);
        }
        let color = if self.state.disabled {
            disabled_tint(self.text_color)
        } else {
            self.text_color
        };
        draw_centered_text(cx, bounds, &self.label, LABEL_FONT_SIZE, color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// TextField
// ---------------------------------------------------------------------------

pub type TextCallback = Box<dyn Fn(&str)>;

/// Single-line text input.
///
/// IME composition is deliberately not handled here — `floem-ime` owns that
/// state machine; wiring it to host text inputs is tracked separately.
pub struct TextField {
    value: String,
    placeholder: String,
    /// Caret position in **characters**, not bytes.
    caret: usize,
    on_change: Option<TextCallback>,
    on_submit: Option<TextCallback>,
    state: InteractionState,
    revision: Revision,
}

impl std::fmt::Debug for TextField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextField")
            .field("value", &self.value)
            .field("caret", &self.caret)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl TextField {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            caret: 0,
            on_change: None,
            on_submit: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.set_value(value.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn text(&self) -> &str {
        &self.value
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// Replaces the content, clamping the caret to a valid boundary.
    pub fn set_value(&mut self, value: impl Into<String>) -> &mut Self {
        self.value = value.into();
        let len = self.value.chars().count();
        self.caret = self.caret.min(len);
        self.revision.bump();
        self
    }

    fn byte_caret(&self) -> usize {
        self.value
            .char_indices()
            .nth(self.caret)
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }

    fn insert(&mut self, ch: char) {
        let at = self.byte_caret();
        self.value.insert(at, ch);
        self.caret += 1;
        self.revision.bump();
        if let Some(callback) = &self.on_change {
            callback(&self.value);
        }
    }

    fn erase_before(&mut self) {
        if self.caret == 0 || self.value.is_empty() {
            return;
        }
        let mut chars: Vec<char> = self.value.chars().collect();
        chars.remove(self.caret - 1);
        self.value = chars.into_iter().collect();
        self.caret -= 1;
        self.revision.bump();
        if let Some(callback) = &self.on_change {
            callback(&self.value);
        }
    }

    fn erase_after(&mut self) {
        let len = self.value.chars().count();
        if self.caret >= len {
            return;
        }
        let mut chars: Vec<char> = self.value.chars().collect();
        chars.remove(self.caret);
        self.value = chars.into_iter().collect();
        self.revision.bump();
        if let Some(callback) = &self.on_change {
            callback(&self.value);
        }
    }

    fn move_caret(&mut self, delta: isize) {
        let len = self.value.chars().count() as isize;
        let next = (self.caret as isize + delta).clamp(0, len);
        if next as usize != self.caret {
            self.caret = next as usize;
            self.revision.bump();
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut hint = SemanticsHint::labeled(SemanticsRole::TextField, self.value.clone())
            .with_state(self.state.to_semantics());
        if self.value.is_empty() {
            hint.label = Some(self.placeholder.clone());
        }
        hint
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextField {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the shown text (value, else placeholder) plus the
    /// padding paint applies, floored at [`TEXTFIELD_MIN_WIDTH`] so an empty
    /// input stays usable. Height rests on the line height even when both
    /// value and placeholder are empty.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        let shown = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        Some(
            floem_core::LayoutRequest::new()
                .width((label_width(shown) + BUTTON_PADDING * 2.0).max(TEXTFIELD_MIN_WIDTH))
                .height(line_height() + BUTTON_VERTICAL_PADDING * 2.0),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Ignored => {}
            _ => {
                self.revision.bump();
                return true;
            }
        }
        let Event::Key(key) = cx.event() else {
            return false;
        };
        if self.state.disabled || !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        match key.key {
            Key::Char(ch) if key.is_text_input() => self.insert(ch),
            Key::Named(NamedKey::Backspace) => self.erase_before(),
            Key::Named(NamedKey::Delete) => self.erase_after(),
            Key::Named(NamedKey::ArrowLeft) => self.move_caret(-1),
            Key::Named(NamedKey::ArrowRight) => self.move_caret(1),
            Key::Named(NamedKey::Home) => self.move_caret(-(self.value.chars().count() as isize)),
            Key::Named(NamedKey::End) => self.move_caret(self.value.chars().count() as isize),
            Key::Named(NamedKey::Enter) => {
                if let Some(callback) = &self.on_submit {
                    callback(&self.value);
                }
                self.revision.bump();
            }
            _ => return false,
        }
        true
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let border = if self.state.focused {
            self.state.hovered
        } else {
            false
        };
        let _ = border;
        let fill = resolved_color(
            Color::WHITE,
            Color::WHITE,
            Color::rgb(0xF5, 0xF7, 0xFA),
            self.state,
        );
        cx.fill_rect(bounds, fill);
        let shown = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        let color = if self.value.is_empty() {
            Color::rgb(0x9A, 0xA5, 0xB1)
        } else {
            Color::BLACK
        };
        draw_centered_text(cx, bounds, shown, 14.0, color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Checkbox / Radio / Switch
// ---------------------------------------------------------------------------

pub type BoolCallback = Box<dyn Fn(bool)>;

/// A tri-state-free boolean checkbox.
pub struct Checkbox {
    label: String,
    checked: bool,
    on_toggle: Option<BoolCallback>,
    state: InteractionState,
    revision: Revision,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            on_toggle: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// Flips the value and notifies the change callback.
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
        self.revision.bump();
        if let Some(callback) = &self.on_toggle {
            callback(self.checked);
        }
    }

    /// The box glyph area (a square inset from the left edge).
    pub fn box_bounds(&self, bounds: Rect) -> Rect {
        let side = bounds.height().min(16.0);
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - side) / 2.0,
            x1: bounds.x0 + side,
            y1: bounds.y0 + (bounds.height() + side) / 2.0,
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.checked {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::CheckBox, self.label.clone()).with_state(state)
    }
}

impl std::fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .finish_non_exhaustive()
    }
}

impl Widget for Checkbox {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: indicator square + gap + label extent. The requested
    /// height keeps the box at its full 16px side (paint shrinks it to fit
    /// shorter bounds).
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        Some(
            floem_core::LayoutRequest::new()
                .width(INDICATOR_SIDE + INDICATOR_LABEL_GAP + label_width(&self.label))
                .height(INDICATOR_SIDE.max(label_height(&self.label))),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Activate => {
                self.toggle();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event() {
                    if is_activation_key(&key, self.state) {
                        self.toggle();
                        return true;
                    }
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let box_rect = self.box_bounds(bounds);
        let accent = Color::rgb(0x2B, 0x6C, 0xB0);
        let fill = if self.checked {
            resolved_color(accent, mix(accent, Color::BLACK, 0.18), accent, self.state)
        } else {
            resolved_color(
                Color::WHITE,
                Color::rgb(0xE2, 0xE8, 0xF0),
                Color::rgb(0xF5, 0xF7, 0xFA),
                self.state,
            )
        };
        cx.fill_rect(box_rect, fill);
        if self.checked {
            let mut buf = [0u8; 4];
            let check = '\u{2714}'.encode_utf8(&mut buf);
            let origin = Point::new(box_rect.x0 + 2.0, box_rect.y0 + 1.0);
            cx.fill_text(origin, check, box_rect.height() as f32, Color::WHITE);
        }
        let label_origin = Point::new(box_rect.x1 + 6.0, bounds.y0);
        cx.fill_text(label_origin, &self.label, LABEL_FONT_SIZE, Color::BLACK);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

/// One radio option. Grouping (mutual exclusion) is owned by the caller.
pub struct Radio {
    label: String,
    selected: bool,
    on_select: Option<Callback>,
    state: InteractionState,
    revision: Revision,
}

impl Radio {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            on_select: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_select(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_select = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    pub fn select(&mut self) {
        if self.selected {
            return;
        }
        self.selected = true;
        self.revision.bump();
        if let Some(callback) = &self.on_select {
            callback();
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.selected {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::Radio, self.label.clone()).with_state(state)
    }
}

impl std::fmt::Debug for Radio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Radio")
            .field("label", &self.label)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl Widget for Radio {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: indicator circle + gap + label extent (same geometry
    /// as [`Checkbox`]).
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        Some(
            floem_core::LayoutRequest::new()
                .width(INDICATOR_SIDE + INDICATOR_LABEL_GAP + label_width(&self.label))
                .height(INDICATOR_SIDE.max(label_height(&self.label))),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Activate => {
                self.select();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event() {
                    if is_activation_key(&key, self.state) {
                        self.select();
                        return true;
                    }
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let outer = self.dot_bounds(bounds);
        if self.selected {
            let inner = Rect {
                x0: outer.x0 + outer.width() * 0.25,
                y0: outer.y0 + outer.height() * 0.25,
                x1: outer.x1 - outer.width() * 0.25,
                y1: outer.y1 - outer.height() * 0.25,
            };
            cx.fill_rounded_rect(
                outer,
                outer.width() as f32 / 2.0,
                Color::rgb(0x9A, 0xA5, 0xB1),
            );
            cx.fill_rounded_rect(
                inner,
                inner.width() as f32 / 2.0,
                Color::rgb(0x2B, 0x6C, 0xB0),
            );
        } else {
            cx.fill_rounded_rect(
                outer,
                outer.width() as f32 / 2.0,
                resolved_color(
                    Color::WHITE,
                    Color::rgb(0xE2, 0xE8, 0xF0),
                    Color::rgb(0xF5, 0xF7, 0xFA),
                    self.state,
                ),
            );
        }
        let label_origin = Point::new(outer.x1 + 6.0, bounds.y0);
        cx.fill_text(label_origin, &self.label, LABEL_FONT_SIZE, Color::BLACK);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl Radio {
    /// The circular indicator bounds.
    pub fn dot_bounds(&self, bounds: Rect) -> Rect {
        let diameter = bounds.height().min(16.0);
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - diameter) / 2.0,
            x1: bounds.x0 + diameter,
            y1: bounds.y0 + (bounds.height() + diameter) / 2.0,
        }
    }
}

/// An on/off switch.
pub struct Switch {
    label: String,
    on: bool,
    on_toggle: Option<BoolCallback>,
    track_width: f64,
    state: InteractionState,
    revision: Revision,
}

impl Switch {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on: false,
            on_toggle: None,
            track_width: 40.0,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
        self.revision.bump();
        if let Some(callback) = &self.on_toggle {
            callback(self.on);
        }
    }

    /// The track rectangle, laid flush to the left edge.
    pub fn track_bounds(&self, bounds: Rect) -> Rect {
        let width = self.track_width.min(bounds.width());
        let height = (width * 0.5).min(bounds.height());
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - height) / 2.0,
            x1: bounds.x0 + width,
            y1: bounds.y0 + (bounds.height() + height) / 2.0,
        }
    }

    /// The knob rectangle for the current state.
    pub fn knob_bounds(&self, bounds: Rect) -> Rect {
        let track = self.track_bounds(bounds);
        let diameter = track.height();
        let x = if self.on {
            track.x1 - diameter
        } else {
            track.x0
        };
        Rect {
            x0: x,
            y0: track.y0,
            x1: x + diameter,
            y1: track.y1,
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.on {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::Switch, self.label.clone()).with_state(state)
    }
}

impl std::fmt::Debug for Switch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switch")
            .field("label", &self.label)
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

impl Widget for Switch {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: track (width × width/2) + gap + label extent; the
    /// height also fits the label so the track never clips it.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        let track_w = if self.track_width > 0.0 {
            self.track_width as f32
        } else {
            SWITCH_TRACK_WIDTH
        };
        let track_h = track_w * 0.5;
        Some(
            floem_core::LayoutRequest::new()
                .width(track_w + INDICATOR_LABEL_GAP + label_width(&self.label))
                .height(track_h.max(label_height(&self.label))),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Activate => {
                self.toggle();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event() {
                    if is_activation_key(&key, self.state) {
                        self.toggle();
                        return true;
                    }
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let track = self.track_bounds(bounds);
        let knob = self.knob_bounds(bounds);
        let on_color = Color::rgb(0x38, 0xA1, 0x69);
        let track_color = if self.on {
            resolved_color(
                on_color,
                mix(on_color, Color::BLACK, 0.18),
                on_color,
                self.state,
            )
        } else {
            resolved_color(
                Color::rgb(0xCB, 0xD5, 0xE0),
                Color::rgb(0xA0, 0xAE, 0xC0),
                Color::rgb(0xB8, 0xC2, 0xCC),
                self.state,
            )
        };
        cx.fill_rounded_rect(track, track.height() as f32 / 2.0, track_color);
        cx.fill_rounded_rect(knob, knob.height() as f32 / 2.0, Color::WHITE);
        let label_origin = Point::new(track.x1 + 6.0, bounds.y0);
        cx.fill_text(label_origin, &self.label, LABEL_FONT_SIZE, Color::BLACK);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Slider / Progress / Select
// ---------------------------------------------------------------------------

pub type ValueCallback = Box<dyn Fn(f64)>;

/// A continuous or stepped value selector along the horizontal axis.
pub struct Slider {
    value: f64,
    min: f64,
    max: f64,
    step: Option<f64>,
    on_change: Option<ValueCallback>,
    state: InteractionState,
    revision: Revision,
}

impl Slider {
    pub fn new(value: f64, min: f64, max: f64) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
            step: None,
            on_change: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f64) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    fn quantize(&self, raw: f64) -> f64 {
        let clamped = raw.clamp(self.min, self.max);
        match self.step {
            Some(step) if step > 0.0 => {
                let snapped = ((clamped - self.min) / step).round() * step + self.min;
                snapped.clamp(self.min, self.max)
            }
            _ => clamped,
        }
    }

    /// Maps an x coordinate to a (quantized) value.
    pub fn value_at(&self, bounds: Rect, x: f64) -> f64 {
        if bounds.width() <= 0.0 {
            return self.value;
        }
        let t = ((x - bounds.x0) / bounds.width()).clamp(0.0, 1.0);
        self.quantize(self.min + (self.max - self.min) * t)
    }

    fn set_value(&mut self, value: f64) {
        let next = self.quantize(value);
        if next == self.value {
            return;
        }
        self.value = next;
        self.revision.bump();
        if let Some(callback) = &self.on_change {
            callback(self.value);
        }
    }

    /// Normalized position of the thumb within the track (0..1).
    pub fn fraction(&self) -> f64 {
        let span = self.max - self.min;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.value - self.min) / span).clamp(0.0, 1.0)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(SemanticsRole::Slider, format!("{:.0}", self.value))
            .with_state(self.state.to_semantics())
    }
}

impl std::fmt::Debug for Slider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .finish_non_exhaustive()
    }
}

impl Widget for Slider {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the thumb's diameter tall (it is the tallest thing the
    /// slider paints) and a default track length wide — a track is
    /// meaningless at zero width, and callers who want it to fill their
    /// container pin the width explicitly.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        Some(
            floem_core::LayoutRequest::new()
                .width(SLIDER_TRACK_WIDTH)
                .height(SLIDER_THUMB),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        if self.state.disabled {
            return false;
        }
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                self.state.hovered = true;
                // Drag while held: keep following the pointer.
                if self.state.pressed {
                    let next = self.value_at(cx.bounds(), pointer.position.x);
                    self.set_value(next);
                    return true;
                }
                false
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.state.hovered = false;
                self.state.pressed = false;
                false
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.state.hovered = true;
                self.state.pressed = true;
                cx.request_focus(id);
                let next = self.value_at(cx.bounds(), pointer.position.x);
                self.set_value(next);
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                let was_pressed = self.state.pressed;
                self.state.pressed = false;
                was_pressed
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let track = self.track_rect(bounds);
        cx.fill_rect(
            track,
            resolved_color(
                Color::rgb(0xE2, 0xE8, 0xF0),
                Color::rgb(0xE2, 0xE8, 0xF0),
                Color::rgb(0xED, 0xF2, 0xF7),
                self.state,
            ),
        );
        let filled = Rect {
            x0: track.x0,
            y0: track.y0,
            x1: track.x0 + track.width() * self.fraction(),
            y1: track.y1,
        };
        cx.fill_rect(
            filled,
            resolved_color(
                Color::rgb(0x2B, 0x6C, 0xB0),
                Color::rgb(0x20, 0x53, 0x85),
                Color::rgb(0x36, 0x7F, 0xC9),
                self.state,
            ),
        );
        let thumb = self.thumb_rect(bounds);
        cx.fill_rounded_rect(thumb, thumb.width() as f32 / 2.0, Color::WHITE);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl Slider {
    fn track_rect(&self, bounds: Rect) -> Rect {
        let height = 4.0_f64.min(bounds.height());
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - height) / 2.0,
            x1: bounds.x1,
            y1: bounds.y0 + (bounds.height() + height) / 2.0,
        }
    }

    fn thumb_rect(&self, bounds: Rect) -> Rect {
        let diameter = bounds.height().min(16.0);
        let usable = bounds.width() - diameter;
        let x = bounds.x0 + usable * self.fraction();
        Rect {
            x0: x,
            y0: bounds.y0 + (bounds.height() - diameter) / 2.0,
            x1: x + diameter,
            y1: bounds.y0 + (bounds.height() + diameter) / 2.0,
        }
    }
}

/// Determinate or indeterminate progress indicator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    /// `None` renders the indeterminate appearance.
    value: Option<f64>,
    disabled: bool,
    revision: Revision,
}

impl Progress {
    pub fn determinate(value: f64) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            disabled: false,
            revision: Revision::default(),
        }
    }

    pub fn indeterminate() -> Self {
        Self {
            value: None,
            disabled: false,
            revision: Revision::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn value(&self) -> Option<f64> {
        self.value
    }

    pub fn set_value(&mut self, value: Option<f64>) -> &mut Self {
        self.value = value.map(|v| v.clamp(0.0, 1.0));
        self.revision.bump();
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let label = match self.value {
            Some(v) => format!("{}%", (v * 100.0).round() as i32),
            None => "busy".to_string(),
        };
        let mut hint = SemanticsHint::labeled(SemanticsRole::ProgressBar, label);
        if self.disabled {
            hint = hint.with_state(SemanticsState::DISABLED);
        }
        hint
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::indeterminate()
    }
}

impl Widget for Progress {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the bar fills its box, so its extent is a convention —
    /// a thin bar of default length. A pinned style still wins.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        Some(
            floem_core::LayoutRequest::new()
                .width(PROGRESS_WIDTH)
                .height(PROGRESS_HEIGHT),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let base = if self.disabled {
            disabled_tint(Color::rgb(0x2B, 0x6C, 0xB0))
        } else {
            Color::rgb(0x2B, 0x6C, 0xB0)
        };
        match self.value {
            Some(value) => {
                cx.fill_rect(bounds, Color::rgb(0xE2, 0xE8, 0xF0));
                let filled = Rect {
                    x1: bounds.x0 + bounds.width() * value.clamp(0.0, 1.0),
                    ..bounds
                };
                cx.fill_rect(filled, base);
            }
            None => {
                // Indeterminate: a full-width dim bar; animation comes later
                // with `floem-animation`.
                cx.fill_rect(bounds, Color::rgb(0xE2, 0xE8, 0xF0));
                cx.fill_rect(bounds, disabled_tint(base));
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

pub type IndexCallback = Box<dyn Fn(usize)>;

/// A single-choice selector.
///
/// The option list currently paints **inside** the control's own bounds — real
/// popup layering (escaping the parent's clip) needs the `floem-overlay`
/// integration, which is a later task.
pub struct Select {
    options: Vec<String>,
    selected: Option<usize>,
    open: bool,
    row_height: f64,
    on_change: Option<IndexCallback>,
    state: InteractionState,
    revision: Revision,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("open", &self.open)
            .finish_non_exhaustive()
    }
}

impl Select {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            open: false,
            row_height: 22.0,
            on_change: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn options(&self) -> &[String] {
        &self.options
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn selected_label(&self) -> &str {
        match self.selected.and_then(|i| self.options.get(i)) {
            Some(label) => label,
            None => "—",
        }
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// Chooses an option; no-op when out of range.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        self.open = false;
        if changed {
            self.revision.bump();
            if let Some(callback) = &self.on_change {
                callback(index);
            }
        }
        changed
    }

    /// Index of the row at `y`, if the list is open and the point is inside it.
    pub fn row_at(&self, bounds: Rect, y: f64) -> Option<usize> {
        if !self.open {
            return None;
        }
        if y < bounds.y0 + self.row_height {
            return None;
        }
        let index = ((y - bounds.y0 - self.row_height) / self.row_height) as usize;
        self.options.get(index).map(|_| index)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.open {
            state |= SemanticsState::EXPANDED;
        }
        SemanticsHint::labeled(SemanticsRole::List, self.selected_label().to_string())
            .with_state(state)
    }
}

impl Widget for Select {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size of the *closed* control: the widest label (selected or
    /// option) plus the paint padding, one [`Select::row_height`] tall. The
    /// open option list paints below the bounds and deliberately does not
    /// contribute to the intrinsic extent.
    fn layout_request(&self) -> Option<floem_core::LayoutRequest> {
        let widest = self
            .options
            .iter()
            .map(|option| label_width(option))
            .chain(std::iter::once(label_width(self.selected_label())))
            .fold(0.0_f32, f32::max);
        Some(
            floem_core::LayoutRequest::new()
                .width(widest + SELECT_PADDING * 2.0)
                .height(self.row_height as f32),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        if self.state.disabled {
            return false;
        }
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                self.state.hovered = true;
                false
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.state.hovered = false;
                false
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.state.hovered = true;
                cx.request_focus(id);
                false
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                if let Some(index) = self.row_at(cx.bounds(), pointer.position.y) {
                    self.select(index);
                    return true;
                }
                let was_open = self.open;
                self.open = !self.open;
                self.revision.bump();
                let _ = was_open;
                true
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let (darker, lighter) = emphasis(Color::WHITE);
        cx.fill_rect(
            bounds,
            resolved_color(Color::WHITE, darker, lighter, self.state),
        );
        draw_centered_text(cx, bounds, self.selected_label(), 14.0, Color::BLACK);

        if self.open {
            let list_top = bounds.y0 + self.row_height;
            cx.fill_rect(
                Rect {
                    x0: bounds.x0,
                    y0: list_top,
                    x1: bounds.x1,
                    y1: list_top + self.row_height * self.options.len() as f64,
                },
                Color::rgb(0xED, 0xF2, 0xF7),
            );
            for (index, option) in self.options.iter().enumerate() {
                let row = Rect {
                    x0: bounds.x0,
                    y0: list_top + index as f64 * self.row_height,
                    x1: bounds.x1,
                    y1: list_top + (index + 1) as f64 * self.row_height,
                };
                let selected_row = self.selected == Some(index);
                if selected_row {
                    cx.fill_rect(row, Color::rgb(0xD6, 0xE4, 0xF0));
                }
                let origin = Point::new(row.x0 + 8.0, row.y0 + 3.0);
                cx.fill_text(origin, option, 14.0, Color::BLACK);
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}
