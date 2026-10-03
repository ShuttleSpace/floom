pub use floem_core_theme::component_tokens::ProgressCircularTokens;
pub use floem_core_theme::props::progress_circular::ProgressCircularProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `ProgressCircular` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_progress_circular::CupertinoProgressCircular as ProgressCircular;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_progress_circular::CupertinoProgressCircularStyle as ProgressCircularStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_progress_circular::FluentProgressCircular as ProgressCircular;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_progress_circular::FluentProgressCircularStyle as ProgressCircularStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_progress_circular::MdProgressCircular as ProgressCircular;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_progress_circular::MdProgressCircularStyle as ProgressCircularStyle;

/// Runtime-dispatch `ProgressCircular` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::progress_circular::ProgressCircularProps;

    use floem_widget_md_progress_circular::MdProgressCircular;
    use floem_widget_cupertino_progress_circular::CupertinoProgressCircular;
    use floem_widget_fluent_progress_circular::FluentProgressCircular;

    pub struct ProgressCircular {
        props: ProgressCircularProps,
    }

    impl ProgressCircular {
        pub fn new() -> Self {
            Self {
                props: ProgressCircularProps::default(),
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

        pub fn size(mut self, size: f32) -> Self {
            self.props.size = size;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdProgressCircular::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .size(self.props.size)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoProgressCircular::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .size(self.props.size)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentProgressCircular::new()
                    .value(self.props.value)
                    .indeterminate(self.props.indeterminate)
                    .size(self.props.size)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for ProgressCircular {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ProgressCircular;
#[cfg(feature = "multi")]
pub use floem_widget_md_progress_circular::MdProgressCircularStyle as ProgressCircularStyle;
