use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Decorators, h_stack_from_iter};
use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::list_item::ListItemProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

/// Material 3 list item: a single row with an optional leading slot, a title /
/// subtitle text column, and an optional trailing slot. Clickable when an
/// `on_click` is supplied and not `disabled`.
pub struct MdListItem {
    props: ListItemProps,
    theme: RwSignal<MultiTheme>,
    leading: Option<AnyView>,
    trailing: Option<AnyView>,
}

impl MdListItem {
    pub fn new() -> Self {
        Self {
            props: ListItemProps::default(),
            theme: theme_signal(),
            leading: None,
            trailing: None,
        }
    }

    /// Primary line of text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    /// Optional secondary line of text.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.props.subtitle = Some(subtitle.into());
        self
    }

    /// Slot rendered before the text column (typically an icon/avatar).
    pub fn leading(mut self, leading: impl IntoView) -> Self {
        self.leading = Some(leading.into_any());
        self
    }

    /// Slot rendered after the text column (typically a switch/chevron).
    pub fn trailing(mut self, trailing: impl IntoView) -> Self {
        self.trailing = Some(trailing.into_any());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Navigation styling (extra trailing affordance space).
    pub fn nav(mut self, nav: bool) -> Self {
        self.props.nav = nav;
        self
    }

    /// Clicked callback (ignored when `disabled`).
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &ListItemProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let on_click = self.props.on_click;
        let title = self.props.title.clone();
        let subtitle = self.props.subtitle.clone();
        let leading = self.leading;
        let trailing = self.trailing;

        let tokens = match theme.get().component(ComponentId::ListItem) {
            ComponentTokens::ListItem(t) => t,
            _ => unreachable!(),
        };
        let min_height = tokens.min_height;
        let pad_h = tokens.padding_horizontal;
        let gap = tokens.gap;
        let title_fs = tokens.title_font_size;
        let subtitle_fs = tokens.subtitle_font_size;

        let mut row_kids: Vec<AnyView> = Vec::new();
        if let Some(lead) = leading {
            row_kids.push(lead);
        }

        let mut text_col_kids: Vec<AnyView> = Vec::new();
        text_col_kids.push(
            text(title)
                .style(move |s| {
                    let t = theme.get();
                    s.font_size(title_fs)
                        .font_weight(t.typography().title.medium.font_weight.to_floem())
                        .color(t.colors().on_surface.to_peniko())
                })
                .into_any(),
        );
        if let Some(sub) = subtitle {
            text_col_kids.push(
                text(sub)
                    .style(move |s| {
                        let t = theme.get();
                        s.font_size(subtitle_fs).color(t.colors().on_surface_variant.to_peniko())
                    })
                    .into_any(),
            );
        }
        row_kids.push(
            h_stack_from_iter(text_col_kids)
                .style(move |s| s.gap(2.0).flex_grow(1.0).items_center())
                .into_any(),
        );
        if let Some(trail) = trailing {
            row_kids.push(trail);
        }

        let row = h_stack_from_iter(row_kids).style(move |s| {
            s.gap(gap)
                .items_center()
                .width_pct(100.0)
                .min_height(min_height)
                .padding_horiz(pad_h)
        });

        Container::new(row)
            .style(move |s| {
                let t = theme.get();
                let mut st = s
                    .background(t.colors().surface.to_peniko())
                    .items_center()
                    .set_disabled(disabled);
                if !disabled {
                    st = st.cursor(CursorStyle::Pointer);
                }
                st
            })
            .action(move || {
                if !disabled {
                    if let Some(cb) = &on_click {
                        cb();
                    }
                }
            })
    }
}

impl Default for MdListItem {
    fn default() -> Self {
        Self::new()
    }
}
