//! The list item row: a pressable band with a leading slot, a title/subtitle
//! column, and a trailing slot.
//!
//! ## Why this is in the engine
//!
//! Two of the three kits are the same component. Measured across
//! `md_list_item`, `cupertino_list_item` and `fluent_list_item`: **90 of 107
//! substantive lines are identical**, and the row's whole structure — the optional
//! leading slot, the two-line text column, the optional trailing slot, the
//! background, the press ripple with *sharp* corners, and the disabled click gate —
//! is in that shared part.
//!
//! The third (`md_list_item`) differs in exactly one way, and it is the reason this
//! file takes its numbers as arguments rather than reading tokens:
//!
//! | | `min_height` | `title_font_size` | `subtitle_font_size` |
//! |---|---|---|---|
//! | Material | `tokens.min_height` | `tokens.title_font_size` | `tokens.subtitle_font_size` |
//! | Cupertino | **44.0 literal** | **17.0 literal** | **15.0 literal** |
//! | Fluent | **40.0 literal** | **14.0 literal** | **12.0 literal** |
//!
//! So two kits ignore their own token table for three of the five fields, and
//! disagree with it: `ListItemTokens::default()` says `56.0 / 16.0 / 14.0` for every
//! kit, and only Material reads it. That is a real inconsistency in v1 and it is
//! **preserved rather than fixed** — a port that quietly routed all three through
//! the token table would change Cupertino's rows from 44pt to 56pt tall, which is a
//! design change wearing a refactor's clothes. Each kit crate therefore passes its
//! own numbers, and the tokens remain the Material path they already were.
//!
//! ## What is here rather than in a kit
//!
//! * the structure, and the fact that the text column is the only part that grows;
//! * that the row is a **band**, not a card: the ripple is not rounded, because
//!   v1 passed no radius and a rounded ink on a full-width row reads as a card;
//! * that `disabled` gates the click in one place. v1 wrote `if !disabled { .. }`
//!   inside the action closure, which guarded the pointer but not the keyboard.
//!
//! ## The one thing a kit must still supply
//!
//! The title's **font weight**, which v1 read from the theme's typography scale
//! (`typography().title.medium.font_weight`) rather than from the list-item tokens.
//! It is passed in as a resolved [`FontWeight`] because the engine has no business
//! knowing how a kit names its type ramp.

use std::sync::Arc;

use floom_core::{
    Align, Axis, Child, ChildKey, Color, CreateContext, LayoutRequest, UpdateContext, Widget,
    WidgetAny,
};
use floom_types::FontWeight;

use floom_widgets::compose::Stack;
use floom_widgets::controls::Pressable;
use floom_widgets::primitives::Text;
use floom_widgets::ripple::Ripple;
use floom_widgets::surface::Surface;

/// A replayed content builder — the shape of every 2.0 slot.
pub type SlotBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Everything about a list row that is not its content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListItemStyle {
    /// A floor for the row's height. v1's `min_height(..)`, and the reason
    /// [`floom_core::LayoutRequest::min_height`] exists.
    pub min_height: f32,
    /// Padding inside the row, left and right.
    pub padding_horizontal: f64,
    /// Gap between the leading slot, the text column and the trailing slot.
    pub gap: f32,
    /// The gap *inside* the text column, between the title and the subtitle. v1
    /// wrote `4.0` at the call site rather than as a token, so it is a field here
    /// rather than being hard-coded a second time.
    pub text_gap: f32,

    pub title_size: f32,
    pub title_weight: FontWeight,
    pub title_color: Color,
    /// `None` means no subtitle line, which is a different box from a subtitle of
    /// zero height — so this is an `Option` and not `f32`.
    pub subtitle_size: Option<f32>,
    pub subtitle_color: Color,

    /// The band's fill.
    pub background: Color,
}

impl Default for ListItemStyle {
    /// Material's token values, so an un-styled row is a Material row rather than a
    /// zero-height one.
    fn default() -> Self {
        ListItemStyle {
            min_height: 56.0,
            padding_horizontal: 16.0,
            gap: 16.0,
            text_gap: 4.0,
            title_size: 16.0,
            title_weight: FontWeight::Medium,
            title_color: Color::rgb(0x1D, 0x1B, 0x20),
            subtitle_size: Some(14.0),
            subtitle_color: Color::rgb(0x49, 0x4A, 0x5C),
            background: Color::rgb(0xFF, 0xFB, 0xFE),
        }
    }
}

/// A pressable band with a leading slot, a text column and a trailing slot.
pub struct ListItemRow {
    title: String,
    subtitle: Option<String>,
    leading: Option<SlotBuilder>,
    trailing: Option<SlotBuilder>,
    disabled: bool,
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    style: ListItemStyle,
}

