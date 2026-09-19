//! Tier-1 interactive controls.
//!
//! Every control carries an [`InteractionState`] (hover / press / focus /
//! disable) and reports it through [`SemanticsHint::state`], so assistive
//! technology sees the same state the control paints. Mutation goes through
//! builders that bump the internal revision, keeping a single invalidation
//! model across all widgets.

use floem_accessibility::{SemanticsRole, SemanticsState};
use floem_core::{
    AnimationSpec, Color, CreateContext, Easing, ElementId, Event, EventContext, Key, KeyEvent,
    KeyPhase, NamedKey, OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point, Rect,
    RepeatSpec, SemanticsContext, Size, UpdateContext, Widget, WidgetAny,
};
use floem_ime::{CompositionState, ImeEvent, ImeState};
use floem_types::{ImeUpdate, PointerButton, PointerPhase};

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
/// Panel behind an open Select's option list.
const SELECT_LIST_BACKGROUND: Color = Color::rgb(0xED, 0xF2, 0xF7);
/// Band drawn behind the row a Select popup has its highlight on.
const SELECT_ROW_HIGHLIGHT: Color = Color::rgb(0xD6, 0xE4, 0xF0);

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
/// IME composition is owned here via [`ImeState`]: the host routes
/// [`Event::Ime`] updates to the focused field, which feeds them to that
/// state machine and renders the preedit inline. Committed text arrives
/// through the ordinary text-input key path (the runner turns an OS
/// `ImeCommit` into one character key-down per scalar), so the preedit must
/// be cleared the moment a composition finalizes.
pub struct TextField {
    value: String,
    placeholder: String,
    /// Caret position in **characters**, not bytes.
    caret: usize,
    on_change: Option<TextCallback>,
    on_submit: Option<TextCallback>,
    state: InteractionState,
    revision: Revision,
    /// Live IME composition state (`floem-ime`). Empty / `Inactive` when no
    /// composition is in flight.
    ime: ImeState,
    /// Last composition caret rectangle reported to the host, in logical
    /// window pixels; `None` when nothing is composing.
    ime_caret: Option<Rect>,
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
            ime: ImeState::new(),
            ime_caret: None,
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

    /// The in-progress composition text, empty when nothing is composing.
    pub fn preedit(&self) -> &str {
        &self.ime.preedit
    }

    /// Whether an IME composition is currently open.
    pub fn is_composing(&self) -> bool {
        self.ime.is_composing()
    }

