//! Fluent sheet, ported to the Floom 2.0 widget API.

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Insets, Shadow};

use floom_ui_composites::Dialog;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::sheet::{SheetProps, SheetSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const SCRIM_ALPHA: u8 = 160;
const SHADOW_ALPHA: u8 = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSheetStyle {
    pub design_system: &'static str,
    pub panel_color: Color,
    pub content_color: Color,
    pub corner_radius: f32,
    pub token_corner_radius: f32,
    pub side_width: f32,
    pub elevated: bool,
    pub padding: f32,
    pub scrim_color: Color,
    pub shadow_blur: f32,
    pub shadow_spread: f32,
    pub shadow_color: Color,
}

pub struct FluentSheet {
    props: SheetProps,
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl FluentSheet {
    pub fn new() -> Self {
        let props = SheetProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.props.side = side;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    pub fn elevated(mut self, elevated: bool) -> Self {
        self.props.elevated = elevated;
        self
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self.props.open = read_flag(&self.open);
        self
    }

    pub fn props(&self) -> &SheetProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentSheetStyle {
        let theme = self.theme.get();
        let ComponentTokens::Sheet(tokens) = theme.component(ComponentId::Sheet) else {
            unreachable!("ComponentId::Sheet must resolve to SheetTokens")
        };
        FluentSheetStyle {
            design_system: "fluent",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            corner_radius: theme.radius().sm,
            token_corner_radius: tokens.corner_radius,
            side_width: tokens.side_width,
            elevated: self.props.elevated,
            padding: tokens.padding,
            scrim_color: tokens.scrim_color,
            shadow_blur: tokens.shadow_blur,
            shadow_spread: tokens.shadow_spread,
            shadow_color: with_alpha(BLACK, SHADOW_ALPHA),
        }
    }

    pub fn view(self) -> Dialog {
        let style = self.resolved_style();
        let side = self.props.side;
        let child = self.child.clone();
        let open = self.open.clone();

        Dialog::new()
            .with_shared_presence(open)
            .card_with(move || build_card(style, side, child.clone()))
    }
}

impl Default for FluentSheet {
    fn default() -> Self {
        Self::new()
    }
}

fn build_card(
    style: FluentSheetStyle,
    side: SheetSide,
    child: Option<Child>,
) -> Box<dyn WidgetAny> {
    let scrim = to_paint(with_alpha(style.scrim_color, SCRIM_ALPHA));
    let panel = build_panel(style, side, child);

    Box::new(
        Surface::new()
            .request(LayoutRequest::new().fill())
            .background(scrim)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().fill(),
                move || {
                    let panel = panel.clone();
                    let mut stack = match side {
                        SheetSide::Bottom => {
                            Stack::new(Axis::Vertical).align(Align::End, Align::Center)
                        }
                        SheetSide::Top => {
                            Stack::new(Axis::Vertical).align(Align::Start, Align::Center)
                        }
                        SheetSide::Start => {
                            Stack::new(Axis::Horizontal).align(Align::Start, Align::Center)
                        }
                        SheetSide::End => {
                            Stack::new(Axis::Horizontal).align(Align::End, Align::Center)
                        }
                    };
                    stack = stack.request(LayoutRequest::new().fill());
                    stack = stack.child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(panel.clone()),
                    ));
                    Box::new(stack)
                },
            )),
    )
}

fn build_panel(style: FluentSheetStyle, side: SheetSide, child: Option<Child>) -> Surface {
    let mut request = LayoutRequest::new().padding(Insets::uniform(f64::from(style.padding)));
    match side {
        SheetSide::Bottom | SheetSide::Top => {
            request = request.width_pct(100.0);
        }
        SheetSide::Start | SheetSide::End => {
            request = request.height_pct(100.0).width(style.side_width);
        }
    }

    let mut panel = Surface::new()
        .request(request)
        .background(to_paint(style.panel_color));

    if matches!(side, SheetSide::Bottom | SheetSide::Top) {
        panel = panel.radius(style.corner_radius);
    }

    if style.elevated {
        panel = panel.shadow(Shadow {
            color: to_paint(style.shadow_color),
            blur: f64::from(style.shadow_blur),
            spread: f64::from(style.shadow_spread),
            offset_x: 0.0,
            offset_y: 0.0,
        });
    }

    if let Some(child) = child {
        panel = panel.child(child);
    }

    panel
}
