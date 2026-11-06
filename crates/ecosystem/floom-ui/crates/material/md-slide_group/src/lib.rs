use std::sync::Arc;

use floom_core::{Child, ChildKey, Inset, LayoutRequest, Position};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slide_group::SlideGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// An item's width. A v1 literal.
const ITEM_W: f32 = 120.0;
/// An arrow's width. A v1 literal.
const ARROW_W: f32 = 32.0;
const ARROW_FONT_RATIO: f32 = 0.6;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The previous arrow's glyph.
const PREV: &str = "\u{2039}";
/// The next arrow's glyph.
const NEXT: &str = "\u{203a}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSlideGroupStyle {
    pub design_system: &'static str,
    /// The arrows' ink.
    pub on_surface_variant: Color,
    /// The chosen item's indicator.
    pub primary: Color,
    /// An unchosen item's indicator: nothing at all.
    pub outline: Color,
    /// The arrows' own background: the surface, nearly opaque.
    pub arrow_background: Color,
    pub height: f32,
    pub gap: f32,
    pub active_indicator_height: f32,
    pub arrow_size: f32,
    /// An item's width. A v1 literal.
    pub item_width: f32,
    /// An arrow's width. A v1 literal.
    pub arrow_width: f32,
    pub arrow_font_size: f32,
}

/// Material's slide group.
pub struct MaterialSlideGroup {
    props: SlideGroupProps,
    theme: RwSignal<MultiTheme>,
    /// The items in the strip.
    items: Vec<Child>,
    /// The item that is chosen before mount.
    selected: usize,
    on_change: Option<Callback>,
}

impl MaterialSlideGroup {
    pub fn new() -> Self {
        Self {
            props: SlideGroupProps::default(),
            theme: theme_signal(),
            items: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }

    /// The items in the strip, left to right.
    pub fn items(mut self, items: Vec<Child>) -> Self {
        self.items = items;
        self
    }

    /// The item that is chosen before mount.
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    /// Whether the arrows at the ends of the strip are shown.
    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    /// Called with the newly chosen item's index when an item is tapped.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &SlideGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialSlideGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideGroup(tokens) = theme.component(ComponentId::SlideGroup) else {
            unreachable!("ComponentId::SlideGroup must resolve to SlideGroupTokens")
        };
        MaterialSlideGroupStyle {
            design_system: "material",
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            outline: theme.colors().outline,
            arrow_background: with_alpha(theme.colors().surface, 220),
            height: tokens.height,
            gap: tokens.gap,
            active_indicator_height: tokens.active_indicator_height,
            arrow_size: tokens.arrow_size,
            item_width: ITEM_W,
            arrow_width: ARROW_W,
            arrow_font_size: tokens.arrow_size * ARROW_FONT_RATIO,
        }
    }

    /// The strip, and the arrows over its ends.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let count = self.items.len();
        let initial = self.selected.min(count.saturating_sub(1));
        let flags: Arc<Vec<SharedFlag>> = Arc::new(
            (0..count)
                .map(|index| shared_flag(index == initial))
                .collect(),
        );

        let height = style.height;
        let gap = style.gap;
        let indicator = style.active_indicator_height;
        let item_width = style.item_width;
        let chosen_fill = to_paint(style.primary);
        let unchosen_fill = to_paint(with_alpha(style.outline, 0));

        let mut row = Stack::new(Axis::Horizontal)
            .gap(0.0)
            .align(Align::Start, Align::Center)
            .request(LayoutRequest::new().height(height));

        for (index, item) in self.items.into_iter().enumerate() {
            let flag = flags[index].clone();
            let flags = flags.clone();
            let on_change = self.on_change.clone();
            let mandatory = self.props.mandatory;
            let chosen = index == initial;

            let content = Surface::new()
                .request(
                    LayoutRequest::new()
                        .height(height - indicator)
                        .main_align(Align::Center)
                        .cross_align(Align::Center),
                )
                .padding_horiz(f64::from(gap))
                .child(item);

            let band = Surface::new()
                .request(LayoutRequest::new().height(indicator))
                .background(unchosen_fill)
                .child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width_pct(100.0),
                    {
                        let flag = flag.clone();
                        move || Box::new(indicator_of(flag.clone(), chosen, chosen_fill, indicator))
                    },
                ));

            let box_ = Stack::new(Axis::Vertical)
                .request(LayoutRequest::new().height(height).min_width(item_width))
                .push(move || Box::new(content.clone()))
                .push(move || Box::new(band.clone()));

            row = row.push(move || {
                let box_ = box_.clone();
                let flag = flag.clone();
                let flags = flags.clone();
                let on_change = on_change.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(box_.clone()),
                    ))
                    .selected_from(move || read_flag(&flag))
                    .on_click(move || {
                        let current = chosen_of(&flags);
                        if mandatory && current == index {
                            return;
                        }
                        choose(&flags, index);
                        if let Some(cb) = &on_change {
                            cb(index);
                        }
                    }),
                )
            });
        }

        let mut area = Stack::new(Axis::Horizontal)
            .request(LayoutRequest::new().width_pct(100.0).height(height));
        area = area.push(move || Box::new(row.clone()));

        if self.props.show_arrows && count > 1 {
            let arrow_width = style.arrow_width;
            let arrow_font_size = style.arrow_font_size;
            let arrow_ink = to_paint(style.on_surface_variant);
            let arrow_background = to_paint(style.arrow_background);

            let prev_flags = flags.clone();
            area = area.push_layout(arrow_slot(arrow_width, false), move || {
                let flags = prev_flags.clone();
                Box::new(
                    arrow_widget(
                        PREV.to_string(),
                        arrow_font_size,
                        arrow_ink,
                        arrow_background,
                        arrow_width,
                    )
                    .on_click(move || {
                        let current = chosen_of(&flags);
                        if current > 0 {
                            choose(&flags, current - 1);
                        }
                    }),
                )
            });

            let next_flags = flags.clone();
            area = area.push_layout(arrow_slot(arrow_width, true), move || {
                let flags = next_flags.clone();
                Box::new(
                    arrow_widget(
                        NEXT.to_string(),
                        arrow_font_size,
                        arrow_ink,
                        arrow_background,
                        arrow_width,
                    )
                    .on_click(move || {
                        let current = chosen_of(&flags);
                        if current + 1 < flags.len() {
                            choose(&flags, current + 1);
                        }
                    }),
                )
            });
        }

        area
    }
}

