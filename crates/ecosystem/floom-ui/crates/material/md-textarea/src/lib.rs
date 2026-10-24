//! Material textarea, ported to the Floom 2.0 widget API.
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
//!   token's font size: the tokens;
//! * the text's ink: `on_surface`; the box's fill: `surface`; the rim:
//!   `outline`;
//! * the disabled palette: `on_surface` at 4 for the fill and `outline` at 97
//!   for the rim -- v1's numbers;
//! * the row height: the token's `min_height` plus one line per extra row,
//!   with a line being the token's font size plus 6.
//!
//! ## The numbers v1 wrote and never drew
//!
//! v1's `resolved_style` reported four numbers that the paint did not use,
//! and the port keeps all four and names them here rather than quietly
//! correcting them:
//!
//! * **the disabled ink at 97.** The style said `on_surface` at 97; the view
//!   painted the disabled text at `on_surface` at **38**. 38 is what the
//!   screen showed, so 38 is what the port paints, and the style still
//!   reports the 97 v1 claimed.
//! * **the font size.** The style said the *token's* font size; the view
//!   painted the text at the theme's `typography().body.medium.font_size`.
//!   The port paints at the typography's size and adds
//!   [`MaterialTextareaStyle::text_font_size`] to report it, because a style
//!   that does not say what the widget draws is not the widget's output. The
//!   dead `font_size` stays, because v1's style had it.
//! * **the rows.** The style's `min_height` was the token's, with no rows in
//!   it; the view sized the box by the rows. The port's style reports the
//!   token's number, as v1 did, and the box is sized by the rows.
//!
//! ## The theme
//!
//! v1's Material textarea read the *app-wide* theme signal, so a change of
//! theme re-themes it with everything else. The port keeps that.
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
//! * **The accessors.** v1's Material textarea had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;
use floom_widgets::{Align, SharedBuffer, Surface, TextInput, TextInputStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::textarea::TextareaProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// v1's disabled ink and rim, as the *style* reported them: `on_surface` at
/// 97.
const DISABLED_ALPHA: u8 = 97;
/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;
/// v1's disabled ink and rim, as the *view* painted them: `on_surface` at 38.
const DISABLED_TEXT_ALPHA: u8 = 38;
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
/// v1's Material textarea had a style struct, and the port keeps its fields.
/// Two of the numbers it reports are dead -- the disabled ink and the font
/// size -- and one is added; see the module doc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialTextareaStyle {
    pub design_system: &'static str,
    /// The text's ink.
    ///
    /// v1's style reported `on_surface` at 97 while disabled; the view
    /// painted 38. The dead number is kept and the paint follows the view.
    pub text_color: Color,
    /// The box's fill.
    pub background: Color,
    /// The box's rim.
    pub border_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    /// The token's minimum height, without the rows. v1's style reported this
    /// and the view sized the box by the rows instead.
    pub min_height: f32,
    /// The *token's* font size. v1's style reported it; the view painted the
    /// text at the theme's body size instead.
    pub font_size: f32,
    /// The size the text is actually painted at. Added by the port: v1's
    /// style did not report it.
    pub text_font_size: f32,
    pub max_width: f32,
    /// The height of one line, as v1 derived it: the token's font size plus
    /// 6.
    pub line_height: f32,
}

/// Material's textarea.
pub struct MaterialTextarea {
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

impl MaterialTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            buffer: Arc::new(Mutex::new(props.value.clone())),
            props,
            // The app-wide signal, as v1 did: Material is the only one of the
            // three kits whose textarea follows a theme swap at runtime.
            theme: theme_signal(),
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
    pub fn buffer(&self) -> SharedBuffer {
        self.buffer.clone()
    }

    /// The text, read.
    pub fn text(&self) -> String {
        read_text(&self.buffer)
    }

    pub fn resolved_style(&self) -> MaterialTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        let colors = theme.colors();
        let disabled = self.props.disabled;
        let line_height = tokens.font_size + LINE_EXTRA;
        let min_height =
            tokens.min_height + (self.props.rows.saturating_sub(1) as f32) * line_height;
        MaterialTextareaStyle {
            design_system: "material",
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
            background: if disabled {
                with_alpha(colors.on_surface, DISABLED_FILL_ALPHA)
            } else {
                colors.surface
            },
            border_color: if disabled {
                with_alpha(colors.outline, DISABLED_ALPHA)
            } else {
                colors.outline
            },
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            // Added by the port: the size the text is painted at.
            text_font_size: theme.typography().body.medium.font_size,
            max_width: tokens.max_width,
            line_height,
        }
    }

    /// The textarea: the box, and the field inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let buffer = self.buffer.clone();
        let on_change = self.on_change.clone();
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let font_size = style.text_font_size;
        let on_surface = self.theme.get().colors().on_surface;
        let text_color = to_paint(if disabled {
            with_alpha(on_surface, DISABLED_TEXT_ALPHA)
        } else {
            on_surface
        });
        // The field is as tall as the rows ask for, and the box grows around
        // it -- which is where v1 put the height.
        let field_height =
            style.min_height + (self.props.rows.saturating_sub(1) as f32) * style.line_height;
        let caret = buffer.lock().map(|text| text.len()).unwrap_or(0);

        let field = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().height(field_height).grow(1.0),
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

impl Default for MaterialTextarea {
    fn default() -> Self {
        Self::new()
    }
}
