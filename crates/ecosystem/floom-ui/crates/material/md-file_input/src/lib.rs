//! Material file input, ported to the Floom 2.0 widget API.
//!
//! A file input is a trigger -- a bordered box the user clicks -- and a readout
//! of the path that was chosen. The native picker is the host's: the trigger
//! fires `on_change` with the current path so the host can open a real dialog,
//! and the host writes the result back through [`MaterialFileInput::path_buffer`].
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Pressable`] around a [`Surface`] for the box, and
//! a horizontal `Stack` of two `Text`s inside it -- the trigger's label and the
//! readout. There is nothing here that a widget could own, so the shape stays in
//! the kit.
//!
//! ## What this kit answers
//!
//! * the box's height, its horizontal padding, its radius and the text's size:
//!   the tokens;
//! * the trigger label's ink: `primary`; the readout's: `on_surface` when a path
//!   is chosen and `on_surface_variant` when it is not;
//! * the box's rim: `outline`, or `on_surface` at 38 while disabled; the box's
//!   fill: `surface`, or `on_surface` at 4 while disabled;
//! * the trigger label's weight: the theme's `label.large` weight, which is
//!   where v1 read it from;
//! * the gap between the two texts (12), the rim's width (1) and the readout's
//!   "No file chosen": v1 literals.
//!
//! ## The style the port adds
//!
//! v1's file input had **no** `resolved_style` at all -- the colours were read
//! inline inside two style closures and were never reported anywhere. The port
//! adds [`MaterialFileInputStyle`], because a kit whose appearance cannot be
//! asked about cannot be tested against one, and because the style is the only
//! place the differences between the three kits can be written down.
//!
//! ## The readout does not follow the path
//!
//! v1's readout was a `Label::derived` over the path signal, so it changed the
//! moment the host wrote the path. 2.0's tree is mounted once -- a slot's build
//! closure runs when the key is new and never again -- so the readout shows the
//! path as it was when the kit was built. The *trigger* is live: it reports the
//! path as it is at the moment of the click, which is what the host needs in
//! order to open a picker. A host that wants a live readout rebuilds the kit.
//!
//! ## The content slot
//!
//! v1's file input had **no** slot for the caller's own content: the box held
//! the trigger label and the readout, and nothing else. The port adds an
//! optional [`Child`] slot after the readout, for one reason: the readout
//! cannot follow the path in 2.0 (see above), so a caller who needs a live
//! one has to supply it. With the slot empty -- which is how the port is used
//! by default -- the box is exactly v1's.
//!
//! With the slot filled, the row's `justify_between` spreads three children
//! instead of two, so the readout is no longer hard against the right edge.
//! That is named here rather than left to be discovered.
//!
//! ## The path is a buffer, not a signal
//!
//! v1's `path_signal()` returned an `RwSignal<Option<String>>`. 2.0's slot
//! closures must be `Send + Sync` and an `RwSignal` is `Rc`-based, so the path is
//! the engine's [`SharedBuffer`]: an `Arc<Mutex<String>>`. An empty buffer is
//! v1's `None` -- "no file chosen".
//!
//! ## The callback
//!
//! v1's `on_change` was a `Box<dyn Fn(Option<String>)>` in the shared props.
//! 2.0's slots are shared between threads, so the kit keeps its own
//! `Arc<dyn Fn(Option<String>) + Send + Sync>` and the props' slot stays empty.
//!
//! ## The weight the port maps
//!
//! v1's `to_floom()` answered in parley's `FontWeight`, because v1's text style
//! took parley's. 2.0's `Text` takes the engine's own nine weights, so the kit
//! maps between them. The two scales are the same nine names except that the
//! theme says `Regular` where the engine says `Normal`.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`.** 2.0's paint context has no cursor
//!   vocabulary. The trigger is still a `Pressable`, so it is still activatable
//!   by keyboard, which the pointer was not.
//! * **`accept` and `multiple`.** v1 stored both and the view read neither: they
//!   are hints for the host's picker, not something the view enforces. The port
//!   keeps them in the props, and the props still report them.
//! * **The accessors.** v1's file inputs had no `with_theme()`. The port gives
//!   all three kits the same set, because a kit that cannot be given a theme
//!   cannot be tested against one.
//!
//! ## The one thing the port adds to the trigger
//!
//! v1's box published nothing to assistive technology. 2.0's `Pressable` does,
//! so the port gives it the trigger's label as its accessible name. That is a
//! fix rather than a translation, and it is named here rather than slipped in.

