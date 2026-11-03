
use floom_core::{Child, ChildKey, Inset, LayoutRequest, Position, PresenceTransition, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

const SCRIM_ALPHA: u8 = 150;
const MOTION_SECS: f64 = 0.26;
const SHADOW_ALPHA: u8 = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialNavigationDrawerStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub panel_color: Color,
    pub content_color: Color,
    /// The scrim's colour, before the alpha below.
    pub scrim_color: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    pub scrim_alpha: u8,
    pub shadow_blur: f32,
    pub shadow_spread: f32,
    pub shadow_color: Color,
}

/// Material's navigation drawer.
pub struct MaterialNavigationDrawer {
    props: NavigationDrawerProps,
    /// Whether the drawer is open.
    open: SharedFlag,
    header: Option<Child>,
    content: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl MaterialNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            header: None,
            content: None,
            theme: theme_signal(),
        }
    }

    /// Open or close the drawer.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// The panel's width, in dp.
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = width;
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    /// Choose the edge the drawer slides in from (`Left` or `Right`).
    pub fn side(mut self, side: NavigationDrawerSide) -> Self {
        self.props.side = side;
        self
    }

    /// The header shown at the top of the drawer.
    pub fn header(mut self, header: Child) -> Self {
        self.header = Some(header);
        self
    }

    /// The drawer's content.
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

    /// Whether the drawer is open.
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

    pub fn resolved_style(&self) -> MaterialNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        MaterialNavigationDrawerStyle {
            design_system: "material",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            scrim_color: theme.colors().scrim,
            width: if self.props.width > 0.0 {
                self.props.width
            } else {
                tokens.width
            },
            elevation: self.props.elevation,
            padding: tokens.padding,
            scrim_alpha: SCRIM_ALPHA,
            shadow_blur: 2.0 + 3.0 * f32::from(self.props.elevation),
            shadow_spread: 1.0,
            shadow_color: with_alpha(Color::rgba(0, 0, 0, 255), SHADOW_ALPHA),
        }
    }

    /// The drawer: the layer, the scrim and the panel.
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

        let scrim_color = to_paint(with_alpha(style.scrim_color, style.scrim_alpha));
        let scrim = Child::new(ChildKey::index(0), LayoutRequest::new().absolute_fill(), {
            let dismiss = open.clone();
            move || {
                let dismiss = dismiss.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().absolute_fill(),
                        move || Box::new(Surface::new().background(scrim_color)),
                    ))
                    .on_click(move || {
                        write_flag(&dismiss, false);
                    }),
                ) as Box<dyn WidgetAny>
            }
        });

        let panel_color = to_paint(style.panel_color);
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
        let panel = Surface::new()
            .request(panel_request(side, style.width))
            .padding(Insets::uniform(f64::from(padding)))
            .background(panel_color)
            .shadow(shadow)
            .push(move || {
                let header = header.clone();
                let content = content.clone();
                let header = header.clone();
                let content = content.clone();
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

        let scrim_flag = open.clone();
        let panel_flag = open.clone();
        let layer = Stack::new(Axis::Vertical)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().absolute_fill(),
                move || {
                    Box::new(presence(
                        &scrim_flag,
                        PresenceTransition::fade(MOTION_SECS),
                        scrim.clone(),
                    )) as Box<dyn WidgetAny>
                },
            ))
            .child(Child::new(
                ChildKey::index(1),
                LayoutRequest::new().absolute_fill(),
                move || {
                    let panel = panel.clone();
                    let from = match side {
                        NavigationDrawerSide::Left => -style.width,
                        NavigationDrawerSide::Right => style.width,
                    };
                    Box::new(presence(
                        &panel_flag,
                        PresenceTransition::slide_x(MOTION_SECS, from),
                        Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                            let panel = panel.clone();
                            Box::new(panel) as Box<dyn WidgetAny>
                        }),
                    )) as Box<dyn WidgetAny>
                },
            ));

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(layer.clone()),
        ))
    }
}

impl Default for MaterialNavigationDrawer {
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

fn panel_request(side: NavigationDrawerSide, width: f32) -> LayoutRequest {
    let mut request = LayoutRequest::new().width(width).height_pct(100.0);
    request.position = Some(Position::Absolute);
    request.inset = Some(Inset {
        left: match side {
            NavigationDrawerSide::Left => Some(0.0),
            NavigationDrawerSide::Right => None,
        },
        top: Some(0.0),
        right: match side {
            NavigationDrawerSide::Left => None,
            NavigationDrawerSide::Right => Some(0.0),
        },
        bottom: None,
    });
    request
}
