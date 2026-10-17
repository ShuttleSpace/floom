//! Cupertino rounded text field, ported to the 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1 was a styled `Container` wrapped around a `TextInput`, with the kit's
//!   `value` and `focused` `RwSignal`s read inside three separate style closures
//!   and an `Rc<Cell<bool>>` called `initialized` whose only job was to make a
//!   `create_effect` skip its own first run. It is the engine's `TextField` now:
//!   one `Widget` with `layout_request` and `paint`, holding the string, the
//!   caret, the IME composition and an `InteractionState`.
//! * Both signals are gone, and `initialized` with them. The engine owns the
//!   value and the focus and reports through `on_change`, so there is nothing
//!   left to keep in step and no first callback left to swallow. **No bridge was
//!   built in their place.** v1 needed a signal because the value lived outside
//!   the widget; a signal here would be a second source of truth for a widget
//!   that already has one, and it would put the host rebuild back, which is the
//!   thing v1's signal existed to avoid.
//! * `initial_value` is now `value`, and the `value(&self) -> String` getter is
//!   gone with the signal it read. There is exactly one write of the value
//!   before mount, so the two names were describing one call. A mounted field
//!   reports its own value through `TextField::text()`.
//! * `set_value` is gone for the same reason. The engine's `TextField` has one,
//!   and it does not fire `on_change`, because a programmatic write is not a
//!   keystroke -- which is the very thing v1's kit could not express.
//! * The two `FocusGained` / `FocusLost` listeners are gone. The engine's
//!   `InteractionState.focused` is set by the host's focus dispatch, so the same
//!   event arrives by one door instead of two, and a kit asking "am I focused?"
//!   was reaching past its own layer.
//! * `cursor(CursorStyle::Text)` is gone with the style layer -- `TextFieldStyle`
//!   has no cursor field. Reported rather than papered over: the text caret is
//!   now whatever the host applies to a focused field.
//! * `items_center()` is gone because it no longer has anything to centre. The
//!   engine's paint puts the text block on `(height - font_size) / 2` itself,
//!   which is what the wrapping `Container` was doing.
//! * `set_disabled` is gone with the style layer's generic dim. The disabled
//!   palette is now composed per field, from this kit's own tokens, in
//!   `resolved_style` -- which is what `TextFieldStyle` asks of a kit.
//!
//! ## What this kit answers, and what the other two answer instead
//!
//! Cupertino tints on hover. Its `view()` ended with
//! `.hover(|s| s.border_color(with_alpha(t.colors().primary, 120)))`, so the only
//! difference from the other two crates is that `border_color_hovered` is
//! a `Some` carrying `primary` at alpha 120 rather than `None`. Its rim stays
//! one point thick while focused and it drew no focus ring, so
//! `border_width_focused` is `BORDER_WIDTH` and `focus_outline_width` is `0.0`.
//! Material thickens its rim instead and
//! Fluent rings the box; neither of those is this kit's answer, and a test says
//! so.
//!
//! The three `view()` bodies in v1 were line-for-line identical apart from those
//! two lines, which is measured rather than remembered: with the crate name, the
//! design-system enum and the one accessor that moved position removed, the
//! three files differed by `border(if focused { 2.0 } else { 1.0 })` against
//! `border(1.0)`, and a `hover` closure against a `focus` closure. Everything
//! else they agreed on is the table `resolved_style` writes.
//!
//! ## The field filled its container in v1, and does not here
//!
//! v1 put `.width_pct(100.0)` on both the `Container` and the `TextInput`, so a
//! text field was always as wide as its parent. The engine's `TextField` reports
//! an *intrinsic* width -- `max(shown text + 2 x padding_horizontal,
//! min_width)` -- and lets the host size it, so a field mounted from this kit is
//! as wide as the text in it rather than as wide as its row.
//!
//! That is a real loss and it is reported rather than repaired. Putting it back
//! is a *layout* decision, not a colour: either a width on the widget, or a host
//! that stretches it. An `Option<f32>` on `TextFieldStyle` would be the wrong
//! shape -- the style describes ink, and a layout policy wearing a style field is
//! how a widget ends up with three different width answers and no note saying
//! which is which. A host that wants a full-width field lays one out at the
//! width it wants; the pinned height and the border radius are still this kit's.
//!
//! ## Two colours no v1 source ever named
//!
//! `placeholder_color` and `preedit_color` are not in any of the three kits' style
//! structs, because v1's `TextInput` carried its own defaults for both and those
//! defaults died with the v1 widget. **They are inherited from
//! `TextFieldStyle::default()` here and nothing more.** Whether a placeholder
//! should be `on_surface_variant` and an in-progress IME composition `primary`
//! is a design question, and it belongs to whoever owns the theme; answering it
//! in three kit crates would be three unreviewed decisions wearing a port's
//! clothes. The gap is kept visible by a test rather than by a comment alone.
//!
//! ## The 97 that v1's `resolved_style` claimed
//!
//! All three kits' `resolved_style()` wrote `on_surface` at alpha **97** for the
//! disabled rim and the disabled ink, and not one of them was ever called: v1's
//! `view()` read `with_alpha(t.colors().on_surface, 38)` inline, three times per
//! kit. So 38 is what these fields painted, and 38 is what this port composes.
//! The dead struct's number is recorded here rather than quietly dropped,
//! because it is the sort of thing that looks authoritative. Pinned by a test,
//! along with the 4 that v1's disabled fill used.
//!
//! ## `MultiTheme::new_for` does not choose a design system
//!
//! Worth knowing before reading the tests. `MultiTheme::activate` is documented
//! in the theme crate as a no-op placeholder, and `MultiTheme::ds()` reads a
//! process-wide signal at access time. So `with_theme(MultiTheme::new_for(
//! DesignSystem::Cupertino))` does **not** make a Cupertino field read Cupertino's
//! tokens: it reads whichever design system the process is currently in, and
//! that is Material until something says otherwise. This is a property of the
//! theme crate rather than of this kit, so nothing here works around it -- but it
//! is the reason the tests pin the design system per thread before reading a
//! number, and the reason "this kit's 44pt field" is currently a claim
//! about a global rather than about the crate.