    /// The composition caret rectangle last reported to the host, in logical
    /// window pixels (where the OS input-method popup should anchor).
    pub fn ime_caret_rect(&self) -> Option<Rect> {
        self.ime_caret
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

    /// On-screen rectangle of the composition caret (where the OS IME popup
    /// should anchor), in logical window pixels. Includes the preedit width
    /// when composing, so the popup sits just past the last candidate glyph.
    fn caret_rect(&self, bounds: Rect) -> Rect {
        let prefix = &self.value[..self.byte_caret()];
        let mut width = floem_text::measure_text(prefix, LABEL_FONT_SIZE, f64::INFINITY).width;
        if self.ime.is_composing() {
            width +=
                floem_text::measure_text(&self.ime.preedit, LABEL_FONT_SIZE, f64::INFINITY).width;
        }
        let x = bounds.x0 + f64::from(BUTTON_PADDING) + width;
        let top = bounds.y0 + (bounds.height() - f64::from(LABEL_FONT_SIZE)) / 2.0;
        Rect::from_origin_size(
            Point::new(x, top),
            Size::new(1.0, f64::from(LABEL_FONT_SIZE)),
        )
    }

    /// Keeps the IME state machine's committed-buffer caret in sync with the
    /// field's caret, and clears any lingering composition so the next insert
    /// starts clean. Called after every value/caret mutation.
    fn sync_ime_caret(&mut self) {
        self.ime.caret = self.byte_caret();
        self.ime.composition = CompositionState::Inactive;
        self.ime.preedit.clear();
        self.ime.preedit_caret = 0;
        self.ime.preedit_selection = None;
    }

    /// Applies an IME update from the host, handing it to the state machine.
    ///
    /// While a composition is open the caret rectangle is reported back to the
    /// host so the runner can anchor the OS popup; once it ends the area is
    /// cleared.
    fn apply_ime(&mut self, update: ImeUpdate, cx: &mut dyn EventContext) -> bool {
        let event = match update {
            ImeUpdate::OpenChanged(open) => ImeEvent::OpenChanged(open),
            ImeUpdate::Preedit {
                text,
                caret,
                selection,
            } => ImeEvent::Preedit {
                text,
                caret,
                selection,
            },
        };
        let result = self.ime.apply(event);
        if result.is_noop() {
            // Reached the field and matched its current state: still handled.
            return true;
        }
        self.revision.bump();
        if self.ime.is_composing() {
            let rect = self.caret_rect(cx.bounds());
            self.ime_caret = Some(rect);
            cx.set_ime_cursor_area(Some(rect));
        } else {
            self.ime_caret = None;
            cx.set_ime_cursor_area(None);
        }
        true
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
        // IME composition updates are delivered to the focused field; they are
        // not keyboard events and must be handled before the key path.
        if let Event::Ime(update) = cx.event() {
            return self.apply_ime(update, cx);
        }
        let Event::Key(key) = cx.event() else {
            return false;
        };
        if self.state.disabled || !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        match key.key {
            Key::Char(ch) if key.is_text_input() => {
                // The OS finalized a composition as simulated keys; drop the
                // stale preedit first so the committed text replaces it.
                if self.ime.is_composing() {
                    self.ime.cancel();
                }
                self.insert(ch);
                self.sync_ime_caret();
            }
            Key::Named(NamedKey::Backspace) => {
                self.erase_before();
                self.sync_ime_caret();
            }
            Key::Named(NamedKey::Delete) => {
                self.erase_after();
                self.sync_ime_caret();
            }
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
        let fill = resolved_color(
            Color::WHITE,
            Color::WHITE,
            Color::rgb(0xF5, 0xF7, 0xFA),
            self.state,
        );
        cx.fill_rect(bounds, fill);

        let pad = f64::from(BUTTON_PADDING);
        let size = f64::from(LABEL_FONT_SIZE);
        let origin_y = bounds.y0 + (bounds.height() - size) / 2.0;

        // Empty and not composing: show the placeholder, dimmed.
        if self.value.is_empty() && !self.ime.is_composing() {
            if !self.placeholder.is_empty() {
                let color = Color::rgb(0x9A, 0xA5, 0xB1);
                cx.fill_text(
                    Point::new(bounds.x0 + pad, origin_y),
                    &self.placeholder,
                    LABEL_FONT_SIZE,
                    color,
                );
            }
            return;
        }

        // Draw the committed text split at the caret, with the in-progress
        // composition inserted at the caret and underlined as provisional.
        let left = &self.value[..self.byte_caret()];
        let right = &self.value[self.byte_caret()..];
        let preedit = if self.ime.is_composing() {
            self.ime.preedit.clone()
        } else {
            String::new()
        };

        let committed = Color::BLACK;
        let mut x = bounds.x0 + pad;
        if !left.is_empty() {
            cx.fill_text(Point::new(x, origin_y), left, LABEL_FONT_SIZE, committed);
            x += floem_text::measure_text(left, LABEL_FONT_SIZE, f64::INFINITY).width;
        }
        if !preedit.is_empty() {
            let accent = Color::rgb(0x2B, 0x6C, 0xB0);
            cx.fill_text(Point::new(x, origin_y), &preedit, LABEL_FONT_SIZE, accent);
            let width = floem_text::measure_text(&preedit, LABEL_FONT_SIZE, f64::INFINITY).width;
            // Underline the composition so it reads as provisional text.
            let underline =
                Rect::from_origin_size(Point::new(x, origin_y + size - 1.0), Size::new(width, 1.0));
            cx.fill_rect(underline, accent);
            x += width;
        }
        if !right.is_empty() {
            cx.fill_text(Point::new(x, origin_y), right, LABEL_FONT_SIZE, committed);
        }
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

    /// How far one arrow key moves the value: the configured step, or a
    /// hundredth of the range on a continuous slider (which has no step to
    /// quantize to, but still needs a sensible key granularity).
    fn key_step(&self) -> f64 {
        match self.step {
            Some(step) if step > 0.0 => step,
            _ => (self.max - self.min) / 100.0,
        }
    }

    /// Keyboard control for the focused slider — the pointer's equivalent for
    /// anyone not using one, and the only way in for assistive technology.
    fn key_event(&mut self, key: &KeyEvent) -> bool {
        if !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        let step = self.key_step();
        match key.key {
            Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowUp) => {
                self.set_value(self.value + step)
            }
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowDown) => {
                self.set_value(self.value - step)
            }
            // A page is ten arrow steps: enough to cross a long range quickly
            // without losing the fine control the arrows give.
            Key::Named(NamedKey::PageUp) => self.set_value(self.value + step * 10.0),
            Key::Named(NamedKey::PageDown) => self.set_value(self.value - step * 10.0),
            Key::Named(NamedKey::Home) => self.set_value(self.min),
            Key::Named(NamedKey::End) => self.set_value(self.max),
            _ => return false,
        }
        // Consumed even when the value did not move (already at an end):
        // swallowing the key is what keeps it from scrolling the page behind.
        true
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
        if self.state.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) => self.key_event(&key),
                _ => false,
            };
        };
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

