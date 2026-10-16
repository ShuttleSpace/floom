//! Fluent's select (dropdown), ported to the Floom 2.0 widget API.
//!
//! ## What the port kept
//!
//! v1's `view()` was a hand-built dropdown: an `RwSignal<bool>` for open, an
//! `Overlay::new(panel)` placed from a `LayoutChangedListener`, one `Label` per
//! option rebuilt on every hover. All of that is the engine's [`Select`] now --
//! the overlay, the keyboard, the highlight -- so what is left here is the token
//! translation plus the value that is not a token.
//!
//! Fluent is the third answer to the three questions the kits answer
//! differently: its hover band agrees with Material (`on_surface`@8) and not
//! Cupertino's (`primary`@12); its missing panel border agrees with Cupertino
//! and not Material's (`outline`@120); and its chevron is a third value of its
//! own (`on_surface`@160, where Cupertino wrote 140 and Material used the full
//! `on_surface_variant`).
//!
//! ## The chevron
//!
//! `fluent-icon` is still a v1 crate, so its path data now lives in
//! `floom-widget-fluent-icon-paths`: data has no v1 dependency, and the SVG
//! wrapper that crate also exports is what the v1 half uses.

use std::sync::Arc;

use floom_core::{Point, Rect, Size};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Select, SelectStyle};
use floom_widget_fluent_icon_paths::fluent_icon_path;
use floom_widgets::VectorIcon;

const DESIGN_SYSTEM: &str = "fluent";

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Fluent's select (dropdown).
pub struct FluentSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl FluentSelect {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props: SelectProps::default(),
            on_change: None,
        }
    }

    /// Replace the option list.
    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

    /// Choose an option up front.
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
        self.on_change = Some(Arc::new(callback));
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
            // --- geometry, from `SelectTokens` ---
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,
            icon_size: tokens.icon_size,

            // --- v1's call-site constants, Fluent's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            panel_gap: 0.0,
            panel_padding: 4.0,
            border_width: 1.0,
            text_size: theme.typography().body.medium.font_size,
            panel_shadow_blur: 12.0,
            panel_shadow_spread: 1.0,

            // --- base colours ---
            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            outline: to_paint(on_surface),
            // v1's Fluent trigger painted its label in `on_surface` and never
            // distinguished a placeholder, so the two are the same colour --
            // unlike the other two kits, where the placeholder is dimmer.
            placeholder: to_paint(on_surface),
            // This kit's third answer: the chevron is `on_surface`@160, where
            // Cupertino wrote 140 and Material used the undimmed
            // `on_surface_variant`.
            chevron: to_paint(with_alpha(on_surface, 160)),
            // The panel has no border, as Cupertino's; Material's is `outline`@120.
            panel_border: to_paint(Color::rgba(0, 0, 0, 0)),
            panel_shadow: to_paint(Color::rgba(0, 0, 0, 20)),

            // --- the interactive bands ---
            hover_row: to_paint(with_alpha(on_surface, 8)),
            selected_row: to_paint(with_alpha(on_surface, 8)),

            // --- disabled ---
            // v1's Fluent trigger kept `surface` when disabled, so there is no
            // disabled background to translate: it is the same colour.
            disabled_text: to_paint(with_alpha(on_surface, 97)),
            disabled_chevron: to_paint(with_alpha(on_surface, 97)),
            disabled_border: to_paint(with_alpha(on_surface, 38)),
            disabled_background: to_paint(surface),
        }
    }

    /// The select, ready to mount.
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

        if let Some(on_change) = self.on_change {
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
