//! Fluent list item, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};

use floom_ui_composites::SlotBuilder;
use floom_ui_composites::{ListItemRow, ListItemStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::list_item::ListItemProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

/// That is a bug fix, not a translation, and it is unverifiable from here: no kit
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        // See the note above: v1 sent this to 800.
        ThemeFontWeight::Black => FontWeight::Black,
    }
}

pub struct FluentListItem {
    props: ListItemProps,
    theme: RwSignal<MultiTheme>,
    leading: Option<SlotBuilder>,
    trailing: Option<SlotBuilder>,
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl FluentListItem {
    pub fn new() -> Self {
        Self {
            props: ListItemProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            leading: None,
            trailing: None,
            on_click: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.props.subtitle = Some(subtitle.into());
        self
    }

    pub fn leading(
        mut self,
        leading: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.leading = Some(Arc::new(leading));
        self
    }

    pub fn trailing(
        mut self,
        trailing: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.trailing = Some(Arc::new(trailing));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn nav(mut self, nav: bool) -> Self {
        self.props.nav = nav;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_click = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_click = Some(shared);
        self
    }

    pub fn props(&self) -> &ListItemProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// table at the top of this file, and `floom_ui_composites::ListItemRow`'s note on why
    pub fn resolved_style(&self) -> ListItemStyle {
        let theme = self.theme.get();
        let ComponentTokens::ListItem(tokens) = theme.component(ComponentId::ListItem) else {
            unreachable!("ComponentId::ListItem must resolve to ListItemTokens")
        };
        ListItemStyle {
            min_height: 40.0,
            padding_horizontal: f64::from(tokens.padding_horizontal),
            gap: tokens.gap,
            text_gap: 4.0,
            title_size: 14.0,
            title_weight: to_engine_weight(theme.typography().title.medium.font_weight),
            title_color: to_paint(theme.colors().on_surface),
            subtitle_size: Some(12.0),
            subtitle_color: to_paint(theme.colors().on_surface_variant),
            background: to_paint(theme.colors().surface),
        }
    }

    pub fn view(self) -> ListItemRow {
        let style = self.resolved_style();
        let mut row = ListItemRow::new(self.props.title.clone())
            .style(style)
            .disabled(self.props.disabled);

        if let Some(subtitle) = &self.props.subtitle {
            row = row.subtitle(subtitle.clone());
        }
        if let Some(leading) = &self.leading {
            row = row.leading({
                let leading = leading.clone();
                move || leading()
            });
        }
        if let Some(trailing) = &self.trailing {
            row = row.trailing({
                let trailing = trailing.clone();
                move || trailing()
            });
        }
        if let Some(on_click) = &self.on_click {
            row = row.on_click({
                let on_click = on_click.clone();
                move || on_click()
            });
        }
        row
    }
}

impl Default for FluentListItem {
    fn default() -> Self {
        Self::new()
    }
}
