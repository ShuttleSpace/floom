//! Material 3 dialog, ported to the Floom 2.0 widget API.
//!
//! ## What the port settled
//!
//! Correction 11 stopped at "a dialog cannot own its `open: RwSignal<bool>`". The answer
//! turned out to be that the architecture had already made the decision — the host owns
//! presence, through `OverlayKind::Dialog`, `OverlayHandle` and the state-driven overlay
//! door — and the only thing missing was a flag a caller can *hold*. That is
//! [`floom_widgets::text_input::SharedFlag`], an `Arc<Mutex<bool>>`, for the same reason
//! `TextInput`'s buffer is an `Arc<Mutex<String>>`: a slot closure has to be `Send + Sync`.
//!
//! So the five v1 methods around `open` all become three:
//!
//! | v1 | here |
//! |---|---|
//! | `open(bool)` | `open(bool)` / `close()`, writing the flag |
//! | `is_open()` | `is_open()`, reading it |
//! | `open_signal() -> RwSignal<bool>` | [`MdDialog::open_flag`] |
//! | `with_open_signal(RwSignal<bool>)` | [`MdDialog::with_open_flag`] |
//! | `close()` | [`MdDialog::close`] |
//!
//! ## The card is the kit's, because of the close button
//!
//! v1's card is a composition — a title row carrying a close button, then the content,
//! then a row of action buttons — and the engine's `Dialog::card` is a title above a
//! body with no slot for a close button. Rather than give the engine a
//! `close_button: Option<Child>` that three kits would fill differently (and whose
//! placement differs per design system), the card is built here and handed over through
//! [`floom_widgets::Dialog::card_with`]. The engine keeps doing the one thing only it can:
//! mounting it as a modal overlay over the viewport.
//!
//! With a card supplied, `Dialog::title` applies **only to semantics** — the accessible
//! name — so this does not get two titles.
//!
//! ## Two losses
//!
//! * **v1's pop-in animation is gone.** v1 scaled the card from 95% to 100%. 2.0 has
//!   `AnimationSpec` and `AnimatedContent`, and the host drives animations, so this is
//!   reachable — but wiring a scale into a card that lives inside an overlay's factory is
//!   a separate piece of work and is recorded rather than half-done.
//! * **v1's click-to-close-on-the-scrim is the host's job now**, which is better: a
//!   widget cannot see a press that lands outside itself, so v1's `on_event(Click)` on the
//!   scrim only ever worked because the scrim covered the window. The host's
//!   `dismiss_top_overlay_on_press` does it for real, and a *modal* overlay is
//!   deliberately not light-dismissed — so this dialog now stays up when the user clicks
//!   outside it, where v1's would have closed. **That is a behaviour change**, and it is
//!   the host's rule rather than the kit's.
//!
//! ## `FontWeight` is mapped a second time
//!
//! [`to_engine_weight`] mirrors `floom_widget_md_list_item::to_engine_weight` rather than
//! importing it. That is duplication, and it is deliberate for now: the theme's weight
//! enum lives in `floom-core-theme`, which the engine does not depend on, so the mapping
//! cannot live in `floom-widgets` without inverting the dependency. A second caller is
//! the signal that it wants a home, and this note is where that is recorded.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::FontWeight;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_ui_composites::{Dialog};
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::dialog::DialogProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;


/// The theme's weight enum -> the engine's.
///
/// Mirrors `floom_widget_md_list_item::to_engine_weight`. Duplicated rather than imported
/// for the reason the module note gives: the enum is in `floom-core-theme`, which the
/// engine does not depend on, so this cannot move into `floom-widgets` yet.
///
/// v1 called `FontWeight::to_floom()`, which returns a `parley::style::FontWeight` — a
/// third type with the same name. Going through parley's numeric weight would mean
/// reaching into a dependency's internals from a kit crate to avoid naming nine variants.
///
/// One difference from v1, recorded rather than inherited: v1's `to_floom()` sends
/// **`Black` to `EXTRA_BOLD` (800)**, because parley's table has no 900. Here `Black`
/// maps to `floom_types::Black` (900). Unverifiable from here — no call site reads
/// `Black` — and it is deliberately the only place the two answers differ.
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
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
    /// The shared "is it up" flag.
    ///
    /// Not an `RwSignal`: that is `Rc`-based and cannot cross a `Send + Sync` slot
    /// boundary, which is the whole of correction 11.
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
    /// v1's `Box<dyn Fn()>`, stored so `props()` keeps reporting it.
    on_close: Option<Callback>,
    /// The action buttons, as `(label, callback)` in `Send + Sync` form.
    actions: Vec<(String, Callback)>,
}

impl MdDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        MdDialog {
            // v1 seeded the signal from `props.open`, and the props default is `false`.
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

    /// Open or close the dialog.
    ///
    /// Writes the shared flag rather than asking the widget, because the widget is built
    /// by [`Self::view`] and a caller holding only this value must still be able to open
    /// it — which is the whole reason v1 handed out `open_signal()`.
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

    /// The shared open flag, so an external trigger (a button, a route) can drive it.
    ///
    /// v1's `open_signal()`, and the reason this whole shape exists.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Drive this dialog from a flag somebody else owns.
    ///
    /// v1's `with_open_signal(RwSignal<bool>)`. Two dialogs sharing one flag is
    /// meaningful — a dialog and the button that opens it — and `Arc::ptr_eq` in the test
    /// is what makes "shares" mean shares rather than "coincidentally holds the same
    /// value twice".
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

