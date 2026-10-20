//! Fluent colour picker, ported to the Floom 2.0 widget API.
//!
//! A colour picker is a square of colour -- saturation across, value down --
//! under a hue strip and an alpha strip, with a preview of the chosen colour
//! and a field that spells it out in hex.
//!
//! ## What the port uses
//!
//! The engine's brush. v1 drew the square and the two strips as inline SVG
//! documents -- a `<linearGradient>` for the white ramp, another for the black
//! one, and a third for the hue -- and handed the markup to an SVG widget.
//! 2.0 has no SVG pipeline, but it does have a linear brush, so the port
//! paints the same gradients with the same stops at the same offsets: the
//! picture is the same, the mechanism is not.
//!
//! ## What this kit answers
//!
//! * the square's size, the strips' height, the thumb's size and the corner
//!   radius: the tokens;
//! * the picker's fill: `surface`; its border: `outline` at 80;
//! * the hex field's ink: `on_surface`; its placeholder's: `on_surface_variant`;
//! * the thumb's border: white.
//!
//! ## The numbers v1 wrote as literals
//!
//! The picker's padding (12), the gap under the square and under each strip
//! (12), the gap between the preview and the field (12), the preview's size
//! (40), the field's width (120) and its text size (14), the border's
//! thickness (1), and the thumb's border (2).
//!
//! ## The differences this port names rather than reproduces
//!
//! * **The square's box.** v1's outer box was `width(sv_width)` with a padding
//!   of 12, so its content box was 24 narrower than the 240-wide square it
//!   held and the square overflowed it. The port's outer box is
//!   `sv_width + 24` wide, so the square fits inside the padding.
//! * **The hex is committed as it is typed.** v1 committed the field on
//!   focus-lost; 2.0's text input has no focus-lost event, so the port
//!   commits whenever the text parses -- and only when the parsed colour is
//!   not the one already held, so a colour dragged in the square does not
//!   report itself twice.
//! * **A drag that leaves the area ends.** v1 asked the host for pointer
//!   capture, so a drag that ran off the square kept tracking. 2.0's
//!   `EventContext` has no capture door, so the drag ends where the pointer
//!   leaves the area.
//!
//! ## The theme source
//!
//! v1's Fluent colour picker took a *snapshot* of its own design system --
//! `MultiTheme::new_for(DesignSystem::Fluent)` -- so a change of the
//! app-wide theme does not reach it. The port keeps that.

//! ## What v1 wrote that this port does not carry
//!
//! * **`color_signal()` was an `RwSignal<Color>`.** 2.0's shared state is a
//!   mutex-guarded cell, the same shape the engine's `SharedFlag` and
//!   `SharedBuffer` have, because a value that changes after mount cannot be
//!   held in a signal.
//! * **The accessors.** v1's Fluent colour picker had no `props()`,
//!   `theme()` or `with_theme()`. The port gives all three kits the same
//!   three, because a kit that cannot be given a theme cannot be tested
//!   against one.

use std::sync::{Arc, Mutex};

use floom_core::{
    CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext, Point, Rect,
    UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Brush, Color as PaintColor, CursorStyle, Insets, PointerButton, PointerPhase};
