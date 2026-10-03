use floom::prelude::*;
use floom::view::AnyView;
use floom::views::{Decorators, Label};
use floom::reactive::{RwSignal, SignalGet};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::kbd::KbdProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// Material 3 `<kbd>`: a keycap-style box around inline content (typically a
/// key name). Defaults to a monospace font.
pub struct CupertinoKbd {
    props: KbdProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    content: Option<String>,
}

impl CupertinoKbd {
    pub fn new() -> Self {
        Self {
            props: KbdProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn props(&self) -> &KbdProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let tokens = match theme.get().component(ComponentId::Kbd) {
            ComponentTokens::Kbd(t) => t,
            _ => unreachable!(),
        };
        let pad_h = tokens.padding_horizontal;
        let pad_v = tokens.padding_vertical;
        let radius = tokens.border_radius;
        let bw = tokens.border_width;
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
                .border(bw)
                .border_color(t.colors().outline.to_peniko())
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

impl Default for CupertinoKbd {
    fn default() -> Self {
        Self::new()
    }
}
