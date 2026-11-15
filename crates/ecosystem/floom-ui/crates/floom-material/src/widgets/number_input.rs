use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Insets;
use floom_widgets::{
    Align, Axis, Pressable, SharedBuffer, Stack, Surface, Text, TextInput, TextInputStyle,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::number_input::NumberInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

pub type SharedNumber = Arc<Mutex<i64>>;

const LABEL_FONT_SIZE: f32 = 12.0;
const LABEL_GAP: f32 = 4.0;
const DISABLED_ALPHA: u8 = 38;
const DISABLED_FILL_ALPHA: u8 = 4;

fn read_number(number: &SharedNumber) -> i64 {
    number.lock().map(|guard| *guard).unwrap_or(0)
}

/// Write the shared number.
fn write_number(number: &SharedNumber, value: i64) {
    if let Ok(mut guard) = number.lock() {
        *guard = value;
    }
}

/// Write the shared text.
fn write_text(buffer: &SharedBuffer, text: &str) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.to_string();
    }
}

fn clamp_value(v: i64, min: Option<i64>, max: Option<i64>) -> i64 {
    let mut r = v;
    if let Some(m) = min {
        r = r.max(m);
    }
    if let Some(m) = max {
        r = r.min(m);
    }
    r
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialNumberInputStyle {
    pub design_system: &'static str,
    /// The text's ink.
    pub text_color: Color,
    /// The field's fill.
    pub background: Color,
    /// The field's rim.
    pub border_color: Color,
    /// The two stepper buttons' fill.
    pub button_background: Color,
    /// The two stepper buttons' ink.
    pub button_color: Color,
    /// The label's ink.
    pub label_color: Color,
    pub field_height: f32,
    pub field_width: f32,
    pub button_size: f32,
    pub padding_horizontal: f32,
    pub border_width: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub label_font_size: f32,
}

/// Material's number input.
pub struct MaterialNumberInput {
    props: NumberInputProps,
    theme: RwSignal<MultiTheme>,
    /// The field's text. The engine's shared buffer, because the engine's
    /// `TextInput` holds the text in one.
    buffer: SharedBuffer,
    /// The number the field holds. See [`SharedNumber`].
    number: SharedNumber,
    on_change: Option<Arc<dyn Fn(i64) + Send + Sync>>,
}

impl MaterialNumberInput {
    pub fn new() -> Self {
        let props = NumberInputProps::default();
        let number: SharedNumber = Arc::new(Mutex::new(props.value));
        let buffer: SharedBuffer = Arc::new(Mutex::new(props.value.to_string()));
        Self {
            props,
            theme: theme_signal(),
            buffer,
            number,
            on_change: None,
        }
    }

    /// The number the field starts with.
    pub fn value(mut self, value: i64) -> Self {
        self.props.value = value;
        let clamped = clamp_value(value, self.props.min, self.props.max);
        self.commit(clamped);
        self
    }

    /// Inclusive lower bound (clamped).
    pub fn min(mut self, min: i64) -> Self {
        self.props.min = Some(min);
        self.reclamp();
        self
    }

    /// Inclusive upper bound (clamped).
    pub fn max(mut self, max: i64) -> Self {
        self.props.max = Some(max);
        self.reclamp();
        self
    }

    pub fn step(mut self, step: i64) -> Self {
        self.props.step = step;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(i64) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &NumberInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The field's text, as a shared buffer the host can read and write.
    pub fn buffer(&self) -> SharedBuffer {
        self.buffer.clone()
    }

    /// The number the field holds, as a shared handle.
    pub fn number(&self) -> SharedNumber {
        self.number.clone()
    }

    /// The number the field holds, read.
    pub fn current_value(&self) -> i64 {
        read_number(&self.number)
    }

    /// Puts `value` in the field: the number, and the text that shows it.
    fn commit(&self, value: i64) {
        write_number(&self.number, value);
        write_text(&self.buffer, &value.to_string());
    }

    /// Re-clamps the number against the bounds, which may have moved.
    fn reclamp(&self) {
        let current = read_number(&self.number);
        let clamped = clamp_value(current, self.props.min, self.props.max);
        if clamped != current {
            self.commit(clamped);
        }
    }

    pub fn resolved_style(&self) -> MaterialNumberInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::NumberInput(tokens) = theme.component(ComponentId::NumberInput) else {
            unreachable!("ComponentId::NumberInput must resolve to NumberInputTokens")
        };
        let colors = theme.colors();
        let disabled = self.props.disabled;
        MaterialNumberInputStyle {
            design_system: "material",
            text_color: if disabled {
                with_alpha(colors.on_surface, DISABLED_ALPHA)
            } else {
                colors.on_surface
            },
            background: if disabled {
                with_alpha(colors.on_surface, DISABLED_FILL_ALPHA)
            } else {
                colors.surface
            },
            border_color: if disabled {
                with_alpha(colors.on_surface, DISABLED_ALPHA)
            } else {
                colors.outline
            },
            button_background: colors.surface_variant,
            button_color: colors.on_surface,
            label_color: colors.on_surface_variant,
            field_height: tokens.height,
            field_width: tokens.width,
            button_size: tokens.button_size,
            padding_horizontal: tokens.padding_horizontal,
            border_width: tokens.border,
            border_radius: tokens.radius,
            font_size: tokens.font_size,
            label_font_size: LABEL_FONT_SIZE,
        }
    }

    pub fn view(self) -> Stack {
        let style = self.resolved_style();

        self.reclamp();

        let number = self.number.clone();
        let buffer = self.buffer.clone();
        let on_change = self.on_change.clone();
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let disabled = self.props.disabled;
        let label = self.props.label.clone();

        let dec = {
            let number = number.clone();
            let buffer = buffer.clone();
            let on_change = on_change.clone();
            let size = style.button_size;
            let radius = style.border_radius;
            let background = to_paint(style.button_background);
            let ink = to_paint(style.button_color);
            move || {
                let tile = Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width(size).height(size),
                    {
                        let background = background;
                        let radius = radius;
                        let ink = ink;
                        move || {
                            Box::new(Surface::new().background(background).radius(radius).push(
                                move || Box::new(Text::new("-").color(ink)) as Box<dyn WidgetAny>,
                            )) as Box<dyn WidgetAny>
                        }
                    },
                );
                Box::new(
                    Pressable::new(tile)
                        .on_click({
                            let number = number.clone();
                            let buffer = buffer.clone();
                            let on_change = on_change.clone();
                            move || {
                                let current = read_number(&number);
                                let next = clamp_value(current - step, min, max);
                                write_number(&number, next);
                                write_text(&buffer, &next.to_string());
                                if let Some(callback) = &on_change {
                                    callback(next);
                                }
                            }
                        })
                        .disabled(disabled)
                        .label("Decrease"),
                ) as Box<dyn WidgetAny>
            }
        };

        let inc = {
            let number = number.clone();
            let buffer = buffer.clone();
            let on_change = on_change.clone();
            let size = style.button_size;
            let radius = style.border_radius;
            let background = to_paint(style.button_background);
            let ink = to_paint(style.button_color);
            move || {
                let tile = Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width(size).height(size),
                    {
                        let background = background;
                        let radius = radius;
                        let ink = ink;
                        move || {
                            Box::new(Surface::new().background(background).radius(radius).push(
                                move || Box::new(Text::new("+").color(ink)) as Box<dyn WidgetAny>,
                            )) as Box<dyn WidgetAny>
                        }
                    },
                );
                Box::new(
                    Pressable::new(tile)
                        .on_click({
                            let number = number.clone();
                            let buffer = buffer.clone();
                            let on_change = on_change.clone();
                            move || {
                                let current = read_number(&number);
                                let next = clamp_value(current + step, min, max);
                                write_number(&number, next);
                                write_text(&buffer, &next.to_string());
                                if let Some(callback) = &on_change {
                                    callback(next);
                                }
                            }
                        })
                        .disabled(disabled)
                        .label("Increase"),
                ) as Box<dyn WidgetAny>
            }
        };

        let text = {
            let buffer = buffer.clone();
            let number = number.clone();
            let on_change = on_change.clone();
            let font_size = style.font_size;
            let text_color = to_paint(style.text_color);
            let placeholder = label.clone().unwrap_or_default();
            let caret = buffer.lock().map(|text| text.len()).unwrap_or(0);
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
                    let number = number.clone();
                    let on_change = on_change.clone();
                    move |text: String| {
                        if disabled {
                            return;
                        }
                        let Ok(parsed) = text.trim().parse::<i64>() else {
                            return;
                        };
                        let clamped = clamp_value(parsed, min, max);
                        if clamped == read_number(&number) {
                            return;
                        }
                        write_number(&number, clamped);
                        if let Some(callback) = &on_change {
                            callback(clamped);
                        }
                    }
                });
                Box::new(input) as Box<dyn WidgetAny>
            }
        };

        let row = Stack::new(Axis::Horizontal)
            .align(Align::Start, Align::Center)
            .push_layout(
                LayoutRequest::new()
                    .width(style.button_size)
                    .height(style.button_size),
                dec,
            )
            .push_layout(LayoutRequest::new().grow(1.0), text)
            .push_layout(
                LayoutRequest::new()
                    .width(style.button_size)
                    .height(style.button_size),
                inc,
            );

        let field = Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.field_height)
                    .width(style.field_width)
                    .cross_align(Align::Center),
            )
            .padding(Insets {
                left: f64::from(style.padding_horizontal),
                right: f64::from(style.padding_horizontal),
                top: 0.0,
                bottom: 0.0,
            })
            .border(style.border_width, to_paint(style.border_color))
            .radius(style.border_radius)
            .background(to_paint(style.background))
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(row.clone()) as Box<dyn WidgetAny>
            });

        let mut column = Stack::new(Axis::Vertical)
            .gap(LABEL_GAP)
            .align(Align::Start, Align::Start);
        if let Some(label) = label {
            let ink = to_paint(style.label_color);
            column = column.push(move || {
                Box::new(Text::new(label.clone()).size(LABEL_FONT_SIZE).color(ink))
                    as Box<dyn WidgetAny>
            });
        }
        column = column.push(move || Box::new(field.clone()) as Box<dyn WidgetAny>);
        column
    }
}

impl Default for MaterialNumberInput {
    fn default() -> Self {
        Self::new()
    }
}