use floom_widgets::primitives::Revision;
use floom_widgets::{Align, Axis, Stack, Surface, TextInput, TextInputStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::color_picker::ColorPickerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::{with_alpha, WHITE};

/// The hex field's placeholder, as v1 wrote it.
const HEX_PLACEHOLDER: &str = "#rrggbb";

/// The chosen colour, shared between the kit and the tree it built.
///
/// v1 held an `RwSignal<Color>` and a `RwSignal<String>` for the hex. 2.0's
/// slots are shared between threads and an `RwSignal` is `Rc`-based, so a
/// value that changes after mount cannot be a signal; these are
/// mutex-guarded cells instead, the same shape the engine's `SharedFlag` and
/// `SharedBuffer` have.
pub type SharedColor = Arc<Mutex<Color>>;

/// The hex text, shared with the text input.
pub type SharedHex = Arc<Mutex<String>>;

/// The colour v1 started on.
const DEFAULT_COLOR: Color = Color::rgb(33, 150, 243);

// ----- HSV <-> RGB helpers (pure; safe to swap across design systems) -----

fn rgb_to_hsv(c: Color) -> (f64, f64, f64) {
    let r = f64::from(c.r) / 255.0;
    let g = f64::from(c.g) / 255.0;
    let b = f64::from(c.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let v = max;
    let s = if max <= 1e-6 { 0.0 } else { d / max };
    let h = if d <= 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let h = h * 60.0;
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Color {
    let h = (h % 360.0 + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color::rgb(
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn color_to_hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn hex_to_color(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    let s = if s.len() == 8 { &s[0..6] } else { s };
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::rgb(r, g, b))
}

/// The chosen colour, treating a poisoned cell as the default.
fn read_color(color: &SharedColor) -> Color {
    match color.lock() {
        Ok(guard) => *guard,
        Err(_) => DEFAULT_COLOR,
    }
}

/// Sets the colour, the hex text and tells the caller -- in v1's order.
fn commit_color(
    color: &SharedColor,
    hex: &SharedHex,
    on_change: &Option<Arc<dyn Fn(Color) + Send + Sync>>,
    value: Color,
) {
    if let Ok(mut guard) = color.lock() {
        *guard = value;
    }
    let text = color_to_hex(value);
    if let Ok(mut buffer) = hex.lock() {
        if *buffer != text {
            *buffer = text;
        }
    }
    if let Some(callback) = on_change {
        callback(value);
    }
}

/// The colour picker's appearance, in the terms v1 used.
///
/// The first block is v1's own list. The second block is what v1's paint used
/// but v1's style never reported -- the padding, the border's thickness, the
/// margins, the preview's size and the field's width and size. A style that
/// does not say what the widget draws is not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentColorPickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub outline: Color,
    pub on_surface: Color,
    pub sv_width: f32,
    pub sv_height: f32,
    pub slider_height: f32,
    pub thumb_size: f32,
    pub radius: f32,
    // --- what v1's paint used and v1's style did not report ---
    /// The hex field's ink.
    pub on_surface_variant: Color,
    /// The picker's padding. v1's `.padding(12.0)`.
    pub padding: f32,
    /// The border's thickness. v1's `.border(1.0)`.
    pub border_width: f32,
    /// The border's alpha. v1's `with_alpha(outline, 80)`.
    pub border_alpha: u8,
    /// The gap above each of the strips and above the preview row.
    /// v1's `.margin_top(12.0)`.
    pub margin_top: f32,
    /// The gap between the preview and the hex field. v1's `.gap(12.0)`.
    pub row_gap: f32,
    /// The preview's size. v1's `.size(40.0, 40.0)`.
    pub preview_size: f32,
    /// The hex field's width. v1's `.width(120.0)`.
    pub hex_width: f32,
    /// The hex field's text size. v1's `.font_size(14.0)`.
    pub hex_font_size: f32,
    /// The thumb's border. v1's `.border(2.0)`.
    pub thumb_border_width: f32,
    /// The thumb's border colour. v1's `WHITE`.
    pub thumb_border_color: Color,
}

/// Fluent's colour picker.
pub struct FluentColorPicker {
    props: ColorPickerProps,
    theme: RwSignal<MultiTheme>,
    /// The chosen colour, shared with the tree it builds.
    color: SharedColor,
    /// The hex text, shared with the text input.
    hex: SharedHex,
    /// The callback, held as a `Send + Sync` handle because the tree's slots
    /// have to be.
    on_change: Option<Arc<dyn Fn(Color) + Send + Sync>>,
}

impl FluentColorPicker {
    pub fn new() -> Self {
        Self {
            props: ColorPickerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            color: Arc::new(Mutex::new(DEFAULT_COLOR)),
            hex: Arc::new(Mutex::new(color_to_hex(DEFAULT_COLOR))),
            on_change: None,
        }
    }

    /// The colour to start on.
    pub fn value(mut self, value: Color) -> Self {
        if let Ok(mut guard) = self.color.lock() {
            *guard = value;
        }
        if let Ok(mut buffer) = self.hex.lock() {
            *buffer = color_to_hex(value);
        }
        self
    }

    /// Whether the alpha strip is shown.
    pub fn show_alpha(mut self, show_alpha: bool) -> Self {
        self.props.show_alpha = show_alpha;
        self
    }

    /// Called with the colour the user chose.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside
    /// a slot closure the host may hand between threads, so the callback has
    /// to be both clonable and thread-safe to be rebuilt into the tree.
    pub fn on_change(mut self, callback: impl Fn(Color) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &ColorPickerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The chosen colour, shared with the tree: the 2.0 answer to v1's
    /// `color_signal()`.
    pub fn color(&self) -> SharedColor {
        self.color.clone()
    }

    /// The hex text, shared with the field.
    pub fn hex(&self) -> SharedHex {
        self.hex.clone()
    }

    pub fn resolved_style(&self) -> FluentColorPickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::ColorPicker(tokens) = theme.component(ComponentId::ColorPicker) else {
            unreachable!("ComponentId::ColorPicker must resolve to ColorPickerTokens")
        };
        FluentColorPickerStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            outline: theme.colors().outline,
            on_surface: theme.colors().on_surface,
            sv_width: tokens.sv_width,
            sv_height: tokens.sv_height,
            slider_height: tokens.slider_height,
            thumb_size: tokens.thumb_size,
            radius: tokens.radius,
            on_surface_variant: theme.colors().on_surface_variant,
            padding: 12.0,
            border_width: 1.0,
            border_alpha: 80,
            margin_top: 12.0,
            row_gap: 12.0,
            preview_size: 40.0,
            hex_width: 120.0,
            hex_font_size: 14.0,
            thumb_border_width: 2.0,
            thumb_border_color: WHITE,
        }
    }

    /// The picker, ready to mount.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let color = self.color.clone();
        let hex = self.hex.clone();
        let on_change = self.on_change.clone();
        let show_alpha = self.props.show_alpha;

        let mut stack = Stack::new(Axis::Vertical);

        // The square: saturation across, value down.
        let square_color = color.clone();
        let square_hex = hex.clone();
        let square_change = on_change.clone();
        stack = stack.push_layout(
            LayoutRequest::new()
                .width(style.sv_width)
                .height(style.sv_height),
            move || {
                let color = square_color.clone();
                let painted = read_color(&color);
                Box::new(FluentColorArea {
                    kind: AreaKind::Square,
                    style,
                    color,
                    hex: square_hex.clone(),
                    on_change: square_change.clone(),
                    dragging: false,
                    painted,
                    revision: Revision::default(),
                }) as Box<dyn WidgetAny>
            },
        );

        // The hue strip.
        let hue_color = color.clone();
        let hue_hex = hex.clone();
        let hue_change = on_change.clone();
        stack = stack.push_layout(
            LayoutRequest::new()
                .width(style.sv_width)
                .height(style.slider_height)
                .margin(Insets {
                    top: f64::from(style.margin_top),
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
            move || {
                let color = hue_color.clone();
                let painted = read_color(&color);
                Box::new(FluentColorArea {
                    kind: AreaKind::Hue,
                    style,
                    color,
                    hex: hue_hex.clone(),
                    on_change: hue_change.clone(),
                    dragging: false,
                    painted,
                    revision: Revision::default(),
                }) as Box<dyn WidgetAny>
            },
        );

        // The alpha strip, if the picker was asked for one.
        if show_alpha {
            let alpha_color = color.clone();
            let alpha_hex = hex.clone();
            let alpha_change = on_change.clone();
            stack = stack.push_layout(
                LayoutRequest::new()
                    .width(style.sv_width)
                    .height(style.slider_height)
                    .margin(Insets {
                        top: f64::from(style.margin_top),
                        right: 0.0,
                        bottom: 0.0,
                        left: 0.0,
                    }),
                move || {
                    let color = alpha_color.clone();
                    let painted = read_color(&color);
                    Box::new(FluentColorArea {
                        kind: AreaKind::Alpha,
                        style,
                        color,
                        hex: alpha_hex.clone(),
                        on_change: alpha_change.clone(),
                        dragging: false,
                        painted,
                        revision: Revision::default(),
                    }) as Box<dyn WidgetAny>
                },
            );
        }

        // The preview and the hex field.
        let preview_color = color.clone();
        let field_color = color.clone();
        let field_hex = hex.clone();
        let field_change = on_change.clone();
        let field_ink = to_paint(style.on_surface);
        let field_placeholder = to_paint(style.on_surface_variant);
        let bottom = Stack::new(Axis::Horizontal)
            .gap(style.row_gap)
            // v1's `.items_center()`.
            .align(Align::Center, Align::Center)
            .push_layout(
                LayoutRequest::new()
                    .width(style.preview_size)
                    .height(style.preview_size),
                move || {
                    let color = preview_color.clone();
                    let painted = read_color(&color);
                    Box::new(FluentColorSwatch {
                        style,
                        color,
                        painted,
                        revision: Revision::default(),
                    }) as Box<dyn WidgetAny>
                },
            )
            .push_layout(LayoutRequest::new().width(style.hex_width), move || {
                Box::new(
                    TextInput::new(field_hex.clone())
                        .placeholder(HEX_PLACEHOLDER)
                        .style(TextInputStyle {
                            font_size: style.hex_font_size,
                            color: field_ink,
                            placeholder_color: field_placeholder,
                            // v1's field had no fill of its own.
                            background: None,
                            border_width: 0.0,
                            ..TextInputStyle::default()
                        })
                        .on_change({
                            // v1 committed the hex on focus-lost; 2.0's text
                            // input has no focus-lost event, so the port
                            // commits whenever the text parses -- and only
                            // when the parsed colour is not the one already
                            // held, so a colour dragged in the square does
                            // not report itself twice.
                            let color = field_color.clone();
                            let hex = field_hex.clone();
                            let on_change = field_change.clone();
                            move |text: String| {
                                let Some(parsed) = hex_to_color(&text) else {
                                    return;
                                };
                                let current = read_color(&color);
                                if parsed.r != current.r
                                    || parsed.g != current.g
                                    || parsed.b != current.b
                                {
                                    commit_color(&color, &hex, &on_change, parsed);
                                }
                            }
                        }),
                ) as Box<dyn WidgetAny>
            });

        stack = stack.push_layout(
            LayoutRequest::new()
                .width(style.sv_width)
                .height(style.preview_size)
                .margin(Insets {
                    top: f64::from(style.margin_top),
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
            move || Box::new(bottom.clone()) as Box<dyn WidgetAny>,
        );

        Surface::new()
            .request(LayoutRequest::new().width(style.sv_width + style.padding * 2.0))
            .padding(Insets::uniform(f64::from(style.padding)))
            .background(to_paint(style.surface))
            .radius(style.radius)
            .border(
                style.border_width,
                to_paint(with_alpha(style.outline, style.border_alpha)),
            )
            .push(move || Box::new(stack.clone()) as Box<dyn WidgetAny>)
    }
}

impl Default for FluentColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

/// Which gradient an area draws, and what a drag on it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaKind {
    /// The square: saturation across, value down.
    Square,
    /// The hue strip.
    Hue,
    /// The alpha strip.
    Alpha,
}

/// One of the picker's three areas: the square, the hue strip or the alpha
/// strip.
///
/// Each paints itself and handles its own drag, because 2.0 keeps a mounted
/// child's widget when its key survives a rebuild -- so an area that read the
/// colour when it was built would keep the colour it was built with.
struct FluentColorArea {
    kind: AreaKind,
    style: FluentColorPickerStyle,
    color: SharedColor,
    hex: SharedHex,
    on_change: Option<Arc<dyn Fn(Color) + Send + Sync>>,
    /// Whether a drag is in progress.
    dragging: bool,
    /// The colour the area last painted, so a change is a change.
    painted: Color,
    revision: Revision,
}

impl FluentColorArea {
    /// The colour a point in the area means, and whether it means anything.
    fn colour_at(&self, bounds: Rect, point: Point) -> Option<Color> {
        let current = read_color(&self.color);
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return None;
        }
        match self.kind {
            AreaKind::Square => {
                let (hue, _, _) = rgb_to_hsv(current);
                let sat = ((point.x - bounds.x0) / bounds.width()).clamp(0.0, 1.0);
                let val = (1.0 - (point.y - bounds.y0) / bounds.height()).clamp(0.0, 1.0);
                let mut value = hsv_to_rgb(hue, sat, val);
                value.a = current.a;
                Some(value)
            }
            AreaKind::Hue => {
                let (_, sat, val) = rgb_to_hsv(current);
                let ratio = ((point.x - bounds.x0) / bounds.width()).clamp(0.0, 1.0);
                let mut value = hsv_to_rgb(ratio * 360.0, sat, val);
                value.a = current.a;
                Some(value)
            }
            AreaKind::Alpha => {
                let ratio = ((point.x - bounds.x0) / bounds.width()).clamp(0.0, 1.0);
                let mut value = current;
                value.a = (ratio * 255.0).round().clamp(0.0, 255.0) as u8;
                Some(value)
            }
        }
    }

    /// The thumb's box, for the colour the area currently shows.
    fn thumb_rect(&self, bounds: Rect) -> Rect {
        let style = self.style;
        let current = read_color(&self.color);
        let thumb = f64::from(style.thumb_size);
        let (x, y) = match self.kind {
            AreaKind::Square => {
                let (_, sat, val) = rgb_to_hsv(current);
                (
                    bounds.x0 + sat * bounds.width() - thumb / 2.0,
                    bounds.y0 + (1.0 - val) * bounds.height() - thumb / 2.0,
                )
            }
            AreaKind::Hue => {
                let (hue, _, _) = rgb_to_hsv(current);
                (
                    bounds.x0 + hue / 360.0 * bounds.width() - thumb / 2.0,
                    bounds.y0 + (bounds.height() - thumb) / 2.0,
                )
            }
            AreaKind::Alpha => {
                let ratio = f64::from(current.a) / 255.0;
                (
                    bounds.x0 + ratio * bounds.width() - thumb / 2.0,
                    bounds.y0 + (bounds.height() - thumb) / 2.0,
                )
            }
        };
        Rect {
            x0: x,
            y0: y,
            x1: x + thumb,
            y1: y + thumb,
        }
    }

    /// The colour the thumb is filled with: the chosen one, except on the hue
    /// strip, where it is the pure hue.
    fn thumb_colour(&self) -> Color {
        let current = read_color(&self.color);
        match self.kind {
            AreaKind::Hue => {
                let (hue, _, _) = rgb_to_hsv(current);
                hsv_to_rgb(hue, 1.0, 1.0)
            }
            _ => current,
        }
    }
}

impl WidgetAny for FluentColorArea {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The three areas are sliders, and each says what it slides.
    ///
    /// v1 published nothing at all here: the square and the two strips were
    /// SVG documents with no name. The port gives them a slider role and a
    /// label, which is an addition rather than a translation.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let label = match self.kind {
            AreaKind::Square => "Saturation and brightness",
            AreaKind::Hue => "Hue",
            AreaKind::Alpha => "Opacity",
        };
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Slider,
            label,
        ))
    }
}

