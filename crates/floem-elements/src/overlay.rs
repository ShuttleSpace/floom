pub use floem_core_theme::component_tokens::OverlayTokens;
pub use floem_core_theme::props::overlay::OverlayProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_overlay::CupertinoOverlay as Overlay;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_overlay::CupertinoOverlayStyle as OverlayStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_overlay::FluentOverlay as Overlay;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_overlay::FluentOverlayStyle as OverlayStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_overlay::MdOverlay as Overlay;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_overlay::MdOverlayStyle as OverlayStyle;

/// Runtime-dispatch `Overlay` for the `multi` feature.
///
/// It stores the neutral builder state and, at `view()` time, reads the active
/// Design System to construct the matching concrete overlay (Material /
/// Cupertino / Fluent). The selected page in the demo is rebuilt whenever the
/// Design System signal changes, so the `view()` call picks up the new DS.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
    use floem::view::View;
    use floem::views::{dyn_view, Empty};
    use std::rc::Rc;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_overlay::MdOverlay;
    use floem_widget_cupertino_overlay::CupertinoOverlay;
    use floem_widget_fluent_overlay::FluentOverlay;

    pub struct Overlay {
        show: RwSignal<bool>,
        on_click: Option<Rc<dyn Fn()>>,
        // `AnyView` is not `Clone`, so the content is kept as a factory closure
        // that can be re-invoked for whichever concrete DS is matched below.
        child: Option<Rc<dyn Fn() -> floem::view::AnyView>>,
        theme: Option<MultiTheme>,
    }

    impl Overlay {
        pub fn new() -> Self {
            Self {
                show: RwSignal::new(false),
                on_click: None,
                child: None,
                theme: None,
            }
        }

        /// Accept a child factory so the content can be rebuilt for each
        /// concrete DS in the `dyn_view` below.
        pub fn child<F, IV>(mut self, child: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.child = Some(Rc::new(move || child().into_any()));
            self
        }

        pub fn show(mut self, show: bool) -> Self {
            self.show.set(show);
            self
        }

        pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_click = Some(Rc::new(callback));
            self
        }

        pub fn show_signal(&self) -> RwSignal<bool> {
            self.show
        }

        /// Replace the internal show signal with an external one so a trigger
        /// (built separately, e.g. on the same page) can show/hide the overlay.
        pub fn with_show_signal(mut self, show: RwSignal<bool>) -> Self {
            self.show = show;
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let show = self.show;
            let on_click = self.on_click;
            let child = self.child;
            let theme = self.theme;
            dyn_view(move || {
                let ds = design_system_signal().get();
                let on_click = on_click.clone();
                let on_click_cb = move || {
                    if let Some(f) = on_click.as_ref() {
                        f();
                    }
                };
                // `child` is `Option<Rc<...>>`; clone it here (inside the
                // `Fn` closure) before moving into the inner factory so each
                // re-skin gets its own copy and the outer closure keeps its
                // captured value. Mirrors the `on_click` pattern above.
                let child = child.clone();
                let child_factory = move || match child.as_ref() {
                    Some(f) => f(),
                    None => floem::views::Empty::new().into_any(),
                };
                match ds {
                    DesignSystem::Material => {
                        let mut c = MdOverlay::new()
                            .child(child_factory())
                            .on_click(on_click_cb)
                            .with_show_signal(show);
                        if let Some(th) = theme.clone() {
                            c = c.with_theme(th);
                        }
                        c.view().into_any()
                    }
                    DesignSystem::Cupertino => {
                        let mut c = CupertinoOverlay::new()
                            .child(child_factory())
                            .on_click(on_click_cb)
                            .with_show_signal(show);
                        if let Some(th) = theme.clone() {
                            c = c.with_theme(th);
                        }
                        c.view().into_any()
                    }
                    DesignSystem::Fluent => {
                        let mut c = FluentOverlay::new()
                            .child(child_factory())
                            .on_click(on_click_cb)
                            .with_show_signal(show);
                        if let Some(th) = theme.clone() {
                            c = c.with_theme(th);
                        }
                        c.view().into_any()
                    }
                }
            })
        }
    }

    impl Default for Overlay {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Overlay;
#[cfg(feature = "multi")]
pub use floem_widget_md_overlay::MdOverlayStyle as OverlayStyle;
