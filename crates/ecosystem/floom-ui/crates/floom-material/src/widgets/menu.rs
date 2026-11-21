use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Shadow;
use floom_ui_composites::{Menu, MenuItem, MenuStyle};
use floom_widgets::SharedFlag;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::menu::{MenuItem as MenuItemProps, MenuProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialMenuStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub container_color: Color,
    /// The rows' ink.
    pub text_color: Color,
    /// The fill behind the row the pointer is over.
    pub hover_color: Color,
    pub min_width: f32,
    pub max_width: f32,
    pub item_height: f32,
    pub item_padding_horizontal: f32,
    pub item_padding_vertical: f32,
    pub item_font_size: f32,
    pub padding: f32,
    pub radius: f32,
    pub shadow: Shadow,
}

/// Material's menu.
pub struct MaterialMenu {
    props: MenuProps,
    /// Whether the menu is up, shared with whoever opens it.
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
}

impl MaterialMenu {
    pub fn new() -> Self {
        Self {
            props: MenuProps::default(),
            open: floom_widgets::shared_flag(false),
            theme: theme_signal(),
        }
    }

    /// Append a row.
    pub fn item(
        mut self,
        label: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        self.props.items.push(MenuItemProps {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }

    /// Whether the menu starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        floom_widgets::write_flag(&self.open, open);
        self
    }

    pub fn props(&self) -> &MenuProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Use a flag the caller already has, so the thing that opens the menu and
    /// the menu itself share one truth.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    pub fn select(&self, index: usize) {
        if let Some(item) = self.props.items.get(index) {
            if let Some(callback) = &item.on_select {
                callback();
            }
        }
    }

    pub fn resolved_style(&self) -> MaterialMenuStyle {
        let theme = self.theme.get();
        let ComponentTokens::Menu(tokens) = theme.component(ComponentId::Menu) else {
            unreachable!("ComponentId::Menu must resolve to MenuTokens")
        };
        MaterialMenuStyle {
            design_system: "material",
            container_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            hover_color: with_alpha(theme.colors().on_surface, 8),
            min_width: tokens.min_width,
            max_width: tokens.max_width,
            item_height: tokens.item_height,
            item_padding_horizontal: 12.0,
            item_padding_vertical: 8.0,
            item_font_size: theme.typography().label.large.font_size,
            padding: tokens.padding,
            radius: 4.0,
            shadow: Shadow {
                color: to_paint(with_alpha(Color::BLACK, 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
        }
    }

    /// The menu: the panel, its rows, and the flag that says whether it is up.
    pub fn view(self) -> Menu {
        let style = self.resolved_style();
        let mut menu = Menu::new()
            .style(MenuStyle {
                background: to_paint(style.container_color),
                text_color: to_paint(style.text_color),
                hover_color: to_paint(style.hover_color),
                min_width: f64::from(style.min_width),
                max_width: f64::from(style.max_width),
                item_height: f64::from(style.item_height),
                item_padding_horizontal: f64::from(style.item_padding_horizontal),
                item_padding_vertical: f64::from(style.item_padding_vertical),
                item_font_size: style.item_font_size,
                padding: f64::from(style.padding),
                radius: style.radius,
                shadow: style.shadow,
            })
            .with_open_flag(self.open.clone());
        for item in self.props.items {
            menu = menu.item(MenuItem {
                label: item.label,
                on_select: item.on_select,
            });
        }
        menu
    }
}

impl Default for MaterialMenu {
    fn default() -> Self {
        Self::new()
    }
}