impl Default for MaterialSlideGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// The item that is chosen right now.
fn chosen_of(flags: &[SharedFlag]) -> usize {
    flags
        .iter()
        .position(|flag| read_flag(flag))
        .unwrap_or_default()
}

/// Makes `index` the chosen item, and every other item un-chosen.
fn choose(flags: &[SharedFlag], index: usize) {
    for (other, flag) in flags.iter().enumerate() {
        write_flag(flag, other == index);
    }
}

fn indicator_of(flag: SharedFlag, chosen: bool, fill: PaintColor, height: f32) -> Conditional {
    let mut conditional = if chosen {
        Conditional::new(true)
    } else {
        Conditional::new(false)
    };
    conditional = conditional.when_flag(flag);
    conditional.show_with(ChildKey::index(0), LayoutRequest::new(), move || {
        Box::new(
            Surface::new()
                .request(LayoutRequest::new().height(height))
                .background(fill),
        )
    })
}

fn arrow_slot(width: f32, at_right: bool) -> LayoutRequest {
    let mut slot = LayoutRequest::new().height_pct(100.0).width(width);
    slot.position = Some(Position::Absolute);
    slot.inset = Some(if at_right {
        Inset {
            left: None,
            top: Some(0.0),
            right: Some(0.0),
            bottom: None,
        }
    } else {
        Inset {
            left: Some(0.0),
            top: Some(0.0),
            right: None,
            bottom: None,
        }
    });
    slot
}

/// One of the arrows: a box over the end of the strip, with a glyph in it.
fn arrow_widget(
    glyph: String,
    font_size: f32,
    ink: PaintColor,
    background: PaintColor,
    width: f32,
) -> Pressable {
    let box_ = Surface::new()
        .request(
            LayoutRequest::new()
                .height_pct(100.0)
                .width(width)
                .cross_align(Align::Center),
        )
        .background(background)
        .push(move || Box::new(Text::new(glyph.clone()).size(font_size).color(ink)));
    Pressable::new(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || Box::new(box_.clone()),
    ))
}
