pub use floom_core_theme::component_tokens::ResponsiveTokens;
pub use floom_core_theme::props::responsive::ResponsiveProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `Responsive` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_responsive::CupertinoResponsive as Responsive;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_responsive::FluentResponsive as Responsive;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_responsive::MdResponsive as Responsive;

/// Runtime-dispatch `Responsive` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom::views::{Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::responsive::ResponsiveProps;

    use floom_widget_md_responsive::MdResponsive;
    use floom_widget_cupertino_responsive::CupertinoResponsive;
    use floom_widget_fluent_responsive::FluentResponsive;

    pub struct Responsive {
        props: ResponsiveProps,
        child: Option<Rc<dyn Fn() -> AnyView>>,
    }

    impl Responsive {
        pub fn new(aspect_ratio: f32) -> Self {
            Self {
                props: ResponsiveProps {
                    aspect_ratio,
                    ..Default::default()
                },
                child: None,
            }
        }

        pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
            self.child = Some(Rc::new(child));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let aspect = self.props.aspect_ratio;
            let child = self.child;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdResponsive::new(aspect).child(child_factory).view().into_any(),
                DesignSystem::Cupertino => {
                    CupertinoResponsive::new(aspect).child(child_factory).view().into_any()
                }
                DesignSystem::Fluent => {
                    FluentResponsive::new(aspect).child(child_factory).view().into_any()
                }
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Responsive;