/// This kit's design system, as v1's style struct spelled it.
///
/// v1 carried it as a `design_system: &'static str` field on `CupertinoTextFieldStyle`, and
/// `tests/api.rs` asserted it. The engine's `TextFieldStyle` has no such field
/// and no room for one -- it describes ink, and a design system is not ink -- so
/// the string moved to a constant of its own rather than disappearing with the
/// struct. Material never had such a field, so Material has no such constant
/// either, and that is a fact about Material's v1 struct rather than about
/// Material's design.
pub const DESIGN_SYSTEM: &str = "cupertino";

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::text_field::TextFieldProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::{TextField, TextFieldStyle};

/// v1's rim at rest and while focused -- Cupertino's focus affordance is the rim's
/// *colour*, not its thickness, so both widths are this number.
const BORDER_WIDTH: f32 = 1.0;

/// The same number, written down separately because Material's is not: v1 wrote
/// `.border(1.0)` here and `.border(if focused { 2.0 } else { 1.0 })` there, and
/// the difference between the two is the whole disagreement.
const BORDER_WIDTH_FOCUSED: f32 = BORDER_WIDTH;

/// v1's disabled ink and disabled rim: `on_surface` at 38.
const DISABLED_ALPHA: u8 = 38;

/// v1's disabled fill: `on_surface` at 4.
const DISABLED_FILL_ALPHA: u8 = 4;

/// v1's `.hover(|s| s.border_color(with_alpha(primary, 120)))`: the only hovered
/// rim of the three kits. Material and Fluent answered hover with nothing.
const HOVERED_BORDER_ALPHA: u8 = 120;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// The design-system colour at an explicit alpha, in the engine's type.
///
/// Written out because `with_alpha` answers in the theme crate's `Color` while
/// `TextFieldStyle` speaks the engine's, so every resolved colour below is two
/// types and one conversion wide.
fn tinted(c: Color, alpha: u8) -> PaintColor {
    to_paint(with_alpha(c, alpha))
}

