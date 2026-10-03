use std::time::Duration;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::sheet::{SheetProps, SheetSide};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSheetStyle {
    pub panel_color: Color,
    pub content_color: Color,
    pub corner_radius: f32,
    pub side_width: f32,
    pub elevated: bool,
}

pub struct FluentSheet {
    props: SheetProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentSheet {
    pub fn new() -> Self {
        let props = SheetProps::default();
        Self {
            open: RwSignal::new(props.open),
            props,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            child: None,
        }
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.props.side = side;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self.open.set(open);
        self
    }

    pub fn elevated(mut self, elevated: bool) -> Self {
        self.props.elevated = elevated;
        self
    }

    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }

    pub fn props(&self) -> &SheetProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> FluentSheetStyle {
        let theme = self.theme.get();
        let ComponentTokens::Sheet(tokens) = theme.component(ComponentId::Sheet) else {
            unreachable!("ComponentId::Sheet must resolve to SheetTokens")
        };
        FluentSheetStyle {
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            corner_radius: tokens.corner_radius,
            side_width: tokens.side_width,
            elevated: self.props.elevated,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let side = props.side;
        let open = self.open;
        let elevated = props.elevated;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::Sheet) {
            ComponentTokens::Sheet(t) => t,
            _ => unreachable!(),
        };
        let corner = theme.get().radius().sm;
        let pad = tokens.padding;
        let side_w = tokens.side_width;
        let scrim_color = tokens.scrim_color;

        let panel: AnyView = Container::new(child)
            .style(move |s| {
                let t = theme.get();
                let mut s = s
                    .background(t.colors().surface.to_peniko())
                    .color(t.colors().on_surface.to_peniko())
                    .padding(pad);
                match side {
                    SheetSide::Bottom | SheetSide::Top => {
                        s = s.width_pct(100.0);
                    }
                    SheetSide::Start | SheetSide::End => {
                        s = s.height_pct(100.0).width(side_w);
                    }
                }
                if matches!(side, SheetSide::Bottom | SheetSide::Top) {
                    s = s.border_radius(corner);
                }
                if elevated {
                    s = s
                        .box_shadow_blur(tokens.shadow_blur)
                        .box_shadow_spread(tokens.shadow_spread)
                        .box_shadow_color(with_alpha(BLACK, 40).to_peniko());
                }
                s
            })
            // Slide in from the anchored edge, slide back out on close.
            .animation(move |a| {
                a.run_on_create(true)
                    .run_on_remove(true)
                    .duration(Duration::from_millis(300))
                    .keyframe(0, |f| {
                        f.style(|s| match side {
                            SheetSide::Bottom => s.translate_y(56.0),
                            SheetSide::Top => s.translate_y(-56.0),
                            SheetSide::Start => s.translate_x(-side_w),
                            SheetSide::End => s.translate_x(side_w),
                        })
                    })
                    .keyframe(100, |f| f.computed_style())
            })
            .into_any();

        let scrim: AnyView = match side {
            SheetSide::Bottom => Container::new(panel)
                .style(move |s| {
                    s.flex_col()
                        .width_pct(100.0)
                        .height_pct(100.0)
                        .justify_end()
                        .items_center()
                        .background(with_alpha(scrim_color, 160).to_peniko())
                        .apply_if(!open.get(), |s| s.hide())
                })
                .animation(|a| {
                    a.run_on_create(true)
                        .run_on_remove(true)
                        .duration(Duration::from_millis(300))
                        .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                        .keyframe(100, |f| f.computed_style())
                })
                .into_any(),
            SheetSide::Top => Container::new(panel)
                .style(move |s| {
                    s.flex_col()
                        .width_pct(100.0)
                        .height_pct(100.0)
                        .justify_start()
                        .items_center()
                        .background(with_alpha(scrim_color, 160).to_peniko())
                        .apply_if(!open.get(), |s| s.hide())
                })
                .animation(|a| {
                    a.run_on_create(true)
                        .run_on_remove(true)
                        .duration(Duration::from_millis(300))
                        .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                        .keyframe(100, |f| f.computed_style())
                })
                .into_any(),
            SheetSide::Start => Container::new(panel)
                .style(move |s| {
                    s.flex_row()
                        .width_pct(100.0)
                        .height_pct(100.0)
                        .justify_start()
                        .items_center()
                        .background(with_alpha(scrim_color, 160).to_peniko())
                        .apply_if(!open.get(), |s| s.hide())
                })
                .animation(|a| {
                    a.run_on_create(true)
                        .run_on_remove(true)
                        .duration(Duration::from_millis(300))
                        .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                        .keyframe(100, |f| f.computed_style())
                })
                .into_any(),
            SheetSide::End => Container::new(panel)
                .style(move |s| {
                    s.flex_row()
                        .width_pct(100.0)
                        .height_pct(100.0)
                        .justify_end()
                        .items_center()
                        .background(with_alpha(scrim_color, 160).to_peniko())
                        .apply_if(!open.get(), |s| s.hide())
                })
                .animation(|a| {
                    a.run_on_create(true)
                        .run_on_remove(true)
                        .duration(Duration::from_millis(300))
                        .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                        .keyframe(100, |f| f.computed_style())
                })
                .into_any(),
        };

        Overlay::new(scrim)
    }
}

impl Default for FluentSheet {
    fn default() -> Self {
        Self::new()
    }
}
