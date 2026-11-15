use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::FontWeight;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_ui_composites::Dialog;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::dialog::DialogProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}

/// A v1-shaped callback, kept so `props()` still reports one.
type Callback = std::sync::Arc<dyn Fn() + Send + Sync + 'static>;

/// Material's dialog.
pub struct MdDialog {
    props: DialogProps,
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
    on_close: Option<Callback>,
    /// The action buttons, as `(label, callback)` in `Send + Sync` form.
    actions: Vec<(String, Callback)>,
}

impl MdDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        MdDialog {
            open: shared_flag(props.open),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props,
            on_close: None,
            actions: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// Close it. Returns whether anything changed.
    pub fn close(&mut self) -> bool {
        self.props.open = false;
        write_flag(&self.open, false)
    }

    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self.props.open = read_flag(&self.open);
        self
    }

    pub fn on_close(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.props.on_close = Some(Box::new({
            let shared = std::sync::Arc::new(callback);
            self.on_close = Some(shared.clone());
            move || shared()
        }));
        self
    }

    pub fn action(
        mut self,
        label: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let label = label.into();
        let shared: Callback = std::sync::Arc::new(callback);
        self.props.actions.push((
            label.clone(),
            Box::new({
                let shared = shared.clone();
                move || shared()
            }),
        ));
        self.actions.push((label, shared));
        self
    }

    pub fn props(&self) -> &DialogProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> DialogStyle {
        let theme = self.theme.get();
        let ComponentTokens::Dialog(tokens) = theme.component(ComponentId::Dialog) else {
            unreachable!("ComponentId::Dialog must resolve to DialogTokens")
        };
        DialogStyle {
            max_width: tokens.max_width,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            surface: to_paint(theme.colors().surface),
            on_surface: to_paint(theme.colors().on_surface),
            content_alpha: 222,
            primary: to_paint(theme.colors().primary),
            title_size: theme.typography().headline.small.font_size,
            body_size: theme.typography().body.medium.font_size,
            action_size: theme.typography().label.large.font_size,
            action_weight: to_engine_weight(theme.typography().label.large.font_weight),
            action_padding_horizontal: 12.0,
            action_padding_vertical: 10.0,
            action_radius: 8.0,
            action_gap: 8.0,
            close_padding: 4.0,
            close_icon_size: 20.0,
            shadow_alpha: 30,
            action_hover_alpha: 8,
        }
    }

    /// The dialog, ready to mount.
    pub fn view(self) -> Dialog {
        let style = self.resolved_style();
        let title = self.props.title.clone();
        let content = self.props.content.clone();
        let actions = self.actions.clone();
        let on_close = self.on_close.clone();
        let open = self.open.clone();
        let close_icon: Option<floom_widgets::VectorIcon> =
            crate::widgets::icon::MdIcon::new("close").resolve().into();

        Dialog::new()
            .title(title.clone())
            .with_shared_presence(open.clone())
            .card_with(move || {
                build_card(
                    style,
                    title.clone(),
                    content.clone(),
                    actions.clone(),
                    on_close.clone(),
                    close_icon.clone(),
                    open.clone(),
                )
            })
    }
}

impl Default for MdDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything `resolved_style` resolves, so a caller can read it without mounting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogStyle {
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
    pub surface: PaintColor,
    pub on_surface: PaintColor,
    pub content_alpha: u8,
    pub primary: PaintColor,
    pub title_size: f32,
    pub body_size: f32,
    pub action_size: f32,
    pub action_weight: FontWeight,
    pub action_padding_horizontal: f32,
    pub action_padding_vertical: f32,
    pub action_radius: f32,
    pub action_gap: f32,
    pub close_padding: f32,
    pub close_icon_size: f32,
    pub shadow_alpha: u8,
    pub action_hover_alpha: u8,
}

