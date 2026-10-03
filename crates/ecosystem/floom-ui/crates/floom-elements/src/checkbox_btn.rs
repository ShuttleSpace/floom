pub use floom_core_theme::component_tokens::CheckboxBtnTokens;
pub use floom_core_theme::props::checkbox_btn::CheckboxBtnProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `CheckboxBtn` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_checkbox_btn::CupertinoCheckboxBtn as CheckboxBtn;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_checkbox_btn::FluentCheckboxBtn as CheckboxBtn;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_checkbox_btn::MdCheckboxBtn as CheckboxBtn;

/// Runtime-dispatch `CheckboxBtn` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::checkbox_btn::CheckboxBtnProps;

    use floom_widget_md_checkbox_btn::MdCheckboxBtn;
    use floom_widget_cupertino_checkbox_btn::CupertinoCheckboxBtn;
    use floom_widget_fluent_checkbox_btn::FluentCheckboxBtn;

    pub struct CheckboxBtn {
        props: CheckboxBtnProps,
    }

    impl CheckboxBtn {
        pub fn new() -> Self {
            Self {
                props: CheckboxBtnProps::default(),
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

        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.props.label = Some(label.into());
            self
        }

        pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
            self.props.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdCheckboxBtn::new().checked(props.checked).disabled(props.disabled);
                    if let Some(l) = props.label {
                        b = b.label(l);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b =
                        CupertinoCheckboxBtn::new().checked(props.checked).disabled(props.disabled);
                    if let Some(l) = props.label {
                        b = b.label(l);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentCheckboxBtn::new().checked(props.checked).disabled(props.disabled);
                    if let Some(l) = props.label {
                        b = b.label(l);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for CheckboxBtn {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::CheckboxBtn;
