
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
use floom_widget_runtime::theme_signal;

const MASK: &str = "\u{2022}";

const BORDER_WIDTH: f32 = 1.0;

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

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialOtpInputStyle {
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
    pub border_width: f32,
    pub gap: f32,
    pub font_size: f32,
}

/// Material's OTP input.
pub struct MaterialOtpInput {
    props: OtpInputProps,
    theme: RwSignal<MultiTheme>,
    /// The master code: the whole string, the thing the host reads.
    code: SharedBuffer,
    /// The text each box shows. One buffer per box.
    boxes: Vec<SharedBuffer>,
    /// The characters behind the boxes, which the mask hides.
    reals: Reals,
    on_change: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl MaterialOtpInput {
    pub fn new() -> Self {
        let props = OtpInputProps::default();
        let length = props.length.max(1);
        Self {
            props,
            theme: theme_signal(),
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

    pub fn code(&self) -> SharedBuffer {
        self.code.clone()
    }

    /// The master code, read.
    pub fn code_text(&self) -> String {
        read_text(&self.code)
    }

    pub fn code_writer(&self) -> Arc<dyn Fn(&str) + Send + Sync> {
        let code = self.code.clone();
        let boxes = self.boxes.clone();
        let reals = self.reals.clone();
        let masked = self.props.masked;
        Arc::new(move |text: &str| {
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

    pub fn resolved_style(&self) -> MaterialOtpInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::OtpInput(tokens) = theme.component(ComponentId::OtpInput) else {
            unreachable!("ComponentId::OtpInput must resolve to OtpInputTokens")
        };
        let colors = theme.colors();
        MaterialOtpInputStyle {
            design_system: "material",
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
            let shown = if masked { MASK } else { "0" };
            let padding_horiz =
                ((size - floom_widgets::text_width_at(shown, font_size)) / 2.0).max(0.0) as f64;
            let padding_vert = ((size - font_size) / 2.0).max(0.0) as f64;

            row = row.push_layout(LayoutRequest::new().width(size).height(size), move || {
                let field = Child::new(
                    ChildKey::index(0),
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
                                    if disabled {
                                        return;
                                    }
                                    let ch = text
                                        .chars()
                                        .next()
                                        .map(|c| c.to_string())
                                        .unwrap_or_default();
                                    if ch == MASK {
                                        return;
                                    }
                                    {
                                        let mut reals =
                                            reals.lock().unwrap_or_else(|e| e.into_inner());
                                        if let Some(slot) = reals.get_mut(index) {
                                            if *slot != ch {
                                                *slot = ch.clone();
                                            }
                                        }
                                    }
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

impl Default for MaterialOtpInput {
    fn default() -> Self {
        Self::new()
    }
}
