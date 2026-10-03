pub use floom_core_theme::component_tokens::SpeedDialTokens;
pub use floom_core_theme::props::speed_dial::{SpeedDialProps, SpeedDialDirection};

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `SpeedDial` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_speed_dial::CupertinoSpeedDial as SpeedDial;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_speed_dial::FluentSpeedDial as SpeedDial;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_speed_dial::MdSpeedDial as SpeedDial;

/// Runtime-dispatch `SpeedDial` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::speed_dial::{SpeedDialProps, SpeedDialDirection};

    use floom_widget_md_speed_dial::MdSpeedDial;
    use floom_widget_cupertino_speed_dial::CupertinoSpeedDial;
    use floom_widget_fluent_speed_dial::FluentSpeedDial;

    struct DialAction {
        icon: String,
        label: Option<String>,
    }

    pub struct SpeedDial {
        props: SpeedDialProps,
        actions: Vec<DialAction>,
    }

    impl SpeedDial {
        pub fn new() -> Self {
            Self {
                props: SpeedDialProps::default(),
                actions: Vec::new(),
            }
        }

        pub fn action(mut self, icon: impl Into<String>, label: Option<String>) -> Self {
            self.actions.push(DialAction {
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
            let ds = design_system_signal().get();
            let props = self.props;
            let actions = self.actions;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdSpeedDial::new();
                    for a in actions {
                        b = b.action(a.icon, a.label);
                    }
                    b = b
                        .direction(props.direction)
                        .open(props.open)
                        .fab_icon(props.fab_icon)
                        .disabled(props.disabled);
                    if let Some(cb) = props.on_action {
                        b = b.on_action(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoSpeedDial::new();
                    for a in actions {
                        b = b.action(a.icon, a.label);
                    }
                    b = b
                        .direction(props.direction)
                        .open(props.open)
                        .fab_icon(props.fab_icon)
                        .disabled(props.disabled);
                    if let Some(cb) = props.on_action {
                        b = b.on_action(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentSpeedDial::new();
                    for a in actions {
                        b = b.action(a.icon, a.label);
                    }
                    b = b
                        .direction(props.direction)
                        .open(props.open)
                        .fab_icon(props.fab_icon)
                        .disabled(props.disabled);
                    if let Some(cb) = props.on_action {
                        b = b.on_action(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for SpeedDial {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::SpeedDial;
