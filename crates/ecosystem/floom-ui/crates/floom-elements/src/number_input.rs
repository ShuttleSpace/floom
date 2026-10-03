pub use floom_core_theme::component_tokens::NumberInputTokens;
pub use floom_core_theme::props::number_input::NumberInputProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `NumberInput` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_number_input::CupertinoNumberInput as NumberInput;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_number_input::FluentNumberInput as NumberInput;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_number_input::MdNumberInput as NumberInput;

/// Runtime-dispatch `NumberInput` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::number_input::NumberInputProps;

    use floom_widget_md_number_input::MdNumberInput;
    use floom_widget_cupertino_number_input::CupertinoNumberInput;
    use floom_widget_fluent_number_input::FluentNumberInput;

    pub struct NumberInput {
        props: NumberInputProps,
    }

    impl NumberInput {
        pub fn new() -> Self {
            Self {
                props: NumberInputProps::default(),
            }
        }

        pub fn value(mut self, v: i64) -> Self {
            self.props.value = v;
            self
        }

        pub fn min(mut self, m: i64) -> Self {
            self.props.min = Some(m);
            self
        }

        pub fn max(mut self, m: i64) -> Self {
            self.props.max = Some(m);
            self
        }

        pub fn step(mut self, s: i64) -> Self {
            self.props.step = s;
            self
        }

        pub fn label(mut self, l: impl Into<String>) -> Self {
            self.props.label = Some(l.into());
            self
        }

        pub fn disabled(mut self, d: bool) -> Self {
            self.props.disabled = d;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(i64) + 'static) -> Self {
            self.props.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdNumberInput::new()
                        .value(props.value)
                        .step(props.step)
                        .disabled(props.disabled);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(l) = props.label.clone() {
                        b = b.label(l);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoNumberInput::new()
                        .value(props.value)
                        .step(props.step)
                        .disabled(props.disabled);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(l) = props.label.clone() {
                        b = b.label(l);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentNumberInput::new()
                        .value(props.value)
                        .step(props.step)
                        .disabled(props.disabled);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(l) = props.label.clone() {
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

    impl Default for NumberInput {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::NumberInput;
