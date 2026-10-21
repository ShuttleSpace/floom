//! Cupertino chip, ported to the Floom 2.0 widget API.
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
//! | disabled | `on_surface`@4 | `on_surface`@97 |
//! | selected | `primary` | `on_primary` |
//! | neither | `surface_variant` | `on_surface` |
//!
//! This kit is the one with an **outline** -- 1px, `primary` when the chip is
//! chosen and `outline` otherwise -- and the one with a **hover fill**:
//! `primary`@200 when chosen, `on_surface`@16 otherwise. The other two kits
//! have neither, and that disagreement is the reason these stay three crates.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`gap(6.0)`.** The token is "the gap between the leading icon (if any)
//!   and the label", and v1's chip has no icon, so the gap never had anything
//!   to separate. The style still reports it.
//! * **`cursor(CursorStyle::Default)` while disabled.** 2.0's paint context has
//!   no cursor vocabulary, so neither cursor is carried over.
//! * **The hover fill of a disabled chip.** v1 named one -- `on_surface`@4 --
//!   but a disabled chip does not hover, so it never showed; 2.0's chip does
//!   not answer the pointer while disabled, which is the same answer.
//!
//! ## The alpha numbers
//!
//! `on_surface`@4, `on_surface`@97 and `primary`@200 are v1's numbers, and
//! they are *bytes* of alpha rather than percentages.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Chip;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip::ChipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
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
        // v1's hover fill, which is this kit's alone.
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