    /// Add an action button, as v1's `action(label, callback)` did.
    pub fn action(mut self, label: impl Into<String>, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let label = label.into();
        let shared: Callback = std::sync::Arc::new(callback);
        self.props
            .actions
            .push((label.clone(), Box::new({
                let shared = shared.clone();
                move || shared()
            })));
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
            // v1's `with_alpha(on_surface, 222)` on the content — the body text is
            // deliberately lighter than the title, and the number is the reason.
            content_alpha: 222,
            primary: to_paint(theme.colors().primary),
            title_size: theme.typography().headline.small.font_size,
            body_size: theme.typography().body.medium.font_size,
            action_size: theme.typography().label.large.font_size,
            action_weight: to_engine_weight(theme.typography().label.large.font_weight),
            // v1's call-site numbers, none of them tokens.
            action_padding_horizontal: 12.0,
            action_padding_vertical: 10.0,
            action_radius: 8.0,
            action_gap: 8.0,
            close_padding: 4.0,
            close_icon_size: 20.0,
            // v1's `with_alpha(black, 30)` on the card's shadow.
            shadow_alpha: 30,
            // v1's `with_alpha(primary, 8)` on an action's hover fill.
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
        // `MdIcon::resolve()` is total, so this is `Some` for Material. It is still an
        // `Option` because a kit whose table *can* miss — Cupertino and Fluent, which
        // fall back to a dot — has to have somewhere to put that.
        let close_icon: Option<floom_widgets::VectorIcon> =
            floom_widget_md_icon::MdIcon::new("close").resolve().into();

        Dialog::new()
            // Semantics only: the card below carries its own title row, so setting the
            // engine's title as well would render two.
            .title(title.clone())
            .with_shared_presence(open.clone())
            .card_with(move || build_card(style, title.clone(), content.clone(), actions.clone(), on_close.clone(), close_icon.clone(), open.clone()))
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

/// The card: v1's composition, built here because of the close button.
///
/// A free function taking ownership, for the reason `combobox::row_widget` is: it is
/// three `Fn` levels deep otherwise, and each level would need its own clone of the same
/// six values.
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

    // The action row.
    let mut action_row = Stack::new(Axis::Horizontal)
        .gap(style.action_gap)
        .align(floom_widgets::Align::End, floom_widgets::Align::Center);
    for (label, callback) in actions.iter().cloned() {
        action_row = action_row.push({
            let callback = callback.clone();
            let open = open.clone();
            move || {
                let label = label.clone();
                // `.label(..)` runs after the inner closures have taken `label` by
                // move, so it gets its own copy taken here.
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
                                            .padding_horiz(f64::from(style.action_padding_horizontal))
                                            .padding_vert(f64::from(style.action_padding_vertical)),
                                    )
                                    .radius(style.action_radius)
                                    .child(Child::new(
                                        ChildKey::index(0),
                                        LayoutRequest::new(),
                                        {
                                            let label = label.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(label.clone())
                                                        .size(style.action_size)
                                                        .weight(style.action_weight)
                                                        .color(style.primary),
                                                ) as Box<dyn floom_widgets::WidgetAny>
                                            }
                                        },
                                    )),
                            ) as Box<dyn floom_widgets::WidgetAny>
                        },
                    ))
                    .on_click(move || {
                        // v1: the callback, then close.
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
        .child(Child::new(ChildKey::index(0), LayoutRequest::new().fill(), {
            let title = title.clone();
            move || {
                Box::new(Text::new(title.clone()).size(style.title_size).color(style.on_surface))
                    as Box<dyn floom_widgets::WidgetAny>
            }
        }))
        .child(Child::new(ChildKey::index(1), LayoutRequest::new(), {
            let on_close = on_close.clone();
            let open = open.clone();
            let close_icon = close_icon.clone();
            move || {
                // Built here, not captured: `Pressable` is a builder, not a `Clone`.
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
                Text::new(content.clone()).size(style.body_size).color(PaintColor {
                    a: style.content_alpha,
                    ..style.on_surface
                }),
            ) as Box<dyn floom_widgets::WidgetAny>
        }
    });

    let inner = Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            Box::new(title_row.clone()) as Box<dyn floom_widgets::WidgetAny>
        }))
        .child(content_child)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            Box::new(action_row.clone()) as Box<dyn floom_widgets::WidgetAny>
        }));

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
            .child(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                Box::new(inner.clone()) as Box<dyn floom_widgets::WidgetAny>
            })),
    )
}

/// The close button: v1's icon in a padded box, and the only thing in the card that
/// closes the dialog.
///
/// Its own function because `Pressable` is a builder rather than a `Clone` value, so it
/// has to be constructed inside the slot closure that mounts it — the same rule
/// `combobox`'s chevron follows.
fn close_button(
    style: DialogStyle,
    icon: Option<floom_widgets::VectorIcon>,
    on_close: Option<Callback>,
    open: SharedFlag,
) -> impl floom_widgets::WidgetAny {
    use floom_widgets::{Pressable, Surface};

    Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
        match icon.clone() {
            Some(mut icon) => {
                icon.set_size(style.close_icon_size);
                icon.set_color(style.on_surface);
                Box::new(icon) as Box<dyn floom_widgets::WidgetAny>
            }
            None => Box::new(Surface::new().request(
                LayoutRequest::new()
                    .width(style.close_icon_size)
                    .height(style.close_icon_size),
            )),
        }
    }))
    .on_click(move || {
        // v1 fired `on_close` and *then* set `open = false`. Both, in that order: a
        // callback that reads the flag sees it still true, exactly as v1's did.
        if let Some(on_close) = &on_close {
            on_close();
        }
        write_flag(&open, false);
    })
    .label("Close")
}

/// Type alias for `floom-elements` compatibility.
pub type MdDialogStyle = DialogStyle;