impl Widget for FluentColorArea {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The colour moved, so the gradient or the thumb moved with it.
        let current = read_color(&self.color);
        if current != self.painted {
            self.painted = current;
            self.revision.bump();
        }
        self.revision.sync(cx, id);
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        let bounds = cx.bounds();
        match pointer.phase {
            PointerPhase::Down => {
                // A move carries no button, so the button is only checked where
                // there is one.
                if pointer.button != Some(PointerButton::Primary) {
                    return false;
                }
                if let Some(value) = self.colour_at(bounds, pointer.position) {
                    commit_color(&self.color, &self.hex, &self.on_change, value);
                }
                self.dragging = true;
                true
            }
            PointerPhase::Move => {
                if self.dragging {
                    if let Some(value) = self.colour_at(bounds, pointer.position) {
                        commit_color(&self.color, &self.hex, &self.on_change, value);
                    }
                    true
                } else {
                    false
                }
            }
            PointerPhase::Up => {
                self.dragging = false;
                true
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        match self.kind {
            AreaKind::Square => {
                let current = read_color(&self.color);
                let (hue, _, _) = rgb_to_hsv(current);
                let pure = hsv_to_rgb(hue, 1.0, 1.0);
                // The hue underneath, then the white ramp across it, then the
                // black ramp down it -- v1's three SVG layers, in this order.
                cx.fill_rect(bounds, to_paint(pure));
                let white = Brush::linear(
                    &[
                        (0.0, PaintColor::rgb(255, 255, 255)),
                        (1.0, PaintColor::rgba(255, 255, 255, 0)),
                    ],
                    Point::new(bounds.x0, bounds.y0),
                    Point::new(bounds.x1, bounds.y0),
                );
                cx.fill_rect_brush(bounds, &white);
                let black = Brush::linear(
                    &[
                        (0.0, PaintColor::rgba(0, 0, 0, 0)),
                        (1.0, PaintColor::rgb(0, 0, 0)),
                    ],
                    Point::new(bounds.x0, bounds.y0),
                    Point::new(bounds.x0, bounds.y1),
                );
                cx.fill_rect_brush(bounds, &black);
            }
            AreaKind::Hue => {
                let brush = Brush::linear(
                    &[
                        (0.0, PaintColor::rgb(255, 0, 0)),
                        (0.167, PaintColor::rgb(255, 255, 0)),
                        (0.333, PaintColor::rgb(0, 255, 0)),
                        (0.5, PaintColor::rgb(0, 255, 255)),
                        (0.667, PaintColor::rgb(0, 0, 255)),
                        (0.833, PaintColor::rgb(255, 0, 255)),
                        (1.0, PaintColor::rgb(255, 0, 0)),
                    ],
                    Point::new(bounds.x0, bounds.y0),
                    Point::new(bounds.x1, bounds.y0),
                );
                cx.fill_rounded_rect_brush(bounds, style.radius, &brush);
            }
            AreaKind::Alpha => {
                let current = read_color(&self.color);
                let brush = Brush::linear(
                    &[
                        (0.0, PaintColor::rgba(current.r, current.g, current.b, 0)),
                        (1.0, PaintColor::rgba(current.r, current.g, current.b, 255)),
                    ],
                    Point::new(bounds.x0, bounds.y0),
                    Point::new(bounds.x1, bounds.y0),
                );
                cx.fill_rounded_rect_brush(bounds, style.radius, &brush);
            }
        }
        // The thumb: a white ring around the colour the area shows.
        let thumb = self.thumb_rect(bounds);
        let border = f64::from(style.thumb_border_width);
        cx.fill_rounded_rect(
            thumb,
            style.thumb_size / 2.0,
            to_paint(style.thumb_border_color),
        );
        cx.fill_rounded_rect(
            Rect {
                x0: thumb.x0 + border,
                y0: thumb.y0 + border,
                x1: (thumb.x1 - border).max(thumb.x0 + border),
                y1: (thumb.y1 - border).max(thumb.y0 + border),
            },
            (style.thumb_size / 2.0 - border as f32).max(0.0),
            to_paint(self.thumb_colour()),
        );
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Pointer
    }
}

/// The preview: the chosen colour in a rounded square.
struct FluentColorSwatch {
    style: FluentColorPickerStyle,
    color: SharedColor,
    /// The colour the swatch last painted, so a change is a change.
    painted: Color,
    revision: Revision,
}

impl WidgetAny for FluentColorSwatch {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The preview is an image of the chosen colour.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(floom_core::SemanticsHint::labeled(
            floom_core::SemanticsRole::Image,
            color_to_hex(read_color(&self.color)),
        ))
    }
}

impl Widget for FluentColorSwatch {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let current = read_color(&self.color);
        if current != self.painted {
            self.painted = current;
            self.revision.bump();
        }
        self.revision.sync(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        let current = read_color(&self.color);
        // The border, then the colour inside it.
        cx.fill_rounded_rect(bounds, style.radius, to_paint(style.outline));
        let inset = f64::from(style.border_width);
        cx.fill_rounded_rect(
            Rect {
                x0: bounds.x0 + inset,
                y0: bounds.y0 + inset,
                x1: (bounds.x1 - inset).max(bounds.x0 + inset),
                y1: (bounds.y1 - inset).max(bounds.y0 + inset),
            },
            (style.radius - style.border_width).max(0.0),
            to_paint(current),
        );
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