use std::sync::{Arc, Mutex};

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::text_input::SharedBuffer;
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::file_input::FileInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(Option<String>) + Send + Sync + 'static>;

/// The gap between the trigger label and the readout. A v1 literal.
const GAP: f32 = 12.0;
/// The rim's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// What the readout says when no path has been chosen. A v1 literal.
const NO_FILE_CHOSEN: &str = "No file chosen";
/// v1's disabled rim: `on_surface` at 38.
const DISABLED_ALPHA: u8 = 38;
/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;

/// The file input's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialFileInputStyle {
    pub design_system: &'static str,
    /// The trigger label's ink.
    pub trigger_color: Color,
    /// The readout's ink when a path is chosen.
    pub path_color: Color,
    /// The readout's ink when no path is chosen.
    pub empty_path_color: Color,
    /// The box's rim.
    pub border_color: Color,
    /// The box's rim while disabled.
    pub disabled_border_color: Color,
    /// The box's fill.
    pub background: Color,
    /// The box's fill while disabled.
    pub disabled_background: Color,
    /// The box's height, from the token.
    pub height: f32,
    /// The box's horizontal padding, from the token.
    pub padding_horizontal: f32,
    /// The box's radius, from the token.
    pub border_radius: f32,
    /// The text's size, from the token.
    pub font_size: f32,
    /// The trigger label's weight, from the theme's type scale.
    pub trigger_font_weight: FontWeight,
    /// The gap between the trigger label and the readout. A v1 literal.
    pub gap: f32,
    /// The rim's width. A v1 literal.
    pub border_width: f32,
    /// What the readout says when no path has been chosen. A v1 literal.
    pub empty_path_label: &'static str,
}

