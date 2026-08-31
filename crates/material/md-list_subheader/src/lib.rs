use floem::prelude::*;
use floem::views::{Container, Decorators, text, Label};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::list_subheader::ListSubheaderProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

pub struct MdListSubheader {
    props: ListSubheaderProps,
    theme: RwSignal<MultiTheme>,
}

impl MdListSubheader {
    pub fn new(text_content: impl Into<String>) -> Self {
        Self {
            props: ListSubheaderProps {
                text: text_content.into(),
                inset: false,
            },
            theme: theme_signal(),
        }
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let text_content = self.props.text;
        let inset = self.props.inset;

        let tokens = match theme.get().component(ComponentId::ListSubheader) {
            ComponentTokens::ListSubheader(t) => t,
            _ => unreachable!(),
        };
        let tk = tokens.clone();

        Container::new(Label::new(text_content)).style(move |s| {
            let t = theme.get();
            let pad = if inset {
                tk.inset_padding
            } else {
                tk.padding_horizontal
            };
            s.height(tk.height)
                .padding_horiz(pad)
                .items_center()
                .color(t.colors().on_surface_variant.to_peniko())
                .font_size(tk.font_size)
        })
    }
}
