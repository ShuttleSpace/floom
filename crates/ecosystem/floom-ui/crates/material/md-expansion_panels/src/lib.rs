use std::sync::Arc;

use floom_core::PresenceTransition;
use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Accordion, AccordionStyle};
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::expansion_panels::ExpansionPanelsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

use floom_widget_md_icon::{MdIcon, MD_ICON_VIEW_BOX};

#[derive(Debug, Clone, PartialEq)]
pub struct MdPanelIndicator {
    pub collapsed: VectorIcon,
    pub expanded: VectorIcon,
}

/// Resolved, theme-agnostic style for a Material expansion panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdExpansionPanelsStyle {
    pub header_height: f32,
    pub header_padding: f32,
    pub content_padding: f32,
    pub content_indent: f32,
    pub gap: f32,
    pub radius: f32,
    pub border_width: f32,
    pub border_color: PaintColor,
    pub title_color: PaintColor,
    pub title_size: f32,
}

pub struct MdExpansionPanels {
    props: ExpansionPanelsProps,
    theme: RwSignal<MultiTheme>,
    /// One entry per declared panel, in declaration order.
    panels: Vec<PanelItem>,
    on_change: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
    transition: Option<PresenceTransition>,
}

struct PanelItem {
    title: String,
    content: Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync>,
}

impl MdExpansionPanels {
    pub fn new() -> Self {
        Self {
            props: ExpansionPanelsProps::default(),
            theme: theme_signal(),
            panels: Vec::new(),
            on_change: None,
            transition: None,
        }
    }

    /// Adds a panel. `content` builds the body, which is mounted only while the
    /// panel is open.
    pub fn panel(
        mut self,
        title: impl Into<String>,
        content: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.panels.push(PanelItem {
            title: title.into(),
            content: Arc::new(content),
        });
        self
    }

    /// Allow more than one panel open at a time. Off by default, which is the
    /// accordion rule.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize, bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> ExpansionPanelsProps {
        ExpansionPanelsProps {
            multiple: self.props.multiple,
            on_change: None,
        }
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn titles(&self) -> Vec<&str> {
        self.panels.iter().map(|p| p.title.as_str()).collect()
    }

    pub fn resolved_style(&self) -> MdExpansionPanelsStyle {
        let theme = self.theme.get();
        let ComponentTokens::ExpansionPanels(tokens) =
            theme.component(ComponentId::ExpansionPanels)
        else {
            unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
        };
        MdExpansionPanelsStyle {
            header_height: tokens.header_height,
            header_padding: tokens.content_padding,
            content_padding: tokens.content_padding,
            content_indent: tokens.indent,
            gap: tokens.gap,
            radius: tokens.radius,
            border_width: tokens.border,
            border_color: to_paint(with_alpha(theme.colors().outline, 30)),
            title_color: to_paint(theme.colors().on_surface),
            title_size: LABEL_FONT_SIZE,
        }
    }

    /// The two indicator glyphs, at the token's size and the theme's muted ink.
    pub fn indicator(&self) -> MdPanelIndicator {
        let color = to_paint(self.theme.get().colors().on_surface_variant);
        MdPanelIndicator {
            collapsed: MdIcon::new("chevron_right")
                .with_theme(self.theme.get())
                .size(self.icon_size())
                .resolve()
                .color(color),
            expanded: MdIcon::new("expand_more")
                .with_theme(self.theme.get())
                .size(self.icon_size())
                .resolve()
                .color(color),
        }
    }

    /// The expand/collapse indicator's edge length, from the tokens.
    pub fn icon_size(&self) -> f32 {
        let theme = self.theme.get();
        let ComponentTokens::ExpansionPanels(tokens) =
            theme.component(ComponentId::ExpansionPanels)
        else {
            unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
        };
        tokens.icon_size
    }

    pub fn content_layout(&self) -> LayoutRequest {
        LayoutRequest::new().margin_horiz(f64::from(
            self.resolved_style().content_padding + self.resolved_style().content_indent,
        ))
    }

    /// The engine view box every Material glyph is authored against.
    pub const fn icon_view_box() -> floom_types::Rect {
        MD_ICON_VIEW_BOX
    }

    pub fn content_transition(mut self, transition: PresenceTransition) -> Self {
        self.transition = Some(transition);
        self
    }

    pub fn resolve(&self) -> Accordion {
        let style = self.resolved_style();
        let indicator = self.indicator();
        let mut accordion = Accordion::new()
            .multiple(self.props.multiple)
            .style(AccordionStyle {
                header_height: style.header_height,
                header_padding: style.header_padding,
                content_padding: style.content_padding,
                content_indent: style.content_indent,
                gap: style.gap,
                title_color: style.title_color,
                title_size: style.title_size,
                border_width: style.border_width,
                border_color: style.border_color,
                radius: style.radius,
            });

        for panel in &self.panels {
            let (collapsed, expanded) = (indicator.collapsed.clone(), indicator.expanded.clone());
            accordion = accordion.panel(
                panel.title.clone(),
                move |open| {
                    Box::new(if open {
                        expanded.clone()
                    } else {
                        collapsed.clone()
                    })
                },
                {
                    let content = panel.content.clone();
                    move || content()
                },
            );
        }

        if let Some(callback) = &self.on_change {
            accordion = accordion.on_change({
                let callback = callback.clone();
                move |index, open| callback(index, open)
            });
        }
        if let Some(transition) = self.transition {
            accordion = accordion.content_transition(transition);
        }
        accordion
    }
}

impl Default for MdExpansionPanels {
    fn default() -> Self {
        Self::new()
    }
}

const LABEL_FONT_SIZE: f32 = 14.0;
