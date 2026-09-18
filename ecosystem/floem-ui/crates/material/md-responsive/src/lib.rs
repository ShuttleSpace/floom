use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, dyn_view};
use floem_core_theme::props::responsive::ResponsiveProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

pub struct MdResponsive {
    props: ResponsiveProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Box<dyn Fn() -> AnyView>>,
}

impl MdResponsive {
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            props: ResponsiveProps {
                aspect_ratio,
                ..Default::default()
            },
            theme: theme_signal(),
            child: None,
        }
    }

    pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let aspect = self.props.aspect_ratio;
        let child = self
            .child
            .unwrap_or_else(|| Box::new(|| Empty::new().into_any()));

        Container::new(dyn_view(move || -> AnyView { (child)() })).style(move |s| {
            let t = theme.get();
            s.aspect_ratio(aspect)
                .width_full()
                .background(t.colors().surface_variant.to_peniko())
                .items_center()
                .justify_center()
        })
    }
}
