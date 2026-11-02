//! The text input: an editable single-line buffer.
//!
//! ## Why this exists, and why it is shaped the way it is
//!
//! `combobox` and `autocomplete` both build on a text input, so this is the first
//! *primitive* in the port that is not a port of anything, and its
//! shape is worth stating.
//!
//! A text input buffer cannot be an `RwSignal<String>`: `RwSignal` is `Rc`-based
//! and a slot closure must be `Send + Sync`, for the same reason `md-dialog`
//! cannot own its `open` signal. The buffer is an [`Arc<Mutex<String>>`]: `Send +
//! Sync`, so it can be captured by a builder, and *shared*, so a caller can still
//! hold a handle and read or write it. It is the same move as `TreeView`'s forest,
//! and for the same reason.
//!
//! So:
//!
//! * [`TextInput::buffer`] hands out the handle, and [`TextInput::text`] reads it —
//!   a caller does not need a signal to bind to, and can use the same handle from two
//!   places;
//! * [`TextInput::on_change`] fires when the buffer differs from what the widget last
//!   saw, which covers **both** directions: a keystroke and an external write. The
//!   external write is not delivered by a signal, so it has to be noticed; that is what
//!   the widget's `update` is for.
//!
//! ## Two limits worth knowing before using it
//!
//! * **A click cannot place the caret.** `EventContext` carries `cursor_position`
//!   and `bounds` but no text measurement, so there is no way to turn an x coordinate
//!   into a byte offset. A click therefore focuses the field and puts the caret at the
//!   end. Everything else — typing, backspace, delete, arrows, Home/End, Enter — works
//!   from the caret's current position.
//! * **The text is not clipped to the box.** `PaintContext` has no clip op, although
//!   the *host* already has the scene operation (`write_clip_push`, and `resolve_clip`
//!   in `floom-types`), so this is a missing method rather than a missing capability.
//!   A field narrower than its text will paint the text past its own edge.
//!
//! Both are recorded rather than worked around, and both are the kind of thing that
//! should be fixed before a text input grows selection handles.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use floom_core::{
    Color, CreateContext, ElementId, Event, Key, KeyPhase, LayoutRequest, NamedKey, PaintContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_types::{FontWeight, Rect};

use crate::primitives::Revision;

/// The shared buffer. `Send + Sync` because a slot builder must be, and shared
/// because a caller needs to read and write it from outside the widget.
pub type SharedBuffer = Arc<Mutex<String>>;

/// A flag shared between a widget and whoever drives it.
///
/// The same shape as [`SharedBuffer`] and for the same reason, and named separately
/// because `SharedBuffer` is `Arc<Mutex<String>>` and a type that says `Buffer` while
/// holding a `bool` is a lie at every call site. It exists because dialogs expose
/// `open_signal()` — "so an external trigger (e.g. a button) can drive it" — and an
/// `RwSignal` is `Rc`-based and cannot cross a `Send + Sync` slot boundary.
pub type SharedFlag = Arc<Mutex<bool>>;

/// A map shared between a widget and whoever drives it.
///
/// The third of the family, and for the same reason as the other two: a value a
/// caller can read and write from outside the tree, in a form a `Send + Sync`
/// slot closure can hold. A caller's overrides are held where they can be read and
/// written from outside the tree; an `Rc`-based form could not cross a slot boundary.
/// slot boundary at all.
pub type SharedMap = Arc<Mutex<HashMap<String, String>>>;

/// A fresh shared map, initialised to `map`.
pub fn shared_map(map: HashMap<String, String>) -> SharedMap {
    Arc::new(Mutex::new(map))
}

/// A shared list of strings.
///
/// The third shape of shared state the engine has, after the flag and the
/// buffer: a list that changes as a whole. A validation list is the case that
/// needed it -- a set of messages the caller pushes and pulls, which is neither
/// one string nor a pair of them.
pub type SharedList = Arc<Mutex<Vec<String>>>;

