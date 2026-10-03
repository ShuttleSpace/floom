use floom::prelude::*;
use floom::view::AnyView;
use floom::views::{Container, Empty, Stack};
use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
/// Material 3 main content region.
///
/// Reads the shared layout signal and offsets its content by the registered
/// top/bottom/left/right insets, so it never hides under a fixed bar or an
/// open navigation drawer. The insets are applied as reactive spacers
/// (top/bottom height, left/right width) rather than per-side padding, because
/// the API only exposes combined `padding_horiz` / `padding_vert`.
pub struct CupertinoMain {
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl CupertinoMain {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
        }
    }

    /// The scrollable/static content of the application.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Build the main content region with automatic layout insets.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let layout = layout_signal();
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let top = Empty::new().style(move |s| {
            let l = layout.get();
            s.width_pct(100.0).height(l.top())
        });
        let bottom = Empty::new().style(move |s| {
            let l = layout.get();
            s.width_pct(100.0).height(l.bottom())
        });
        let left = Empty::new().style(move |s| {
            let l = layout.get();
            s.height_pct(100.0).width(l.left())
        });
        let right = Empty::new().style(move |s| {
            let l = layout.get();
            s.height_pct(100.0).width(l.right())
        });

        let content = Container::new(child).style(move |s| {
            let t = theme.get();
            s.flex_grow(1.0)
                .background(t.colors().background.to_peniko())
        });

        let middle = Stack::horizontal((left, content, right)).style(|s| {
            s.width_pct(100.0).flex_grow(1.0)
        });

        Stack::vertical((top, middle, bottom)).style(|s| {
            s.width_pct(100.0).height_pct(100.0)
        })
    }
}

impl Default for CupertinoMain {
    fn default() -> Self {
        Self::new()
    }
}