/// Material's file input.
pub struct MaterialFileInput {
    props: FileInputProps,
    theme: RwSignal<MultiTheme>,
    /// The caller's own content, after the readout.
    ///
    /// v1's file input had no such slot; see the module documentation.
    child: Option<Child>,
    /// The chosen path, shared with whoever writes it.
    ///
    /// A [`SharedBuffer`] rather than v1's `RwSignal<Option<String>>`: a 2.0 slot
    /// closure has to be `Send + Sync` and an `RwSignal` is `Rc`-based. An empty
    /// buffer is v1's `None`.
    path: SharedBuffer,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(..)>` in the shared
    /// props, which cannot cross that boundary.
    on_change: Option<Callback>,
}

impl MaterialFileInput {
    pub fn new() -> Self {
        Self {
            props: FileInputProps::default(),
            theme: theme_signal(),
            child: None,
            path: Arc::new(Mutex::new(String::new())),
            on_change: None,
        }
    }

    /// The trigger's label. v1's default is "Upload".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// Supply content to put in the box, after the readout.
    ///
    /// v1 had no such slot: the box held the trigger label and the readout,
    /// and nothing else. The port adds one for one reason -- the readout cannot
    /// follow the path in 2.0 (see the module documentation), so a caller who
    /// needs a live one has to supply it. With the slot empty, which is how the
    /// port is used by default, the box is exactly v1's.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Comma-separated MIME / extension hint passed to the host picker.
    pub fn accept(mut self, accept: impl Into<String>) -> Self {
        self.props.accept = accept.into();
        self
    }

    /// Allow multiple files (stored for the host picker; the readout shows the
    /// first selected path).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Fires when the user clicks the trigger; the current path is passed so the
    /// host can open a native picker and write the result back.
    pub fn on_change(mut self, callback: impl Fn(Option<String>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props, for a caller that reads the design-system-neutral shape.
    ///
    /// `on_change` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and `FileInputProps`
    /// is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0 cannot use.
    pub fn props(&self) -> &FileInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared path buffer, so the host can write the picker's result.
    ///
    /// v1's `path_signal()`. The type changed because the type had to: a 2.0
    /// slot closure cannot hold an `RwSignal`.
    pub fn path_buffer(&self) -> SharedBuffer {
        self.path.clone()
    }

    /// The path as it is right now. Empty means "no file chosen".
    pub fn path(&self) -> String {
        self.path.lock().map(|guard| guard.clone()).unwrap_or_default()
    }

    /// Write the path, as the host would after a picker returns.
    pub fn set_path(&self, path: impl Into<String>) {
        if let Ok(mut guard) = self.path.lock() {
            *guard = path.into();
        }
    }

    pub fn resolved_style(&self) -> MaterialFileInputStyle {
        let theme = self.theme.get();
        let ComponentTokens::FileInput(tokens) = theme.component(ComponentId::FileInput) else {
            unreachable!("ComponentId::FileInput must resolve to FileInputTokens")
        };
        let c = theme.colors();
        MaterialFileInputStyle {
            design_system: "material",
            trigger_color: c.primary,
            path_color: c.on_surface,
            empty_path_color: c.on_surface_variant,
            border_color: c.outline,
            disabled_border_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            background: c.surface,
            disabled_background: with_alpha(c.on_surface, DISABLED_FILL_ALPHA),
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            trigger_font_weight: engine_weight(theme.typography().label.large.font_weight),
            gap: GAP,
            border_width: BORDER_WIDTH,
            empty_path_label: NO_FILE_CHOSEN,
        }
    }

    /// The trigger: the box, its two texts, and the click that reports the path.
    pub fn view(self) -> Pressable {
        let style = self.resolved_style();
        let disabled = self.props.disabled;
        let label_text = self.props.label.clone();

        // The readout is read once, here: 2.0's tree is mounted once.
        let path = self.path();
        let chosen = !path.is_empty();
        let readout_text = if chosen { path } else { style.empty_path_label.to_string() };

        let trigger_color = to_paint(style.trigger_color);
        let readout_color = to_paint(if chosen { style.path_color } else { style.empty_path_color });
        let font_size = style.font_size;
        let trigger_weight = style.trigger_font_weight;

        // v1's `Stack::horizontal((label, readout))`.
        let mut row = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            // v1's `.items_center().justify_between()`.
            .align(Align::SpaceBetween, Align::Center)
            .request(LayoutRequest::new().width_pct(100.0))
            .push({
                let label_text = label_text.clone();
                move || {
                    Box::new(
                        Text::new(label_text.clone())
                            .size(font_size)
                            .weight(trigger_weight)
                            .color(trigger_color),
                    )
                }
            })
            .push({
                let readout_text = readout_text.clone();
                move || Box::new(Text::new(readout_text.clone()).size(font_size).color(readout_color))
            });

        // The caller's own content, if any. It goes after the readout, so the
        // row's `justify_between` spreads three children instead of two and
        // the readout is no longer hard against the right edge.
        if let Some(child) = self.child.clone() {
            row = row.child(child);
        }

        let background = to_paint(if disabled {
            style.disabled_background
        } else {
            style.background
        });
        let border_color = to_paint(if disabled {
            style.disabled_border_color
        } else {
            style.border_color
        });

        let surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.height)
                    // v1's `.items_center()` on the box.
                    .cross_align(Align::Center),
            )
            .padding_horiz(f64::from(style.padding_horizontal))
            .background(background)
            .radius(style.border_radius)
            .border(style.border_width, border_color)
            .push({
                let row = row.clone();
                move || Box::new(row.clone())
            });

        let on_change = self.on_change.clone();
        let path = self.path.clone();

        Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            Box::new(surface.clone())
        }))
        // v1's `set_disabled(disabled.get())`.
        .disabled(disabled)
        .label(label_text)
        .on_click(move || {
            // v1's `.action(..)`: the callback fires only when the control is not
            // disabled, and it is handed the path as it is at that moment.
            if disabled {
                return;
            }
            if let Some(cb) = &on_change {
                let current = path.lock().map(|guard| guard.clone()).unwrap_or_default();
                cb(if current.is_empty() { None } else { Some(current) });
            }
        })
    }
}

impl Default for MaterialFileInput {
    fn default() -> Self {
        Self::new()
    }
}


/// The theme's weight, in the engine's vocabulary.
///
/// v1's `to_floom()` answered in parley's `FontWeight`, because v1's text style
/// took parley's. 2.0's `Text` takes the engine's own nine weights, and the two
/// scales differ in one name: the theme says `Regular` where the engine says
/// `Normal`.
fn engine_weight(weight: ThemeFontWeight) -> FontWeight {
    match weight {
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
