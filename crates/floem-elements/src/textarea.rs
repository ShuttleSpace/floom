pub use floem_core_theme::component_tokens::TextareaTokens;
pub use floem_core_theme::props::textarea::TextareaProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Textarea` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_textarea::CupertinoTextarea as Textarea;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_textarea::CupertinoTextareaStyle as TextareaStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_textarea::FluentTextarea as Textarea;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_textarea::FluentTextareaStyle as TextareaStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_textarea::MdTextarea as Textarea;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_textarea::MdTextareaStyle as TextareaStyle;

/// Runtime-dispatch `Textarea` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::textarea::TextareaProps;

    use floem_widget_md_textarea::MdTextarea;
    use floem_widget_cupertino_textarea::CupertinoTextarea;
    use floem_widget_fluent_textarea::FluentTextarea;

    pub struct Textarea {
        props: TextareaProps,
        on_change: Option<Box<dyn Fn(String)>>,
    }

    impl Textarea {
        pub fn new() -> Self {
            Self {
                props: TextareaProps::default(),
                on_change: None,
            }
        }

        pub fn value(mut self, value: impl Into<String>) -> Self {
            self.props.value = value.into();
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

        pub fn rows(mut self, rows: usize) -> Self {
            self.props.rows = rows;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(String) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdTextarea::new()
                    .value(self.props.value.clone())
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .rows(self.props.rows)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoTextarea::new()
                    .value(self.props.value.clone())
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .rows(self.props.rows)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentTextarea::new()
                    .value(self.props.value.clone())
                    .placeholder(self.props.placeholder.clone())
                    .disabled(self.props.disabled)
                    .rows(self.props.rows)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Textarea {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Textarea;
#[cfg(feature = "multi")]
pub use floem_widget_md_textarea::MdTextareaStyle as TextareaStyle;
