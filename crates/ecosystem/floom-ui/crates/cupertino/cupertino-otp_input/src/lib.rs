//! Cupertino OTP input, ported to the Floom 2.0 widget API.
//!
//! An OTP input is a row of single-character boxes bound to one master code:
//! typing in a box writes that character into the code, and the host can read
//! or overwrite the whole code. When the input is masked, the box shows a dot
//! while the real character stays in the code.
//!
//! ## What the port uses
//!
//! The engine's [`TextInput`] for each box, the engine's `SharedBuffer` for
//! the master code, and the engine's vocabulary -- a `Surface` for each box, a
//! `Stack` for the row -- for everything else.
//!
//! ## What this kit answers
//!
//! * the box's size, the corner radius, the gap between the boxes and the
//!   character's size: the tokens;
//! * the character's ink: `on_surface`; the box's fill: `surface`; the rim:
//!   `outline`;
//! * the rim's width: 1, a v1 literal;
//! * the mask glyph: `•`, a v1 literal.
//!
//! ## The style struct v1 did not have
//!
//! v1's OTP input had no `resolved_style` at all: the colours were resolved
//! inside a `style` closure and never reported. The port adds
//! [`CupertinoOtpInputStyle`], which reports the tokens, the colours the paint
//! uses, and the two numbers that decide what the widget *is* -- how many
//! boxes there are and whether they are masked.
//!
//! ## The master code, and the one direction that does not come back
//!
//! v1's `code_signal()` was a signal the host could write, and a pair of
//! effects kept the boxes and the code in step: one read the boxes and wrote
//! the code, the other read the code and wrote the boxes. The port keeps the
//! first direction exactly -- typing in a box writes the code and fires
//! `on_change` -- and closes the second with [`CupertinoOtpInput::code_writer`],
//! a handle that writes the code *and* the boxes together.
//!
//! The gap that leaves is the *automatic* re-sync. v1's effect ran on every
//! change of the code, so a host that wrote the signal saw the boxes follow
//! on the next tick. In 2.0 a kit's `view()` is a composition of engine
//! widgets, and nothing in that composition observes an external write and
//! pushes it into the boxes: there is no per-frame hook a kit can reach. The
//! engine piece that would close it is a widget that owns the boxes and syncs
//! them in its own `update` -- which is `floom-ui-composites`' business, and
//! not a kit's.
//!
//! ## The quirk v1 had, and the port does not
//!
//! Because v1's two effects read each other, the code was always *packed to
//! the left*: a character typed into the third box with the first two empty
//! moved to the first box on the next tick, because the code held the
//! characters in order and the boxes took them from the code in order. The
//! port's `on_change` still reports the same string v1 did -- the boxes'
//! characters, left to right, with the empty ones left out -- but the
//! character stays in the box it was typed into, because nothing re-reads the
//! code back into the boxes.
//!
//! ## The theme
//!
//! v1's Cupertino OTP input took a *snapshot* of its own design system, so a
//! change of the app-wide theme did not reach it. The port keeps that.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **A disabled box still takes keystrokes.** v1's `TextInput` had a
//!   `disabled` flag; the engine's has none. The port keeps the *code* from
//!   moving while the input is disabled, and the box's own text still shows
//!   what was typed into it.
//! * **The cursor.** v1's boxes asked for no cursor at all, and the engine's
//!   `TextInput` answers with the platform arrow.
//! * **`justify_center` and `items_center`.** The engine's `TextInput` has no
//!   alignment, so the port centres the character by padding instead: the
//!   horizontal padding from the measured width of the glyph the box shows,
//!   the vertical from the font size. A character whose line box is taller
//!   than its size sits a little high, and the padding is exact for the
//!   digits an OTP is made of rather than for every glyph.
//! * **`with_theme()`.** v1's OTP input had `props()` and `theme()` but no
//!   `with_theme()`. The port gives all three kits the same three.

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, SharedBuffer, Stack, Surface, TextInput, TextInputStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::otp_input::OtpInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The glyph a masked box shows. A v1 literal.
const MASK: &str = "\u{2022}";

/// The rim's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;

/// The characters behind the boxes, one per box.
///
/// The mask hides the character on screen, so it has to live somewhere the
/// screen is not: the engine has a shared buffer for a string and a shared
/// flag for a bool, but nothing for a list of characters, so the kit keeps
/// them in a `Mutex<Vec<String>>` of its own.
type Reals = Arc<Mutex<Vec<String>>>;

/// Read the shared text.
fn read_text(buffer: &SharedBuffer) -> String {
    buffer.lock().map(|guard| guard.clone()).unwrap_or_default()
}

/// Write the shared text.
fn write_text(buffer: &SharedBuffer, text: &str) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.to_string();
    }
}

