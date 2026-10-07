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

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_widgets::{Select, SelectStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Icon names are resolved by the kit, because three kits' icon sets differ and the
/// engine has no business choosing between them.
fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    floom_widget_md_icon::MdIcon::new(name).resolve().into()
}

/// Material's select (dropdown).
pub struct MdSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
    on_change: Option<std::sync::Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
}

impl MdSelect {
    pub fn new() -> Self {
        let props = SelectProps::default();
        MdSelect {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props,
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

    pub fn on_change(mut self, callback: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new({
            let shared = std::sync::Arc::new(callback);
            self.on_change = Some(shared.clone());
            move |index: Option<usize>| shared(index)
        }));
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
            icon_size: tokens.icon_size,

            // --- v1's call-site constants, Material's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            panel_gap: 0.0,
            border_width: 1.0,

            // --- v1's colours, as alpha values off theme colours ---
            disabled_text_alpha: 38,
            disabled_chevron_alpha: 97,
            disabled_border_alpha: 97,
            disabled_background_alpha: 12,
            selected_row_alpha: 24,
            hover_row_alpha: 8,
            panel_border_alpha: 120,
            panel_shadow_alpha: 20,

            // --- resolved theme colours ---
            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            on_surface_variant: to_paint(on_surface_variant),
            outline: to_paint(outline),
            primary: to_paint(primary),
            text_size: theme.typography().body.medium.font_size,
        }
    }

    /// The select, ready to mount.
    pub fn view(self) -> Select {
        let style = self.resolved_style();
        let options = self.props.options.clone();
        let selected = self.props.selected;
        let disabled = self.props.disabled;
        let on_change = self.on_change.clone();

        Select::new(options)
            .selected(selected.unwrap_or(0))
            .disabled(disabled)
            .style(style)
            .on_change(move |index| {
                if let Some(on_change) = &on_change {
                    on_change(Some(index));
                }
            })
    }
}

impl Default for MdSelect {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for the engine's `SelectStyle`, re-exported for `floom-elements` compatibility.
pub type MdSelectStyle = SelectStyle;