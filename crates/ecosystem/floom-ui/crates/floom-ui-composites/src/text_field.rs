//! `TextField`: single-line text input, with the ink the three kits disagree on.
//!
//! It moved out of `floom-widgets/src/controls.rs` for the same reason `Select`,
//! `Counter`, `Slider` and `Radio` did. What a text field *does* is primitive --
//! hold a string, keep a caret in it, answer Backspace and the arrows, own an
//! IME composition and render the preedit inline, report itself to assistive
//! technology -- but what it *looks like* is a design decision, and the old
//! `paint` had no way to express one. It hard-coded a white fill that shifted to
//! `#F5F7FA` under the pointer, a `#9AA5B1` placeholder, black committed text
//! and a `#2B6CB0` underlined preedit at 14px, with square corners, no rim and
//! no focus affordance at all.
//!
//! ## Only the paint is new
//!
//! The IME state machine, the character/byte caret split, the committed text
//! drawn as two runs around the caret, the underlined preedit and the semantics
//! all move across verbatim. [`TextFieldStyle`] reaches exactly four places in
//! `paint` -- the fill, the rim, the focus ring and the ink -- plus
//! `layout_request`, which reads it for the same reason the `Slider` and `Radio`
//! ports did: a kit's 56pt field measured at the engine's 14 is measured wrong.
//!
//! ## The default is the old pixels, not a design decision
//!
//! [`TextFieldStyle::default`] is a snapshot of what the old `paint` wrote
//! inline. Two of those values read as odd until the field docs are in place.
//! `border_width` is `0.0` because the engine drew no rim at all -- it filled
//! the whole bounds and stopped -- and `background_color_disabled` is white at
//! half alpha rather than white, because the old fill went through
//! `floom_widgets::resolved_color`, which ranks disabled first and halves alpha.
//! A `TextField` built without a style still paints what it painted before the
//! move.

use floom_accessibility::SemanticsRole;
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyPhase, NamedKey, PaintContext,
    Point, Rect, SemanticsContext, Size, UpdateContext, Widget,
};
use floom_ime::{CompositionState, ImeEvent, ImeState};
use floom_types::{FontWeight, ImeUpdate, PointerButton, PointerPhase};

use floom_widgets::{
    emit_semantics, stroke_rounded_outline, InteractionState, Revision, SemanticsHint,
};

pub type TextCallback = Box<dyn Fn(&str)>;

/// How a text field looks.
///
/// A text field has more kinds of ink than any other control: a rim, a fill, a
/// placeholder, committed text, and a provisional preedit that is underlined
/// while the IME composes it. The field list below is the three kits'
/// disagreement rather than one kit's markup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextFieldStyle {
    // --- geometry ---
    /// The field's height when it is pinned to one, as `TextFieldTokens::height`
    /// is. `None` derives it from the font metrics plus `vertical_padding`,
    /// which is what the engine did before this type existed.
    pub height: Option<f32>,
    /// Padding above and below the text, used only when `height` is `None`.
    pub vertical_padding: f32,
    /// Text to the left and right of the field's contents.
    pub padding_horizontal: f32,
    /// The rim's corner radius. `0.0` is a square field.
    pub border_radius: f32,
    /// Never lay the field out narrower than this; the engine's is 120.
    pub min_width: f32,

    // --- the rim ---
    /// The rim's thickness at rest. `0.0` draws no rim at all, which is what the
    /// engine did: it filled the whole bounds and stopped there. All three kits
    /// set `1.0`.
    pub border_width: f32,
    /// The rim's thickness while focused.
    ///
    /// Material thickens `1.0` to `2.0` and calls that its focus affordance.
    /// Cupertino and Fluent leave the rim alone and answer focus elsewhere, so
    /// they set this equal to `border_width`.
    pub border_width_focused: f32,
    /// The rim at rest. All three kits: `outline`.
    pub border_color: Color,
    /// The rim while focused. All three kits: `primary`.
    pub border_color_focused: Color,
    /// The rim while hovered, `None` for no variant. Only Cupertino had one:
    /// `primary`@120. Material and Fluent answered hover with nothing.
    pub border_color_hovered: Option<Color>,
    /// The rim while disabled. All three kits: `on_surface`@38.
    pub border_color_disabled: Color,

    // --- the focus outline, drawn *outside* the rim ---
    /// Only Fluent drew one: a `2.0`-wide `primary` ring around the field while
    /// focused. `0.0` draws none, which is what Material and Cupertino want --
    /// Material thickens its rim instead.
    pub focus_outline_width: f32,
    pub focus_outline_color: Color,

    // --- the fill ---
    /// Behind the text. All three kits: `surface`.
    pub background_color: Color,
    /// The fill while hovered, `None` for no variant. None of the kits had one;
    /// the engine did, so the default carries it.
    pub background_color_hovered: Option<Color>,
    /// The fill while pressed, `None` for no variant.
    ///
    /// The default is `Some(WHITE)`, and that is not a typo: the engine's fill
    /// went through `resolved_color(base, pressed, hovered, state)`, which
    /// ranks `pressed` *above* `hovered`, and the engine passed white for
    /// pressed. So the old field greyed on hover and went back to white when
    /// pressed, which reads like an artifact rather than a design -- and it is
    /// still what the engine painted, so changing it here would be a silent
    /// pixel change rather than a port. `None` falls back to
    /// `background_color`, the same rule every other interaction field here
    /// follows; a kit that wants press to look like hover sets both to the same
    /// colour.
    pub background_color_pressed: Option<Color>,
    /// The fill while disabled. All three kits: `on_surface`@4.
    pub background_color_disabled: Color,

    // --- the ink ---
    /// Committed text. All three kits: `on_surface`.
    pub text_color: Color,
    /// Committed text while disabled. All three kits: `on_surface`@38.
    pub text_color_disabled: Color,
    /// The placeholder, shown only while the field is empty and not composing.
    pub placeholder_color: Color,
    /// The in-progress IME composition, which is drawn underlined so it reads as
    /// provisional rather than committed.
    pub preedit_color: Color,
    /// Every piece of text in the field, at one size. The kits all read
    /// `typography().body.medium.font_size`.
    pub font_size: f32,
}

