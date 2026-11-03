
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
use floom_widget_md_icon::{MdIcon, MD_ICON_VIEW_BOX};
use floom_widget_runtime::theme_signal;

const DEFAULT_FAB_ICON: &str = "add";

const ICON_SIZE: f32 = 24.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// One action of the fan.
struct SpeedDialAction {
    icon: String,
    #[allow(dead_code)]
    label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSpeedDialStyle {
    pub design_system: &'static str,
    /// The button's fill.
    pub fab_background: Color,
    /// The button's ink.
    pub fab_foreground: Color,
    /// An action button's fill.
    pub action_background: Color,
    /// An action button's ink.
    pub action_foreground: Color,
    pub fab_size: f32,
    pub action_size: f32,
    pub gap: f32,
    pub radius: f32,
    pub animation_ms: f32,
}

/// Material's speed dial.
pub struct MaterialSpeedDial {
    props: SpeedDialProps,
    actions: Vec<SpeedDialAction>,
    theme: RwSignal<MultiTheme>,
    /// Whether the fan is open, shared with whoever opens it.
    open: SharedFlag,
    /// The action callback in the shape 2.0's slots need.
    on_action: Option<Callback>,
}

impl MaterialSpeedDial {
    pub fn new() -> Self {
        Self {
            props: SpeedDialProps::default(),
            actions: Vec::new(),
            theme: theme_signal(),
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

    /// Which way the fan opens.
    pub fn direction(mut self, direction: SpeedDialDirection) -> Self {
        self.props.direction = direction;
        self
    }

    /// Whether the dial starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// The button's icon, by name in this kit's table. Empty means the
    /// default plus.
    pub fn fab_icon(mut self, icon: impl Into<String>) -> Self {
        self.props.fab_icon = icon.into();
        self
    }

    /// Whether the button is inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the action's index when an action button is pressed.
    pub fn on_action(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

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

    /// Use a flag the caller already has, so the thing that opens the dial and
    /// the dial itself share one truth.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    pub fn resolved_style(&self) -> MaterialSpeedDialStyle {
        let theme = self.theme.get();
        let ComponentTokens::SpeedDial(tokens) = theme.component(ComponentId::SpeedDial) else {
            unreachable!("ComponentId::SpeedDial must resolve to SpeedDialTokens")
        };
        let disabled = self.props.disabled;
        MaterialSpeedDialStyle {
            design_system: "material",
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

    /// The dial: the fan and the button.
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
        let fab_glyph = VectorIcon::from_path_data(
            MdIcon::new(fab_icon.as_str()).path_data(),
            MD_ICON_VIEW_BOX,
        )
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
            let glyph = VectorIcon::from_path_data(
                MdIcon::new(action.icon.as_str()).path_data(),
                MD_ICON_VIEW_BOX,
            )
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

impl Default for MaterialSpeedDial {
    fn default() -> Self {
        Self::new()
    }
}

/// The axis the dial runs on: vertical for up and down, horizontal for left
/// and right.
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

/// Whether the button comes before the fan. Up and left open upwards and
/// leftwards, so the fan leads; down and right put the button first.
fn fab_first(direction: SpeedDialDirection) -> bool {
    matches!(
        direction,
        SpeedDialDirection::Down | SpeedDialDirection::Right
    )
}
