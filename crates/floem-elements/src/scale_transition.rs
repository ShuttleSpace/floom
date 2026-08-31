pub use floem_core_theme::component_tokens::ScaleTransitionTokens;
pub use floem_core_theme::props::scale_transition::ScaleTransitionProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ScaleTransition` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_scale_transition::CupertinoScaleTransition as ScaleTransition;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_scale_transition::CupertinoScaleTransitionStyle as ScaleTransitionStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_scale_transition::FluentScaleTransition as ScaleTransition;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_scale_transition::FluentScaleTransitionStyle as ScaleTransitionStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_scale_transition::MdScaleTransition as ScaleTransition;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_scale_transition::MdScaleTransitionStyle as ScaleTransitionStyle;

/// Runtime-dispatch `ScaleTransition` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet};
    use floem::view::{AnyView, View};
    use floem::views::{Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_scale_transition::MdScaleTransition;
    use floem_widget_cupertino_scale_transition::CupertinoScaleTransition;
    use floem_widget_fluent_scale_transition::FluentScaleTransition;

    pub struct ScaleTransition {
        child: Option<Rc<dyn Fn() -> AnyView>>,
        show: RwSignal<bool>,
    }

    impl ScaleTransition {
        pub fn new() -> Self {
            Self {
                child: None,
                show: RwSignal::new(false),
            }
        }

        /// Set the element that scales in / out. Takes a factory closure because the
        /// child is mounted and unmounted (re-created) on every enter/leave.
        pub fn child<F, IV>(mut self, child: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.child = Some(Rc::new(move || child().into_any()));
            self
        }

        /// Expose the internal show signal so an external trigger can enter/leave.
        pub fn show_signal(&self) -> RwSignal<bool> {
            self.show
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let show = self.show;
            let child = self.child;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => {
                    MdScaleTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
                DesignSystem::Cupertino => {
                    CupertinoScaleTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
                DesignSystem::Fluent => {
                    FluentScaleTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
            }
        }
    }

    impl Default for ScaleTransition {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ScaleTransition;
#[cfg(feature = "multi")]
pub use floem_widget_md_scale_transition::MdScaleTransitionStyle as ScaleTransitionStyle;
