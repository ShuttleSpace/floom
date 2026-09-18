pub use floem_core_theme::component_tokens::SlideTransitionTokens;
pub use floem_core_theme::props::slide_transition::{SlideDirection, SlideTransitionProps};

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `SlideTransition` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_slide_transition::CupertinoSlideTransition as SlideTransition;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_slide_transition::CupertinoSlideTransitionStyle as SlideTransitionStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_slide_transition::FluentSlideTransition as SlideTransition;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_slide_transition::FluentSlideTransitionStyle as SlideTransitionStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_slide_transition::MdSlideTransition as SlideTransition;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_slide_transition::MdSlideTransitionStyle as SlideTransitionStyle;

/// Runtime-dispatch `SlideTransition` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet};
    use floem::view::{AnyView, View};
    use floem::views::{Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::slide_transition::{SlideDirection, SlideTransitionProps};

    use floem_widget_md_slide_transition::MdSlideTransition;
    use floem_widget_cupertino_slide_transition::CupertinoSlideTransition;
    use floem_widget_fluent_slide_transition::FluentSlideTransition;

    pub struct SlideTransition {
        props: SlideTransitionProps,
        child: Option<Rc<dyn Fn() -> AnyView>>,
        show: RwSignal<bool>,
    }

    impl SlideTransition {
        pub fn new() -> Self {
            Self {
                props: SlideTransitionProps::default(),
                child: None,
                show: RwSignal::new(false),
            }
        }

        /// Set the element that slides in / out. Takes a factory closure because the
        /// child is mounted and unmounted (re-created) on every enter/leave.
        pub fn child<F, IV>(mut self, child: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.child = Some(Rc::new(move || child().into_any()));
            self
        }

        /// Choose the travel axis (X = horizontal, Y = vertical). Defaults to X.
        pub fn direction(mut self, direction: SlideDirection) -> Self {
            self.props.direction = direction;
            self
        }

        /// Expose the internal show signal so an external trigger can enter/leave.
        pub fn show_signal(&self) -> RwSignal<bool> {
            self.show
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let direction = self.props.direction;
            let show = self.show;
            let child = self.child;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdSlideTransition::new()
                    .child(child_factory)
                    .direction(direction)
                    .with_show_signal(show)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSlideTransition::new()
                    .child(child_factory)
                    .direction(direction)
                    .with_show_signal(show)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSlideTransition::new()
                    .child(child_factory)
                    .direction(direction)
                    .with_show_signal(show)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for SlideTransition {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::SlideTransition;
#[cfg(feature = "multi")]
pub use floem_widget_md_slide_transition::MdSlideTransitionStyle as SlideTransitionStyle;
