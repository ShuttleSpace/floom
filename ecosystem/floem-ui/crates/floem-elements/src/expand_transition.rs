pub use floem_core_theme::component_tokens::ExpandTransitionTokens;
pub use floem_core_theme::props::expand_transition::ExpandTransitionProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ExpandTransition` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_expand_transition::CupertinoExpandTransition as ExpandTransition;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_expand_transition::CupertinoExpandTransitionStyle as ExpandTransitionStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_expand_transition::FluentExpandTransition as ExpandTransition;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_expand_transition::FluentExpandTransitionStyle as ExpandTransitionStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_expand_transition::MdExpandTransition as ExpandTransition;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_expand_transition::MdExpandTransitionStyle as ExpandTransitionStyle;

/// Runtime-dispatch `ExpandTransition` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet};
    use floem::view::{AnyView, View};
    use floem::views::{Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_expand_transition::MdExpandTransition;
    use floem_widget_cupertino_expand_transition::CupertinoExpandTransition;
    use floem_widget_fluent_expand_transition::FluentExpandTransition;

    pub struct ExpandTransition {
        child: Option<Rc<dyn Fn() -> AnyView>>,
        show: RwSignal<bool>,
    }

    impl ExpandTransition {
        pub fn new() -> Self {
            Self {
                child: None,
                show: RwSignal::new(false),
            }
        }

        /// Set the element that expands / collapses. Takes a factory closure because the
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
                    MdExpandTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
                DesignSystem::Cupertino => {
                    CupertinoExpandTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
                DesignSystem::Fluent => {
                    FluentExpandTransition::new()
                        .child(child_factory)
                        .with_show_signal(show)
                        .view()
                        .into_any()
                }
            }
        }
    }

    impl Default for ExpandTransition {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ExpandTransition;
#[cfg(feature = "multi")]
pub use floem_widget_md_expand_transition::MdExpandTransitionStyle as ExpandTransitionStyle;