/// Cupertino rounded text field.
pub struct CupertinoTextField {
    props: TextFieldProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTextField {
    pub fn new() -> Self {
        Self {
            props: TextFieldProps::default(),
            // A fixed binding, not the app-wide signal -- the same choice v1 made,
            // and the same one `fluent-slider` and `fluent-radio` make. Only
            // Material's kits follow the theme signal. Note that
            // `MultiTheme::new_for` does not itself select a design system at
            // read time; see the module doc.
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Cupertino)),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    /// The value the field starts with.
    ///
    /// v1 called this `initial_value`, and there was also a `value(&self) ->
    /// String` getter over the same signal. The getter is gone with the signal,
    /// so the setter takes the short name: there is one write before mount, and
    /// this is it. A mounted field's value is `TextField::text()`.
    ///
    /// Nothing is clamped here any more. v1 ran `clamp_value` over this string
    /// in three methods, all of them to keep its own `value` signal in step;
    /// with no signal there is nothing to keep in step, and `max_length` is now
    /// enforced by the widget on the builder path *and* on the typing path.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
    }

    /// Marks the field as holding a secret.
    ///
    /// **This does not mask anything, and it did not in v1 either.** All three
    /// kits carried `is_password` on the shared props and none of their `view()`
    /// bodies read it, so the flag has been inert since before the port; the
    /// engine's `TextField` has no mask of its own. The prop and this builder
    /// stay so a caller that wants a mask says so out loud rather than silently
    /// rendering a secret in the clear, and a test says so out loud too.
    ///
    /// Masking is an engine feature, not a kit style field: the widget would have
    /// to substitute the run it draws *and* the label it publishes to assistive
    /// technology. Inventing either here would be the kit quietly disagreeing
    /// with the widget about what the user is looking at.
    pub fn password(mut self, is_password: bool) -> Self {
        self.props.is_password = is_password;
        self
    }

    /// Caps the field at `max_length` **characters**.
    ///
    /// v1 took a `usize` and wrote `Some(n)`, and this does the same. What
    /// changed is what the cap reaches. v1's kit truncated only the string it
    /// was handed -- in `initial_value`, `max_length` and `set_value` -- so
    /// `max_length(8)` did nothing whatsoever about the eight characters the user
    /// then typed: a half-implemented limit, shipped identically by three
    /// designs. The engine clamps the typing path and the IME-commit path as
    /// well, because all three arrive as character keys. See `view`.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.props.max_length = Some(max_length);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &TextFieldProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Cupertino rounded text field appearance, as the engine's `TextFieldStyle`.
    ///
    /// Every line in here was written by v1's `view()` style closures. Nothing
    /// here is a number this port chose, and nothing is read from v1's dead
    /// `resolved_style` struct -- see the module doc for what that struct
    /// claimed and what the screen actually showed.
    pub fn resolved_style(&self) -> TextFieldStyle {
        let theme = self.theme.get();
        let ComponentTokens::TextField(tokens) = theme.component(ComponentId::TextField) else {
            unreachable!("ComponentId::TextField must resolve to TextFieldTokens")
        };
        let colors = theme.colors();

        TextFieldStyle {
            // --- from `TextFieldTokens`, as v1 read them ---
            height: Some(tokens.height),
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,

            // --- the rim ---
            border_width: BORDER_WIDTH,
            border_width_focused: BORDER_WIDTH_FOCUSED,
            border_color: to_paint(colors.outline),
            border_color_focused: to_paint(colors.primary),
            // v1 wrote no hovered rim for this kit, so `None` -- "no variant of
            // this kit's own", which paints the resting `outline`. The engine's
            // default is also `None`, so this is a decision that agrees with it
            // rather than one it carries.
            border_color_hovered: Some(tinted(colors.primary, HOVERED_BORDER_ALPHA)),
            border_color_disabled: tinted(colors.on_surface, DISABLED_ALPHA),

            // --- the focus ring, drawn outside the rim ---
            // No ring outside the box. This kit's focus answer is the rim's
            // colour, so the width is zero -- and the colour, which `paint` only
            // reads when the width is positive, stays at the engine's default
            // rather than being named here. Material draws no ring either;
            // Fluent's `2.0` is the third answer.
            focus_outline_width: 0.0,

            // --- the fill ---
            background_color: to_paint(colors.surface),
            background_color_disabled: tinted(colors.on_surface, DISABLED_FILL_ALPHA),

            // --- the ink ---
            text_color: to_paint(colors.on_surface),
            text_color_disabled: tinted(colors.on_surface, DISABLED_ALPHA),
            font_size: theme.typography().body.medium.font_size,

            // Deliberately inherited from `TextFieldStyle::default()`. Each of
            // these is a field v1 never named, so each is a claim this port is
            // not making:
            //
            // * `vertical_padding` and `min_width` -- v1 pinned the height from
            //   the token and filled the parent, so it had neither a derived row
            //   height nor a width floor to express.
            // * `background_color_hovered` and `background_color_pressed` -- v1
            //   styled its `TextInput` with `color` and `font_size` only and
            //   left the fill to the wrapping `Container`, which had no hover or
            //   press rule either. So the engine's own fills apply: `#F5F7FA`
            //   under the pointer, white while pressed. That is engine
            //   behaviour these kits neither chose nor contradicted, and
            //   replacing it with `None` would be a design decision about
            //   hover that this port has no licence to make.
            // * `placeholder_color` and `preedit_color` -- the two colours no v1
            //   source ever named; see the module doc.
            ..TextFieldStyle::default()
        }
    }

