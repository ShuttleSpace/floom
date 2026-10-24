//! Material 3 select (dropdown), ported to the Floom 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1's `RwSignal` state (`selected`, `open`) became the engine's `Select` widget
//!   which manages its own open/closed state and selection.
//! * v1's `Overlay::new(panel)` plus a `LayoutChangedListener` for positioning
//!   became the engine's built-in overlay handling via `Select::open_popup`.
//! * v1's `List` of `AnyView`s became the engine's `SelectPopup` which is a proper
//!   widget with hit-testing and keyboard navigation.
//! * v1's `MdIcon::view()` became `MdIcon::resolve()` returning a `VectorIcon`.
//!
//! The engine's `Select` handles the trigger, the popup, and the selection state.
//! This crate provides Material's token translation and styling.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::{Select, SelectStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// Icon names are resolved by the kit, because three kits' icon sets differ and the
/// engine has no business choosing between them.
fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    floom_widget_md_icon::MdIcon::new(name).resolve().into()
}

/// Material's select (dropdown).
pub struct MdSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
    /// The chevron, as a *name* rather than a `VectorIcon`.
    ///
    /// It is resolved at `view()` so the kit can pick its own glyph without
    /// `MdSelect` holding a widget it does not own yet. It defaults to
    /// Material's `arrow_drop_down`, which is the answer this kit gives to a
    /// question the other two answer differently.
    icon: String,
}

impl MdSelect {
    pub fn new() -> Self {
        let props = SelectProps::default();
        MdSelect {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props,
            icon: "arrow_drop_down".to_string(),
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

    /// Replace the chevron. The *shape* is the kit's; the size and the colour are
    /// `SelectStyle`'s.
    pub fn icon(mut self, name: impl Into<String>) -> Self {
        self.icon = name.into();
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
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
        let on_surface_variant = theme.colors().on_surface_variant;
        let outline = theme.colors().outline;
        let primary = theme.colors().primary;

        SelectStyle {
            // --- from `SelectTokens` ---
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,
            icon_size: tokens.icon_size,

            // --- v1's call-site constants, Material's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            panel_gap: 0.0,
            panel_padding: 4.0,
            border_width: 1.0,
            text_size: theme.typography().body.medium.font_size,
            panel_shadow_blur: 8.0,
            panel_shadow_spread: 1.0,

            // --- base colours ---
            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            placeholder: to_paint(on_surface_variant),
            outline: to_paint(outline),
            chevron: to_paint(on_surface_variant),
            // Material is the only kit whose panel has a border: `outline`@120.
            panel_border: to_paint(with_alpha(outline, 120)),
            panel_shadow: to_paint(Self::shadow()),

            // --- the interactive bands, which are alphas over a source colour ---
            //
            // `SelectStyle` takes resolved colours rather than `alpha: u8`, and
            // this is where the three kits' disagreement actually lives: a
            // hovered row is `on_surface`@8 in Material and Fluent but `primary`
            // @12 in Cupertino. That is a different *source* colour, so no
            // alpha field could have carried it -- the kit composes it.
            hover_row: to_paint(with_alpha(on_surface, 8)),
            selected_row: to_paint(with_alpha(primary, 24)),

            // --- disabled, also composed ---
            disabled_text: to_paint(with_alpha(on_surface_variant, 97)),
            disabled_chevron: to_paint(with_alpha(on_surface_variant, 97)),
            disabled_border: to_paint(with_alpha(outline, 97)),
            // A disabled Material trigger keeps its surface: the disabled state is
            // a *border* change here, where Cupertino's is a fill change.
            disabled_background: to_paint(surface),
        }
    }

    /// Material's popup shadow: elevation level 2, `#000000` at 20%.
    fn shadow() -> Color {
        const SHADOW: Color = Color {
            r: 0x00,
            g: 0x00,
            b: 0x00,
            a: 20,
        };
        SHADOW
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

        // The chevron is a *shape*, and three kits' icon sets disagree on what
        // "expand" looks like, so the engine has no default. A kit that names
        // none gets no chevron at all rather than a glyph chosen for it.
        if let Some(icon) = resolve_icon(&self.icon) {
            select = select.chevron(icon);
        }

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

impl Default for MdSelect {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for the engine's `SelectStyle`, re-exported for `floom-elements` compatibility.
pub type MdSelectStyle = SelectStyle;
