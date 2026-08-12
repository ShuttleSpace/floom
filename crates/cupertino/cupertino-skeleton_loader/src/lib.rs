use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::skeleton_loader::{SkeletonType, SkeletonLoaderProps};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoSkeletonStyle {
    pub color: Color,
    pub outer_width: f32,
}

pub struct CupertinoSkeletonLoader {
    props: SkeletonLoaderProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSkeletonLoader {
    pub fn new() -> Self {
        Self {
            props: SkeletonLoaderProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn skeleton_type(mut self, skeleton_type: SkeletonType) -> Self {
        self.props.skeleton_type = skeleton_type;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    pub fn lines(mut self, lines: usize) -> Self {
        self.props.lines = lines;
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.props.animated = animated;
        self
    }

    pub fn resolved_style(&self) -> CupertinoSkeletonStyle {
        let theme = self.theme.get();
        let ComponentTokens::SkeletonLoader(tokens) = theme.component(ComponentId::SkeletonLoader) else {
            unreachable!("ComponentId::SkeletonLoader must resolve to SkeletonTokens")
        };
        let outer_width = match self.props.skeleton_type {
            SkeletonType::Card => self.props.width.unwrap_or(200.0),
            SkeletonType::Rect => self.props.width.unwrap_or(160.0),
            _ => self.props.width.unwrap_or(240.0),
        };
        CupertinoSkeletonStyle {
            color: tokens.color,
            outer_width,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let tokens = match theme.get().component(ComponentId::SkeletonLoader) {
            ComponentTokens::SkeletonLoader(t) => t,
            _ => unreachable!(),
        };
        let color = tokens.color;
        let corner = tokens.corner_radius;
        let text_h = tokens.text_height;
        let avatar = tokens.avatar_size;
        let gap = tokens.gap;
        let card_r = tokens.card_radius;

        let outer_width = match props.skeleton_type {
            SkeletonType::Card => props.width.unwrap_or(200.0),
            SkeletonType::Rect => props.width.unwrap_or(160.0),
            _ => props.width.unwrap_or(240.0),
        };

        let content: AnyView = match props.skeleton_type {
            SkeletonType::Circle => {
                let size = props.width.or(props.height).unwrap_or(avatar);
                Container::new(empty())
                    .style(move |s| {
                        s.size(size, size)
                            .background(color.to_peniko())
                            .border_radius(9999.0)
                    })
                    .into_any()
            }
            SkeletonType::Rect => {
                let h = props.height.unwrap_or(text_h * 2.0);
                let w = props.width.unwrap_or(160.0);
                Container::new(empty())
                    .style(move |s| {
                        s.width(w)
                            .height(h)
                            .background(color.to_peniko())
                            .border_radius(corner)
                    })
                    .into_any()
            }
            SkeletonType::Card => {
                let w = props.width.unwrap_or(200.0);
                let h = props.height.unwrap_or(120.0);
                Container::new(empty())
                    .style(move |s| {
                        s.width(w)
                            .height(h)
                            .background(color.to_peniko())
                            .border_radius(card_r)
                    })
                    .into_any()
            }
            SkeletonType::Text => {
                let lines = props.lines.max(1);
                let mut bars: Vec<AnyView> = Vec::with_capacity(lines);
                for _ in 0..lines {
                    bars.push(
                        Container::new(empty())
                            .style(move |s| {
                                s.width_pct(100.0)
                                    .height(text_h)
                                    .background(color.to_peniko())
                                    .border_radius(corner)
                            })
                            .into_any(),
                    );
                }
                list(bars).into_any()
            }
            SkeletonType::Avatar => {
                let circ = Container::new(empty())
                    .style(move |s| {
                        s.size(avatar, avatar)
                            .background(color.to_peniko())
                            .border_radius(9999.0)
                    })
                    .into_any();
                let lines = props.lines.max(1).min(3);
                let mut bars: Vec<AnyView> = Vec::with_capacity(lines);
                for _ in 0..lines {
                    bars.push(
                        Container::new(empty())
                            .style(move |s| {
                                s.width_pct(100.0)
                                    .height(text_h)
                                    .background(color.to_peniko())
                                    .border_radius(corner)
                            })
                            .into_any(),
                    );
                }
                let right = list(bars).style(move |s| s.gap(gap));
                h_stack((circ, right.into_any()))
                    .style(move |s| s.gap(gap).items_center())
                    .into_any()
            }
        };

        Container::new(content)
            .style(move |s| s.width(outer_width).items_start())
    }
}

impl Default for CupertinoSkeletonLoader {
    fn default() -> Self {
        Self::new()
    }
}
