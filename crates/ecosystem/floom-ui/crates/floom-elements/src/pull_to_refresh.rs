pub use floom_core_theme::component_tokens::PullToRefreshTokens;
pub use floom_core_theme::props::pull_to_refresh::PullToRefreshProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `PullToRefresh` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_pull_to_refresh::CupertinoPullToRefresh as PullToRefresh;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_pull_to_refresh::FluentPullToRefresh as PullToRefresh;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_pull_to_refresh::MdPullToRefresh as PullToRefresh;

/// Runtime-dispatch `PullToRefresh` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floom::IntoView;
    use floom::reactive::{RwSignal, SignalGet};
    use floom::view::{AnyView, View};
    use floom::views::{Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::pull_to_refresh::PullToRefreshProps;

    use floom_widget_md_pull_to_refresh::MdPullToRefresh;
    use floom_widget_cupertino_pull_to_refresh::CupertinoPullToRefresh;
    use floom_widget_fluent_pull_to_refresh::FluentPullToRefresh;

    pub struct PullToRefresh {
        props: PullToRefreshProps,
        child: Option<Rc<dyn Fn() -> AnyView>>,
        refreshing: RwSignal<bool>,
    }

    impl PullToRefresh {
        pub fn new() -> Self {
            let props = PullToRefreshProps::default();
            Self {
                props,
                child: None,
                refreshing: RwSignal::new(false),
            }
        }

        pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
            self.child = Some(Rc::new(child));
            self
        }

        pub fn loading(mut self, loading: bool) -> Self {
            self.props.loading = loading;
            self
        }

        pub fn threshold(mut self, threshold: f32) -> Self {
            self.props.threshold = threshold;
            self
        }

        pub fn on_refresh(mut self, callback: impl Fn() + 'static) -> Self {
            self.props.on_refresh = Some(Box::new(callback));
            self
        }

        /// Expose the internal refreshing signal so the host can clear it when done.
        pub fn refreshing_signal(&self) -> RwSignal<bool> {
            self.refreshing
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            let child = self.child;
            let refreshing = self.refreshing;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdPullToRefresh::new()
                    .child(child_factory)
                    .loading(props.loading)
                    .threshold(props.threshold)
                    .with_refreshing_signal(refreshing)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoPullToRefresh::new()
                    .child(child_factory)
                    .loading(props.loading)
                    .threshold(props.threshold)
                    .with_refreshing_signal(refreshing)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentPullToRefresh::new()
                    .child(child_factory)
                    .loading(props.loading)
                    .threshold(props.threshold)
                    .with_refreshing_signal(refreshing)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for PullToRefresh {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::PullToRefresh;
