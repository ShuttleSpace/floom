pub use floom_core_theme::component_tokens::ProgressLinearTokens;
pub use floom_core_theme::props::progress_linear::ProgressLinearProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `ProgressLinear` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_progress_linear::CupertinoProgressLinear as ProgressLinear;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_progress_linear::CupertinoProgressLinearStyle as ProgressLinearStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_progress_linear::FluentProgressLinear as ProgressLinear;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_progress_linear::FluentProgressLinearStyle as ProgressLinearStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_progress_linear::MdProgressLinear as ProgressLinear;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_progress_linear::MdProgressLinearStyle as ProgressLinearStyle;

/// Runtime-dispatch `ProgressLinear` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::progress_linear::ProgressLinearProps;

    use floom_widget_md_progress_linear::MdProgressLinear;
    use floom_widget_cupertino_progress_linear::CupertinoProgressLinear;
    use floom_widget_fluent_progress_linear::FluentProgressLinear;

    pub struct ProgressLinear {
        props: ProgressLinearProps,
    }

    impl ProgressLinear {
        pub fn new() -> Self {
            Self {
                props: ProgressLinearProps::default(),
            }
        }

        pub fn value(mut self, value: f32) -> Self {
            self.props.value = value;
            self
        }

        pub fn indeterminate(mut self, indeterminate: bool) -> Self {
            self.props.indeterminate = indeterminate;
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.props.height = height;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdProgressLinear::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .height(self.props.height)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoProgressLinear::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .height(self.props.height)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentProgressLinear::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .height(self.props.height)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for ProgressLinear {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ProgressLinear;
#[cfg(feature = "multi")]
pub use floom_widget_md_progress_linear::MdProgressLinearStyle as ProgressLinearStyle;
