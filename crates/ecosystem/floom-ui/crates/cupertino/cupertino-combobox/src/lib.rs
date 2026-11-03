//! Cupertino combobox, ported to the Floom 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::SharedPick;
use floom_ui_composites::{Combobox, ComboboxStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::combobox::ComboboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// Icon names are resolved by the kit, from **this kit's own table**.
///
/// That is not tidiness: v1 used `cupertino_icon_svg`, and Cupertino's `expand_more` is
/// a different glyph from Material's. Using the Material table here would render a
/// Material chevron in a Cupertino control, and nothing would fail.
///
/// Four other kits did exactly that: `fluent-treeview`, `cupertino-treeview`,
/// `fluent-icon_button` and `cupertino-icon_button` all reached for
/// `MdIcon` during the port. The first two claimed to be exceptions -- their
/// comments said v1 "genuinely used" it -- which turned out to be a reading of
/// their own import rather than of v1: both had used their own table and
/// lost it. All four are repointed now.

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
