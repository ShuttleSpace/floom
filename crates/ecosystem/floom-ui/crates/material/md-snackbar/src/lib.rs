
use std::sync::Arc;
use std::time::Duration;

use floom_core::{Child, ChildKey, LayoutRequest, PresenceTransition};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{FontWeight, Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::snackbar::SnackbarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;
use floom_widget_runtime::theme_signal;

const ACTION_PADDING_HORIZONTAL: f64 = 8.0;

/// How far the snackbar slides up into place, in points. A v1 literal.
const SLIDE_DISTANCE: f32 = 40.0;

/// How long the slide takes, in seconds. v1 wrote 250 milliseconds.
const ANIMATION_SECONDS: f64 = 0.25;

/// How far above the bottom edge the bar sits. A v1 literal.
const BOTTOM_PADDING: f64 = 24.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSnackbarStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub surface: Color,
    /// The message's ink.
    pub on_surface: Color,
    /// The action's ink.
    pub action_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub action_gap: f32,
    /// The action's own size: the theme's `label.large`.
    pub action_font_size: f32,
    /// The action's own weight.
    pub action_font_weight: ThemeFontWeight,
    /// The action's padding at the sides.
    pub action_padding_horizontal: f32,
    /// The bar's shadow.
    pub shadow: Shadow,
    pub layer_padding: Insets,
    /// How long the slide takes, in milliseconds.
    pub animation_ms: f32,
}

/// Material's snackbar.
pub struct MaterialSnackbar {
    props: SnackbarProps,
    /// Whether the snackbar is up, shared with whoever shows it.
    show: SharedFlag,
    theme: RwSignal<MultiTheme>,
    /// The action callback in the shape 2.0's slots need.
    on_action: Option<Callback>,
}

impl MaterialSnackbar {
    pub fn new() -> Self {
        Self {
            props: SnackbarProps::default(),
            show: shared_flag(false),
            theme: theme_signal(),
            on_action: None,
        }
    }

    /// The message in the bar.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    /// The action's label. `None` leaves the bar without an action.
    pub fn action_label(mut self, action_label: Option<String>) -> Self {
        self.props.action_label = action_label;
        self
    }

    /// Called when the action is pressed.
    pub fn on_action(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

    pub fn duration_ms(mut self, duration_ms: u32) -> Self {
        self.props.duration_ms = duration_ms;
        self
    }

    pub fn props(&self) -> &SnackbarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn open_flag(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Use a flag the caller already has, so the thing that shows the snackbar
    /// and the snackbar itself share one truth.
    pub fn with_open_flag(mut self, show: SharedFlag) -> Self {
        self.show = show;
        self
    }

    pub fn dismiss_after(&self, duration_ms: u32) {
        let show = self.show.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(u64::from(duration_ms)));
            write_flag(&show, false);
        });
    }

    pub fn resolved_style(&self) -> MaterialSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        MaterialSnackbarStyle {
            design_system: "material",
            surface: theme.colors().on_surface,
            on_surface: theme.colors().surface,
            action_color: theme.colors().primary,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            action_gap: tokens.action_gap,
            action_font_size: theme.typography().label.large.font_size,
            action_font_weight: ThemeFontWeight::Medium,
            action_padding_horizontal: ACTION_PADDING_HORIZONTAL as f32,
            shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 40)),
                blur: 16.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            layer_padding: Insets {
                top: 0.0,
                right: 0.0,
                bottom: BOTTOM_PADDING,
                left: 0.0,
            },
            animation_ms: (ANIMATION_SECONDS * 1000.0) as f32,
        }
    }

    /// The snackbar: the layer, the bar, the message and the action.
    pub fn view(self) -> Overlay {
        let style = self.resolved_style();
        let show = self.show.clone();
        let on_action = self.on_action.clone();
        let message = self.props.message.clone();
        let action_label = self.props.action_label.clone();

        let mut row = Stack::new(Axis::Horizontal)
            .gap(style.action_gap)
            .align(Align::Start, Align::Center)
            .push({
                let message = message.clone();
                let ink = to_paint(style.on_surface);
                move || Box::new(Text::new(message.clone()).size(style.font_size).color(ink))
            });

        if let Some(label) = action_label {
            let show = show.clone();
            row = row.push(move || {
                let show = show.clone();
                let on_action = on_action.clone();
                let label = label.clone();
                let ink = to_paint(style.action_color);
                let size = style.action_font_size;
                let weight = style.action_font_weight;
                let pad = style.action_padding_horizontal;
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || {
                            let label = label.clone();
                            Box::new(
                                Stack::new(Axis::Horizontal)
                                    .request(LayoutRequest::new().padding_horiz(f64::from(pad)))
                                    .push(move || {
                                        Box::new(
                                            Text::new(label.clone())
                                                .size(size)
                                                .weight(to_engine_weight(weight))
                                                .color(ink),
                                        )
                                    }),
                            )
                        },
                    ))
                    .on_click(move || {
                        if let Some(cb) = &on_action {
                            cb();
                        }
                        write_flag(&show, false);
                    })
                    .label("Snackbar action"),
                )
            });
        }

        let surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .min_height(style.min_height)
                    .max_width(style.max_width)
                    .cross_align(Align::Center),
            )
            .padding(Insets {
                left: f64::from(style.padding),
                right: f64::from(style.padding),
                top: f64::from(style.padding),
                bottom: f64::from(style.padding),
            })
            .background(to_paint(style.surface))
            .radius(style.border_radius)
            .shadow(style.shadow)
            .push(move || Box::new(row.clone()));

        let layer = Stack::new(Axis::Vertical)
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .height_pct(100.0)
                    .main_align(Align::End)
                    .cross_align(Align::Center)
                    .padding(style.layer_padding),
            )
            .push(move || Box::new(surface.clone()));

        let mut content = Conditional::new(false).when_flag(show.clone());
        if !read_flag(&show) {
            content.sync_when(false);
        }
        let content = content
            .enter(PresenceTransition::slide(ANIMATION_SECONDS, SLIDE_DISTANCE))
            .exit(PresenceTransition::slide(ANIMATION_SECONDS, SLIDE_DISTANCE))
            .show(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(layer.clone()),
            ));

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(content.clone()),
        ))
    }
}

impl Default for MaterialSnackbar {
    fn default() -> Self {
        Self::new()
    }
}

/// The theme's weight -> the engine's.
fn to_engine_weight(weight: ThemeFontWeight) -> FontWeight {
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
