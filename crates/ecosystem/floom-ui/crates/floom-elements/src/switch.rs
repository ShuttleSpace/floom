pub use floom_core_theme::component_tokens::SwitchTokens;
pub use floom_core_theme::props::switch::SwitchProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Switch` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_switch::CupertinoSwitch as Switch;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_switch::CupertinoSwitchStyle as SwitchStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_switch::FluentSwitch as Switch;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_switch::FluentSwitchStyle as SwitchStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_switch::MdSwitch as Switch;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_switch::MdSwitchStyle as SwitchStyle;

/// Runtime-dispatch `Switch` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::switch::SwitchProps;
    use floom_core_theme::theme::MultiTheme;

    use floom_widget_md_switch::MdSwitch;
    use floom_widget_cupertino_switch::CupertinoSwitch;
    use floom_widget_fluent_switch::FluentSwitch;

    pub struct Switch {
        props: SwitchProps,
        on_change: Option<Box<dyn Fn(bool)>>,
        theme: Option<MultiTheme>,
    }

    impl Switch {
        pub fn new() -> Self {
            Self {
                props: SwitchProps::default(),
                on_change: None,
                theme: None,
            }
        }

        pub fn checked(mut self, checked: bool) -> Self {
            self.props.checked = checked;
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.props.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let b = MdSwitch::new()
                        .checked(self.props.checked)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoSwitch::new()
                        .checked(self.props.checked)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentSwitch::new()
                        .checked(self.props.checked)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for Switch {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Switch;
#[cfg(feature = "multi")]
pub use floom_widget_md_switch::MdSwitchStyle as SwitchStyle;