/// The palette and geometry the engine hard-coded before a text field had a
/// style.
///
/// Every value here is what `TextField::paint` used to write inline, so the
/// default is not a design decision: it is a snapshot of the old behaviour,
/// kept so that moving the widget did not also repaint every existing caller.
///
/// The white rim colours are not a mistake. At `border_width: 0.0` the rim is
/// never stroked, so its colour is unreadable; white is the fill it would have
/// to disappear into.
impl Default for TextFieldStyle {
    fn default() -> Self {
        TextFieldStyle {
            height: None,
            vertical_padding: 4.0,
            padding_horizontal: 8.0,
            border_radius: 0.0,
            min_width: 120.0,
            border_width: 0.0,
            border_width_focused: 0.0,
            border_color: Color::WHITE,
            border_color_focused: Color::WHITE,
            border_color_hovered: None,
            border_color_disabled: Color::WHITE,
            focus_outline_width: 0.0,
            focus_outline_color: Color::TRANSPARENT,
            background_color: Color::WHITE,
            background_color_hovered: Some(Color::rgb(0xF5, 0xF7, 0xFA)),
            background_color_pressed: Some(Color::WHITE),
            // White at half alpha, not white: the old fill was
            // `floom_widgets::resolved_color(Color::WHITE, Color::WHITE, hover, state)`,
            // which ranks `disabled` first and halves alpha. `255 / 2` is 127.
            background_color_disabled: Color::rgba(0xFF, 0xFF, 0xFF, 0x7F),
            text_color: Color::BLACK,
            text_color_disabled: Color::BLACK,
            placeholder_color: Color::rgb(0x9A, 0xA5, 0xB1),
            preedit_color: Color::rgb(0x2B, 0x6C, 0xB0),
            font_size: 14.0,
        }
    }
}

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
    /// Live IME composition state (`floom-ime`). Empty / `Inactive` when no
    /// composition is in flight.
    ime: ImeState,
    /// Last composition caret rectangle reported to the host, in logical
    /// window pixels; `None` when nothing is composing.
    ime_caret: Option<Rect>,
    /// Longest value the field will hold, in **characters**. `None` is v1's
    /// `None`, and what the engine always was.
    max_length: Option<usize>,
    /// Visual style. Named `style_` because the builder below is
    /// [`TextField::style`], the same split `Radio` and `Slider` use.
    style_: TextFieldStyle,
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
            max_length: None,
            style_: TextFieldStyle::default(),
        }
    }

    /// The field's initial text.
    ///
    /// The caret lands at the **end** of it, not at the offset it had before.
    /// `set_value` clamps the caret into range so a programmatic update mid-typing
    /// does not throw away the user's position, but at construction there is no
    /// position to keep: it is zero, and leaving it there meant that a field
    /// pre-filled with "Ada" inserted the first keystroke *before* the A.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.set_value(value.into());
        self.caret = self.value.chars().count();
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

    /// Caps how many **characters** the field will hold, `None` for no cap.
    ///
    /// All three v1 kits had this prop with a `clamp_value` helper behind it,
    /// and all three called that helper from exactly three places: `value()`,
    /// `set_value()` and their own `max_length` builder. So v1's `max_length(8)`
    /// truncated the string you passed in and did nothing whatsoever about the
    /// eight characters you then typed -- a half-implemented limit, shipped
    /// identically by three designs.
    ///
    /// This one clamps typing too, which is a deliberate widening rather than a
    /// port. There are two entry paths into `value`, not one:
    ///
    /// - [`TextField::set_value`] (and the `value()` builder, which calls it)
    ///   for a programmatic write, and
    /// - [`TextField::insert`] for everything that arrives as a character key,
    ///   which is every keystroke and -- because the runner turns an OS
    ///   `ImeCommit` into one character key-down per scalar -- every committed
    ///   composition as well.
    ///
    /// Both call the same `clamp_value`, and both clamp in characters rather
    /// than bytes, so a CJK or emoji value is never cut mid-scalar.
    pub fn max_length(mut self, max_length: Option<usize>) -> Self {
        self.max_length = max_length;
        // Re-clamp what is already there: `max_length` may have been set after
        // `value`, and a value that has since got shorter must be legal again.
        // Taken out first because `clamp_value` reads `self.max_length`.
        let current = std::mem::take(&mut self.value);
        self.value = self.clamp_value(current);
        let len = self.value.chars().count();
        self.caret = self.caret.min(len);
        self.revision.bump();
        self
    }

    /// The character cap, or `None` when the field is uncapped.
    pub fn length_limit(&self) -> Option<usize> {
        self.max_length
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

    /// Replaces the content, clamping the caret to a valid boundary and the
    /// value to [`TextField::max_length`].
    pub fn set_value(&mut self, value: impl Into<String>) -> &mut Self {
        self.value = self.clamp_value(value.into());
        let len = self.value.chars().count();
        self.caret = self.caret.min(len);
        self.revision.bump();
        self
    }

    /// Replaces the whole appearance. A kit resolves its `TextFieldTokens` into
    /// a [`TextFieldStyle`] once and hands it over here, rather than the widget
    /// holding a partial set of knobs a kit has to guess the rest of.
    pub fn style(mut self, style: TextFieldStyle) -> Self {
        self.style_ = style;
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

    /// Truncates `value` to the field's character cap, if it has one.
    ///
    /// Character-wise, which is what v1 did (`value.chars().take(n)`) and the
    /// only thing that is safe: a byte-wise `truncate` on a multi-byte scalar
    /// would leave `String` invalid UTF-8 rather than merely short.
    fn clamp_value(&self, value: String) -> String {
        match self.max_length {
            Some(max) => value.chars().take(max).collect(),
            None => value,
        }
    }

    /// On-screen rectangle of the composition caret (where the OS IME popup
    /// should anchor), in logical window pixels. Includes the preedit width
    /// when composing, so the popup sits just past the last candidate glyph.
    fn caret_rect(&self, bounds: Rect) -> Rect {
        let prefix = &self.value[..self.byte_caret()];
        let size = self.style_.font_size;
        let mut width = floom_text::measure_text(prefix, size, f64::INFINITY).width;
        if self.ime.is_composing() {
            width += floom_text::measure_text(&self.ime.preedit, size, f64::INFINITY).width;
        }
        let x = bounds.x0 + f64::from(self.style_.padding_horizontal) + width;
        let top = bounds.y0 + (bounds.height() - f64::from(size)) / 2.0;
        Rect::from_origin_size(Point::new(x, top), Size::new(1.0, f64::from(size)))
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

    /// Inserts one character at the caret, unless the field is already full.
    ///
    /// The full case is the `max_length` clamp on the typing path, and it is a
    /// refusal rather than a truncation: inserting the ninth character into an
    /// eight-character field would otherwise have to *remove* a character the
    /// user did not ask to lose, and which one -- the new one or the oldest --
    /// is a decision no kit made. Nothing is bumped and `on_change` does not
    /// fire, because nothing changed.
    fn insert(&mut self, ch: char) {
        if let Some(max) = self.max_length {
            if self.value.chars().count() >= max {
                return;
            }
        }
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

    /// The rim's thickness, resting or focused.
    fn rim_width(&self) -> f32 {
        if self.state.focused {
            self.style_.border_width_focused
        } else {
            self.style_.border_width
        }
    }

    /// The fill's colour.
    ///
    /// `disabled` is checked first and covers the fill on its own, exactly as
    /// the first clause of the resolution rule says. Everything else is the
    /// hover variant where the style names one and the base where it does not --
    /// a `None` variant falls back to the unvariant colour rather than to
    /// "nothing", which is the distinction that matters and the same rule
    /// `RadioStyle` follows.
    ///
    /// The pressed state is deliberately absent. The old fill went through
    /// `floom_widgets::resolved_color`, whose argument order is
    /// `(base, pressed, hovered)` and which ranked `pressed` *above* `hovered`;
    /// the engine passed white for pressed, so a field being pressed while the
    /// pointer was over it painted white rather than `#F5F7FA`. The resolution
    /// rule for this widget names only hover and focus, so the pressed case now
    /// paints the hover colour.
    fn fill_color(&self) -> Color {
        let style = self.style_;
        if self.state.disabled {
            return style.background_color_disabled;
        }
        // `pressed` outranks `hovered`, matching the engine's old
        // `resolved_color(base, pressed, hovered, state)`: a pointer-down sets
        // both flags at once, so checking hover first would never let a press
        // be seen.
        if self.state.pressed {
            return style
                .background_color_pressed
                .unwrap_or(style.background_color);
        }
        if self.state.hovered {
            return style
                .background_color_hovered
                .unwrap_or(style.background_color);
        }
        style.background_color
    }

    /// The rim's colour, or `None` when there is no rim to colour.
    ///
    /// The second half of the guard is not in the resolution rule but follows
    /// from it: a style that names a resting rim and *no* focused one resolves
    /// to a zero-width stroke while focused, and stroking a path zero wide is a
    /// degenerate op rather than a thinner rim. "Is there a rim to draw" is one
    /// question, and this answers it for both widths.
    fn rim_color(&self) -> Option<Color> {
        let style = self.style_;
        if style.border_width <= 0.0 || self.rim_width() <= 0.0 {
            return None;
        }
        Some(if self.state.disabled {
            style.border_color_disabled
        } else if self.state.focused {
            style.border_color_focused
        } else if self.state.hovered {
            style.border_color_hovered.unwrap_or(style.border_color)
        } else {
            style.border_color
        })
    }

    /// The committed text's ink, and -- because there is no separate caret ink
    /// to resolve -- the ink of nothing else.
    ///
    /// The old paint never dimmed its text: a disabled field drew its
    /// placeholder and its committed runs at full strength, which is why
    /// `text_color_disabled` defaults to `text_color`.
    fn text_color(&self) -> Color {
        if self.state.disabled {
            self.style_.text_color_disabled
        } else {
            self.style_.text_color
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

/// The engine's shared pointer tracking, reduced to the one question a text
/// field asks of it: did this event touch the field's state?
///
/// `floom-widgets`' `track_pointer` answers with three outcomes because its
/// callers also need to know whether a release completed a press. A text field
/// has no action to run, and the old `event` treated "consumed" and "activated"
/// identically -- both bumped the revision and returned `true` -- so the
/// distinction is dropped rather than carried.
fn track_pointer(state: &mut InteractionState, cx: &mut dyn EventContext, id: ElementId) -> bool {
    let Event::Pointer(pointer) = cx.event() else {
        return false;
    };
    if state.disabled {
        return false;
    }
    match pointer.phase {
        PointerPhase::Enter | PointerPhase::Move => {
            state.hovered = true;
            true
        }
        PointerPhase::Leave | PointerPhase::Cancel => {
            state.hovered = false;
            state.pressed = false;
            true
        }
        PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
            state.hovered = true;
            state.pressed = true;
            cx.request_focus(id);
            true
        }
        PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
            // A release that completes a press inside the field is "consumed"
            // here for the same reason it was `Activate` before: the caller
            // only distinguishes it from a release that started elsewhere.
            let completing = state.pressed;
            state.pressed = false;
            completing
        }
        _ => false,
    }
}

impl Widget for TextField {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        TextField::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the shown text (value, else placeholder) plus the
    /// padding paint applies, floored at [`TextFieldStyle::min_width`] so an
    /// empty input stays usable.
    ///
    /// Both halves read the style, as the `Slider` and `Radio` ports did. The
    /// height is [`TextFieldStyle::height`] when the style pins one and is
    /// otherwise the font's own line height plus `vertical_padding` -- which is
    /// the old `line_height() + BUTTON_VERTICAL_PADDING * 2.0`, spelled in terms
    /// of the font the field is actually drawn at. A kit's 56pt field measured at
    /// the engine's 14 would otherwise be laid out a row tall and clipped.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let shown = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        let style = self.style_;
        let height = match style.height {
            Some(h) => h,
            None => {
                floom_text::measure_text(" ", style.font_size, f64::INFINITY).height as f32
                    + style.vertical_padding * 2.0
            }
        };
        Some(
            floom_core::LayoutRequest::new()
                .width(
                    (floom_widgets::text_width_at(shown, style.font_size)
                        + style.padding_horizontal * 2.0)
                        .max(style.min_width),
                )
                .height(height),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        if track_pointer(&mut self.state, cx, id) {
            self.revision.bump();
            return true;
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

    /// Fill, then the focus ring outside the box, then the rim inside it, then
    /// the ink.
    ///
    /// The order is the load-bearing part and it is inverted from the obvious
    /// one: the focus ring is stroked *after* the fill because a ring painted
    /// first would be half-covered by the fill it surrounds, and the rim before
    /// the text so a thick rim cannot sit under the first glyph -- which is why
    /// the text origin is inset by `border_width` on top of
    /// `padding_horizontal`.
    ///
    /// The focus ring is stroked around a box grown by its own width, so its
    /// *inner* edge is the layout bounds and its outer edge is
    /// `focus_outline_width` beyond them. A ring drawn inside the bounds would
    /// be clipped by the layout box and so never seen at all.
    ///
    /// Everything below the ink is unchanged from the old paint: the placeholder
    /// shows only while the field is empty and not composing, committed text is
    /// drawn as two runs either side of the caret with the composition
    /// underlined between them, and the whole block is vertically centred on the
    /// font size rather than on a baseline.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style_;

        let fill = self.fill_color();
        if style.border_radius > 0.0 {
            cx.fill_rounded_rect(bounds, style.border_radius, fill);
        } else {
            cx.fill_rect(bounds, fill);
        }

        if style.focus_outline_width > 0.0 && self.state.focused {
            let width = f64::from(style.focus_outline_width);
            let outside = Rect {
                x0: bounds.x0 - width,
                y0: bounds.y0 - width,
                x1: bounds.x1 + width,
                y1: bounds.y1 + width,
            };
            stroke_rounded_outline(
                cx,
                outside,
                f64::from(style.border_radius) + width,
                width,
                style.focus_outline_color,
            );
        }

        let rim_width = self.rim_width();
        if let Some(rim) = self.rim_color() {
            stroke_rounded_outline(
                cx,
                bounds,
                f64::from(style.border_radius),
                f64::from(rim_width),
                rim,
            );
        }

        let pad = f64::from(style.padding_horizontal) + f64::from(style.border_width);
        let size = f64::from(style.font_size);
        let origin_y = bounds.y0 + (bounds.height() - size) / 2.0;

        // Empty and not composing: show the placeholder, dimmed.
        if self.value.is_empty() && !self.ime.is_composing() {
            if !self.placeholder.is_empty() {
                cx.fill_text(
                    Point::new(bounds.x0 + pad, origin_y),
                    &self.placeholder,
                    style.font_size,
                    FontWeight::Normal,
                    style.placeholder_color,
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

        let committed = self.text_color();
        let mut x = bounds.x0 + pad;
        if !left.is_empty() {
            cx.fill_text(
                Point::new(x, origin_y),
                left,
                style.font_size,
                FontWeight::Normal,
                committed,
            );
            x += floom_text::measure_text(left, style.font_size, f64::INFINITY).width;
        }
        if !preedit.is_empty() {
            cx.fill_text(
                Point::new(x, origin_y),
                &preedit,
                style.font_size,
                FontWeight::Normal,
                style.preedit_color,
            );
            let width = floom_text::measure_text(&preedit, style.font_size, f64::INFINITY).width;
            // Underline the composition so it reads as provisional text.
            let underline =
                Rect::from_origin_size(Point::new(x, origin_y + size - 1.0), Size::new(width, 1.0));
            cx.fill_rect(underline, style.preedit_color);
            x += width;
        }
        if !right.is_empty() {
            cx.fill_text(
                Point::new(x, origin_y),
                right,
                style.font_size,
                FontWeight::Normal,
                committed,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
