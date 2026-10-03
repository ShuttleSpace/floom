pub use floem_core_theme::component_tokens::SelectTokens;
pub use floem_core_theme::props::select::SelectProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_select::CupertinoSelect as Select;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_select::CupertinoSelectStyle as SelectStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_select::FluentSelect as Select;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_select::FluentSelectStyle as SelectStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_select::MdSelect as Select;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_select::MdSelectStyle as SelectStyle;

/// Runtime-dispatch `Select` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_select::MdSelect;
    use floem_widget_cupertino_select::CupertinoSelect;
    use floem_widget_fluent_select::FluentSelect;

    pub struct Select {
        options: Vec<String>,
        selected: Option<usize>,
        placeholder: String,
        disabled: bool,
        on_change: Option<Box<dyn Fn(Option<usize>)>>,
    }

    impl Select {
        pub fn new() -> Self {
            Self {
                options: Vec::new(),
                selected: None,
                placeholder: "Select…".to_string(),
                disabled: false,
                on_change: None,
            }
        }

        pub fn options(mut self, options: Vec<String>) -> Self {
            self.options = options;
            self
        }

        pub fn selected(mut self, selected: Option<usize>) -> Self {
            self.selected = selected;
            self
        }

        pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
            self.placeholder = placeholder.into();
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(Option<usize>) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdSelect::new()
                    .options(self.options)
                    .selected(self.selected)
                    .placeholder(self.placeholder)
                    .disabled(self.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSelect::new()
                    .options(self.options)
                    .selected(self.selected)
                    .placeholder(self.placeholder)
                    .disabled(self.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSelect::new()
                    .options(self.options)
                    .selected(self.selected)
                    .placeholder(self.placeholder)
                    .disabled(self.disabled)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Select {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Select;
#[cfg(feature = "multi")]
pub use floem_widget_md_select::MdSelectStyle as SelectStyle;
