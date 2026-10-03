pub use floom_core_theme::component_tokens::CounterTokens;
pub use floom_core_theme::props::counter::CounterProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `Counter` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_counter::CupertinoCounter as Counter;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_counter::FluentCounter as Counter;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_counter::MdCounter as Counter;

/// Runtime-dispatch `Counter` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::counter::CounterProps;

    use floom_widget_md_counter::MdCounter;
    use floom_widget_cupertino_counter::CupertinoCounter;
    use floom_widget_fluent_counter::FluentCounter;

    pub struct Counter {
        props: CounterProps,
    }

    impl Counter {
        pub fn new() -> Self {
            Self {
                props: CounterProps::default(),
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

        pub fn on_change(mut self, callback: impl Fn(i64) + 'static) -> Self {
            self.props.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdCounter::new().value(props.value).step(props.step);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoCounter::new().value(props.value).step(props.step);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentCounter::new().value(props.value).step(props.step);
                    if let Some(m) = props.min {
                        b = b.min(m);
                    }
                    if let Some(m) = props.max {
                        b = b.max(m);
                    }
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for Counter {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Counter;
