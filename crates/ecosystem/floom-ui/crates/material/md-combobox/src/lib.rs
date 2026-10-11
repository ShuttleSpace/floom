//! Material 3 combobox, ported to the Floom 2.0 widget API.
//!
//! ## Why this crate is a token translation
//!
//! 91% cross-kit overlap — the three v1 crates differ by 26 lines, all of it type
//! names, the icon source, the theme source, and one `.margin_top(4.0)`. So the shared
//! part went to the engine as [`floom_widgets::Combobox`] and what is left here is
//! Material's numbers, its icon names, and its props.
//!
//! ## What the port changed
//!
//! * the three `RwSignal`s v1 kept — `selected`, `open`, `query` — became the
//!   engine's shared `pick` and `query`, for the reason `TextInput` gives: an
//!   `RwSignal` is `Rc`-based and cannot cross a `Send + Sync` slot boundary;
//! * v1's `Overlay::new(panel)` plus a `LayoutChangedListener` reading the trigger's
//!   window origin and height became [`floom_widgets::OverlayHost`] with
//!   [`floom_widgets::PopupPlacement::Below`] — which is the same arithmetic said the
//!   short way, and the `LayoutChangedListener` is gone because the box the widget
//!   already has is the bottom edge of the thing it owns;
//! * v1 mounted **every** option and hid the non-matching ones, so a keystroke rebuilt
//!   the whole list; the engine mounts only the matching rows, keyed by option index;
//! * v1's row hover fill has **no 2.0 equivalent**: `Pressable`'s interaction state is
//!   not readable from outside, so a row paints its selected fill or nothing. Recorded
//!   rather than faked with a fill that is always on;
//! * v1's `List` of `AnyView`s became a `Stack` of rows, and v1's pointer cursor is
//!   gone for the fifth time in this port — 2.0's `PaintContext` has no cursor
//!   vocabulary.
//!
//! ## Material's `.margin_top(4.0)` and the other kits' absence of it
//!
//! Preserved, not reconciled: `panel_gap` is a parameter, and Material's 4.0 is the
//! default because it is the only one of the three that wrote a number. A default
//! invented here would have been a fourth opinion.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_widgets::{Combobox, ComboboxStyle, SharedPick};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::combobox::ComboboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

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

/// Material's combobox.
pub struct MdCombobox {
    props: ComboboxProps,
    theme: RwSignal<MultiTheme>,
    /// The shared selection state the widget reads and writes.
    ///
    /// Held here as well as inside the widget so a caller that already had a
    /// `ComboboxProps` can watch the choice, and so `selected(..)` can replace it.
    pick: SharedPick,
    on_change: Option<std::sync::Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
}

impl MdCombobox {
    pub fn new() -> Self {
        let props = ComboboxProps::default();
        MdCombobox {
            pick: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props,
            on_change: None,
        }
    }

    /// Replace the option list.
    ///
    /// v1's setter also copied `options[selected]` into the query, which is what
    /// `floom_widgets::sync_query_to_selection` is for; here the engine's
    /// `Combobox::set_options` does the whole job, including dropping a selection that
    /// no longer resolves and clearing a field that would otherwise keep claiming it.
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
            // Held so `props()` keeps reporting what v1 reported; the engine gets the
            // `Send + Sync` handle, because a slot closure has to be one.
            let shared = std::sync::Arc::new(callback);
            self.on_change = Some(shared.clone());
            move |index: Option<usize>| shared(index)
        }));
        self
    }

    pub fn props(&self) -> &ComboboxProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared selection state, for a caller that wants to read it.
    pub fn pick(&self) -> SharedPick {
        self.pick.clone()
    }

    pub fn resolved_style(&self) -> ComboboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Combobox(tokens) = theme.component(ComponentId::Combobox) else {
            unreachable!("ComponentId::Combobox must resolve to ComboboxTokens")
        };
        let surface = theme.colors().surface;
        let on_surface = theme.colors().on_surface;
        let outline = theme.colors().outline;
        ComboboxStyle {
            // --- from `ComboboxTokens` ---
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            // v1 declared this and never read it; the host now needs it, because a
            // panel request is a bound the content is laid out against and clipped to.
            panel_max_height: tokens.panel_max_height,

            // --- v1's call-site constants, Material's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            // The one number Material wrote and the other two kits did not.
            panel_gap: 4.0,
            chevron_size: 14.0,
            disabled_text_alpha: 38,
            disabled_chevron_alpha: 97,
            disabled_border_alpha: 97,
            selected_row_alpha: 24,
            hover_row_alpha: 8,
            panel_border_alpha: 120,
            panel_shadow_alpha: 20,
            border_width: 1.0,

            // --- resolved theme colours ---
            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            on_surface_variant: to_paint(theme.colors().on_surface_variant),
            outline: to_paint(outline),
            primary: to_paint(theme.colors().primary),
            text_size: theme.typography().body.medium.font_size,
        }
    }

    /// The combobox, ready to mount.
    pub fn view(self) -> Combobox {
        let mut combo = Combobox::with_shared(
            self.props.options.clone(),
            floom_widgets::SharedBuffer::default(),
            self.pick.clone(),
        )
        .style(self.resolved_style())
        .icons(resolve_icon)
        .placeholder(self.props.placeholder.clone())
        .disabled(self.props.disabled);
        if let Some(on_change) = &self.on_change {
            let on_change = on_change.clone();
            combo = combo.on_change(move |index| on_change(index));
        }
        combo
    }
}

impl Default for MdCombobox {
    fn default() -> Self {
        Self::new()
    }
}
/// Type alias for `floom-elements` compatibility.
pub type MdComboboxStyle = ComboboxStyle;
