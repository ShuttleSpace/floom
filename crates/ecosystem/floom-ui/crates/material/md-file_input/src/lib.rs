
use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::text_input::SharedBuffer;
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::file_input::FileInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(Option<String>) + Send + Sync + 'static>;

const GAP: f32 = 12.0;
const BORDER_WIDTH: f32 = 1.0;
/// What the readout says when no path has been chosen. A v1 literal.
const NO_FILE_CHOSEN: &str = "No file chosen";
const DISABLED_ALPHA: u8 = 38;
const DISABLED_FILL_ALPHA: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialFileInputStyle {
    pub design_system: &'static str,
    /// The trigger label's ink.
    pub trigger_color: Color,
    /// The readout's ink when a path is chosen.
    pub path_color: Color,
    /// The readout's ink when no path is chosen.
    pub empty_path_color: Color,
    /// The box's rim.
    pub border_color: Color,
    /// The box's rim while disabled.
    pub disabled_border_color: Color,
    /// The box's fill.
    pub background: Color,
    /// The box's fill while disabled.
    pub disabled_background: Color,
    /// The box's height, from the token.
    pub height: f32,
    /// The box's horizontal padding, from the token.
    pub padding_horizontal: f32,
    /// The box's radius, from the token.
    pub border_radius: f32,
    /// The text's size, from the token.
    pub font_size: f32,
    /// The trigger label's weight, from the theme's type scale.
    pub trigger_font_weight: FontWeight,
    pub gap: f32,
    pub border_width: f32,
    /// What the readout says when no path has been chosen. A v1 literal.
    pub empty_path_label: &'static str,
}

/// Material's file input.
pub struct MaterialFileInput {
    props: FileInputProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
    path: SharedBuffer,
    on_change: Option<Callback>,
}

impl MaterialFileInput {
    pub fn new() -> Self {
        Self {
            props: FileInputProps::default(),
            theme: theme_signal(),
            child: None,
            path: Arc::new(Mutex::new(String::new())),
            on_change: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Comma-separated MIME / extension hint passed to the host picker.
    pub fn accept(mut self, accept: impl Into<String>) -> Self {
        self.props.accept = accept.into();
        self
    }

    /// Allow multiple files (stored for the host picker; the readout shows the
    /// first selected path).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Fires when the user clicks the trigger; the current path is passed so the
    /// host can open a native picker and write the result back.
    pub fn on_change(mut self, callback: impl Fn(Option<String>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &FileInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn path_buffer(&self) -> SharedBuffer {
        self.path.clone()
    }

    /// The path as it is right now. Empty means "no file chosen".
    pub fn path(&self) -> String {
        self.path
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Write the path, as the host would after a picker returns.
    pub fn set_path(&self, path: impl Into<String>) {
        if let Ok(mut guard) = self.path.lock() {
            *guard = path.into();
        }
    }

    pub fn resolved_style(&self) -> MaterialFileInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::FileInput(tokens) = theme.component(ComponentId::FileInput) else {
            unreachable!("ComponentId::FileInput must resolve to FileInputTokens")
        };
        let c = theme.colors();
        MaterialFileInputStyle {
            design_system: "material",
            trigger_color: c.primary,
            path_color: c.on_surface,
            empty_path_color: c.on_surface_variant,
            border_color: c.outline,
            disabled_border_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            background: c.surface,
            disabled_background: with_alpha(c.on_surface, DISABLED_FILL_ALPHA),
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            trigger_font_weight: engine_weight(theme.typography().label.large.font_weight),
            gap: GAP,
            border_width: BORDER_WIDTH,
            empty_path_label: NO_FILE_CHOSEN,
        }
    }

    /// The trigger: the box, its two texts, and the click that reports the path.
    pub fn view(self) -> Pressable {
        let style = self.resolved_style();
        let disabled = self.props.disabled;
        let label_text = self.props.label.clone();

        let path = self.path();
        let chosen = !path.is_empty();
        let readout_text = if chosen {
            path
        } else {
            style.empty_path_label.to_string()
        };

        let trigger_color = to_paint(style.trigger_color);
        let readout_color = to_paint(if chosen {
            style.path_color
        } else {
            style.empty_path_color
        });
        let font_size = style.font_size;
        let trigger_weight = style.trigger_font_weight;

        let mut row = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            .align(Align::SpaceBetween, Align::Center)
            .request(LayoutRequest::new().width_pct(100.0))
            .push({
                let label_text = label_text.clone();
                move || {
                    Box::new(
                        Text::new(label_text.clone())
                            .size(font_size)
                            .weight(trigger_weight)
                            .color(trigger_color),
                    )
                }
            })
            .push({
                let readout_text = readout_text.clone();
                move || {
                    Box::new(
                        Text::new(readout_text.clone())
                            .size(font_size)
                            .color(readout_color),
                    )
                }
            });

        if let Some(child) = self.child.clone() {
            row = row.child(child);
        }

        let background = to_paint(if disabled {
            style.disabled_background
        } else {
            style.background
        });
        let border_color = to_paint(if disabled {
            style.disabled_border_color
        } else {
            style.border_color
        });

        let surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.height)
                    .cross_align(Align::Center),
            )
            .padding_horiz(f64::from(style.padding_horizontal))
            .background(background)
            .radius(style.border_radius)
            .border(style.border_width, border_color)
            .push({
                let row = row.clone();
                move || Box::new(row.clone())
            });

        let on_change = self.on_change.clone();
        let path = self.path.clone();

        Pressable::new(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(surface.clone()),
        ))
        .disabled(disabled)
        .label(label_text)
        .on_click(move || {
            if disabled {
                return;
            }
            if let Some(cb) = &on_change {
                let current = path.lock().map(|guard| guard.clone()).unwrap_or_default();
                cb(if current.is_empty() {
                    None
                } else {
                    Some(current)
                });
            }
        })
    }
}

impl Default for MaterialFileInput {
    fn default() -> Self {
        Self::new()
    }
}

fn engine_weight(weight: ThemeFontWeight) -> FontWeight {
    match weight {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
