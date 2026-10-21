//! Fluent number input, ported to the Floom 2.0 widget API.
//!
//! A number input is a text field with a `-` and a `+` button beside it: the
//! text can be edited freely, and the two buttons step the number.
//!
//! ## What the port uses
//!
//! The engine's [`TextInput`] for the text, the engine's `Pressable` for the
//! two buttons, and the engine's vocabulary -- a `Surface` for the field's box,
//! a `Stack` for the row and for the column -- for everything else. There is
//! nothing here that a widget could own and the vocabulary does not already
//! say.
//!
//! ## What this kit answers
//!
//! * the field's height and width, the buttons' size, the field's horizontal
//!   padding, the rim's width, the corner radius and the text's size: the
//!   tokens;
//! * the text's ink: `on_surface`; the field's fill: `surface`; the rim:
//!   `outline`; the buttons' fill: `surface_variant` and their ink:
//!   `on_surface`; the label's ink: `on_surface_variant`;
//! * the disabled palette: `on_surface` at 38 for the ink and for the rim,
//!   and `on_surface` at 4 for the fill. v1's numbers, read out of the style
//!   closures v1 wrote them in.
//!
//! ## The style struct v1 did not have
//!
//! v1's number input had no `resolved_style` at all: every colour was
//! resolved inside a `style` closure, so nothing reported what the widget
//! drew. The port adds [`FluentNumberInputStyle`], which reports the tokens
//! and the colours the paint uses -- including the two v1 never named, the
//! buttons' fill and the label's ink, each of which v1 wrote in a closure
//! and never surfaced.
//!
//! ## The number is a shared number, and the engine has none
//!
//! v1 kept the value in an `RwSignal<i64>` beside the text's
//! `RwSignal<String>`. 2.0 cannot: a slot closure has to be `Send + Sync`,
//! and an `RwSignal` is `Rc`-based. The text is the engine's `SharedBuffer`;
//! the number is [`SharedNumber`], which is `Arc<Mutex<i64>>` -- the same
//! shape as the engine's three shared types, and the piece the engine is
//! missing.
//!
//! ## The theme
//!
//! v1's Fluent number input took a *snapshot* of its own design system, so a
//! change of the app-wide theme did not reach it. The port keeps that.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **A disabled field still takes keystrokes.** v1's `TextInput` had a
//!   `disabled` flag and refused the keyboard; the engine's has none. The port
//!   keeps the *value* and the callback from moving while the field is
//!   disabled, and the field's own text still shows what was typed into it.
//! * **The cursor.** v1's field asked for the text cursor and its buttons for
//!   the pointer. Neither the engine's `TextInput` nor its `Pressable` answers
//!   the cursor question, so both show the platform arrow.
//! * **`width_pct(100.0)` on the field's text.** The port asks the text to
//!   grow instead, which is the flexbox way of saying "take the rest of the
//!   row": a percentage of the row's width would be the whole row *and* the
//!   two buttons beside it.
//! * **The accessors.** v1's number input had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.

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
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The number the field holds.
///
/// **The engine has no shared number.** `floom_widgets` names three shared
/// shapes -- `SharedBuffer` (a string), `SharedFlag` (a bool) and `SharedMap`
/// (a map) -- and a number is the fourth shape this component needs: v1 held
/// the value in an `RwSignal<i64>` next to the text's `RwSignal<String>`, and
/// 2.0 cannot, because a slot closure has to be `Send + Sync` and an
/// `RwSignal` is `Rc`-based.
///
/// This is the same shape as the engine's three, spelled out here so that the
/// kit compiles today. When the engine grows a `SharedNumber` of its own,
/// this alias is the one line that changes.
pub type SharedNumber = Arc<Mutex<i64>>;

/// The label's size. A v1 literal.
const LABEL_FONT_SIZE: f32 = 12.0;
/// The gap between the label and the field. A v1 literal.
const LABEL_GAP: f32 = 4.0;
/// v1's disabled ink and disabled rim: `on_surface` at 38.
const DISABLED_ALPHA: u8 = 38;
/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;

/// Read the shared number. A poisoned mutex reads as zero rather than
/// panicking: the widget would be stuck either way, and a paint pass over a
/// number nobody is looking at should not take the app down.
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

/// v1's `clamp_value`, unchanged: the lower bound first, then the upper one,
/// so a range that is itself inverted resolves to the upper bound.
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

/// The number input's appearance, in the terms v1 used.
///
/// v1 had no style struct for this component -- the colours were resolved
/// inside `style` closures and never reported -- so this is the port's own,
/// and it reports what the paint actually uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentNumberInputStyle {
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
    /// The label's size. A v1 literal.
    pub label_font_size: f32,
}

/// Fluent's number input.
pub struct FluentNumberInput {
    props: NumberInputProps,
    theme: RwSignal<MultiTheme>,
    /// The field's text. The engine's shared buffer, because the engine's
    /// `TextInput` holds the text in one.
    buffer: SharedBuffer,
    /// The number the field holds. See [`SharedNumber`].
    number: SharedNumber,
    /// The kit's own `on_change`, `Send + Sync` so that a slot closure can
    /// hold it. The shared props' own slot stays empty: its callback is a
    /// `Box<dyn Fn(i64)>`, which is neither `Send` nor `Sync`.
    on_change: Option<Arc<dyn Fn(i64) + Send + Sync>>,
}