/// How much of an indeterminate bar's width the moving segment covers.
const INDETERMINATE_SEGMENT: f64 = 0.3;

/// One end-to-end pass of the indeterminate shuttle.
///
/// A convention rather than a spec value: the bar is decoration whose only job
/// is to say "still working", and this is long enough to read as motion and
/// short enough to stay out of the way.
const INDETERMINATE_CYCLE_SECS: f64 = 1.2;

/// The sweep an indeterminate bar runs on until it is given a value.
///
/// A shuttle — out and back — rather than a one-way sweep that restarts. A
/// restart jumps the segment from the far edge back to the near one every
/// cycle, which reads as a glitch rather than as progress; easing the ends
/// keeps the turnarounds from looking like a bounce.
const INDETERMINATE_SWEEP: AnimationSpec = AnimationSpec::repeating(
    0.0,
    1.0,
    INDETERMINATE_CYCLE_SECS,
    RepeatSpec::INFINITE_REVERSE,
)
.with_easing(Easing::EASE_IN_OUT);

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

    /// An indeterminate bar keeps a shuttle running on itself; a determinate
    /// one declares nothing, because a bar that has a value is not animating.
    ///
    /// The declaration is what makes the bar move: this widget cannot start
    /// anything itself (it holds no host handle), so it states the motion it
    /// wants and the host owns running it.
    fn animation(&self) -> Option<AnimationSpec> {
        if self.value.is_some() {
            return None;
        }
        Some(INDETERMINATE_SWEEP)
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
                // Indeterminate: a shuttle over a dim track. The phase is the
                // value the host sampled for this element, so this widget draws
                // the motion itself rather than having the host move the whole
                // bar for it. With nothing animating — a host that has no
                // animation, or a frame drawn before the first tick — the
                // segment parks at the start rather than vanishing, because
                // "not moving yet" is not the same as "nothing here".
                cx.fill_rect(bounds, Color::rgb(0xE2, 0xE8, 0xF0));
                let phase = cx.animation_value().unwrap_or(0.0).clamp(0.0, 1.0);
                let segment = (bounds.width() * INDETERMINATE_SEGMENT).min(bounds.width());
                let travel = (bounds.width() - segment).max(0.0);
                let x0 = bounds.x0 + travel * phase;
                cx.fill_rect(
                    Rect {
                        x0,
                        x1: x0 + segment,
                        ..bounds
                    },
                    base,
                );
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

pub type IndexCallback = Box<dyn Fn(usize)>;

/// The open option list, mounted as an overlay rather than painted under the
/// Select's own bounds.
///
/// The inline version could never be clicked. A click is routed by hit-testing
/// the tree, and the rows were raw paint ops inside the Select's own node —
/// which is one row tall — so every point below the header landed somewhere
/// else entirely. Layering it as an overlay fixes both halves at once: the
/// rows become real nodes that hit-test, and the overlay paints after the main
/// tree so it escapes its parent's clip instead of being cut off by it.
///
/// The popup owns the highlight while it is up — the arrow keys belong to it,
/// not to the Select, because it is what the host gave focus to — and reports
/// the row the user picked back to the opening Select through
/// [`Select::overlay_closed`]. `None` means it was dismissed without choosing.
pub struct SelectPopup {
    options: Vec<String>,
    /// Row the pointer is over or the arrow keys last moved to.
    highlight: usize,
    row_height: f64,
    /// Set once the user picks a row; `None` means dismissed.
    chosen: Option<usize>,
    /// Handed in by the content factory so the popup can take itself down
    /// without reaching back into the Select that opened it.
    handle: Option<OverlayHandle>,
}

impl std::fmt::Debug for SelectPopup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectPopup")
            .field("options", &self.options)
            .field("highlight", &self.highlight)
            .field("chosen", &self.chosen)
            .finish_non_exhaustive()
    }
}