    /// The field, ready to mount.
    ///
    /// ## `max_length` is now more than v1's was
    ///
    /// `props.max_length` crosses into the engine unchanged, and the engine
    /// clamps three doors rather than the one v1's kit clamped: the `value()`
    /// builder, `set_value`, and -- the widening -- `insert`, which is every
    /// keystroke and every committed IME composition, since the runner turns an
    /// OS `ImeCommit` into one character key-down per scalar. In v1
    /// `max_length(8)` bounded the string you passed in and nothing you typed.
    ///
    /// `value` is applied before `max_length` on purpose. The engine's
    /// `max_length` re-clamps whatever is already there, so either order ends up
    /// with the same value, but this is the order the engine's own doc names.
    ///
    /// ## What is not here
    ///
    /// No signal, no `create_effect`, no `initialized` flag: the engine fires
    /// `on_change` for a keystroke and for nothing else, so a field built with a
    /// value no longer reports that value back as though the user had typed it.
    /// And no `password` handling -- see `password`, which is honest about being
    /// inert.
    ///
    /// `TextFieldProps::on_change` is a `Box<dyn Fn(&str)>` whose `Clone` impl
    /// deliberately drops the callback, so it is moved rather than borrowed --
    /// which works only because `view` consumes the kit.
    pub fn view(self) -> TextField {
        let style = self.resolved_style();

        let mut field = TextField::new()
            .value(self.props.value.clone())
            .placeholder(self.props.placeholder.clone())
            .disabled(self.props.disabled)
            .max_length(self.props.max_length)
            .style(style);

        if let Some(on_change) = self.props.on_change {
            field = field.on_change(move |text| on_change(text));
        }
        field
    }
}

impl Default for CupertinoTextField {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine's `TextFieldStyle`, under this kit's old name.
///
/// v1's own style struct carried `border_color`, `text_color`, `height`,
/// `padding_horizontal` and `border_radius` -- and nothing ever read it, because
/// v1's `view()` resolved every colour from the theme inside its own style
/// closures. The alias keeps the name resolvable for callers without pretending
/// a struct still exists.
pub type CupertinoTextFieldStyle = TextFieldStyle;
