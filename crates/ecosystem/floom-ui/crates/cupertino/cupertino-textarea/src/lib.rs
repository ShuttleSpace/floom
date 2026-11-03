//! Cupertino textarea, ported to the Floom 2.0 widget API.
//!

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

/// The height of one line. A v1 literal, and *not* the font size: v1's
/// Cupertino textarea used a constant while its two siblings derived the line
/// from the font.
const LINE_HEIGHT: f32 = 20.0;
/// v1's disabled rim and disabled text: `on_surface` at 38.
const DISABLED_ALPHA: u8 = 38;
/// v1's disabled text in the *view*: `on_surface` at 97.
const DISABLED_TEXT_ALPHA: u8 = 97;
/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;
/// The rim's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;

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
/// v1's Cupertino textarea had a style struct, and the port keeps its fields.
/// Two of the numbers it reports are dead -- the disabled ink and the rows --
/// and the port names them rather than quietly correcting them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTextareaStyle {
    pub design_system: &'static str,
    /// The box's rim.
    pub border_color: Color,
    /// The text's ink.
    ///
    /// v1's style reported `on_surface` with no disabled case; the view
    /// painted 97 while disabled. The dead number is kept and the paint
    /// follows the view.
    pub text_color: Color,
    /// The box's fill.
    pub background: Color,
    pub padding: f32,
    pub border_radius: f32,
    /// The token's minimum height, without the rows. v1's style reported this
    /// and the view sized the box by the rows instead.
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
}

/// Cupertino's textarea.
pub struct CupertinoTextarea {
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

impl CupertinoTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            buffer: Arc::new(Mutex::new(props.value.clone())),
            props,
            // v1's Cupertino textarea took a *snapshot* of its own design
            // system, so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
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
    /// v1's `value_signal()` was an `RwSignal<String>`, which cannot cross a
    /// `Send + Sync` slot boundary; this is the engine's shared buffer, which
    /// is the same handle under a name that can.
    pub fn buffer(&self) -> SharedBuffer {
        self.buffer.clone()
    }

    /// The text, read.
    pub fn text(&self) -> String {
        read_text(&self.buffer)
    }

    pub fn resolved_style(&self) -> CupertinoTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        let colors = theme.colors();
        let disabled = self.props.disabled;
        CupertinoTextareaStyle {
            design_system: "cupertino",
            border_color: if disabled {
                with_alpha(colors.on_surface, DISABLED_ALPHA)
            } else {
                colors.outline
            },
            // v1's style had no disabled case here at all; the view painted
            // 97. The dead number is kept and the paint follows the view.
            text_color: colors.on_surface,
            background: if disabled {
                with_alpha(colors.on_surface, DISABLED_FILL_ALPHA)
            } else {
                colors.surface_variant
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
        // The box is as tall as the rows ask for -- which is where v1 put the
        let box_height =
            style.min_height + (self.props.rows.saturating_sub(1) as f32) * LINE_HEIGHT;
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
                        background: None,
                        border_width: 0.0,
                        radius: 0.0,
                        // v1's `TextInput` had a padding of its own that the
                        // v1 source does not record, so the box's padding is
                        padding_horiz: 0.0,
                        padding_vert: 0.0,
                        ..TextInputStyle::default()
                    });
                // The engine's `TextInput::new` opens with the caret at the
                // *start* of the buffer, while `with_text` opens it at the
                // end. A kit that hands out its own buffer cannot ask for the
                input.set_caret(caret);
                input = input.on_change({
                    let on_change = on_change.clone();
                    move |text: String| {
                        // A disabled field does not answer: v1's `TextInput`
                        // refused the keyboard, and the engine's has no such
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

impl Default for CupertinoTextarea {
    fn default() -> Self {
        Self::new()
    }
}
