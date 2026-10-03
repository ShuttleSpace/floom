use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom_widget_cupertino_icon::cupertino_icon_svg;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::rating::RatingProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoRatingStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub star_size: f32,
    pub gap: f32,
}

pub struct CupertinoRating {
    props: RatingProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoRating {
    pub fn new() -> Self {
        Self {
            props: RatingProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn resolved_style(&self) -> CupertinoRatingStyle {
        let theme = self.theme.get();
        let ComponentTokens::Rating(tokens) = theme.component(ComponentId::Rating) else {
            unreachable!("ComponentId::Rating must resolve to RatingTokens")
        };
        CupertinoRatingStyle {
            design_system: DESIGN_SYSTEM,
            active_color: Color::rgb(255, 204, 0),
            inactive_color: theme.colors().outline,
            star_size: tokens.star_size,
            gap: tokens.gap,
        }
    }

    /// Build an interactive row of Cupertino rating stars (system yellow when filled).
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
            let star = svg(move || cupertino_icon_svg("star"))
                .style(move |s| {
                    let v = value.get();
                    let t = theme.get();
                    let c = if (i as f32) < v {
                        Color::rgb(255, 204, 0)
                    } else {
                        t.colors().outline
                    };
                    s.size(star_size, star_size).color(c.to_peniko())
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

impl Default for CupertinoRating {
    fn default() -> Self {
        Self::new()
    }
}