impl std::fmt::Debug for ListItemRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListItemRow")
            .field("title", &self.title)
            .field("has_subtitle", &self.subtitle.is_some())
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl ListItemRow {
    pub fn new(title: impl Into<String>) -> Self {
        ListItemRow {
            title: title.into(),
            subtitle: None,
            leading: None,
            trailing: None,
            disabled: false,
            on_click: None,
            style: ListItemStyle::default(),
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The slot before the text column — an icon or an avatar in v1's terms.
    pub fn leading(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.leading = Some(Arc::new(build));
        self
    }

    /// The slot after the text column — a switch or a chevron.
    pub fn trailing(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.trailing = Some(Arc::new(build));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: ListItemStyle) -> Self {
        self.style = style;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn has_subtitle(&self) -> bool {
        self.subtitle.is_some()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn resolved_style(&self) -> ListItemStyle {
        self.style
    }

    /// The subtree, as one child: `Ripple(Pressable(Surface(row)))`.
    ///
    /// The ripple is **not** rounded. v1 passed no radius to it, and a full-width
    /// row with rounded ink reads as a card rather than a band — so the engine does
    /// not have a knob for it. If a kit ever wants one, this is the line to change,
    /// and it should change with a test that says which it was.
    fn children(&self) -> Vec<Child> {
        let style = self.style;
        let title = self.title.clone();
        let subtitle = self.subtitle.clone();
        let leading = self.leading.clone();
        let trailing = self.trailing.clone();
        let disabled = self.disabled;
        let on_click = self.on_click.clone();

        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || {
                let band = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width_pct(100.0)
                            .min_height(style.min_height)
                            .padding_horiz(style.padding_horizontal)
                            // v1's `.items_center()` on the band: the slots and the
                            // text column are centred in the row's cross axis.
                            .cross_align(Align::Center),
                    )
                    .background(style.background)
                    .push_layout(LayoutRequest::new().grow(1.0), {
                        // Cloned here, in the outer `Fn`'s body. The closure
                        // below is *also* an `Fn`, so anything it captured by
                        // value would be gone on a replay — and `title`,
                        // `subtitle` and `leading` are all read inside it.
                        let title = title.clone();
                        let subtitle = subtitle.clone();
                        let leading = leading.clone();
                        let trailing = trailing.clone();
                        move || {
                            let mut row = Stack::new(Axis::Horizontal)
                                .gap(style.gap)
                                .align(Align::Start, Align::Center);

                            if let Some(leading) = &leading {
                                row = row.push({
                                    let leading = leading.clone();
                                    move || leading()
                                });
                            }

                            // The text column, and the only part that grows: v1 gave
                            // it `flex_grow(1.0)`, which is what pushes the trailing
                            // slot to the far edge of a full-width row.
                            row = row.push_layout(LayoutRequest::new().grow(1.0), {
                                // One more level of `Fn`, so the strings are
                                // cloned out of the capture before the innermost
                                // builders take them by value.
                                let title = title.clone();
                                let subtitle = subtitle.clone();
                                move || {
                                    let title = title.clone();
                                    let subtitle = subtitle.clone();
                                    let mut column = Stack::new(Axis::Vertical)
                                        .gap(style.text_gap)
                                        .align(Align::Center, Align::Start);
                                    column = column.push({
                                        let title = title.clone();
                                        move || {
                                            Box::new(
                                                Text::new(title.clone())
                                                    .size(style.title_size)
                                                    .weight(style.title_weight)
                                                    .color(style.title_color),
                                            )
                                        }
                                    });
                                    if let Some(subtitle) = &subtitle {
                                        let size = style.subtitle_size.unwrap_or(style.title_size);
                                        column = column.push({
                                            let subtitle = subtitle.clone();
                                            move || {
                                                Box::new(
                                                    Text::new(subtitle.clone())
                                                        .size(size)
                                                        .weight(FontWeight::Normal)
                                                        .color(style.subtitle_color),
                                                )
                                            }
                                        });
                                    }
                                    Box::new(column)
                                }
                            });

                            if let Some(trailing) = &trailing {
                                row = row.push({
                                    let trailing = trailing.clone();
                                    move || trailing()
                                });
                            }

                            Box::new(row)
                        }
                    });

                // `Pressable` is built inside the ripple's builder rather than
                // captured into it: its `on_click` callback is deliberately not
                // `Send + Sync`, and a slot builder is required to be.
                Box::new(
                    Ripple::new()
                        .child(LayoutRequest::new().fill(), {
                            let band = band.clone();
                            let on_click = on_click.clone();
                            move || {
                                let mut pressable = Pressable::new(Child::new(
                                    ChildKey::index(0),
                                    LayoutRequest::new().fill(),
                                    {
                                        let band = band.clone();
                                        move || Box::new(band.clone())
                                    },
                                ))
                                .disabled(disabled);
                                if let Some(on_click) = on_click.as_ref() {
                                    let on_click = on_click.clone();
                                    pressable = pressable.on_click(move || on_click());
                                }
                                Box::new(pressable)
                            }
                        })
                        .disabled(disabled),
                )
            },
        )]
    }
}

impl Widget for ListItemRow {
    fn create(&self, cx: &mut dyn CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty, for the reason [`floom_ui_composites::icon_button::IconButton`]
    /// gives: this widget holds no state, so there is no change of its own to
    /// report. Every node is visited each frame regardless.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: floom_core::ElementId) {}

    fn children(&self) -> Vec<Child> {
        self.children()
    }
}

impl WidgetAny for ListItemRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // The row names itself through the `Pressable` inside it, which is where the
        // role and the label live; this node adds nothing and saying so is better
        // than inventing an empty hint.
        None
    }
}
