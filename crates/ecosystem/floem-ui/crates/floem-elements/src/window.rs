pub use floem_core_theme::component_tokens::WindowTokens;
pub use floem_core_theme::props::window::WindowProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_window::CupertinoWindow as Window;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_window::CupertinoWindowStyle as WindowStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_window::FluentWindow as Window;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_window::FluentWindowStyle as WindowStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_window::MdWindow as Window;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_window::MdWindowStyle as WindowStyle;

/// Runtime-dispatch `Window` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::{AnyView, View};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::window::WindowProps;

    use floem_widget_md_window::MdWindow;
    use floem_widget_cupertino_window::CupertinoWindow;
    use floem_widget_fluent_window::FluentWindow;

    pub struct Window {
        items: Vec<AnyView>,
        height: f32,
        selected: usize,
        show_arrows: bool,
        show_dots: bool,
        continuous: bool,
        on_change: Option<Box<dyn Fn(usize)>>,
    }

    impl Window {
        pub fn new() -> Self {
            let props = WindowProps::default();
            Self {
                items: Vec::new(),
                height: 0.0,
                selected: 0,
                show_arrows: props.show_arrows,
                show_dots: props.show_dots,
                continuous: props.continuous,
                on_change: None,
            }
        }

        pub fn items(mut self, items: Vec<AnyView>) -> Self {
            self.items = items;
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = height;
            self
        }

        pub fn selected(mut self, selected: usize) -> Self {
            self.selected = selected;
            self
        }

        pub fn show_arrows(mut self, show_arrows: bool) -> Self {
            self.show_arrows = show_arrows;
            self
        }

        pub fn show_dots(mut self, show_dots: bool) -> Self {
            self.show_dots = show_dots;
            self
        }

        pub fn continuous(mut self, continuous: bool) -> Self {
            self.continuous = continuous;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdWindow::new()
                    .items(self.items)
                    .height(self.height)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .continuous(self.continuous)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoWindow::new()
                    .items(self.items)
                    .height(self.height)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .continuous(self.continuous)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentWindow::new()
                    .items(self.items)
                    .height(self.height)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .show_dots(self.show_dots)
                    .continuous(self.continuous)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Window {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Window;
#[cfg(feature = "multi")]
pub use floem_widget_md_window::MdWindowStyle as WindowStyle;
