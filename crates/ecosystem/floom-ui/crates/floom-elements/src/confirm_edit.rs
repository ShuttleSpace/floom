pub use floom_core_theme::component_tokens::ConfirmEditTokens;
pub use floom_core_theme::props::confirm_edit::ConfirmEditProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ConfirmEdit` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_confirm_edit::CupertinoConfirmEdit as ConfirmEdit;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_confirm_edit::FluentConfirmEdit as ConfirmEdit;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_confirm_edit::MdConfirmEdit as ConfirmEdit;

/// Runtime-dispatch `ConfirmEdit` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;

    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom::views::{Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::confirm_edit::ConfirmEditProps;

    use floom_widget_md_confirm_edit::MdConfirmEdit;
    use floom_widget_cupertino_confirm_edit::CupertinoConfirmEdit;
    use floom_widget_fluent_confirm_edit::FluentConfirmEdit;

    pub struct ConfirmEdit {
        props: ConfirmEditProps,
        child: Option<Rc<dyn Fn() -> AnyView>>,
    }

    impl ConfirmEdit {
        pub fn new() -> Self {
            Self {
                props: ConfirmEditProps::default(),
                child: None,
            }
        }

        pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
            self.child = Some(Rc::new(child));
            self
        }

        pub fn value(mut self, v: impl Into<String>) -> Self {
            self.props.value = v.into();
            self
        }

        pub fn on_save(mut self, callback: impl Fn(String) + 'static) -> Self {
            self.props.on_save = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let value = self.props.value;
            let on_save = self.props.on_save.map(Rc::new);
            let child = self.child;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdConfirmEdit::new()
                    .child(child_factory)
                    .value(value.clone())
                    .on_save({
                        let cb = on_save.clone();
                        move |s| {
                            if let Some(f) = cb.as_ref() {
                                f(s);
                            }
                        }
                    })
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoConfirmEdit::new()
                    .child(child_factory)
                    .value(value.clone())
                    .on_save({
                        let cb = on_save.clone();
                        move |s| {
                            if let Some(f) = cb.as_ref() {
                                f(s);
                            }
                        }
                    })
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentConfirmEdit::new()
                    .child(child_factory)
                    .value(value.clone())
                    .on_save({
                        let cb = on_save.clone();
                        move |s| {
                            if let Some(f) = cb.as_ref() {
                                f(s);
                            }
                        }
                    })
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for ConfirmEdit {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ConfirmEdit;
