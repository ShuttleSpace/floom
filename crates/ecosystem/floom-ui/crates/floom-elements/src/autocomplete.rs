pub use floom_core_theme::component_tokens::AutocompleteTokens;
pub use floom_core_theme::props::autocomplete::AutocompleteProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Autocomplete` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_autocomplete::CupertinoAutocomplete as Autocomplete;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_autocomplete::CupertinoAutocompleteStyle as AutocompleteStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_autocomplete::FluentAutocomplete as Autocomplete;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_autocomplete::FluentAutocompleteStyle as AutocompleteStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_autocomplete::MdAutocomplete as Autocomplete;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_autocomplete::MdAutocompleteStyle as AutocompleteStyle;

/// Runtime-dispatch `Autocomplete` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::autocomplete::AutocompleteProps;

    use floom_widget_md_autocomplete::MdAutocomplete;
    use floom_widget_cupertino_autocomplete::CupertinoAutocomplete;
    use floom_widget_fluent_autocomplete::FluentAutocomplete;

    pub struct Autocomplete {
        props: AutocompleteProps,
        on_change: Option<Box<dyn Fn(Option<String>)>>,
    }

    impl Autocomplete {
        pub fn new() -> Self {
            Self {
                props: AutocompleteProps::default(),
                on_change: None,
            }
        }

        pub fn options(mut self, options: Vec<String>) -> Self {
            self.props.options = options;
            self
        }

        pub fn selected(mut self, selected: Option<usize>) -> Self {
            self.props.selected = selected;
            self
        }

        pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
            self.props.placeholder = placeholder.into();
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.props.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(Option<String>) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdAutocomplete::new()
                    .options(self.props.options.clone())
                    .selected(self.props.selected)
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoAutocomplete::new()
                    .options(self.props.options.clone())
                    .selected(self.props.selected)
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentAutocomplete::new()
                    .options(self.props.options.clone())
                    .selected(self.props.selected)
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Autocomplete {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Autocomplete;
#[cfg(feature = "multi")]
pub use floom_widget_md_autocomplete::MdAutocompleteStyle as AutocompleteStyle;
