use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::IntoView;
use floem::reactive::RwSignal;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::menu::{MenuItem, MenuProps};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoMenuStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub text_color: Color,
    pub min_width: f32,
    pub item_height: f32,
    pub padding: f32,
}

pub struct CupertinoMenu {
    props: MenuProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoMenu {
    pub fn new() -> Self {
        let props = MenuProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
    }
    pub fn item(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
        self.props.items.push(MenuItem {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self.open.set(open);
        self
    }
    pub fn props(&self) -> &MenuProps {
        &self.props
    }
    pub fn select(&self, index: usize) {
        if let Some(item) = self.props.items.get(index) {
            if let Some(callback) = &item.on_select {
                callback();
            }
        }
    }
    /// Expose the internal open signal so an external trigger (e.g. a button)
    /// can show or hide the menu.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoMenuStyle {
        let theme = self.theme.get();
        let ComponentTokens::Menu(tokens) = theme.component(ComponentId::Menu) else {
            unreachable!("ComponentId::Menu must resolve to MenuTokens")
        };
        CupertinoMenuStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            padding: tokens.padding,
        }
    }

    /// Build a renderable Cupertino menu surface listing its items.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let items = self.props.items;

        let tokens = match theme.get().component(ComponentId::Menu) {
            ComponentTokens::Menu(t) => t,
            _ => unreachable!(),
        };
        let item_height = tokens.item_height;
        let radius = theme.get().radius().md;

        let rows: Vec<AnyView> = items
            .into_iter()
            .map(|item| {
                let on_select = item.on_select;
                let label = item.label.clone();
                let open_sig = open;
                let theme_hover = theme;
                Label::new(label)
                    .style(move |s| {
                        let t = theme.get();
                        s.width_pct(100.0)
                            .padding_horiz(12.0)
                            .padding_vert(8.0)
                            .height(item_height)
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                let t = theme_hover.get();
                                s.background(with_alpha(t.colors().primary, 16).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        if let Some(cb) = &on_select {
                            cb();
                        }
                        open_sig.set(false);
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        let panel = Container::new(list(rows)).style(move |s| {
            let t = theme.get();
            s.min_width(tokens.min_width)
                .max_width(tokens.max_width)
                .padding(tokens.padding)
                .background(t.colors().surface.to_peniko())
                .border_radius(radius)
                .box_shadow_blur(12.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 24).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });

        Overlay::new(panel)
    }
}

impl Default for CupertinoMenu {
    fn default() -> Self {
        Self::new()
    }
}