/// A fresh shared list, initialised to `list`.
pub fn shared_list(list: Vec<String>) -> SharedList {
    Arc::new(Mutex::new(list))
}

/// Read a shared list, treating a poisoned mutex as empty.
///
/// A caller that poisoned the mutex is as stuck as the widget is, and
/// panicking here would take down a paint pass over a list nobody was looking
/// at.
pub fn read_list(list: &SharedList) -> Vec<String> {
    list.lock().map(|guard| guard.clone()).unwrap_or_default()
}

/// Write a shared list. Reports whether it changed, so a caller can skip a
/// frame.
pub fn write_list(list: &SharedList, value: Vec<String>) -> bool {
    match list.lock() {
        Ok(mut guard) if *guard != value => {
            *guard = value;
            true
        }
        _ => false,
    }
}

/// A fresh shared flag, initialised to `value`.
pub fn shared_flag(value: bool) -> SharedFlag {
    Arc::new(Mutex::new(value))
}

/// Read a shared flag, treating a poisoned mutex as "not set".
///
/// A caller that poisoned the mutex is as stuck as the widget is, and panicking here
/// would take down a paint pass over a flag nobody was looking at.
pub fn read_flag(flag: &SharedFlag) -> bool {
    flag.lock().map(|guard| *guard).unwrap_or(false)
}

/// Write a shared flag. Reports whether it changed, so a caller can skip a frame.
pub fn write_flag(flag: &SharedFlag, value: bool) -> bool {
    match flag.lock() {
        Ok(mut guard) if *guard != value => {
            *guard = value;
            true
        }
        _ => false,
    }
}

/// How a field looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInputStyle {
    pub font_size: f32,
    pub font_weight: FontWeight,
    /// The committed text.
    pub color: Color,
    /// Shown instead of the text while the buffer is empty.
    pub placeholder_color: Color,
    /// Shown while an IME composition is in progress, in place of nothing.
    pub preedit_color: Color,
    pub caret_color: Color,
    pub caret_width: f32,
    /// Space between the text and the box's edge. Horizontal and vertical, because
    /// Horizontal and vertical, because a caret needs room above and below the text.
    pub padding_horiz: f64,
    pub padding_vert: f64,
    /// `None` draws no fill, which is how a field inside another surface is written.
    pub background: Option<Color>,
    pub radius: f32,
    pub border_color: Color,
    /// `0.0` draws no outline.
    pub border_width: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        TextInputStyle {
            font_size: 16.0,
            font_weight: FontWeight::Normal,
            color: Color::rgb(0x1D, 0x1B, 0x20),
            placeholder_color: Color::rgba(0x1D, 0x1B, 0x20, 105),
            preedit_color: Color::rgb(0x49, 0x4A, 0x5C),
            caret_color: Color::rgb(0x1D, 0x1B, 0x20),
            caret_width: 1.0,
            padding_horiz: 12.0,
            padding_vert: 10.0,
            background: None,
            radius: 4.0,
            border_color: Color::rgba(0x1D, 0x1B, 0x20, 31),
            border_width: 1.0,
        }
    }
}

/// An editable single-line buffer.
pub struct TextInput {
    buffer: SharedBuffer,
    /// What the widget last read out of `buffer`. A difference means somebody else
    /// wrote it, which is the whole of the two-way binding.
    last_seen: String,
    placeholder: Option<String>,
    /// An in-progress IME composition, drawn but **not** in the buffer.
    preedit: Option<String>,
    /// Byte offset into the buffer. Always on a character boundary.
    caret: usize,
    focused: bool,
    style: TextInputStyle,
    on_change: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
    on_enter: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    on_escape: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    revision: Revision,
}

impl std::fmt::Debug for TextInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextInput")
            .field("text", &self.text())
            .field("focused", &self.focused)
            .field("caret", &self.caret)
            .finish_non_exhaustive()
    }
}

