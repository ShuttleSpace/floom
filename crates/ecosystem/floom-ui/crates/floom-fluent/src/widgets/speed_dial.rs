//! Fluent speed dial, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::speed_dial::{SpeedDialDirection, SpeedDialProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use crate::widgets::icon::{fluent_icon_path, VIEW_BOX};

const DEFAULT_FAB_ICON: &str = "add";

const ICON_SIZE: f32 = 24.0;

type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

struct SpeedDialAction {
    icon: String,
    #[allow(dead_code)]
    label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSpeedDialStyle {
    pub design_system: &'static str,
    pub fab_background: Color,
    pub fab_foreground: Color,
    pub action_background: Color,
    pub action_foreground: Color,
    pub fab_size: f32,
    pub action_size: f32,
    pub gap: f32,
    pub radius: f32,
    pub animation_ms: f32,
}

pub struct FluentSpeedDial {
    props: SpeedDialProps,
    actions: Vec<SpeedDialAction>,
    theme: RwSignal<MultiTheme>,
    open: SharedFlag,
    on_action: Option<Callback>,
}

impl FluentSpeedDial {
    pub fn new() -> Self {
        Self {
            props: SpeedDialProps::default(),
            actions: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            open: shared_flag(false),
            on_action: None,
        }
    }

    pub fn action(mut self, icon: impl Into<String>, label: Option<String>) -> Self {
        self.actions.push(SpeedDialAction {
            icon: icon.into(),
            label,
        });
        self
    }

    pub fn direction(mut self, direction: SpeedDialDirection) -> Self {
        self.props.direction = direction;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    pub fn fab_icon(mut self, icon: impl Into<String>) -> Self {
        self.props.fab_icon = icon.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_action(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

    /// that as an `Arc` of its own (see the field's note), and
    pub fn props(&self) -> &SpeedDialProps {
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
        self.open.clone()
    }

    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    pub fn resolved_style(&self) -> FluentSpeedDialStyle {
        let theme = self.theme.get();
        let ComponentTokens::SpeedDial(tokens) = theme.component(ComponentId::SpeedDial) else {
            unreachable!("ComponentId::SpeedDial must resolve to SpeedDialTokens")
        };
        let disabled = self.props.disabled;
        FluentSpeedDialStyle {
            design_system: "fluent",
            fab_background: if disabled {
                with_alpha(theme.colors().on_surface, 12)
            } else {
                theme.colors().primary
            },
            fab_foreground: if disabled {
                with_alpha(theme.colors().on_surface, 38)
            } else {
                theme.colors().on_primary
            },
            action_background: theme.colors().secondary_container,
            action_foreground: theme.colors().on_secondary_container,
            fab_size: tokens.fab_size,
            action_size: tokens.action_size,
            gap: tokens.gap,
            radius: tokens.radius,
            animation_ms: tokens.animation_ms,
        }
    }

    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let open = self.open.clone();
        let disabled = self.props.disabled;
        let on_action = self.on_action.clone();
        let direction = self.props.direction;

        let fab_icon = if self.props.fab_icon.is_empty() {
            DEFAULT_FAB_ICON.to_string()
        } else {
            self.props.fab_icon.clone()
        };
        let fab_glyph = VectorIcon::from_path_data(fluent_icon_path(&fab_icon), VIEW_BOX)
            .size(ICON_SIZE)
            .color(to_paint(style.fab_foreground));
        let fab_surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .width(style.fab_size)
                    .height(style.fab_size)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(style.fab_background))
            .radius(style.radius)
            .push({
                let fab_glyph = fab_glyph.clone();
                move || Box::new(fab_glyph.clone())
            });
        let fab_slot = {
            let surface = fab_surface;
            let open = open.clone();
            move || {
                let surface = surface.clone();
                let open = open.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .disabled(disabled)
                    .on_click(move || {
                        if !disabled {
                            let now = read_flag(&open);
                            write_flag(&open, !now);
                        }
                    })
                    .label("Speed dial"),
                ) as Box<dyn WidgetAny>
            }
        };

        let mut buttons = match direction {
            SpeedDialDirection::Left | SpeedDialDirection::Right => Stack::new(Axis::Horizontal),
            SpeedDialDirection::Up | SpeedDialDirection::Down => Stack::new(Axis::Vertical),
        }
        .gap(style.gap)
        .align(Align::Start, cross_alignment(direction));

        for (index, action) in self.actions.iter().enumerate() {
            let glyph = VectorIcon::from_path_data(fluent_icon_path(&action.icon), VIEW_BOX)
                .size(ICON_SIZE)
                .color(to_paint(style.action_foreground));
            let surface = Surface::new()
                .request(
                    LayoutRequest::new()
                        .width(style.action_size)
                        .height(style.action_size)
                        .main_align(Align::Center)
                        .cross_align(Align::Center),
                )
                .background(to_paint(style.action_background))
                .radius(style.radius)
                .push({
                    let glyph = glyph.clone();
                    move || Box::new(glyph.clone())
                });
            let on_action = on_action.clone();
            buttons = buttons.push(move || {
                let surface = surface.clone();
                let on_action = on_action.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .on_click(move || {
                        if let Some(cb) = &on_action {
                            cb(index);
                        }
                    })
                    .label(format!("Action {index}")),
                )
            });
        }

        let mut fan = Conditional::new(false).when_flag(open.clone());
        if !read_flag(&open) {
            fan.sync_when(false);
        }
        let fan = fan.show(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(buttons.clone()),
        ));

        let fan_slot = {
            let fan = fan;
            move || Box::new(fan.clone()) as Box<dyn WidgetAny>
        };

        let mut dial = Stack::new(dial_axis(direction))
            .gap(style.gap)
            .align(Align::Start, cross_alignment(direction));
        if fab_first(direction) {
            dial = dial.push(fab_slot);
            dial = dial.push(fan_slot);
        } else {
            dial = dial.push(fan_slot);
            dial = dial.push(fab_slot);
        }
        dial
    }
}

impl Default for FluentSpeedDial {
    fn default() -> Self {
        Self::new()
    }
}

fn dial_axis(direction: SpeedDialDirection) -> Axis {
    match direction {
        SpeedDialDirection::Up | SpeedDialDirection::Down => Axis::Vertical,
        SpeedDialDirection::Left | SpeedDialDirection::Right => Axis::Horizontal,
    }
}

fn cross_alignment(direction: SpeedDialDirection) -> Align {
    match direction {
        SpeedDialDirection::Up => Align::End,
        SpeedDialDirection::Down => Align::Start,
        SpeedDialDirection::Left | SpeedDialDirection::Right => Align::Center,
    }
}

fn fab_first(direction: SpeedDialDirection) -> bool {
    matches!(
        direction,
        SpeedDialDirection::Down | SpeedDialDirection::Right
    )
}
