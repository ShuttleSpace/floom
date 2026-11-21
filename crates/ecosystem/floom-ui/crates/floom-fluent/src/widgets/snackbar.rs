//! Fluent snackbar, ported to the Floom 2.0 widget API.

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

const ACTION_PADDING_HORIZONTAL: f64 = 8.0;

const SLIDE_DISTANCE: f32 = 40.0;

const ANIMATION_SECONDS: f64 = 0.25;

const BOTTOM_PADDING: f64 = 24.0;

type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSnackbarStyle {
    pub design_system: &'static str,
    pub surface_color: Color,
    pub text_color: Color,
    pub action_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub painted_border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub action_gap: f32,
    pub action_font_size: f32,
    pub action_font_weight: ThemeFontWeight,
    pub action_padding_horizontal: f32,
    pub shadow: Shadow,
    pub layer_padding: Insets,
    pub animation_ms: f32,
}

pub struct FluentSnackbar {
    props: SnackbarProps,
    show: SharedFlag,
    theme: RwSignal<MultiTheme>,
    on_action: Option<Callback>,
}

impl FluentSnackbar {
    pub fn new() -> Self {
        Self {
            props: SnackbarProps::default(),
            show: shared_flag(false),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_action: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn action_label(mut self, action_label: Option<String>) -> Self {
        self.props.action_label = action_label;
        self
    }

    pub fn on_action(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

    pub fn duration_ms(mut self, duration_ms: u32) -> Self {
        self.props.duration_ms = duration_ms;
        self
    }

    /// that as an `Arc` of its own (see the field's note), and
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

    pub fn resolved_style(&self) -> FluentSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        FluentSnackbarStyle {
            design_system: "fluent",
            surface_color: with_alpha(theme.colors().on_surface, 235),
            text_color: theme.colors().surface,
            action_color: theme.colors().primary,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            painted_border_radius: theme.radius().sm,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            action_gap: tokens.action_gap,
            action_font_size: tokens.font_size,
            action_font_weight: ThemeFontWeight::SemiBold,
            action_padding_horizontal: ACTION_PADDING_HORIZONTAL as f32,
            shadow: Shadow {
                color: to_paint(with_alpha(Color::BLACK, 25)),
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
                let ink = to_paint(style.text_color);
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
            .background(to_paint(style.surface_color))
            .radius(style.painted_border_radius)
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

impl Default for FluentSnackbar {
    fn default() -> Self {
        Self::new()
    }
}

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
