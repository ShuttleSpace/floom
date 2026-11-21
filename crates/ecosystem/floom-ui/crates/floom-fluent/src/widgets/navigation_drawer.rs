//! Fluent navigation drawer, ported to the Floom 2.0 widget API.

use floom_core::{Child, ChildKey, LayoutRequest, PresenceTransition, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Edges, Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const SCRIM_ALPHA: u8 = 160;
const ACCENT_WIDTH: f32 = 3.0;
const SHADOW_ALPHA: u8 = 30;
const MOTION_SECS: f64 = 0.26;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentNavigationDrawerStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub scrim: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    pub scrim_alpha: u8,
    pub accent_width: f32,
    pub accent_color: Color,
    pub shadow_blur: f32,
    pub shadow_spread: f32,
    pub shadow_color: Color,
}

pub struct FluentNavigationDrawer {
    props: NavigationDrawerProps,
    open: SharedFlag,
    header: Option<Child>,
    content: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl FluentNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            header: None,
            content: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.props.width = width;
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    pub fn side(mut self, side: NavigationDrawerSide) -> Self {
        self.props.side = side;
        self
    }

    pub fn header(mut self, header: Child) -> Self {
        self.header = Some(header);
        self
    }

    pub fn child(mut self, child: Child) -> Self {
        self.content = Some(child);
        self
    }

    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.props.open = read_flag(&open);
        self.open = open;
        self
    }

    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    pub fn props(&self) -> &NavigationDrawerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        FluentNavigationDrawerStyle {
            design_system: "fluent",
            background: theme.colors().surface,
            scrim: theme.colors().scrim,
            width: self.props.width,
            elevation: self.props.elevation,
            padding: tokens.padding,
            scrim_alpha: SCRIM_ALPHA,
            accent_width: ACCENT_WIDTH,
            accent_color: theme.colors().primary,
            shadow_blur: f32::from(self.props.elevation) * 3.0,
            shadow_spread: 1.0,
            shadow_color: with_alpha(Color::BLACK, SHADOW_ALPHA),
        }
    }

    pub fn view(self) -> Overlay {
        let style = self.resolved_style();
        let side = self.props.side;
        let open = self.open.clone();

        floom_core_theme::layout::register_layout(
            match side {
                NavigationDrawerSide::Left => floom_core_theme::layout::LayoutSlot::NavDrawerLeft,
                NavigationDrawerSide::Right => floom_core_theme::layout::LayoutSlot::NavDrawerRight,
            },
            if read_flag(&open) { style.width } else { 0.0 },
        );

        let scrim_slot = Child::new(ChildKey::index(1), LayoutRequest::new().grow(1.0), {
            let dismiss = open.clone();
            move || {
                let dismiss = dismiss.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().absolute_fill(),
                        || Box::new(Surface::new()),
                    ))
                    .on_click(move || {
                        write_flag(&dismiss, false);
                    }),
                ) as Box<dyn WidgetAny>
            }
        });

        let panel_color = to_paint(style.background);
        let accent_color = to_paint(style.accent_color);
        let shadow = Shadow {
            color: to_paint(style.shadow_color),
            blur: f64::from(style.shadow_blur),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: f64::from(style.shadow_spread),
        };
        let padding = style.padding;
        let header = self.header.clone();
        let content = self.content.clone();
        let mut panel = Surface::new()
            .request(LayoutRequest::new().width(style.width).height_pct(100.0))
            .padding(Insets::uniform(f64::from(padding)))
            .background(panel_color)
            .border_edges(
                style.accent_width,
                accent_color,
                match side {
                    NavigationDrawerSide::Left => Edges::LEFT,
                    NavigationDrawerSide::Right => Edges::RIGHT,
                },
            );
        if style.elevation > 0 {
            panel = panel.shadow(shadow);
        }
        let panel = panel.push(move || {
            let header = header.clone();
            let content = content.clone();
            let mut column = Stack::new(Axis::Vertical)
                .gap(8.0)
                .align(Align::Start, Align::Start);
            column = column.child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                move || match &header {
                    Some(header) => header.build(),
                    None => Box::new(Surface::new()) as Box<dyn WidgetAny>,
                },
            ));
            column = column.child(Child::new(
                ChildKey::index(1),
                LayoutRequest::new(),
                move || match &content {
                    Some(content) => content.build(),
                    None => Box::new(Surface::new()) as Box<dyn WidgetAny>,
                },
            ));
            Box::new(column)
        });

        let panel_slot = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width(style.width).height_pct(100.0),
            move || Box::new(panel.clone()),
        );
        let mut row = Stack::new(Axis::Horizontal);
        match side {
            NavigationDrawerSide::Left => {
                row = row.child(panel_slot);
                row = row.child(scrim_slot);
            }
            NavigationDrawerSide::Right => {
                row = row.child(scrim_slot);
                row = row.child(panel_slot);
            }
        }

        let scrim_color = to_paint(with_alpha(style.scrim, style.scrim_alpha));
        let layer = Surface::new()
            .request(LayoutRequest::new().absolute_fill())
            .background(scrim_color)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                move || Box::new(row.clone()),
            ));

        let content = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(layer.clone()),
        );

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || {
                Box::new(presence(
                    &open,
                    PresenceTransition::fade(MOTION_SECS),
                    content.clone(),
                )) as Box<dyn WidgetAny>
            },
        ))
    }
}

impl Default for FluentNavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}

fn presence(open: &SharedFlag, transition: PresenceTransition, child: Child) -> Conditional {
    let mut conditional = Conditional::new(false).when_flag(open.clone());
    if !read_flag(open) {
        conditional.sync_when(false);
    }
    conditional.presence(transition).show(child)
}