impl SelectPopup {
    /// Builds the list, highlighting `selected` so the keyboard continues from
    /// where the Select left off.
    pub fn new(
        options: Vec<String>,
        selected: Option<usize>,
        row_height: f64,
        handle: OverlayHandle,
    ) -> Self {
        Self {
            highlight: selected.unwrap_or(0),
            options,
            row_height,
            chosen: None,
            handle: Some(handle),
        }
    }

    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The row the user picked, if they picked one rather than dismissing.
    pub fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    pub fn highlight(&self) -> usize {
        self.highlight
    }

    /// Row whose band contains `y` within `bounds`, if any.
    pub fn row_at(&self, bounds: Rect, y: f64) -> Option<usize> {
        if self.options.is_empty() || self.row_height <= 0.0 {
            return None;
        }
        let offset = y - bounds.y0;
        if offset < 0.0 {
            return None;
        }
        let index = (offset / self.row_height) as usize;
        (index < self.options.len()).then_some(index)
    }

    fn commit(&mut self, cx: &mut dyn EventContext, chosen: Option<usize>) {
        self.chosen = chosen;
        if let Some(handle) = self.handle.take() {
            cx.close_overlay(handle);
        }
    }

    fn move_highlight(&mut self, to: usize) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let next = to.min(self.options.len() - 1);
        let moved = next != self.highlight;
        self.highlight = next;
        moved
    }

    /// The list describes itself by the row under the highlight, which is what
    /// a screen reader should read out as the user walks it.
    pub fn semantics_hint(&self) -> SemanticsHint {
        let label = self
            .options
            .get(self.highlight)
            .cloned()
            .unwrap_or_default();
        SemanticsHint::labeled(SemanticsRole::List, label)
            .with_state(SemanticsState::EXPANDED | SemanticsState::FOCUSED)
    }
}

