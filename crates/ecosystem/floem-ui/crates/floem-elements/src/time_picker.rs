pub use floem_core_theme::component_tokens::TimePickerTokens;
pub use floem_core_theme::props::time_picker::{TimePickerProps, TimeValue};

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `TimePicker` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_time_picker::CupertinoTimePicker as TimePicker;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_time_picker::CupertinoTimePickerStyle as TimePickerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_time_picker::FluentTimePicker as TimePicker;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_time_picker::FluentTimePickerStyle as TimePickerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_time_picker::MdTimePicker as TimePicker;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_time_picker::MdTimePickerStyle as TimePickerStyle;

/// Runtime-dispatch `TimePicker` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::time_picker::{TimePickerProps, TimeValue};

    use floem_widget_md_time_picker::MdTimePicker;
    use floem_widget_cupertino_time_picker::CupertinoTimePicker;
    use floem_widget_fluent_time_picker::FluentTimePicker;

    pub struct TimePicker {
        props: TimePickerProps,
        on_change: Option<Box<dyn Fn(TimeValue)>>,
    }

    impl TimePicker {
        pub fn new() -> Self {
            Self {
                props: TimePickerProps::default(),
                on_change: None,
            }
        }

        pub fn value(mut self, value: TimeValue) -> Self {
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

        pub fn on_change(mut self, callback: impl Fn(TimeValue) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let b = MdTimePicker::new()
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
                    let b = CupertinoTimePicker::new()
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
                    let b = FluentTimePicker::new()
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

    impl Default for TimePicker {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::TimePicker;
#[cfg(feature = "multi")]
pub use floem_widget_md_time_picker::MdTimePickerStyle as TimePickerStyle;
