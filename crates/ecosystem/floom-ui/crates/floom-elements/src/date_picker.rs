pub use floom_core_theme::component_tokens::DatePickerTokens;
pub use floom_core_theme::props::date_picker::{DatePickerProps, DateValue};

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `DatePicker` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_date_picker::CupertinoDatePicker as DatePicker;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_date_picker::CupertinoDatePickerStyle as DatePickerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_date_picker::FluentDatePicker as DatePicker;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_date_picker::FluentDatePickerStyle as DatePickerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_date_picker::MdDatePicker as DatePicker;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_date_picker::MdDatePickerStyle as DatePickerStyle;

/// Runtime-dispatch `DatePicker` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::date_picker::{DatePickerProps, DateValue};

    use floom_widget_md_date_picker::MdDatePicker;
    use floom_widget_cupertino_date_picker::CupertinoDatePicker;
    use floom_widget_fluent_date_picker::FluentDatePicker;

    pub struct DatePicker {
        props: DatePickerProps,
        on_change: Option<Box<dyn Fn(DateValue)>>,
    }

    impl DatePicker {
        pub fn new() -> Self {
            Self {
                props: DatePickerProps::default(),
                on_change: None,
            }
        }

        pub fn value(mut self, value: DateValue) -> Self {
            self.props.value = Some(value);
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

        pub fn on_change(mut self, callback: impl Fn(DateValue) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let b = MdDatePicker::new()
                        .placeholder(self.props.placeholder.clone())
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(v) = self.props.value {
                        b.value(v)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoDatePicker::new()
                        .placeholder(self.props.placeholder.clone())
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(v) = self.props.value {
                        b.value(v)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentDatePicker::new()
                        .placeholder(self.props.placeholder.clone())
                        .disabled(self.props.disabled)
                        .on_change(on_change);
                    let b = if let Some(v) = self.props.value {
                        b.value(v)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for DatePicker {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::DatePicker;
#[cfg(feature = "multi")]
pub use floom_widget_md_date_picker::MdDatePickerStyle as DatePickerStyle;