/// The characters behind the boxes.
fn read_reals(reals: &Reals) -> Vec<String> {
    reals.lock().map(|guard| guard.clone()).unwrap_or_default()
}

/// The OTP input's appearance, in the terms v1 used.
///
/// v1 had no style struct for this component, so this is the port's own, and
/// it reports what the paint actually uses.
#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoOtpInputStyle {
    pub design_system: &'static str,
    /// The character's ink.
    pub text_color: Color,
    /// The box's fill.
    pub background: Color,
    /// The box's rim.
    pub border_color: Color,
    /// How many boxes.
    pub length: usize,
    /// Whether the character is masked.
    pub masked: bool,
    pub box_size: f32,
    pub border_radius: f32,
    /// The rim's width. A v1 literal.
    pub border_width: f32,
    pub gap: f32,
    pub font_size: f32,
}

/// Cupertino's OTP input.
pub struct CupertinoOtpInput {
    props: OtpInputProps,
    theme: RwSignal<MultiTheme>,
    /// The master code: the whole string, the thing the host reads.
    code: SharedBuffer,
    /// The text each box shows. One buffer per box.
    boxes: Vec<SharedBuffer>,
    /// The characters behind the boxes, which the mask hides.
    reals: Reals,
    /// The kit's own `on_change`, `Send + Sync` so that a slot closure can
    /// hold it. The shared props' own slot stays empty: its callback is a
    /// `Box<dyn Fn(String)>`, which is neither `Send` nor `Sync`.
    on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl CupertinoOtpInput {
    pub fn new() -> Self {
        let props = OtpInputProps::default();
        let length = props.length.max(1);
        Self {
            props,
            // v1's Cupertino OTP input took a *snapshot* of its own design
            // system, so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            code: Arc::new(Mutex::new(String::new())),
            boxes: (0..length)
                .map(|_| Arc::new(Mutex::new(String::new())))
                .collect(),
            reals: Arc::new(Mutex::new(vec![String::new(); length])),
            on_change: None,
        }
    }

    /// How many boxes. v1 clamped it to at least one.
    pub fn length(mut self, length: usize) -> Self {
        self.props.length = length;
        let length = length.max(1);
        self.boxes = (0..length)
            .map(|_| Arc::new(Mutex::new(String::new())))
            .collect();
        self.reals = Arc::new(Mutex::new(vec![String::new(); length]));
        self
    }

    /// Whether the character is masked.
    pub fn masked(mut self, masked: bool) -> Self {
        self.props.masked = masked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the whole code whenever any box changes.
    ///
    /// The callback is `Send + Sync` here and only `'static` in v1: 2.0's slot
    /// closures are shared between threads, and a `Box<dyn Fn(String)>` cannot
    /// cross that boundary. The kit keeps its own `Arc` and the shared props'
    /// slot stays empty.
    pub fn on_change(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &OtpInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The master code, as a shared buffer the host can read.
    ///
    /// Writing this buffer does **not** re-sync the boxes: the code and the
    /// boxes are two different pieces of state, and only the writer below
    /// moves both. See the module doc.
    pub fn code(&self) -> SharedBuffer {
        self.code.clone()
    }

    /// The master code, read.
    pub fn code_text(&self) -> String {
        read_text(&self.code)
    }

    /// A handle that writes the whole code and re-syncs the boxes.
    ///
    /// v1's `code_signal()` was a signal the host could write, and an effect
    /// pushed the new code into the boxes. 2.0 has no signal to hand out -- a
    /// slot closure has to be `Send + Sync` and an `RwSignal` is `Rc`-based
    /// -- so the master code is a `SharedBuffer` and the *push* is this
    /// closure: it writes the code, puts each character in its box, and
    /// masks the boxes if the input is masked.
    pub fn code_writer(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let code = self.code.clone();
        let boxes = self.boxes.clone();
        let reals = self.reals.clone();
        let masked = self.props.masked;
        Arc::new(move |text: &str| {
            // The code first: the boxes' own handlers compare what they
            // recompute against it, and v1's code was the host's value until
            // the boxes had something to say about it.
            write_text(&code, text);
            let chars: Vec<char> = text.chars().collect();
            {
                let mut reals = reals.lock().unwrap_or_else(|e| e.into_inner());
                for (slot, ch) in reals.iter_mut().zip(chars.iter()) {
                    *slot = ch.to_string();
                }
                for slot in reals.iter_mut().skip(chars.len()) {
                    slot.clear();
                }
            }
            for (index, buffer) in boxes.iter().enumerate() {
                let ch = chars.get(index).map(|c| c.to_string()).unwrap_or_default();
                let target = if masked && !ch.is_empty() {
                    MASK.to_string()
                } else {
                    ch
                };
                write_text(buffer, &target);
            }
        })
    }

    pub fn resolved_style(&self) -> CupertinoOtpInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::OtpInput(tokens) = theme.component(ComponentId::OtpInput) else {
            unreachable!("ComponentId::OtpInput must resolve to OtpInputTokens")
        };
        let colors = theme.colors();
        CupertinoOtpInputStyle {
            design_system: "cupertino",
            text_color: colors.on_surface,
            background: colors.surface,
            border_color: colors.outline,
            length: self.props.length.max(1),
            masked: self.props.masked,
            box_size: tokens.box_size,
            border_radius: tokens.border_radius,
            border_width: BORDER_WIDTH,
            gap: tokens.gap,
            font_size: tokens.font_size,
        }
    }

    /// The OTP input: the row of boxes.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let disabled = self.props.disabled;
        let masked = self.props.masked;

        let mut row = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            // v1's row was `items_center`.
            .align(Align::Start, Align::Center);

        for index in 0..self.boxes.len() {
            let buffer = self.boxes[index].clone();
            let code = self.code.clone();
            let reals = self.reals.clone();
            let on_change = self.on_change.clone();
            let size = style.box_size;
            let radius = style.border_radius;
            let border_width = style.border_width;
            let border_color = to_paint(style.border_color);
            let background = to_paint(style.background);
            let font_size = style.font_size;
            let text_color = to_paint(style.text_color);
            // The character is centred by padding, because the engine's
            // `TextInput` has no alignment. The glyph the box shows is the
            // mask's dot when the input is masked and a digit otherwise.
            let shown = if masked { MASK } else { "0" };
            let padding_horiz =
                ((size - floom_widgets::text_width_at(shown, font_size)) / 2.0).max(0.0) as f64;
            let padding_vert = ((size - font_size) / 2.0).max(0.0) as f64;

            row = row.push_layout(LayoutRequest::new().width(size).height(size), move || {
                let field = Child::new(
                    ChildKey::index(0),
                    // The character takes the whole box.
                    LayoutRequest::new().grow(1.0),
                    {
                        let buffer = buffer.clone();
                        let code = code.clone();
                        let reals = reals.clone();
                        let on_change = on_change.clone();
                        move || {
                            let mut input = TextInput::new(buffer.clone()).placeholder("").style(
                                TextInputStyle {
                                    font_size,
                                    color: text_color,
                                    // The box carries the fill and the
                                    // rim; the text input is the text.
                                    background: None,
                                    border_width: 0.0,
                                    radius: 0.0,
                                    padding_horiz,
                                    padding_vert,
                                    ..TextInputStyle::default()
                                },
                            );
                            input = input.on_change({
                                let buffer = buffer.clone();
                                let code = code.clone();
                                let reals = reals.clone();
                                let on_change = on_change.clone();
                                move |text: String| {
                                    // A disabled input does not move: v1's
                                    // `TextInput` refused the keyboard, and
                                    // the engine's has no such flag.
                                    if disabled {
                                        return;
                                    }
                                    // v1 took the box's first character
                                    // and ignored the mask glyph itself.
                                    let ch = text
                                        .chars()
                                        .next()
                                        .map(|c| c.to_string())
                                        .unwrap_or_default();
                                    if ch == MASK {
                                        return;
                                    }
                                    // The character behind the box.
                                    {
                                        let mut reals =
                                            reals.lock().unwrap_or_else(|e| e.into_inner());
                                        if let Some(slot) = reals.get_mut(index) {
                                            if *slot != ch {
                                                *slot = ch.clone();
                                            }
                                        }
                                    }
                                    // The code is the boxes' characters,
                                    // left to right, with the empty ones
                                    // left out -- v1's rule.
                                    let new_code: String = read_reals(&reals)
                                        .into_iter()
                                        .filter(|slot| !slot.is_empty())
                                        .collect();
                                    let changed = {
                                        let mut code =
                                            code.lock().unwrap_or_else(|e| e.into_inner());
                                        if *code == new_code {
                                            false
                                        } else {
                                            *code = new_code.clone();
                                            true
                                        }
                                    };
                                    if changed {
                                        if let Some(callback) = &on_change {
                                            callback(new_code);
                                        }
                                    }
                                    // The mask goes back on. v1's code ->
                                    // boxes effect did this a tick later;
                                    // here it is immediate, because there
                                    // is no second effect to do it.
                                    if masked && !ch.is_empty() {
                                        write_text(&buffer, MASK);
                                    }
                                }
                            });
                            Box::new(input) as Box<dyn WidgetAny>
                        }
                    },
                );
                Box::new(
                    Surface::new()
                        .request(LayoutRequest::new().width(size).height(size))
                        .border(border_width, border_color)
                        .radius(radius)
                        .background(background)
                        .child(field),
                ) as Box<dyn WidgetAny>
            });
        }

        row
    }
}

impl Default for CupertinoOtpInput {
    fn default() -> Self {
        Self::new()
    }
}
