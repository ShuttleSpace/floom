use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Decorators, Empty, h_stack_from_iter, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button_group::ButtonGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

/// Material 3 button group: a surface that lays out a set of buttons with a
/// shared gap (or, when `joined`, as a segmented/connected control).
pub struct MdButtonGroup {
    props: ButtonGroupProps,
    theme: RwSignal<MultiTheme>,
    children: Vec<AnyView>,
}

impl MdButtonGroup {
    pub fn new() -> Self {
        Self {
            props: ButtonGroupProps::default(),
            theme: theme_signal(),
            children: Vec::new(),
        }
    }

    /// Append a child view (typically a button) to the group.
    pub fn child(mut self, child: impl IntoView) -> Self {
        self.children.push(child.into_any());
        self
    }

    /// Render buttons with no gap and shared outer rounding (segmented control).
    pub fn joined(mut self, joined: bool) -> Self {
        self.props.joined = joined;
        self
    }

    /// Lay the buttons out vertically instead of horizontally.
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.props.vertical = vertical;
        self
    }

    pub fn props(&self) -> &ButtonGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let joined = self.props.joined;
        let vertical = self.props.vertical;

        let tokens = match theme.get().component(ComponentId::ButtonGroup) {
            ComponentTokens::ButtonGroup(t) => t,
            _ => unreachable!(),
        };
        let gap = if joined { 0.0 } else { tokens.gap };
        let radius = tokens.border_radius;

        let kids = self.children;
        let inner: AnyView = if vertical {
            list(kids).style(move |s| s.gap(gap)).into_any()
        } else {
            Stack::horizontal_from_iter(kids).style(move |s| s.gap(gap)).into_any()
        };

        Container::new(inner).style(move |s| {
            let t = theme.get();
            s.background(t.colors().surface_variant.to_peniko())
                .border_radius(if joined { radius } else { 0.0 })
                .padding(if joined { 0.0 } else { 4.0 })
                .items_center()
        })
    }
}

impl Default for MdButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}
