use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Decorators, Label};
use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::code::CodeProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

/// Material 3 `<code>`: an inline code chip with a subtle surface background.
/// Defaults to a monospace font.
pub struct MdCode {
    props: CodeProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    content: Option<String>,
}

impl MdCode {
    pub fn new() -> Self {
        Self {
            props: CodeProps::default(),
            theme: theme_signal(),
            child: None,
            content: None,
        }
    }

    /// Convenience text content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Arbitrary child view (takes precedence over `content`).
    pub fn child(mut self, child: impl IntoView) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Render the content in a monospace font.
    pub fn mono(mut self, mono: bool) -> Self {
        self.props.mono = mono;
        self
    }

    pub fn props(&self) -> &CodeProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let tokens = match theme.get().component(ComponentId::Code) {
            ComponentTokens::Code(t) => t,
            _ => unreachable!(),
        };
        let pad_h = tokens.padding_horizontal;
        let pad_v = tokens.padding_vertical;
        let radius = tokens.border_radius;
        let fs = tokens.font_size;
        let mono = self.props.mono;
        let child = self.child;
        let content = self.content.clone();

        let inner: AnyView = if let Some(c) = child {
            c
        } else {
            Label::new(content.unwrap_or_default())
                .style(move |s| s.color(theme.get().colors().on_surface.to_peniko()))
                .into_any()
        };

        Container::new(inner).style(move |s| {
            let t = theme.get();
            let mut st = s
                .padding_horiz(pad_h)
                .padding_vert(pad_v)
                .border_radius(radius)
                .background(t.colors().surface_variant.to_peniko())
                .font_size(fs)
                .color(t.colors().on_surface.to_peniko());
            if mono {
                st = st.font_family("monospace");
            }
            st
        })
    }
}

impl Default for MdCode {
    fn default() -> Self {
        Self::new()
    }
}
