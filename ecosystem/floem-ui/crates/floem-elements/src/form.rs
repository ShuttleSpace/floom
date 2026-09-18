pub use floem_core_theme::component_tokens::FormTokens;
pub use floem_core_theme::props::form::FormProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_form::CupertinoForm as Form;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_form::CupertinoFormStyle as FormStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_form::FluentForm as Form;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_form::FluentFormStyle as FormStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_form::MdForm as Form;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_form::MdFormStyle as FormStyle;

/// Runtime-dispatch `Form` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::RwSignal;
    use floem::reactive::SignalGet;
    use floem::view::{AnyView, View};
    use floem::views::{empty, Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::form::FormProps;

    use floem_widget_md_form::MdForm;
    use floem_widget_cupertino_form::CupertinoForm;
    use floem_widget_fluent_form::FluentForm;

    pub struct Form {
        child: Option<AnyView>,
        submit_label: String,
        show_submit: bool,
        on_submit: Option<Box<dyn Fn()>>,
        valid: RwSignal<bool>,
        errors: RwSignal<Vec<String>>,
    }

    impl Form {
        pub fn new() -> Self {
            let props = FormProps::default();
            Self {
                child: None,
                submit_label: props.submit_label,
                show_submit: props.show_submit,
                on_submit: None,
                valid: RwSignal::new(true),
                errors: RwSignal::new(Vec::new()),
            }
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
            self.submit_label = submit_label.into();
            self
        }

        pub fn show_submit(mut self, show_submit: bool) -> Self {
            self.show_submit = show_submit;
            self
        }

        pub fn on_submit(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_submit = Some(Box::new(callback));
            self
        }

        /// Expose the validity signal so the consumer can enable/disable submit.
        pub fn valid_signal(&self) -> RwSignal<bool> {
            self.valid
        }

        /// Expose the form-level error signal so the consumer can show a summary.
        pub fn errors_signal(&self) -> RwSignal<Vec<String>> {
            self.errors
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            let on_submit = self
                .on_submit
                .unwrap_or_else(|| Box::new(|| {}));
            match ds {
                DesignSystem::Material => MdForm::new()
                    .child(child)
                    .submit_label(self.submit_label)
                    .show_submit(self.show_submit)
                    .on_submit(on_submit)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoForm::new()
                    .child(child)
                    .submit_label(self.submit_label)
                    .show_submit(self.show_submit)
                    .on_submit(on_submit)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentForm::new()
                    .child(child)
                    .submit_label(self.submit_label)
                    .show_submit(self.show_submit)
                    .on_submit(on_submit)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Form {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Form;
#[cfg(feature = "multi")]
pub use floem_widget_md_form::MdFormStyle as FormStyle;
