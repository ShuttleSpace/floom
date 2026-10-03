use std::rc::Rc;

use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Container, Decorators, Empty, svg, dyn_view, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::speed_dial::{SpeedDialProps, SpeedDialDirection};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;

struct SpeedDialAction {
    icon: String,
    label: Option<String>,
}

const PLUS_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z\"/></svg>";

pub struct FluentSpeedDial {
    props: SpeedDialProps,
    actions: Vec<SpeedDialAction>,
    theme: RwSignal<MultiTheme>,
}

impl FluentSpeedDial {
    pub fn new() -> Self {
        Self {
            props: SpeedDialProps::default(),
            actions: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
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

    pub fn on_action(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_action = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let direction = self.props.direction;
        let disabled = self.props.disabled;
        let fab_icon = if self.props.fab_icon.is_empty() {
            PLUS_SVG.to_string()
        } else {
            self.props.fab_icon.clone()
        };
        let on_action = Rc::new(self.props.on_action);
        let actions = self.actions;
        let open_sig: RwSignal<bool> = RwSignal::new(self.props.open);

        let tokens = match theme.get().component(ComponentId::SpeedDial) {
            ComponentTokens::SpeedDial(t) => t,
            _ => unreachable!(),
        };
        let tk_fab = tokens.clone();
        let tk_layout = tokens.clone();
        let gap = tk_layout.gap;

        let fab = Container::new(svg(fab_icon))
            .style(move |s| {
                let t = theme.get();
                let (bg, fg) = if disabled {
                    (
                        with_alpha(t.colors().on_surface, 12),
                        with_alpha(t.colors().on_surface, 38),
                    )
                } else {
                    (t.colors().primary, t.colors().on_primary)
                };
                s.size(tk_fab.fab_size, tk_fab.fab_size)
                    .border_radius(tk_fab.radius)
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled)
            })
            .action(move || {
                if !disabled {
                    open_sig.update(|v| *v = !*v);
                }
            });

        let actions_view = dyn_view(move || -> AnyView {
            if open_sig.get() {
                let items: Vec<AnyView> = actions
                    .iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let on_action = on_action.clone();
                        let icon = a.icon.clone();
                        let theme = theme;
                        let tk = tokens.clone();
                        Container::new(svg(icon))
                            .style(move |s| {
                                let t = theme.get();
                                s.size(tk.action_size, tk.action_size)
                                    .border_radius(tk.radius)
                                    .background(t.colors().secondary_container.to_peniko())
                                    .color(t.colors().on_secondary_container.to_peniko())
                                    .items_center()
                                    .justify_center()
                                    .cursor(CursorStyle::Pointer)
                            })
                            .action(move || {
                                if let Some(cb) = on_action.as_ref() {
                                    cb(i);
                                }
                            })
                            .into_any()
                    })
                    .collect();
                match direction {
                    SpeedDialDirection::Up => {
                        Stack::vertical_from_iter(items).style(move |s| s.gap(gap).items_end())
                    }
                    SpeedDialDirection::Down => {
                        Stack::vertical_from_iter(items).style(move |s| s.gap(gap).items_start())
                    }
                    SpeedDialDirection::Left => {
                        Stack::horizontal_from_iter(items).style(move |s| s.gap(gap).items_center())
                    }
                    SpeedDialDirection::Right => {
                        Stack::horizontal_from_iter(items).style(move |s| s.gap(gap).items_center())
                    }
                }
                .into_any()
            } else {
                Empty::new().into_any()
            }
        });

        match direction {
            SpeedDialDirection::Up => {
                Stack::vertical((actions_view, fab)).style(move |s| s.gap(gap).items_end())
            }
            SpeedDialDirection::Down => {
                Stack::vertical((fab, actions_view)).style(move |s| s.gap(gap).items_start())
            }
            SpeedDialDirection::Left => {
                Stack::horizontal((actions_view, fab)).style(move |s| s.gap(gap).items_center())
            }
            SpeedDialDirection::Right => {
                Stack::horizontal((fab, actions_view)).style(move |s| s.gap(gap).items_center())
            }
        }
    }
}

impl Default for FluentSpeedDial {
    fn default() -> Self {
        Self::new()
    }
}
