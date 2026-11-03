//! Fluent slide group, ported to the Floom 2.0 widget API.

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

const ITEM_W: f32 = 120.0;
const ARROW_W: f32 = 32.0;
const ARROW_FONT_RATIO: f32 = 0.6;

type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

const PREV: &str = "\u{2039}";
const NEXT: &str = "\u{203a}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSlideGroupStyle {
    pub design_system: &'static str,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub outline: Color,
    pub arrow_background: Color,
    pub height: f32,
    pub gap: f32,
    pub active_indicator_height: f32,
    pub arrow_size: f32,
    pub item_width: f32,
    pub arrow_width: f32,
    pub arrow_font_size: f32,
}

pub struct FluentSlideGroup {
    props: SlideGroupProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<Child>,
    selected: usize,
    on_change: Option<Callback>,
}

impl FluentSlideGroup {
    pub fn new() -> Self {
        Self {
            props: SlideGroupProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            items: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }

    pub fn items(mut self, items: Vec<Child>) -> Self {
        self.items = items;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

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

    pub fn resolved_style(&self) -> FluentSlideGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideGroup(tokens) = theme.component(ComponentId::SlideGroup) else {
            unreachable!("ComponentId::SlideGroup must resolve to SlideGroupTokens")
        };
        FluentSlideGroupStyle {
            design_system: "fluent",
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

    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let count = self.items.len();
        let initial = self.selected.min(count.saturating_sub(1));
        // one. See the module's note on the choice after mount.
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
                .request(
                    LayoutRequest::new()
                        .height(height)
                        .min_width(item_width),
                )
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

impl Default for FluentSlideGroup {
    fn default() -> Self {
        Self::new()
    }
}

fn chosen_of(flags: &[SharedFlag]) -> usize {
    flags
        .iter()
        .position(|flag| read_flag(flag))
        .unwrap_or_default()
}

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
    let mut slot = LayoutRequest::new()
        .height_pct(100.0)
        .width(width);
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
