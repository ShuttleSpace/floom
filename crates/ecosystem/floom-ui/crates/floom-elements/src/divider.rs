pub use floom_core_theme::component_tokens::DividerTokens;
pub use floom_core_theme::props::divider::{DividerOrientation, DividerProps};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_divider::CupertinoDivider as Divider;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_divider::CupertinoDividerStyle as DividerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_divider::FluentDivider as Divider;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_divider::FluentDividerStyle as DividerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_divider::MdDivider as Divider;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_divider::MdDividerStyle as DividerStyle;

/// Runtime-dispatch `Divider` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::divider::DividerOrientation;
    use floom_core_theme::theme::MultiTheme;

    use floom_widget_md_divider::MdDivider;
    use floom_widget_cupertino_divider::CupertinoDivider;
    use floom_widget_fluent_divider::FluentDivider;

    pub struct Divider {
        orientation: DividerOrientation,
        inset: f32,
        theme: Option<MultiTheme>,
    }

    impl Divider {
        pub fn new() -> Self {
            Self {
                orientation: DividerOrientation::Horizontal,
                inset: 0.0,
                theme: None,
            }
        }

        pub fn orientation(mut self, orientation: DividerOrientation) -> Self {
            self.orientation = orientation;
            self
        }

        pub fn inset(mut self, inset: f32) -> Self {
            self.inset = inset;
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdDivider::new()
                        .orientation(self.orientation)
                        .inset(self.inset);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoDivider::new()
                        .orientation(self.orientation)
                        .inset(self.inset);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentDivider::new()
                        .orientation(self.orientation)
                        .inset(self.inset);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Divider {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Divider;
#[cfg(feature = "multi")]
pub use floom_widget_md_divider::MdDividerStyle as DividerStyle;
