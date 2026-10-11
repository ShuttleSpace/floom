//! Cupertino combobox, ported to the Floom 2.0 widget API.
//!
//! ## Why this crate is a token translation
//!
//! 91% cross-kit overlap — the three v1 crates differ by 26 lines, all of it type
//! names, the icon source, the theme source, and one `.margin_top(4.0)`. So the shared
//! part went to the engine as [`floom_widgets::Combobox`]; what is left here is
//! Cupertino's numbers and its icon names.
//!
//! ## The one number that differs from Material, and stays different
//!
//! **This kit wrote no panel gap.** v1's Cupertino combobox positioned its panel with
//! `.absolute().inset_top(origin.y + trigger_height)` and *no* `margin_top`, so its
//! dropdown sat flush against the field while Material's sat 4px below. That is a
//! disagreement between two design systems, and it is a parameter here rather than a
//! reconciliation: `panel_gap` is `0.0` below, and `the_engine_default_is_materials_four`
//! is the test that keeps the difference visible rather than letting a later "just use
//! the default" edit erase it.
//!
//! ## What the port changed
//!
//! * three `RwSignal`s became the engine's shared state, for the reason `TextInput`
//!   gives: an `RwSignal` is `Rc`-based and cannot cross a `Send + Sync` slot boundary;
//! * `Overlay::new(panel)` plus a `LayoutChangedListener` became `OverlayHost` with
//!   `PopupPlacement::Below` — the same arithmetic said the short way, and the listener
//!   is gone because the widget already has the box whose bottom edge it needs;
//! * v1's `List` of `AnyView`s became a `Stack`, and every option was mounted and then
//!   hidden when it did not match; only matching rows mount now, keyed by option index;
//! * v1's `svg(move || cupertino_icon_svg("expand_more"))` became a `VectorIcon` from
//!   this kit's own table, because v1 handed out SVG *markup* and the engine draws
//!   path data — wrapping a path in `<svg><path d=…/></svg>` only to strip it straight
//!   back off is the detour `cupertino_icon` exists to avoid;
//! * v1's pointer cursor is gone for the fifth time in this port: 2.0's `PaintContext`
//!   has no cursor vocabulary.
//!
//! ## Two losses, recorded rather than faked
//!
//! * **v1's row hover fill has no 2.0 equivalent** — `Pressable`'s interaction state is
//!   not readable from outside, so a row paints its selected fill or nothing, and a
//!   fill that is always on would be a permanent hover;
//! * **the list opens on the first keystroke, not on focus**, because 2.0 has no way for
//!   a parent to observe a child's focus.
//!
//! ## `resolved_style` returns the engine's style
//!
//! v1 returned a `CupertinoComboboxStyle` of its own, carrying a `design_system`
//! string. That string was never read by anything — it is the same class of thing as
//! the kit names themselves — and the port drops it along with the struct, returning
//! [`floom_widgets::ComboboxStyle`] instead. That is the same trade `md-treeview` made,
//! and the same one `md-combobox` made: a kit style struct that only differs by a name
//! is a second thing to keep in step.
//!
//! One consequence worth stating: **nothing here verifies that this is Cupertino's
//! theme.** `MultiTheme::new_for` ignores its argument and reads go through a
//! process-global, so the numbers below are Material's because nothing has called
//! `set_design_system`. That is the third correction of this shape in this port, and the
//! test says so by name.

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

/// Icon names are resolved by the kit, from **this kit's own table**.
///
/// That is not tidiness: v1 used `cupertino_icon_svg`, and Cupertino's `expand_more` is
/// a different glyph from Material's. Using the Material table here would render a
/// Material chevron in a Cupertino control, and nothing would fail — which is exactly
/// what happened to `cupertino-treeview`, whose v1 source genuinely used `MdIcon`, so
/// that one *is* Material's chevron and is left alone.
fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    Some(floom_widget_cupertino_icon::cupertino_icon(name))
}

/// Cupertino's combobox.
pub struct CupertinoCombobox {
    props: ComboboxProps,
    theme: RwSignal<MultiTheme>,
    /// The shared selection state the widget reads and writes.
    pick: SharedPick,
    on_change: Option<std::sync::Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
}

impl CupertinoCombobox {
    pub fn new() -> Self {
        let props = ComboboxProps::default();
        CupertinoCombobox {
            pick: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
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

    pub fn on_change(
        mut self,
        callback: impl Fn(Option<usize>) + Send + Sync + 'static,
    ) -> Self {
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

            // --- v1's call-site constants, Cupertino's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            // **Zero, where Material wrote 4.0.** v1 positioned this panel with an
            // inset and no margin at all, so it sat flush against the field. Preserved
            // rather than reconciled: see the module note.
            panel_gap: 0.0,
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

impl Default for CupertinoCombobox {
    fn default() -> Self {
        Self::new()
    }
}