fn build_card(
    style: DialogStyle,
    title: String,
    content: String,
    actions: Vec<(String, Callback)>,
    on_close: Option<Callback>,
    close_icon: Option<floom_widgets::VectorIcon>,
    open: SharedFlag,
) -> Box<dyn floom_widgets::WidgetAny> {
    use floom_widgets::{Axis, Pressable, Stack, Surface, Text};

    let card_layout = LayoutRequest::new()
        .padding(floom_core::Insets {
            left: f64::from(style.padding),
            right: f64::from(style.padding),
            top: f64::from(style.padding),
            bottom: f64::from(style.padding),
        })
        .max_width(style.max_width);

    let mut action_row = Stack::new(Axis::Horizontal)
        .gap(style.action_gap)
        .align(floom_widgets::Align::End, floom_widgets::Align::Center);
    for (label, callback) in actions.iter().cloned() {
        action_row = action_row.push({
            let callback = callback.clone();
            let open = open.clone();
            move || {
                let label = label.clone();
                let label_text = label.clone();
                let callback = callback.clone();
                let open = open.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new()
                            .padding_horiz(f64::from(style.action_padding_horizontal))
                            .padding_vert(f64::from(style.action_padding_vertical)),
                        move || {
                            Box::new(
                                Surface::new()
                                    .request(
                                        LayoutRequest::new()
                                            .padding_horiz(f64::from(
                                                style.action_padding_horizontal,
                                            ))
                                            .padding_vert(f64::from(style.action_padding_vertical)),
                                    )
                                    .radius(style.action_radius)
                                    .child(Child::new(ChildKey::index(0), LayoutRequest::new(), {
                                        let label = label.clone();
                                        move || {
                                            Box::new(
                                                Text::new(label.clone())
                                                    .size(style.action_size)
                                                    .weight(style.action_weight)
                                                    .color(style.primary),
                                            )
                                                as Box<dyn floom_widgets::WidgetAny>
                                        }
                                    })),
                            ) as Box<dyn floom_widgets::WidgetAny>
                        },
                    ))
                    .on_click(move || {
                        callback();
                        write_flag(&open, false);
                    })
                    .label(label_text),
                ) as Box<dyn floom_widgets::WidgetAny>
            }
        });
    }

    let title_row = Stack::new(Axis::Horizontal)
        .align(floom_widgets::Align::Start, floom_widgets::Align::Center)
        .request(LayoutRequest::new().fill())
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            {
                let title = title.clone();
                move || {
                    Box::new(
                        Text::new(title.clone())
                            .size(style.title_size)
                            .color(style.on_surface),
                    ) as Box<dyn floom_widgets::WidgetAny>
                }
            },
        ))
        .child(Child::new(ChildKey::index(1), LayoutRequest::new(), {
            let on_close = on_close.clone();
            let open = open.clone();
            let close_icon = close_icon.clone();
            move || {
                Box::new(close_button(
                    style,
                    close_icon.clone(),
                    on_close.clone(),
                    open.clone(),
                )) as Box<dyn floom_widgets::WidgetAny>
            }
        }));

    let content_child = Child::new(ChildKey::index(0), LayoutRequest::new(), {
        let content = content.clone();
        move || {
            Box::new(
                Text::new(content.clone())
                    .size(style.body_size)
                    .color(PaintColor {
                        a: style.content_alpha,
                        ..style.on_surface
                    }),
            ) as Box<dyn floom_widgets::WidgetAny>
        }
    });

    let inner = Stack::new(Axis::Vertical)
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(title_row.clone()) as Box<dyn floom_widgets::WidgetAny>,
        ))
        .child(content_child)
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(action_row.clone()) as Box<dyn floom_widgets::WidgetAny>,
        ));

    Box::new(
        Surface::new()
            .request(card_layout)
            .background(style.surface)
            .radius(style.border_radius)
            .shadow(floom_types::Shadow {
                color: PaintColor {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: style.shadow_alpha,
                },
                blur: 0.0,
                spread: 0.0,
                offset_x: 0.0,
                offset_y: 0.0,
            })
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(inner.clone()) as Box<dyn floom_widgets::WidgetAny>,
            )),
    )
}

fn close_button(
    style: DialogStyle,
    icon: Option<floom_widgets::VectorIcon>,
    on_close: Option<Callback>,
    open: SharedFlag,
) -> impl floom_widgets::WidgetAny {
    use floom_widgets::{Pressable, Surface};

    Pressable::new(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || match icon.clone() {
            Some(mut icon) => {
                icon.set_size(style.close_icon_size);
                icon.set_color(style.on_surface);
                Box::new(icon) as Box<dyn floom_widgets::WidgetAny>
            }
            None => Box::new(
                Surface::new().request(
                    LayoutRequest::new()
                        .width(style.close_icon_size)
                        .height(style.close_icon_size),
                ),
            ),
        },
    ))
    .on_click(move || {
        if let Some(on_close) = &on_close {
            on_close();
        }
        write_flag(&open, false);
    })
    .label("Close")
}

/// Type alias for `floom-elements` compatibility.
pub type MdDialogStyle = DialogStyle;