impl TextInput {
    /// A field over `buffer`.
    pub fn new(buffer: SharedBuffer) -> Self {
        let text = buffer.lock().map(|t| t.clone()).unwrap_or_default();
        TextInput {
            last_seen: text,
            buffer,
            placeholder: None,
            preedit: None,
            caret: 0,
            focused: false,
            style: TextInputStyle::default(),
            on_change: None,
            on_enter: None,
            on_escape: None,
            revision: Revision::default(),
        }
    }

    /// A field over a fresh buffer holding `text`.
    pub fn with_text(text: impl Into<String>) -> Self {
        let buffer: SharedBuffer = Arc::new(Mutex::new(text.into()));
        let mut input = TextInput::new(buffer);
        input.caret = input.text().len();
        input.last_seen = input.text();
        input
    }

    /// The shared handle. `None` only if a caller poisoned the mutex, in which case
    /// the widget is as stuck as they are — so this does not panic.
    pub fn buffer(&self) -> SharedBuffer {
        self.buffer.clone()
    }

    /// The committed text.
    pub fn text(&self) -> String {
        self.buffer.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Replace the text and move the caret to the end.
    ///
    /// A write rather than an edit, so it takes `&mut self` like every other
    /// in-place change in this crate, and it goes **through** the buffer so that
    /// `on_change` fires on the next `update` exactly as an external write would.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if let Ok(mut buffer) = self.buffer.lock() {
            *buffer = text;
        }
        self.caret = self.text().len();
        self.revision.bump();
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    /// Called when the buffer changes, from either direction.
    pub fn on_change(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn on_enter(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_enter = Some(Arc::new(callback));
        self
    }

    /// Called on Escape.
    ///
    /// Escape does **not** clear the buffer: clearing the field on escape is a behaviour
    /// this widget does not assume, and inventing "escape reverts" would be a decision a text field
    /// should not make on its own.
    pub fn on_escape(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_escape = Some(Arc::new(callback));
        self
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Move the caret, clamped to the buffer and to a character boundary.
    pub fn set_caret(&mut self, caret: usize) {
        let clamped = clamp_to_char_boundary(&self.text(), caret);
        if clamped != self.caret {
            self.caret = clamped;
            self.revision.bump();
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.focused != focused {
            self.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// The composition text, while an IME is mid-composition.
    pub fn preedit(&self) -> Option<&str> {
        self.preedit.as_deref()
    }

    /// The widget's accessibility description.
    ///
    /// A text field is a `TextField` whose name is its current value. The value *as*
    /// the name is a compromise, and it is the same one a missing label forces: a
    /// field's accessible name should be its label, and this widget
    /// has no separate label, so it publishes what it holds rather than nothing.
    /// Recorded here rather than left to be discovered.
    fn hint(&self) -> crate::primitives::SemanticsHint {
        crate::primitives::SemanticsHint::labeled(crate::primitives::Role::TextField, self.text())
    }

    pub fn resolved_style(&self) -> TextInputStyle {
        self.style
    }

    /// What the field paints: the committed text, or the composition, or the
    /// placeholder — in that order of precedence.
    fn shown(&self) -> (String, Color) {
        if let Some(preedit) = &self.preedit
            && !preedit.is_empty()
        {
            return (preedit.clone(), self.style.preedit_color);
        }
        let text = self.text();
        if text.is_empty() {
            if let Some(placeholder) = &self.placeholder {
                return (placeholder.clone(), self.style.placeholder_color);
            }
            // An empty field with no placeholder still gets a glyph to sit the caret
            // after, which is what every text field does.
            return (" ".to_string(), self.style.placeholder_color);
        }
        (text, self.style.color)
    }

    /// Apply an edit, then tell whoever is listening.
    fn edit(&mut self, f: impl FnOnce(&mut String, &mut usize)) {
        let mut text = self.text();
        let before = text.clone();
        let mut caret = self.caret;
        f(&mut text, &mut caret);
        if text == before {
            return;
        }
        if let Ok(mut buffer) = self.buffer.lock() {
            *buffer = text;
        }
        self.caret = clamp_to_char_boundary(&self.text(), caret);
        self.revision.bump();
        if let Some(on_change) = &self.on_change {
            on_change(self.text());
        }
    }

    /// The intrinsic width: the shown text plus the horizontal padding and the caret.
    ///
    /// The text is measured with the same helper `Button` and `Pressable` use, so a
    /// field's intrinsic width and a button's label width come from one estimate.
    fn intrinsic_width(&self) -> f32 {
        let (text, _) = self.shown();
        let style = self.style;
        let text_width = crate::controls::text_width_at(&text, style.font_size);
        text_width + style.padding_horiz as f32 * 2.0 + style.caret_width
    }

    fn intrinsic_height(&self) -> f32 {
        crate::controls::text_height_at(self.style.font_size) + self.style.padding_vert as f32 * 2.0
    }
}

/// Clamp a byte offset into `text` to a character boundary.
///
/// Every offset this widget holds is a byte index, and a byte index that is not on a
/// boundary would panic on the next `String::slice`. Three things can produce one: an
/// external write that shrank the buffer, a caret set from a caller, and a
/// `Preedit` whose offsets are documented as needing exactly this clamping.
pub fn clamp_to_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// The byte range of the character before `offset`, for backspace.
fn previous_boundary(text: &str, offset: usize) -> usize {
    let start = clamp_to_char_boundary(text, offset);
    if start == 0 {
        return 0;
    }
    let mut previous = start - 1;
    while previous > 0 && !text.is_char_boundary(previous) {
        previous -= 1;
    }
    previous
}

/// The byte range of the character after `offset`, for Delete.
fn next_boundary(text: &str, offset: usize) -> usize {
    let start = clamp_to_char_boundary(text, offset);
    if start >= text.len() {
        return text.len();
    }
    let mut next = start + 1;
    while next < text.len() && !text.is_char_boundary(next) {
        next += 1;
    }
    next
}

impl Widget for TextInput {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        TextInput::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Adopt an external write to the buffer.
    ///
    /// This is where the shared buffer earns its keep. The widget did not make the
    /// change, so nothing else in the tree would repaint it, and a two-way binding
    /// where writing from the model does not update the field is the classic bug —
    /// so the widget compares what it last saw on every frame and reacts.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
        let now = self.text();
        if now == self.last_seen {
            return;
        }
        self.caret = clamp_to_char_boundary(&now, self.caret.min(now.len()));
        self.last_seen = now.clone();
        self.revision.bump();
        if let Some(on_change) = &self.on_change {
            on_change(now);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width(self.intrinsic_width())
                .height(self.intrinsic_height()),
        )
    }

    fn event(&mut self, cx: &mut dyn floom_core::EventContext, id: ElementId) -> bool {
        // A click focuses and puts the caret at the end. Not "at the click" — see
        // the module note: `EventContext` cannot measure text, so an x coordinate
        // cannot become a byte offset.
        if let Event::Pointer(pointer) = cx.event() {
            if pointer.phase == floom_core::PointerPhase::Down {
                cx.request_focus(id);
                self.set_focused(true);
                let end = self.text().len();
                self.set_caret(end);
                return true;
            }
            return false;
        }

        if let Event::Ime(ime) = cx.event().clone() {
            match ime {
                floom_core::ImeUpdate::OpenChanged(open) => {
                    if !open {
                        // Closing the panel commits: the composition becomes text.
                        if let Some(preedit) = self.preedit.take()
                            && !preedit.is_empty()
                        {
                            self.edit(|text, caret| {
                                text.insert_str(*caret, &preedit);
                                *caret += preedit.len();
                            });
                        }
                    }
                    self.revision.bump();
                    return true;
                }
                floom_core::ImeUpdate::Preedit { text, .. } => {
                    self.preedit = if text.is_empty() { None } else { Some(text) };
                    self.revision.bump();
                    return true;
                }
            }
        }

        let Event::Key(key) = cx.event() else {
            return false;
        };
        if key.phase != KeyPhase::Down {
            // `Up` is the only other phase the host delivers, and acting on it would
            // insert every character twice.
            return false;
        }

        match key.key {
            Key::Char(c) => {
                self.edit(|text, caret| {
                    text.insert(*caret, c);
                    *caret += c.len_utf8();
                });
                true
            }
            Key::Named(NamedKey::Backspace) => {
                self.edit(|text, caret| {
                    let start = previous_boundary(text, *caret);
                    if start < *caret {
                        text.replace_range(start..*caret, "");
                        *caret = start;
                    }
                });
                true
            }
            Key::Named(NamedKey::Delete) => {
                self.edit(|text, caret| {
                    let end = next_boundary(text, *caret);
                    if end > *caret {
                        text.replace_range(*caret..end, "");
                    }
                });
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                let start = previous_boundary(&self.text(), self.caret);
                self.set_caret(start);
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                let end = next_boundary(&self.text(), self.caret);
                self.set_caret(end);
                true
            }
            Key::Named(NamedKey::Home) => {
                self.set_caret(0);
                true
            }
            Key::Named(NamedKey::End) => {
                let end = self.text().len();
                self.set_caret(end);
                true
            }
            Key::Named(NamedKey::Enter) => {
                if let Some(on_enter) = &self.on_enter {
                    on_enter();
                }
                true
            }
            Key::Named(NamedKey::Escape) => {
                if let Some(on_escape) = &self.on_escape {
                    on_escape();
                }
                true
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        let style = self.style;

        if let Some(background) = style.background {
            if style.radius > 0.0 {
                cx.fill_rounded_rect(bounds, style.radius, background);
            } else {
                cx.fill_rect(bounds, background);
            }
        }
        if style.border_width > 0.0 {
            let border = f64::from(style.border_width);
            // A filled band rather than a stroke: `PaintContext` has `stroke_path`
            // but no `stroke_rect`, and the porting plan already settled this for
            // single-edge borders — a fill lands entirely inside the box, whereas a
            // stroke straddles the edge and would put half the outline outside the
            // field's own bounds.
            let top = Rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x1,
                y1: bounds.y0 + border,
            };
            let bottom = Rect {
                x0: bounds.x0,
                y0: bounds.y1 - border,
                x1: bounds.x1,
                y1: bounds.y1,
            };
            let left = Rect {
                x0: bounds.x0,
                y0: bounds.y0 + border,
                x1: bounds.x0 + border,
                y1: bounds.y1 - border,
            };
            let right = Rect {
                x0: bounds.x1 - border,
                y0: bounds.y0 + border,
                x1: bounds.x1,
                y1: bounds.y1 - border,
            };
            for edge in [top, bottom, left, right] {
                cx.fill_rect(edge, style.border_color);
            }
        }

        let (shown, color) = self.shown();
        let baseline = bounds.y0
            + style.padding_vert
            + f64::from(crate::controls::text_ascent_at(self.style.font_size));
        cx.fill_text(
            floom_types::Point::new(bounds.x0 + style.padding_horiz, baseline),
            &shown,
            style.font_size,
            style.font_weight,
            color,
        );

        if self.focused {
            // The caret sits after the text. Its exact column needs a measurement the
            // paint context does not offer, so it follows the whole shown string —
            // which is right for the common case of a field whose text is shorter than
            // its box, and drifts on a field that has scrolled, which this widget
            // cannot yet do.
            let width = crate::controls::text_width_at(&shown, style.font_size);
            let x = (bounds.x0 + style.padding_horiz) as f32 + width;
            let top = bounds.y0 + style.padding_vert;
            let bottom = bounds.y1 - style.padding_vert;
            cx.fill_rect(
                Rect {
                    x0: x as f64,
                    y0: top,
                    x1: x as f64 + f64::from(style.caret_width),
                    y1: bottom,
                },
                style.caret_color,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn floom_core::SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.hint(), cx);
    }
}

impl WidgetAny for TextInput {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(self.hint().into())
    }
}
