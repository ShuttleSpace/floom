pub use floem_core_theme::component_tokens::TextFieldTokens;
pub use floem_core_theme::props::text_field::TextFieldProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `TextField` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_text_field::CupertinoTextField as TextField;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_text_field::CupertinoTextFieldStyle as TextFieldStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_text_field::FluentTextField as TextField;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_text_field::FluentTextFieldStyle as TextFieldStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_text_field::MdTextField as TextField;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_text_field::MdTextFieldStyle as TextFieldStyle;

/// Runtime-dispatch `TextField` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::text_field::TextFieldProps;
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_text_field::MdTextField;
    use floem_widget_cupertino_text_field::CupertinoTextField;
    use floem_widget_fluent_text_field::FluentTextField;

    pub struct TextField {
        props: TextFieldProps,
        on_change: Option<Box<dyn Fn(&str)>>,
        theme: Option<MultiTheme>,
    }

    impl TextField {
        pub fn new() -> Self {
            Self {
                props: TextFieldProps::default(),
                on_change: None,
                theme: None,
            }
        }

        pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
            self.props.placeholder = placeholder.into();
            self
        }

        pub fn initial_value(mut self, value: impl Into<String>) -> Self {
            self.props.value = value.into();
            self
        }

        pub fn password(mut self, is_password: bool) -> Self {
            self.props.is_password = is_password;
            self
        }

        pub fn max_length(mut self, max_length: usize) -> Self {
            self.props.max_length = Some(max_length);
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.props.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
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
                    let b = MdTextField::new()
                        .placeholder(self.props.placeholder.clone())
                        .initial_value(self.props.value.clone())
                        .password(self.props.is_password)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(m) = self.props.max_length {
                        b.max_length(m)
                    } else {
                        b
                    };
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoTextField::new()
                        .placeholder(self.props.placeholder.clone())
                        .initial_value(self.props.value.clone())
                        .password(self.props.is_password)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(m) = self.props.max_length {
                        b.max_length(m)
                    } else {
                        b
                    };
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentTextField::new()
                        .placeholder(self.props.placeholder.clone())
                        .initial_value(self.props.value.clone())
                        .password(self.props.is_password)
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(m) = self.props.max_length {
                        b.max_length(m)
                    } else {
                        b
                    };
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for TextField {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::TextField;
#[cfg(feature = "multi")]
pub use floem_widget_md_text_field::MdTextFieldStyle as TextFieldStyle;
