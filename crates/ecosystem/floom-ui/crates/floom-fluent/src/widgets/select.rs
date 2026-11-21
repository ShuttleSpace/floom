//! Fluent's select (dropdown), ported to the Floom 2.0 widget API.

use crate::widgets::icon_paths::fluent_icon_path;
use floom_core::{Point, Rect, Size};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Select, SelectStyle};
use floom_widgets::VectorIcon;

const DESIGN_SYSTEM: &str = "fluent";

pub struct FluentSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSelect {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props: SelectProps::default(),
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.props.selected = selected;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &SelectProps {
        &self.props
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> SelectStyle {
        let theme = self.theme.get();
        let ComponentTokens::Select(tokens) = theme.component(ComponentId::Select) else {
            unreachable!("ComponentId::Select must resolve to SelectTokens")
        };
        let surface = theme.colors().surface;
        let on_surface = theme.colors().on_surface;

        SelectStyle {
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,
            icon_size: tokens.icon_size,

            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            panel_gap: 0.0,
            panel_padding: 4.0,
            border_width: 1.0,
            text_size: theme.typography().body.medium.font_size,
            panel_shadow_blur: 12.0,
            panel_shadow_spread: 1.0,

            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            outline: to_paint(on_surface),
            placeholder: to_paint(on_surface),
            chevron: to_paint(with_alpha(on_surface, 160)),
            panel_border: to_paint(Color::TRANSPARENT),
            panel_shadow: to_paint(with_alpha(Color::BLACK, 20)),

            hover_row: to_paint(with_alpha(on_surface, 8)),
            selected_row: to_paint(with_alpha(on_surface, 8)),

            disabled_text: to_paint(with_alpha(on_surface, 97)),
            disabled_chevron: to_paint(with_alpha(on_surface, 97)),
            disabled_border: to_paint(with_alpha(on_surface, 38)),
            disabled_background: to_paint(surface),
        }
    }

    pub fn view(self) -> Select {
        let style = self.resolved_style();
        let options = self.props.options.clone();
        let selected = self.props.selected;
        let disabled = self.props.disabled;

        let mut select = Select::new(options)
            .style(style)
            .disabled(disabled)
            .placeholder(self.props.placeholder);

        select = select.chevron(VectorIcon::from_path_data(
            fluent_icon_path("expand_more"),
            Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(24.0, 24.0)),
        ));

        if let Some(index) = selected {
            select = select.selected(index);
        }

        let on_change = self.props.on_change;
        if let Some(on_change) = on_change {
            select = select.on_change(move |index| on_change(index));
        }

        select
    }
}

impl Default for FluentSelect {
    fn default() -> Self {
        Self::new()
    }
}
