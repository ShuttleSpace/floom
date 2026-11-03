//! Cupertino chip, ported to the Floom 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Chip;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip::ChipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DESIGN_SYSTEM: &str = "cupertino";

/// The chip's appearance, in the terms v1 used.
#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoChipStyle {
    pub design_system: &'static str,
    /// The pill's fill.
    pub background: Color,
    /// The label's ink.
    pub foreground: Color,
    /// The outline's colour: `primary` when the chip is chosen, `outline`
    /// otherwise.
    pub border_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    /// The gap between a leading icon and the label. v1's chip had no icon.
    pub gap: f32,
}

/// Cupertino's chip.
pub struct CupertinoChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoChip {
    pub fn new() -> Self {
        Self {
            props: ChipProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The chip's text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// The chip in its chosen appearance.
    pub fn selected(mut self, selected: bool) -> Self {
        self.props.selected = selected;
        self
    }

    /// The chip greyed out, and no longer answering the pointer.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called when the chip is clicked. A disabled chip does not call it.
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    /// The shared props, for a caller that reads the design-system-neutral shape.
    ///
    /// `on_click` is deliberately absent from what it reports: the props are
    /// the v1 carrier of a `Box<dyn Fn()>` and the kit holds that here.
    pub fn props(&self) -> &ChipProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The chip's appearance, in v1's terms.
    pub fn resolved_style(&self) -> CupertinoChipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Chip(tokens) = theme.component(ComponentId::Chip) else {
            unreachable!("ComponentId::Chip must resolve to ChipTokens")
        };
        let (background, foreground) = if self.props.disabled {
            (
                with_alpha(theme.colors().on_surface, 4),
                with_alpha(theme.colors().on_surface, 97),
            )
        } else if self.props.selected {
            (theme.colors().primary, theme.colors().on_primary)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface)
        };
        CupertinoChipStyle {
            design_system: DESIGN_SYSTEM,
            background,
            foreground,
            border_color: if self.props.selected {
                theme.colors().primary
            } else {
                theme.colors().outline
            },
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    /// The chip: the pill, its outline, its centred label, and the click.
    pub fn view(self) -> Chip {
        let style = self.resolved_style();
        let selected = self.props.selected;
        let disabled = self.props.disabled;
        let theme = self.theme.get();
        let hover = if disabled {
            with_alpha(theme.colors().on_surface, 4)
        } else if selected {
            with_alpha(theme.colors().primary, 200)
        } else {
            with_alpha(theme.colors().on_surface, 16)
        };
        let mut chip = Chip::new()
            .label(self.props.label.clone())
            .background(to_paint(style.background))
            .hover(Some(to_paint(hover)))
            .foreground(to_paint(style.foreground))
            .height(style.height)
            .padding_horizontal(style.padding_horizontal)
            .radius(style.border_radius)
            .font_size(style.font_size)
            // v1 wrote `.border(1.0)` unconditionally.
            .border(1.0, to_paint(style.border_color))
            .disabled(disabled);
        if let Some(on_click) = self.props.on_click {
            chip = chip.on_click(move || on_click());
        }
        chip
    }
}

impl Default for CupertinoChip {
    fn default() -> Self {
        Self::new()
    }
}
