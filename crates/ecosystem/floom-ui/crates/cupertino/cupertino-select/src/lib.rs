//! Cupertino's select (dropdown), ported to the Floom 2.0 widget API.
//!
//! ## What the port kept, and from where
//!
//! v1's own `view()` was a hand-built dropdown: an `RwSignal<bool>` for open,
//! an `Overlay::new(panel)` positioned from a `LayoutChangedListener`, and one
//! `Label` per option rebuilt on every hover. All of that is the engine's
//! [`Select`] now -- the overlay, the keyboard, the highlight -- so what is left
//! here is the token translation plus the one value that is not a token.
//!
//! Two of v1's numbers survived into the port and are worth naming:
//!
//! * the hovered row is `primary`@**12**, where Material and Fluent use
//!   `on_surface`@8. That is a different *source* colour, which is why
//!   [`SelectStyle`] carries resolved colours rather than an alpha over one
//!   theme colour.
//! * a disabled trigger is a *fill* change here (`on_surface`@4) and a *border*
//!   change in Material. Same state, opposite mechanism.
//!
//! ## The chevron
//!
//! The icon name is this kit's, because three kits' icon sets disagree on what
//! "expand" looks like. `cupertino-icon` is still a v1 crate, so its path data
//! now lives in `floom-widget-cupertino-icon-paths` -- data has no v1
//! dependency, and the SVG wrapper it also exports is what the v1 half uses.

use floom_signal::{RwSignal, SignalGet};

use floom_core::{Point, Rect, Size};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_types::Color as PaintColor;
use floom_ui_composites::{Select, SelectStyle};
use floom_widget_cupertino_icon_paths::cupertino_icon_path;
use floom_widgets::VectorIcon;

const DESIGN_SYSTEM: &str = "cupertino";


/// Cupertino's select (dropdown).
pub struct CupertinoSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSelect {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props: SelectProps::default(),
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
        let outline = theme.colors().outline;
        let primary = theme.colors().primary;

        SelectStyle {
            // --- geometry, from `SelectTokens` ---
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,
            icon_size: tokens.icon_size,

            // --- v1's call-site constants, Cupertino's values ---
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
            outline: to_paint(outline),
            // Placeholder and chevron are both `on_surface`@140 -- undimmed in
            // Material's case (`on_surface_variant` at 255), 160 for Fluent's
            // chevron, so the three kits give three answers to one question.
            placeholder: to_paint(with_alpha(on_surface, 140)),
            chevron: to_paint(with_alpha(on_surface, 140)),
            // v1's Cupertino panel drew no border at all.
            panel_border: to_paint(Color::rgba(0, 0, 0, 0)),
            panel_shadow: to_paint(Color::rgba(0, 0, 0, 24)),

            // --- the interactive bands ---
            //
            // This is the kit's one real disagreement with Material: a hovered
            // row is `primary`@12 here and `on_surface`@8 there.
            hover_row: to_paint(with_alpha(primary, 12)),
            selected_row: to_paint(with_alpha(primary, 24)),

            // --- disabled, which is a fill change here ---
            disabled_text: to_paint(with_alpha(on_surface, 97)),
            disabled_chevron: to_paint(with_alpha(on_surface, 97)),
            disabled_border: to_paint(with_alpha(on_surface, 38)),
            disabled_background: to_paint(with_alpha(on_surface, 4)),
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

        // The chevron's *shape* is this kit's; its size and colour are the
        // style's. `VectorIcon::from_path_data` takes the path the v1 crate
        // still hands to SVG, so nothing is parsed twice.
        // The path data is authored in a 24x24 view box, which is the one thing
        // `from_path_data` needs besides the string.
        select = select.chevron(VectorIcon::from_path_data(
            cupertino_icon_path("expand_more"),
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

impl Default for CupertinoSelect {
    fn default() -> Self {
        Self::new()
    }
}
