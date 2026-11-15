//! Fluent combobox, ported to the Floom 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::SharedPick;
use floom_ui_composites::{Combobox, ComboboxStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::combobox::ComboboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    Some(crate::widgets::icon::fluent_icon(name))
}

pub struct FluentCombobox {
    props: ComboboxProps,
    theme: RwSignal<MultiTheme>,
    pick: SharedPick,
    on_change: Option<std::sync::Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
}

impl FluentCombobox {
    pub fn new() -> Self {
        let props = ComboboxProps::default();
        FluentCombobox {
            pick: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props,
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

    pub fn on_change(mut self, callback: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new({
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
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            panel_max_height: tokens.panel_max_height,

            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
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

impl Default for FluentCombobox {
    fn default() -> Self {
        Self::new()
    }
}
