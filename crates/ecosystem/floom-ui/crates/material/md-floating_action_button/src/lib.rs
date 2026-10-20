//! Material floating action button, ported to the Floom 2.0 widget API.
//!
//! A circular, elevated primary button -- optionally extended with a label.
//!
//! ## What the port reproduces
//!
//! * the circle: the token's `border_radius` (9999, a full circle) on a box of the
//!   token's `size`;
//! * the elevation: v1 wrote `box_shadow_blur(2.0 + elevation)` with a spread of
//!   1 and a black-at-30% ink, and the port writes the same shadow;
//! * the palette: `primary` with `on_primary` ink, and -- when the button is
//!   disabled -- `on_surface` at 12% for the fill and 38% for the ink;
//! * the extended shape: a label beside the icon, the same height as the circle,
//!   20dp of padding at the sides and an 8dp gap, which turns the circle into a
//!   pill.
//!
//! ## The click
//!
//! v1 put the action on the container. 2.0's clickable is a [`Pressable`] --
//! interaction without the drawing -- so the button is a `Pressable` around the
//! `Surface` that paints it, and the pressable carries the disabled state, so a
//! disabled button does not fire.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Shadow};
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, FloatingActionButtonTokens};
use floom_core_theme::props::floating_action_button::FloatingActionButtonProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

/// A builder for the button's leading content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// A v1-shaped callback, kept so `props()` still reports one.
type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialFloatingActionButtonStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub size: f32,
    pub border_radius: f32,
    pub elevation: u8,
    pub icon_size: f32,
    pub extended: bool,
}

/// Material's floating action button.
pub struct MaterialFloatingActionButton {
    props: FloatingActionButtonProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
    /// The activation callback in the shape 2.0's slots need.
    on_click: Option<Callback>,
}

impl MaterialFloatingActionButton {
    pub fn new() -> Self {
        Self {
            props: FloatingActionButtonProps::default(),
            theme: theme_signal(),
            child: None,
            on_click: None,
        }
    }

    /// The leading icon or content of the button.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    /// The text label. A non-empty label turns the button into the extended
    /// (pill) shape, when `extended` is on.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// Render the extended shape when a label is set.
    pub fn extended(mut self, extended: bool) -> Self {
        self.props.extended = extended;
        self
    }

    /// Whether the button is interactive.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called when the button is activated.
    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &FloatingActionButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialFloatingActionButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::FloatingActionButton(tokens) =
            theme.component(ComponentId::FloatingActionButton)
        else {
            unreachable!("ComponentId::FloatingActionButton must resolve to FloatingActionButtonTokens")
        };
        let disabled = self.props.disabled;
        MaterialFloatingActionButtonStyle {
            design_system: "material",
            // v1: the fill is `primary`, or `on_surface` at 12% when disabled.
            background: if disabled {
                with_alpha(theme.colors().on_surface, 12)
            } else {
                theme.colors().primary
            },
            // v1: the ink is `on_primary`, or `on_surface` at 38% when disabled.
            foreground: if disabled {
                with_alpha(theme.colors().on_surface, 38)
            } else {
                theme.colors().on_primary
            },
            size: tokens.size,
            border_radius: tokens.border_radius,
            elevation: tokens.elevation,
            icon_size: tokens.icon_size,
            extended: self.props.extended && !self.props.label.is_empty(),
        }
    }

    /// The button, ready to mount.
    pub fn view(self) -> Pressable {
        let style = self.resolved_style();
        let label = self.props.label.clone();
        let extended = style.extended;
        let size = style.size;
        let foreground = to_paint(style.foreground);
        let child = self.child.clone();
        let on_click = self.on_click.clone();
        let disabled = self.props.disabled;

        // The content is built inside the surface's slot, because the slot is a
        // replayed `Fn` and the content is a fresh subtree every mount.
        let surface = Surface::new()
            .request(LayoutRequest::new().main_align(Align::Center).cross_align(Align::Center))
            .background(to_paint(style.background))
            .radius(style.border_radius)
            .shadow(Shadow {
                // v1: `box_shadow_blur(2.0 + elevation)`, spread 1, black at 30%.
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 30)),
                blur: 2.0 + f64::from(style.elevation),
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            })
            .push(move || {
                if extended {
                    // The extended shape: the leading content and the label side by
                    // side. The label is cloned here, because the slot is a
                    // replayed `Fn` and the inner slot cannot move out of it.
                    let label = label.clone();
                    Box::new(
                        Stack::new(Axis::Horizontal)
                            .request(
                                LayoutRequest::new()
                                    .height(size)
                                    .padding_horiz(20.0)
                                    .main_align(Align::Center)
                                    .cross_align(Align::Center),
                            )
                            .gap(8.0)
                            .push({
                                let child = child.clone();
                                move || match child.clone() {
                                    Some(build) => build(),
                                    None => Box::new(Text::new(String::new())),
                                }
                            })
                            .push(move || {
                                Box::new(Text::new(label.clone()).size(size * 0.5).color(foreground))
                            }),
                    )
                } else {
                    // The circle: the leading content, centred.
                    Box::new(
                        Stack::new(Axis::Horizontal)
                            .request(
                                LayoutRequest::new()
                                    .width(size)
                                    .height(size)
                                    .main_align(Align::Center)
                                    .cross_align(Align::Center),
                            )
                            .push({
                                let child = child.clone();
                                move || match child.clone() {
                                    Some(build) => build(),
                                    None => Box::new(Text::new(String::new())),
                                }
                            }),
                    )
                }
            });

        Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            Box::new(surface.clone())
        }))
        .disabled(disabled)
        .on_click(move || {
            // v1 fired the callback only when the button was not disabled.
            if !disabled {
                if let Some(cb) = &on_click {
                    cb();
                }
            }
        })
        .label("Floating action button")
    }
}

impl Default for MaterialFloatingActionButton {
    fn default() -> Self {
        Self::new()
    }
}

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
