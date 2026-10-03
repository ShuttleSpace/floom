pub use floom_core_theme::component_tokens::IconButtonTokens;
pub use floom_core_theme::props::icon_button::{IconButtonProps, IconButtonVariant};

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `IconButton` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_icon_button::CupertinoIconButton as IconButton;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_icon_button::FluentIconButton as IconButton;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_icon_button::MdIconButton as IconButton;

/// Runtime-dispatch `IconButton` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::icon_button::{IconButtonProps, IconButtonVariant};

    use floom_widget_md_icon_button::MdIconButton;
    use floom_widget_cupertino_icon_button::CupertinoIconButton;
    use floom_widget_fluent_icon_button::FluentIconButton;

    pub struct IconButton {
        props: IconButtonProps,
    }

    impl IconButton {
        pub fn new(icon: impl Into<String>) -> Self {
            Self {
                props: IconButtonProps {
                    icon: icon.into(),
                    ..Default::default()
                },
            }
        }

        pub fn size(mut self, size: f32) -> Self {
            self.props.size = Some(size);
            self
        }

        pub fn variant(mut self, variant: IconButtonVariant) -> Self {
            self.props.variant = variant;
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.props.disabled = disabled;
            self
        }

        pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
            self.props.on_click = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdIconButton::new(props.icon);
                    if let Some(size) = props.size {
                        b = b.size(size);
                    }
                    b = b.variant(props.variant).disabled(props.disabled);
                    if let Some(cb) = props.on_click {
                        b = b.on_click(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoIconButton::new(props.icon);
                    if let Some(size) = props.size {
                        b = b.size(size);
                    }
                    b = b.variant(props.variant).disabled(props.disabled);
                    if let Some(cb) = props.on_click {
                        b = b.on_click(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentIconButton::new(props.icon);
                    if let Some(size) = props.size {
                        b = b.size(size);
                    }
                    b = b.variant(props.variant).disabled(props.disabled);
                    if let Some(cb) = props.on_click {
                        b = b.on_click(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::IconButton;
