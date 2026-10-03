pub use floom_core_theme::component_tokens::RadioGroupTokens;
pub use floom_core_theme::props::radio_group::RadioGroupProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_radio_group::CupertinoRadioGroup as RadioGroup;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_radio_group::CupertinoRadioGroupStyle as RadioGroupStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_radio_group::FluentRadioGroup as RadioGroup;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_radio_group::FluentRadioGroupStyle as RadioGroupStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_radio_group::MdRadioGroup as RadioGroup;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_radio_group::MdRadioGroupStyle as RadioGroupStyle;

/// Runtime-dispatch `RadioGroup` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::theme::MultiTheme;

    use floom_widget_md_radio_group::MdRadioGroup;
    use floom_widget_cupertino_radio_group::CupertinoRadioGroup;
    use floom_widget_fluent_radio_group::FluentRadioGroup;

    pub struct RadioGroup {
        options: Vec<String>,
        selected: Option<usize>,
        disabled: bool,
        on_change: Option<Box<dyn Fn(usize)>>,
        theme: Option<MultiTheme>,
    }

    impl RadioGroup {
        pub fn new() -> Self {
            Self {
                options: Vec::new(),
                selected: None,
                disabled: false,
                on_change: None,
                theme: None,
            }
        }

        pub fn option(mut self, option: impl Into<String>) -> Self {
            self.options.push(option.into());
            self
        }

        pub fn options(mut self, options: Vec<String>) -> Self {
            self.options = options;
            self
        }

        pub fn selected(mut self, selected: Option<usize>) -> Self {
            self.selected = selected;
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
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
                    let mut c = MdRadioGroup::new()
                        .options(self.options)
                        .selected(self.selected)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoRadioGroup::new()
                        .options(self.options)
                        .selected(self.selected)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentRadioGroup::new()
                        .options(self.options)
                        .selected(self.selected)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for RadioGroup {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::RadioGroup;
#[cfg(feature = "multi")]
pub use floom_widget_md_radio_group::MdRadioGroupStyle as RadioGroupStyle;
