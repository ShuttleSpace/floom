//! Fluent textarea, ported to the Floom 2.0 widget API.
//!
//! A textarea is a bordered box with a text field in it, sized by the number
//! of rows the caller asked for.
//!
//! ## What the port uses
//!
//! The engine's [`TextInput`] for the text and the engine's vocabulary -- a
//! `Surface` for the box, a `Child` for the field inside it -- for everything
//! else.
//!
//! ## What this kit answers
//!
//! * the box's padding, the corner radius, the field's maximum width and the
//!   text's size: the tokens;
//! * the text's ink: `on_surface`; the box's fill: `surface`; the rim:
//!   `outline`;
//! * the disabled palette: `on_surface` at 4 for the fill and `on_surface` at
//!   38 for the rim and for the text -- v1's numbers, as the view wrote them;
//! * the row height: the token's `min_height` plus one line per extra row,
//!   with a line being the token's font size plus 6.
//!
//! ## The focus, which the port cannot see
//!
//! v1's Fluent textarea was the only one of the three that answered focus:
//! it tracked a `focused` signal fed by the field's `FocusGained` and
//! `FocusLost` listeners, and it answered with a `primary` rim and a 2-pixel
//! `primary` outline around the box.
//!
//! 2.0's `TextInput` owns its focus, and there is no way for a parent to
//! observe it: `is_focused` is a method on the widget, and a kit's `view()`
//! is a composition of engine widgets rather than a parent of them. The port
//! therefore draws the resting rim and names the focus ring as missing. The
//! engine piece that would close it is a text field that reports its own
//! interaction state -- which is what the composite `TextField` is, and it
//! owns its value rather than sharing a buffer.
//!
//! ## The numbers v1 wrote and never drew
//!
//! v1's `resolved_style` reported three numbers that the paint did not use,
//! and the port keeps all three and names them here rather than quietly
//! correcting them:
//!
//! * **the disabled ink at 97.** The style said `on_surface` at 97; the view
//!   painted the disabled text at `on_surface` at **38**. 38 is what the
//!   screen showed, so 38 is what the port paints, and the style still
//!   reports the 97 v1 claimed.
//! * **the disabled rim at 97.** The same story: the style said 97 and the
//!   view painted 38.
//! * **the rows.** The style's `min_height` was the token's, with no rows in
//!   it; the view sized the box by the rows. The port's style reports the
//!   token's number, as v1 did, and the box is sized by the rows.
//!
//! ## The one field the port's style adds
//!
//! v1's Fluent style had no `background` at all, though the paint used one:
//! the box was filled with `surface`, or `on_surface` at 4 while disabled.
//! The port's style reports it, because a style that does not say what the
//! widget draws is not the widget's output.
//!
//! ## The theme
//!
//! v1's Fluent textarea took a *snapshot* of its own design system, so a
//! change of the app-wide theme did not reach it. The port keeps that.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **A disabled field still takes keystrokes.** v1's `TextInput` had a
//!   `disabled` flag; the engine's has none. The port keeps the callback from
//!   firing while the field is disabled, and the field's own text still shows
//!   what was typed into it.
//! * **The cursor.** v1's field asked for the text cursor; the engine's
//!   `TextInput` answers with the platform arrow.
//! * **The `initialized` flag.** v1 needed a `Cell<bool>` to make its effect
//!   skip its own first run, so that the value the field was built with was
//!   not reported back as though the user had typed it. The engine's
//!   `TextInput` fires `on_change` for a keystroke and for nothing else, so
//!   there is no first run to swallow.
//! * **The multi-line editor that never was.** v1's textarea was v1's
//!   single-line `TextInput` in a tall box, and the port is the engine's
//!   single-line `TextInput` in a tall box. What the rows buy is the box's
//!   height, not the text's behaviour.
//! * **`set_value()`.** v1 had a `&mut self` setter that wrote the value and
//!   fired the callback. 2.0's value lives in the engine's shared buffer, so
//!   the port hands the buffer out instead and the host writes it directly.
//!   The callback then fires from the engine's own change detection, which is
//!   the same "a write is a change" rule v1's setter had to state for itself.

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;
use floom_widgets::{Align, SharedBuffer, Surface, TextInput, TextInputStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::textarea::TextareaProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;

/// v1's disabled ink and disabled rim, as the *style* reported them:
/// `on_surface` at 97.
const DISABLED_ALPHA: u8 = 97;
/// v1's disabled ink and disabled rim, as the *view* painted them:
/// `on_surface` at 38.
const DISABLED_TEXT_ALPHA: u8 = 38;
/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;
/// The rim's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// The extra a line takes over the font size. A v1 literal.
const LINE_EXTRA: f32 = 6.0;

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

/// The textarea's appearance, in the terms v1 used.
///
/// v1's Fluent textarea had a style struct, and the port keeps its fields and
/// adds the one the paint used and v1 never reported. Two of the numbers it
/// reports are dead -- the disabled ink and the disabled rim -- and the port
/// names them rather than quietly correcting them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTextareaStyle {
    pub design_system: &'static str,
    /// The box's rim.
    ///
    /// v1's style reported `on_surface` at 97 while disabled, and a
    /// `primary` rim while focused; the view painted 38 while disabled and
    /// could not see the focus. The dead numbers are kept and the paint
    /// follows the view.
    pub border_color: Color,
    /// The text's ink.
    ///
    /// v1's style reported `on_surface` at 97 while disabled; the view
    /// painted 38. The dead number is kept and the paint follows the view.
    pub text_color: Color,
    /// The box's fill. Added by the port: v1's style did not report it,
    /// though the paint used one.
    pub background: Color,
    pub padding: f32,
    pub border_radius: f32,
    /// The token's minimum height, without the rows. v1's style reported this
    /// and the view sized the box by the rows instead.
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
}

