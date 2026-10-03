pub use floom_core_theme::component_tokens::VirtualScrollerTokens;
pub use floom_core_theme::props::virtual_scroller::VirtualScrollerProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_virtual_scroller::CupertinoVirtualScroller as VirtualScroller;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_virtual_scroller::CupertinoVirtualScrollerStyle as VirtualScrollerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_virtual_scroller::FluentVirtualScroller as VirtualScroller;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_virtual_scroller::FluentVirtualScrollerStyle as VirtualScrollerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_virtual_scroller::MdVirtualScroller as VirtualScroller;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_virtual_scroller::MdVirtualScrollerStyle as VirtualScrollerStyle;

/// Runtime-dispatch `VirtualScroller` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::virtual_scroller::VirtualScrollerProps;

    use floom_widget_md_virtual_scroller::MdVirtualScroller;
    use floom_widget_cupertino_virtual_scroller::CupertinoVirtualScroller;
    use floom_widget_fluent_virtual_scroller::FluentVirtualScroller;

    type ItemFactory = Box<dyn Fn() -> Box<dyn View>>;

    pub struct VirtualScroller {
        items: Vec<ItemFactory>,
        height: f32,
        on_reach_end: Option<Box<dyn Fn()>>,
    }

    impl VirtualScroller {
        pub fn new() -> Self {
            Self {
                items: Vec::new(),
                height: 0.0,
                on_reach_end: None,
            }
        }

        pub fn items(mut self, items: Vec<ItemFactory>) -> Self {
            self.items = items;
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = height;
            self
        }

        pub fn on_reach_end(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_reach_end = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_reach_end = self
                .on_reach_end
                .unwrap_or_else(|| Box::new(|| {}));
            match ds {
                DesignSystem::Material => MdVirtualScroller::new()
                    .items(self.items)
                    .height(self.height)
                    .on_reach_end(on_reach_end)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoVirtualScroller::new()
                    .items(self.items)
                    .height(self.height)
                    .on_reach_end(on_reach_end)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentVirtualScroller::new()
                    .items(self.items)
                    .height(self.height)
                    .on_reach_end(on_reach_end)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for VirtualScroller {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::VirtualScroller;
#[cfg(feature = "multi")]
pub use floom_widget_md_virtual_scroller::MdVirtualScrollerStyle as VirtualScrollerStyle;
