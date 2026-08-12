pub use floem_core_theme::component_tokens::IconTokens;
pub use floem_core_theme::props::icon::IconProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_icon::CupertinoIcon as Icon;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_icon::CupertinoIconStyle as IconStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_icon::FluentIcon as Icon;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_icon::FluentIconStyle as IconStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_icon::MdIcon as Icon;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_icon::MdIconStyle as IconStyle;

/// Runtime-dispatch `Icon` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_icon::MdIcon;
    use floem_widget_cupertino_icon::CupertinoIcon;
    use floem_widget_fluent_icon::FluentIcon;

    pub struct Icon {
        name: String,
        size: Option<f32>,
        label: Option<String>,
        decorative: bool,
        theme: Option<MultiTheme>,
    }

    impl Icon {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                size: None,
                label: None,
                decorative: true,
                theme: None,
            }
        }

        pub fn size(mut self, size: f32) -> Self {
            self.size = Some(size);
            self
        }

        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.label = Some(label.into());
            self.decorative = false;
            self
        }

        pub fn decorative(mut self, decorative: bool) -> Self {
            self.decorative = decorative;
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
                    let mut c = MdIcon::new(self.name).decorative(self.decorative);
                    if let Some(s) = self.size {
                        c = c.size(s);
                    }
                    if let Some(l) = self.label {
                        c = c.label(l);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoIcon::new(self.name).decorative(self.decorative);
                    if let Some(s) = self.size {
                        c = c.size(s);
                    }
                    if let Some(l) = self.label {
                        c = c.label(l);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentIcon::new(self.name).decorative(self.decorative);
                    if let Some(s) = self.size {
                        c = c.size(s);
                    }
                    if let Some(l) = self.label {
                        c = c.label(l);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Icon;
#[cfg(feature = "multi")]
pub use floem_widget_md_icon::MdIconStyle as IconStyle;