impl Widget for SelectPopup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => match pointer.phase {
                PointerPhase::Move | PointerPhase::Enter => {
                    // Hovering is not a change worth reporting: the highlight
                    // follows the pointer, and the host repaints the frame
                    // either way.
                    if let Some(index) = self.row_at(cx.bounds(), pointer.position.y) {
                        self.highlight = index;
                    }
                    false
                }
                PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => true,
                PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                    let row = self.row_at(cx.bounds(), pointer.position.y);
                    let picked = row.or(Some(self.highlight));
                    self.commit(cx, picked);
                    true
                }
                _ => false,
            },
            Event::Key(key) => {
                if key.phase == KeyPhase::Up {
                    return false;
                }
                match key.key {
                    Key::Named(NamedKey::ArrowDown) => {
                        let next = self.highlight + 1;
                        self.move_highlight(next);
                        true
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        let next = self.highlight.saturating_sub(1);
                        self.move_highlight(next);
                        true
                    }
                    Key::Named(NamedKey::Home) => {
                        self.move_highlight(0);
                        true
                    }
                    Key::Named(NamedKey::End) => {
                        let last = self.options.len().saturating_sub(1);
                        self.move_highlight(last);
                        true
                    }
                    Key::Named(NamedKey::Enter) | Key::Char(' ') => {
                        let picked = Some(self.highlight);
                        self.commit(cx, picked);
                        true
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.commit(cx, None);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, SELECT_LIST_BACKGROUND);
        for (index, option) in self.options.iter().enumerate() {
            let row = Rect {
                x0: bounds.x0,
                y0: bounds.y0 + index as f64 * self.row_height,
                x1: bounds.x1,
                y1: bounds.y0 + (index + 1) as f64 * self.row_height,
            };
            if index == self.highlight {
                cx.fill_rect(row, SELECT_ROW_HIGHLIGHT);
            }
            cx.fill_text(
                Point::new(row.x0 + f64::from(SELECT_PADDING), row.y0 + 3.0),
                option,
                LABEL_FONT_SIZE,
                Color::BLACK,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

/// A single-choice selector.
///
/// Choosing opens [`SelectPopup`] as an overlay, so the list escapes the
/// control's own box instead of being painted inside it — see that type for
/// why an inline list could not be clicked.
pub struct Select {
    options: Vec<String>,
    selected: Option<usize>,
    row_height: f64,
    on_change: Option<IndexCallback>,
    state: InteractionState,
    revision: Revision,
    /// The open popup, if any. This *is* the open/closed state: it is set when
    /// the host confirms the overlay opened and cleared from
    /// [`Widget::overlay_closed`], so the control can never believe a list is
    /// up that nothing is painting.
    popup: Option<OverlayHandle>,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("popup", &self.popup)
            .finish_non_exhaustive()
    }
}

impl Select {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            row_height: 22.0,
            on_change: None,
            state: InteractionState::default(),
            revision: Revision::default(),
            popup: None,
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
        self.popup.is_some()
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

    /// The open popup's handle, for a caller that wants to close it from
    /// outside the control.
    pub fn popup_handle(&self) -> Option<OverlayHandle> {
        self.popup
    }

    /// Chooses an option and closes the list; no-op when out of range.
    pub fn select(&mut self, index: usize) -> bool {
        self.choose(index)
    }

    /// Chooses an option.
    ///
    /// Returns whether the event was handled — re-picking the selected option
    /// still counts, because it closes the list.
    fn choose(&mut self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        self.revision.bump();
        if changed {
            if let Some(callback) = &self.on_change {
                callback(index);
            }
        }
        true
    }

    /// Moves the selection by `delta`. From nothing selected, either direction
    /// lands on the end it is heading for.
    fn move_selection(&mut self, delta: isize) -> bool {
        let len = self.options.len();
        if len == 0 {
            return false;
        }
        let next = match self.selected {
            Some(index) => (index as isize + delta).clamp(0, len as isize - 1) as usize,
            None if delta > 0 => 0,
            None => len - 1,
        };
        self.choose(next)
    }

    /// Opens the option list as an overlay anchored just below the control.
    ///
    /// The rect is a guess, and deliberately so: the host lays the content out
    /// against it, so the only thing the Select has to get right is how tall
    /// the list is and where its top-left corner goes. Asking the host to
    /// measure first would mean opening the overlay to find out whether it
    /// should be opened.
    pub fn open_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        if self.is_open() || self.options.is_empty() {
            return false;
        }
        let bounds = cx.bounds();
        let rect = Rect::from_origin_size(
            Point::new(bounds.x0, bounds.y1),
            Size::new(bounds.width(), self.row_height * self.options.len() as f64),
        );
        let options = self.options.clone();
        let selected = self.selected;
        let row_height = self.row_height;
        let opened = cx.open_overlay(OverlayRequest::new(
            OverlayKind::Popup,
            false,
            rect,
            move |handle| Box::new(SelectPopup::new(options, selected, row_height, handle)),
        ));
        if opened {
            self.revision.bump();
        }
        opened
    }

    /// Closes the open list without choosing anything.
    pub fn close_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        match self.popup.take() {
            Some(handle) => {
                cx.close_overlay(handle);
                self.revision.bump();
                true
            }
            None => false,
        }
    }

    fn toggle_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        if self.is_open() {
            self.close_popup(cx)
        } else {
            self.open_popup(cx)
        }
    }

    /// Keyboard control for the focused, *closed* select: open the list, walk
    /// the selection, jump to either end.
    ///
    /// Once the list is open the keyboard belongs to [`SelectPopup`] — focus
    /// moved into the overlay when it opened — so navigation while open is the
    /// popup's business, and picking a row there is what changes the selection.
    fn key_event(&mut self, cx: &mut dyn EventContext, key: &KeyEvent) -> bool {
        if !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        match key.key {
            Key::Named(NamedKey::Enter) | Key::Char(' ') => self.toggle_popup(cx),
            Key::Named(NamedKey::Escape) => self.close_popup(cx),
            _ if self.is_open() => false,
            Key::Named(NamedKey::ArrowDown) => self.move_selection(1),
            Key::Named(NamedKey::ArrowUp) => self.move_selection(-1),
            Key::Named(NamedKey::Home) => self.choose(0),
            Key::Named(NamedKey::End) => self.choose(self.options.len().saturating_sub(1)),
            _ => false,
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.is_open() {
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
    /// open list is an overlay sized from this box, so it deliberately does not
    /// contribute to the intrinsic extent — otherwise the control would grow a
    /// list-height hole in its parent's flow every time it opened.
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
        if self.state.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) => self.key_event(cx, &key),
                _ => false,
            };
        };
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
                // Only the header can be hit: the list is an overlay with its
                // own nodes, so a click below it never reaches the Select.
                self.toggle_popup(cx)
            }
            _ => false,
        }
    }

    /// The open list came down. If it carries a chosen row, that is the new
    /// selection; either way the control stops believing a popup is up.
    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        content: Box<dyn WidgetAny>,
    ) {
        if self.popup != Some(handle) {
            return;
        }
        self.popup = None;
        let chosen = content
            .as_any()
            .downcast_ref::<SelectPopup>()
            .and_then(SelectPopup::chosen);
        if let Some(index) = chosen {
            self.choose(index);
        }
        self.revision.bump();
        cx.mark_dirty(id);
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.popup = Some(handle);
        self.revision.bump();
        cx.mark_dirty(id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let (darker, lighter) = emphasis(Color::WHITE);
        cx.fill_rect(
            bounds,
            resolved_color(
                Color::WHITE,
                darker,
                lighter,
                InteractionState {
                    // An open list is a pressed control: the header stays lit
                    // for as long as the popup is up.
                    pressed: self.state.pressed || self.is_open(),
                    ..self.state
                },
            ),
        );
        draw_centered_text(
            cx,
            bounds,
            self.selected_label(),
            LABEL_FONT_SIZE,
            Color::BLACK,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}
