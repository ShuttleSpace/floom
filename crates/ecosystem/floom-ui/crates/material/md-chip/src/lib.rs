//! Material chip, ported to the Floom 2.0 widget API.
//!
//! v1's chip was a `Container` holding a `Label`: a pill with a label in the
//! middle, in one of three appearances -- disabled, selected, or neither --
//! and a click that a disabled chip ignores.
//!
//! ## What the engine grew
//!
//! **A `Chip` composite.** v1's chip was one box with the label's ink on the
//! box (`color(fg)`) and the label centred in it (`items_center`). 2.0's
//! `Surface` paints a fill and only the child knows it is text, and a box's
//! content sits at the padding's edge rather than in the middle, so the pill
//! and its centred label are one widget in the engine now -- see
//! `floom_ui_composites::Chip`.
//!
//! ## What this kit answers
//!
//! | appearance | fill | ink |
//! | --- | --- | --- |
//! | disabled | `on_surface`@12 | `on_surface`@38 |
//! | selected | `secondary_container` | `on_secondary_container` |
//! | neither | `surface_variant` | `on_surface` |
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`gap(6.0)`.** The token is "the gap between the leading icon (if any)
//!   and the label", and v1's chip has no icon, so the gap never had anything
//!   to separate. The style still reports it.
//! * **`cursor(CursorStyle::Pointer)`.** 2.0's paint context has no cursor
//!   vocabulary, so the pointer is not carried over; the chip is still
//!   clickable, and by keyboard too, which the pointer was not.
//! * **`props()`, `theme()` and `with_theme()`.** v1's Material chip had none
//!   of the three -- it read the app-wide theme signal and had no way to be
//!   given another one. All three kits have them here, because a kit that
//!   cannot be given a theme cannot be tested against one.
//!
//! ## The alpha numbers
//!
//! `on_surface`@12 and `on_surface`@38 are v1's numbers, and they are *bytes*
//! of alpha rather than percentages: 12 of 255 is about 5%, which is what a
//! disabled fill looks like.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Chip;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip::ChipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// The chip's appearance, in the terms v1 used.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialChipStyle {
    pub design_system: &'static str,
    /// The pill's fill.
    pub container_color: Color,
    /// The label's ink.
    pub label_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub font_size: f32,
    /// The gap between a leading icon and the label. v1's chip had no icon.
    pub gap: f32,
}

/// Material's chip.
pub struct MaterialChip {
    props: ChipProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialChip {
    pub fn new() -> Self {
        Self {
            props: ChipProps::default(),
            theme: theme_signal(),
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
    pub fn resolved_style(&self) -> MaterialChipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Chip(tokens) = theme.component(ComponentId::Chip) else {
            unreachable!("ComponentId::Chip must resolve to ChipTokens")
        };
        let (container_color, label_color) = if self.props.disabled {
            (
                with_alpha(theme.colors().on_surface, 12),
                with_alpha(theme.colors().on_surface, 38),
            )
        } else if self.props.selected {
            (
                theme.colors().secondary_container,
                theme.colors().on_secondary_container,
            )
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface)
        };
        MaterialChipStyle {
            design_system: "material",
            container_color,
            label_color,
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    /// The chip: the pill, its centred label, and the click.
    pub fn view(self) -> Chip {
        let style = self.resolved_style();
        let mut chip = Chip::new()
            .label(self.props.label.clone())
            .background(to_paint(style.container_color))
            .foreground(to_paint(style.label_color))
            .height(style.height)
            .padding_horizontal(style.padding_horizontal)
            .radius(style.border_radius)
            .font_size(style.font_size)
            // v1's Material chip had no outline.
            .border(0.0, to_paint(style.container_color))
            .disabled(self.props.disabled);
        if let Some(on_click) = self.props.on_click {
            chip = chip.on_click(move || on_click());
        }
        chip
    }
}

impl Default for MaterialChip {
    fn default() -> Self {
        Self::new()
    }
}

