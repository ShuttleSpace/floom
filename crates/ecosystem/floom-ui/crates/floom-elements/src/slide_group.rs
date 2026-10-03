pub use floom_core_theme::component_tokens::SlideGroupTokens;
pub use floom_core_theme::props::slide_group::SlideGroupProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_slide_group::CupertinoSlideGroup as SlideGroup;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_slide_group::CupertinoSlideGroupStyle as SlideGroupStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_slide_group::FluentSlideGroup as SlideGroup;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_slide_group::FluentSlideGroupStyle as SlideGroupStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_slide_group::MdSlideGroup as SlideGroup;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_slide_group::MdSlideGroupStyle as SlideGroupStyle;

/// Runtime-dispatch `SlideGroup` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::slide_group::SlideGroupProps;

    use floom_widget_md_slide_group::MdSlideGroup;
    use floom_widget_cupertino_slide_group::CupertinoSlideGroup;
    use floom_widget_fluent_slide_group::FluentSlideGroup;

    pub struct SlideGroup {
        items: Vec<AnyView>,
        selected: usize,
        show_arrows: bool,
        mandatory: bool,
        on_change: Option<Box<dyn Fn(usize)>>,
    }

    impl SlideGroup {
        pub fn new() -> Self {
            let props = SlideGroupProps::default();
            Self {
                items: Vec::new(),
                selected: 0,
                show_arrows: props.show_arrows,
                mandatory: props.mandatory,
                on_change: None,
            }
        }

        pub fn items(mut self, items: Vec<AnyView>) -> Self {
            self.items = items;
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

        pub fn mandatory(mut self, mandatory: bool) -> Self {
            self.mandatory = mandatory;
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
                DesignSystem::Material => MdSlideGroup::new()
                    .items(self.items)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .mandatory(self.mandatory)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSlideGroup::new()
                    .items(self.items)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .mandatory(self.mandatory)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSlideGroup::new()
                    .items(self.items)
                    .selected(self.selected)
                    .show_arrows(self.show_arrows)
                    .mandatory(self.mandatory)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for SlideGroup {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::SlideGroup;
#[cfg(feature = "multi")]
pub use floom_widget_md_slide_group::MdSlideGroupStyle as SlideGroupStyle;
