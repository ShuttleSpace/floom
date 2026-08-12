use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::rating::RatingProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentRatingStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub star_size: f32,
    pub gap: f32,
}

pub struct FluentRating {
    props: RatingProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentRating {
    pub fn new() -> Self {
        Self {
            props: RatingProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.props.max = max;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.props.readonly = readonly;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &RatingProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentRatingStyle {
        let theme = self.theme.get();
        let ComponentTokens::Rating(tokens) = theme.component(ComponentId::Rating) else {
            unreachable!("ComponentId::Rating must resolve to RatingTokens")
        };
        FluentRatingStyle {
            design_system: DESIGN_SYSTEM,
            active_color: theme.colors().primary,
            inactive_color: theme.colors().outline_variant,
            star_size: tokens.star_size,
            gap: tokens.gap,
        }
    }

    /// Build an interactive row of Fluent rating stars (accent fill when active).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let max = self.props.max.max(1);
        let readonly = self.props.readonly;
        let on_change: Rc<Option<Box<dyn Fn(f32)>>> = Rc::new(self.props.on_change);

        let value = RwSignal::new(self.props.value.clamp(0.0, max as f32));
        let tokens = match theme.get().component(ComponentId::Rating) {
            ComponentTokens::Rating(t) => t,
            _ => unreachable!(),
        };
        let star_size = tokens.star_size;
        let gap = tokens.gap;

        let mut stars: Vec<AnyView> = Vec::with_capacity(max);
        for i in 0..max {
            let star = Container::new(text("★".to_string()))
                .style(move |s| {
                    let v = value.get();
                    let t = theme.get();
                    let c = if (i as f32) < v {
                        t.colors().primary
                    } else {
                        t.colors().outline_variant
                    };
                    s.font_size(star_size).color(c.to_peniko())
                })
                .style(move |s| {
                    s.cursor(if readonly {
                        CursorStyle::Default
                    } else {
                        CursorStyle::Pointer
                    })
                })
                .on_event(listener::Click, {
                    let on_change = on_change.clone();
                    let value = value;
                    move |_, _| {
                        if !readonly {
                            let nv = (i as f32) + 1.0;
                            value.set(nv);
                            if let Some(cb) = on_change.as_ref() {
                                cb(nv);
                            }
                        }
                        EventPropagation::Continue
                    }
                })
                .into_any();
            stars.push(star);
        }

        Container::new(list(stars)).style(move |s| s.flex_row().gap(gap).items_center())
    }
}

impl Default for FluentRating {
    fn default() -> Self {
        Self::new()
    }
}
