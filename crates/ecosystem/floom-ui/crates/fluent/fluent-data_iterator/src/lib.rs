use floom::prelude::*;
use floom::view::AnyView;
use floom::views::{Decorators, Empty, list};
use floom::reactive::{RwSignal, SignalGet};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::data_iterator::DataIteratorProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// Material 3 data iterator: a container that lays out a sequence of item
/// views with an optional header/footer band and optional dividers.
pub struct FluentDataIterator {
    props: DataIteratorProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<AnyView>,
    header: Option<AnyView>,
    footer: Option<AnyView>,
}

impl FluentDataIterator {
    pub fn new() -> Self {
        Self {
            props: DataIteratorProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            items: Vec::new(),
            header: None,
            footer: None,
        }
    }

    /// Append an item view.
    pub fn item(mut self, item: impl IntoView) -> Self {
        self.items.push(item.into_any());
        self
    }

    /// Optional band rendered above the items.
    pub fn header(mut self, header: impl IntoView) -> Self {
        self.header = Some(header.into_any());
        self
    }

    /// Optional band rendered below the items.
    pub fn footer(mut self, footer: impl IntoView) -> Self {
        self.footer = Some(footer.into_any());
        self
    }

    /// Draw a thin divider between consecutive items.
    pub fn divided(mut self, divided: bool) -> Self {
        self.props.divided = divided;
        self
    }

    pub fn props(&self) -> &DataIteratorProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let divided = self.props.divided;
        let tokens = match theme.get().component(ComponentId::DataIterator) {
            ComponentTokens::DataIterator(t) => t,
            _ => unreachable!(),
        };
        let pad = tokens.padding;

        let mut kids: Vec<AnyView> = Vec::new();
        if let Some(h) = self.header {
            kids.push(h);
        }
        for (i, it) in self.items.into_iter().enumerate() {
            if divided && i > 0 {
                let divider = Container::new(Empty::new()).style(move |s| {
                    let t = theme.get();
                    s.height(1.0)
                        .width_pct(100.0)
                        .background(t.colors().outline.to_peniko())
                });
                kids.push(divider.into_any());
            }
            kids.push(it);
        }
        if let Some(f) = self.footer {
            kids.push(f);
        }

        Container::new(list(kids)).style(move |s| {
            let t = theme.get();
            s.padding(pad)
                .background(t.colors().surface.to_peniko())
                .border_radius(8.0)
                .width_pct(100.0)
        })
    }
}

impl Default for FluentDataIterator {
    fn default() -> Self {
        Self::new()
    }
}