impl FluentNumberInput {
    pub fn new() -> Self {
        let props = NumberInputProps::default();
        let number: SharedNumber = Arc::new(Mutex::new(props.value));
        let buffer: SharedBuffer = Arc::new(Mutex::new(props.value.to_string()));
        Self {
            props,
            // v1's Fluent number input took a *snapshot* of its own design
            // system, so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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

    /// The floating label. v1 used it as the field's *placeholder* as well as
    /// the label above the field.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the new value whenever it changes.
    ///
    /// The callback is `Send + Sync` here and only `'static` in v1: 2.0's slot
    /// closures are shared between threads, and a `Box<dyn Fn(i64)>` cannot
    /// cross that boundary. The kit keeps its own `Arc` and the shared props'
    /// slot stays empty.
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

    pub fn resolved_style(&self) -> FluentNumberInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::NumberInput(tokens) = theme.component(ComponentId::NumberInput) else {
            unreachable!("ComponentId::NumberInput must resolve to NumberInputTokens")
        };
        let colors = theme.colors();
        let disabled = self.props.disabled;
        FluentNumberInputStyle {
            design_system: "fluent",
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
            // v1 dimmed the field and the text but not the two stepper
            // buttons: their style closure never mentioned `disabled`.
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

    /// The number input: the field, and the label above it when there is one.
    ///
    /// v1 returned the bare field when there was no label and a two-child
    /// column when there was. The port always returns the column, which holds
    /// only the field when there is no label: the same pixels, one more
    /// widget.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();

        // The bounds may have arrived after the value, so the number is
        // clamped once more here -- v1 clamped at mount time too.
        self.reclamp();

        let number = self.number.clone();
        let buffer = self.buffer.clone();
        let on_change = self.on_change.clone();
        let min = self.props.min;
        let max = self.props.max;
        let step = self.props.step;
        let disabled = self.props.disabled;
        let label = self.props.label.clone();

        // The `-` button. v1 hung a click listener on a container; 2.0's
        // clickable surface is a `Pressable`, which paints nothing and owns
        // the interaction, so the tile it holds is the v1 container.
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
                            Box::new(
                                Surface::new()
                                    .background(background)
                                    .radius(radius)
                                    .push(move || {
                                        Box::new(Text::new("-").color(ink)) as Box<dyn WidgetAny>
                                    }),
                            ) as Box<dyn WidgetAny>
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

        // The `+` button.
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
                            Box::new(
                                Surface::new()
                                    .background(background)
                                    .radius(radius)
                                    .push(move || {
                                        Box::new(Text::new("+").color(ink)) as Box<dyn WidgetAny>
                                    }),
                            ) as Box<dyn WidgetAny>
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

        // The text between the two buttons.
        let text = {
            let buffer = buffer.clone();
            let number = number.clone();
            let on_change = on_change.clone();
            let font_size = style.font_size;
            let text_color = to_paint(style.text_color);
            let placeholder = label.clone().unwrap_or_default();
            // The engine's `TextInput::new` opens with the caret at the
            // *start* of the buffer, while `with_text` opens it at the end.
            // A kit that hands out its own buffer cannot ask for the second
            // one, so the caret is moved here -- v1's field opened with the
            // caret after the value.
            let caret = buffer.lock().map(|text| text.len()).unwrap_or(0);
            move || {
                let mut input = TextInput::new(buffer.clone())
                    .placeholder(placeholder.clone())
                    .style(TextInputStyle {
                        font_size,
                        color: text_color,
                        // The field's box carries the fill and the rim; the
                        // text input is the text.
                        background: None,
                        border_width: 0.0,
                        radius: 0.0,
                        // v1's `TextInput` had a padding of its own that the
                        // v1 source does not record, so the field's padding is
                        // the only inset here.
                        padding_horiz: 0.0,
                        padding_vert: 0.0,
                        ..TextInputStyle::default()
                    });
                input.set_caret(caret);
                input = input.on_change({
                    let number = number.clone();
                    let on_change = on_change.clone();
                    move |text: String| {
                        // A disabled field does not move: v1's `TextInput`
                        // refused the keyboard, and the engine's has no such
                        // flag.
                        if disabled {
                            return;
                        }
                        // v1 parsed the text and ignored anything that was
                        // not a number, leaving the value where it was.
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

        // The field's row: the two steppers and the text between them.
        let row = Stack::new(Axis::Horizontal)
            // v1's field was `items_center`, so the row sat on the field's
            // middle line.
            .align(Align::Start, Align::Center)
            .push_layout(
                LayoutRequest::new()
                    .width(style.button_size)
                    .height(style.button_size),
                dec,
            )
            .push_layout(
                // The text takes the rest of the row. v1 wrote
                // `width_pct(100.0)`, which in a row asks for the whole row
                // *and* the two buttons beside it.
                LayoutRequest::new().grow(1.0),
                text,
            )
            .push_layout(
                LayoutRequest::new()
                    .width(style.button_size)
                    .height(style.button_size),
                inc,
            );

        // The field: the row in a box.
        let field = Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.field_height)
                    .width(style.field_width)
                    // v1's field was `items_center`.
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

        // The column: the label, then the field.
        let mut column = Stack::new(Axis::Vertical)
            .gap(LABEL_GAP)
            .align(Align::Start, Align::Start);
        if let Some(label) = label {
            let ink = to_paint(style.label_color);
            column = column.push(move || {
                Box::new(
                    Text::new(label.clone())
                        .size(LABEL_FONT_SIZE)
                        .color(ink),
                ) as Box<dyn WidgetAny>
            });
        }
        column = column.push(move || Box::new(field.clone()) as Box<dyn WidgetAny>);
        column
    }
}

impl Default for FluentNumberInput {
    fn default() -> Self {
        Self::new()
    }
}

