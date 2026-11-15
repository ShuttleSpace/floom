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

const DISABLED_ALPHA: u8 = 97;
const DISABLED_FILL_ALPHA: u8 = 4;
const DISABLED_TEXT_ALPHA: u8 = 38;
const BORDER_WIDTH: f32 = 1.0;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialTextareaStyle {
    pub design_system: &'static str,
    pub text_color: Color,
    /// The box's fill.
    pub background: Color,
    /// The box's rim.
    pub border_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub text_font_size: f32,
    pub max_width: f32,
    pub line_height: f32,
}

/// Material's textarea.
pub struct MaterialTextarea {
    props: TextareaProps,
    theme: RwSignal<MultiTheme>,
    /// The text. The engine's shared buffer, because the engine's `TextInput`
    /// holds the text in one.
    buffer: SharedBuffer,
    on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl MaterialTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            buffer: Arc::new(Mutex::new(props.value.clone())),
            props,
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

    pub fn rows(mut self, rows: usize) -> Self {
        self.props.rows = rows.max(1);
        self
    }

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
                        background: None,
                        border_width: 0.0,
                        radius: 0.0,
                        padding_horiz: 0.0,
                        padding_vert: 0.0,
                        ..TextInputStyle::default()
                    });
                input.set_caret(caret);
                input = input.on_change({
                    let on_change = on_change.clone();
                    move |text: String| {
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
