pub use floom_core_theme::component_tokens::StepperTokens;
pub use floom_core_theme::props::stepper::StepperProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_stepper::CupertinoStepper as Stepper;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_stepper::CupertinoStepperStyle as StepperStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_stepper::FluentStepper as Stepper;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_stepper::FluentStepperStyle as StepperStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_stepper::MdStepper as Stepper;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_stepper::MdStepperStyle as StepperStyle;

/// Runtime-dispatch `Stepper` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::stepper::StepperProps;

    use floom_widget_md_stepper::MdStepper;
    use floom_widget_cupertino_stepper::CupertinoStepper;
    use floom_widget_fluent_stepper::FluentStepper;

    pub struct Stepper {
        items: Vec<String>,
        selected: usize,
        on_change: Option<Box<dyn Fn(usize)>>,
        editable: bool,
        non_linear: bool,
    }

    impl Stepper {
        pub fn new() -> Self {
            let props = StepperProps::default();
            Self {
                items: Vec::new(),
                selected: 0,
                on_change: None,
                editable: props.editable,
                non_linear: props.non_linear,
            }
        }

        pub fn items(mut self, items: Vec<String>) -> Self {
            self.items = items;
            self
        }

        pub fn selected(mut self, selected: usize) -> Self {
            self.selected = selected;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn editable(mut self, editable: bool) -> Self {
            self.editable = editable;
            self
        }

        pub fn non_linear(mut self, non_linear: bool) -> Self {
            self.non_linear = non_linear;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdStepper::new()
                    .items(self.items)
                    .selected(self.selected)
                    .on_change(on_change)
                    .editable(self.editable)
                    .non_linear(self.non_linear)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoStepper::new()
                    .items(self.items)
                    .selected(self.selected)
                    .on_change(on_change)
                    .editable(self.editable)
                    .non_linear(self.non_linear)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentStepper::new()
                    .items(self.items)
                    .selected(self.selected)
                    .on_change(on_change)
                    .editable(self.editable)
                    .non_linear(self.non_linear)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Stepper {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Stepper;
#[cfg(feature = "multi")]
pub use floom_widget_md_stepper::MdStepperStyle as StepperStyle;
