

use std::sync::{Arc, Mutex};

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ColorPicker, ColorPickerStyle};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::color_picker::ColorPickerProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::WHITE;
use floom_widget_runtime::theme_signal;

pub type SharedColor = floom_ui_composites::SharedColor;

/// The hex text, shared with the text input.
pub type SharedHex = floom_ui_composites::SharedHex;

const DEFAULT_COLOR: PaintColor = PaintColor::rgb(33, 150, 243);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialColorPickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub outline: Color,
    pub on_surface: Color,
    pub sv_width: f32,
    pub sv_height: f32,
    pub slider_height: f32,
    pub thumb_size: f32,
    pub radius: f32,
    /// The hex field's ink.
    pub on_surface_variant: Color,
    pub padding: f32,
    pub border_width: f32,
    pub border_alpha: u8,
    pub margin_top: f32,
    pub row_gap: f32,
    pub preview_size: f32,
    pub hex_width: f32,
    pub hex_font_size: f32,
    pub thumb_border_width: f32,
    pub thumb_border_color: Color,
}

/// Material's colour picker.
pub struct MaterialColorPicker {
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

impl MaterialColorPicker {
    pub fn new() -> Self {
        Self {
            props: ColorPickerProps::default(),
            theme: theme_signal(),
            color: Arc::new(Mutex::new(DEFAULT_COLOR)),
            hex: Arc::new(Mutex::new(floom_ui_composites::color_to_hex(DEFAULT_COLOR))),
            on_change: None,
        }
    }

    /// The colour to start on.
    pub fn value(self, value: Color) -> Self {
        let value = to_paint(value);
        if let Ok(mut guard) = self.color.lock() {
            *guard = value;
        }
        if let Ok(mut buffer) = self.hex.lock() {
            *buffer = floom_ui_composites::color_to_hex(value);
        }
        self
    }

    /// Whether the alpha strip is shown.
    pub fn show_alpha(mut self, show_alpha: bool) -> Self {
        self.props.show_alpha = show_alpha;
        self
    }

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

    pub fn color(&self) -> SharedColor {
        self.color.clone()
    }

    /// The hex text, shared with the field.
    pub fn hex(&self) -> SharedHex {
        self.hex.clone()
    }

    pub fn resolved_style(&self) -> MaterialColorPickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::ColorPicker(tokens) = theme.component(ComponentId::ColorPicker) else {
            unreachable!("ComponentId::ColorPicker must resolve to ColorPickerTokens")
        };
        MaterialColorPickerStyle {
            design_system: "material",
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
        let mut picker = ColorPicker::new()
            .style(ColorPickerStyle {
                surface: to_paint(style.surface),
                outline: to_paint(style.outline),
                on_surface: to_paint(style.on_surface),
                sv_width: style.sv_width,
                sv_height: style.sv_height,
                slider_height: style.slider_height,
                thumb_size: style.thumb_size,
                radius: style.radius,
                on_surface_variant: to_paint(style.on_surface_variant),
                padding: style.padding,
                border_width: style.border_width,
                border_alpha: style.border_alpha,
                margin_top: style.margin_top,
                row_gap: style.row_gap,
                preview_size: style.preview_size,
                hex_width: style.hex_width,
                hex_font_size: style.hex_font_size,
                thumb_border_width: style.thumb_border_width,
                thumb_border_color: to_paint(style.thumb_border_color),
            })
            .color(self.color.clone())
            .hex(self.hex.clone())
            .show_alpha(self.props.show_alpha);
        if let Some(on_change) = self.on_change.clone() {
            picker = picker.on_change(move |color| on_change(from_paint(color)));
        }
        picker.build()
    }
}

impl Default for MaterialColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

/// Engine colour -> theme colour.
fn from_paint(c: PaintColor) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