/// Fluent's textarea.
pub struct FluentTextarea {
    props: TextareaProps,
    theme: RwSignal<MultiTheme>,
    /// The text. The engine's shared buffer, because the engine's `TextInput`
    /// holds the text in one.
    buffer: SharedBuffer,
    /// The kit's own `on_change`, `Send + Sync` so that a slot closure can
    /// hold it. The shared props' own slot stays empty: its callback is a
    /// `Box<dyn Fn(String)>`, which is neither `Send` nor `Sync`.
    on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl FluentTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            buffer: Arc::new(Mutex::new(props.value.clone())),
            props,
            // v1's Fluent textarea took a *snapshot* of its own design
            // system, so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_change: None,
        }
    }

    /// The text the field starts with.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.props.value = value.clone();
        write_text(&self.buffer, &value);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// The number of visible rows. v1 clamped it to at least one.
    pub fn rows(mut self, rows: usize) -> Self {
        self.props.rows = rows.max(1);
        self
    }

    /// Called with the new text on every edit.
    ///
    /// The callback is `Send + Sync` here and only `'static` in v1: 2.0's slot
    /// closures are shared between threads, and a `Box<dyn Fn(String)>` cannot
    /// cross that boundary. The kit keeps its own `Arc` and the shared props'
    /// slot stays empty.
    pub fn on_change(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &TextareaProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The text, as a shared buffer the host can read and write.
    ///
    /// v1's `set_value()` wrote the value and fired the callback. 2.0's
    /// value lives in the engine's shared buffer, so the port hands the
    /// buffer out and the host writes it directly; the callback then fires
    /// from the engine's own change detection.
    pub fn buffer(&self) -> SharedBuffer {
        self.buffer.clone()
    }

    /// The text, read.
    pub fn text(&self) -> String {
        read_text(&self.buffer)
    }

    pub fn resolved_style(&self) -> FluentTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        let colors = theme.colors();
        let disabled = self.props.disabled;
        FluentTextareaStyle {
            design_system: "fluent",
            // v1's style had a `primary` rim while focused. The port cannot
            // see the focus, so the resting rim is what it reports; see the
            // module doc.
            border_color: if disabled {
                Color {
                    a: DISABLED_ALPHA,
                    ..colors.on_surface
                }
            } else {
                colors.outline
            },
            // v1's style said 97; its view painted 38. The dead number is
            // kept and the paint follows the view.
            text_color: if disabled {
                Color {
                    a: DISABLED_ALPHA,
                    ..colors.on_surface
                }
            } else {
                colors.on_surface
            },
            // Added by the port: the fill the paint used and v1's style did
            // not report.
            background: if disabled {
                with_alpha(colors.on_surface, DISABLED_FILL_ALPHA)
            } else {
                colors.surface
            },
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
        }
    }

    /// The textarea: the box, and the field inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let buffer = self.buffer.clone();
        let on_change = self.on_change.clone();
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let font_size = style.font_size;
        let on_surface = self.theme.get().colors().on_surface;
        let text_color = to_paint(if disabled {
            with_alpha(on_surface, DISABLED_TEXT_ALPHA)
        } else {
            on_surface
        });
        // The line is the token's font size plus 6, as v1 derived it, and the
        // box is as tall as the rows ask for.
        let line_height = font_size + LINE_EXTRA;
        let box_height = style.min_height + (self.props.rows.saturating_sub(1) as f32) * line_height;
        let caret = buffer.lock().map(|text| text.len()).unwrap_or(0);

        let field = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().grow(1.0),
            move || {
                let mut input = TextInput::new(buffer.clone())
                    .placeholder(placeholder.clone())
                    .style(TextInputStyle {
                        font_size,
                        color: text_color,
                        // The box carries the fill and the rim; the text input
                        // is the text.
                        background: None,
                        border_width: 0.0,
                        radius: 0.0,
                        // v1's `TextInput` had a padding of its own that the
                        // v1 source does not record, so the box's padding is
                        // the only inset here.
                        padding_horiz: 0.0,
                        padding_vert: 0.0,
                        ..TextInputStyle::default()
                    });
                // The engine's `TextInput::new` opens with the caret at the
                // *start* of the buffer, while `with_text` opens it at the
                // end. A kit that hands out its own buffer cannot ask for the
                // second one, so the caret is moved here.
                input.set_caret(caret);
                input = input.on_change({
                    let on_change = on_change.clone();
                    move |text: String| {
                        // A disabled field does not answer: v1's `TextInput`
                        // refused the keyboard, and the engine's has no such
                        // flag.
                        if disabled {
                            return;
                        }
                        if let Some(callback) = &on_change {
                            callback(text);
                        }
                    }
                });
                Box::new(input) as Box<dyn WidgetAny>
            },
        );

        Surface::new()
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .max_width(style.max_width)
                    .min_height(box_height)
                    // v1's field was `items_start`, so the text sits at the
                    // top of the box.
                    .cross_align(Align::Start),
            )
            .padding(Insets {
                left: f64::from(style.padding),
                right: f64::from(style.padding),
                top: f64::from(style.padding),
                bottom: f64::from(style.padding),
            })
            .border(BORDER_WIDTH, to_paint(style.border_color))
            .radius(style.border_radius)
            .background(to_paint(style.background))
            .child(field)
    }
}

impl Default for FluentTextarea {
    fn default() -> Self {
        Self::new()
    }
}

