
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::{commit, select, SharedPick};
use floom_ui_composites::{Combobox, ComboboxStyle, Commit};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::autocomplete::AutocompleteProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// Icon names are resolved by the kit, because three kits' icon sets differ and the
/// engine has no business choosing between them.
fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    floom_widget_md_icon::MdIcon::new(name).resolve().into()
}

/// Material's autocomplete.
pub struct MdAutocomplete {
    props: AutocompleteProps,
    theme: RwSignal<MultiTheme>,
    /// The shared selection state, read by the engine widget and by [`Self::commit`].
    pick: SharedPick,
    /// The field's shared buffer — the commit rule needs the typed text, and 2.0 has
    /// one buffer rather than a signal both halves could read.
    query: floom_widgets::SharedBuffer,
    on_change: Option<std::sync::Arc<dyn Fn(Option<String>) + Send + Sync + 'static>>,
}

impl MdAutocomplete {
    pub fn new() -> Self {
        MdAutocomplete {
            pick: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            query: floom_widgets::SharedBuffer::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props: AutocompleteProps::default(),
            on_change: None,
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

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

    pub fn on_change(mut self, callback: impl Fn(Option<String>) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new({
            let shared = std::sync::Arc::new(callback);
            self.on_change = Some(shared.clone());
            move |text: Option<String>| shared(text)
        }));
        self
    }

    pub fn props(&self) -> &AutocompleteProps {
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

    /// The field's shared buffer, for a caller that wants to read or write the text.
    pub fn query(&self) -> floom_widgets::SharedBuffer {
        self.query.clone()
    }

    pub fn commit(&self) -> bool {
        let outcome = commit(&self.pick, &self.query, &self.props.options);
        match outcome {
            Commit::Unchanged => false,
            Commit::Report(text) => {
                if let Some(on_change) = &self.on_change {
                    on_change(text);
                }
                true
            }
        }
    }

    /// Choose an option, as clicking its row would: the selection moves, the field reads
    /// it, and the list closes.
    pub fn select(&mut self, index: usize) -> bool {
        let changed = select(&self.pick, &self.query, &self.props.options, index);
        if changed {
            if let Some(on_change) = &self.on_change {
                on_change(self.props.options.get(index).cloned());
            }
        }
        changed
    }

    pub fn resolved_style(&self) -> ComboboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Autocomplete(tokens) = theme.component(ComponentId::Autocomplete)
        else {
            unreachable!("ComponentId::Autocomplete must resolve to AutocompleteTokens")
        };
        let surface = theme.colors().surface;
        let on_surface = theme.colors().on_surface;
        let outline = theme.colors().outline;
        ComboboxStyle {
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,

            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
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

            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            on_surface_variant: to_paint(theme.colors().on_surface_variant),
            outline: to_paint(outline),
            primary: to_paint(theme.colors().primary),
            text_size: theme.typography().body.medium.font_size,
        }
    }

    pub fn view(self) -> Combobox {
        let options = self.props.options.clone();
        let on_change = self.on_change.clone();
        let mut combo = Combobox::with_shared(
            self.props.options.clone(),
            self.query.clone(),
            self.pick.clone(),
        )
        .style(self.resolved_style())
        .icons(resolve_icon)
        .placeholder(self.props.placeholder.clone())
        .disabled(self.props.disabled);
        if let Some(on_change) = on_change {
            combo = combo.on_change(move |index| {
                on_change(index.and_then(|index| options.get(index).cloned()));
            });
        }
        combo
    }
}

impl Default for MdAutocomplete {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for `floom-elements` compatibility.
pub type MdAutocompleteStyle = ComboboxStyle;
