//! The engine's [`IconButton`]: the composition, the variant rules, and the two
//! things a disabled toggle has to get right.
//!
//! This suite exists because the icon button is *assembled*, not painted. Every
//! assertion is about a decision that could plausibly have gone the other way and
//! would still compile: which wrapper is outermost, whether the icon is centred,
//! whether the disabled ink replaces the variant's or blends with it, and whether
//! the glyph follows the box when the button is disabled.
//!
//! Note on borrows: [`Ripple`] and [`Pressable`] are deliberately not `Clone` — a
//! `Pressable` carries a callback that is deliberately not `Send + Sync`, and
//! cloning one would hand out a second node pretending to be the first. So the
//! walks below hold the type-erased box alive and borrow out of it, rather than
//! copying.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{Align, Color, Length, Widget, WidgetAny};
use floom_types::{Point, PointerEvent, Rect};

use floom_ui_composites::{
    IconButton, IconButtonIcon, IconButtonPaint, IconButtonStyle, IconButtonVariant,
};
use floom_widgets::testkit::{
    paint_snapshot, semantics_snapshot, AllocatingCreateContext, RecordingUpdateContext,
    TestEventContext,
};
use floom_widgets::{Pressable, Ripple, Surface, VectorIcon};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 40.0,
    y1: 40.0,
};

/// A style whose three variants are *distinguishable*, so a test that reads the
/// wrong arm cannot pass by accident.
///
/// Material's real numbers would not do: its text and outlined variants are both
/// transparent fills, so a swapped pair is invisible there and visible only in
/// Cupertino.
fn style() -> IconButtonStyle {
    IconButtonStyle {
        size: 40.0,
        icon_size: 24.0,
        radius: 20.0,
        border_width: 1.0,
        foreground: Color::rgb(0x11, 0x22, 0x33),
        disabled_foreground: Color::rgba(0x11, 0x22, 0x33, 38),
        disabled_background: Color::rgba(0x11, 0x22, 0x33, 12),
        disabled_border: Color::rgba(0x11, 0x22, 0x33, 38),
        text: IconButtonPaint {
            background: Color::rgba(0, 0, 0, 0),
            border: Color::rgba(0, 0, 0, 0),
            ripple_ink: Color::rgba(0xAA, 0x00, 0x00, 24),
        },
        contained: IconButtonPaint {
            background: Color::rgb(0, 0, 0xFF),
            border: Color::rgb(0, 0, 0xFF),
            ripple_ink: Color::rgba(0, 0xFF, 0, 24),
        },
        outlined: IconButtonPaint {
            background: Color::rgba(0, 0, 0, 0),
            border: Color::rgb(0xFF, 0, 0),
            ripple_ink: Color::rgba(0, 0, 0xBB, 24),
        },
    }
}

fn icon() -> VectorIcon {
    VectorIcon::from_path_data(
        "M 0 0 L 10 10 L 20 0 Z",
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 20.0,
            y1: 20.0,
        },
    )
    .size(24.0)
    .color(Color::rgb(0x99, 0x99, 0x99))
}

fn button() -> IconButton {
    IconButton::new(icon()).style(style())
}

/// The button's one child, built.
fn root(button: &IconButton) -> Box<dyn WidgetAny> {
    let children = button.children();
    assert_eq!(children.len(), 1, "the button is one node with one child");
    children[0].build()
}

/// The single child of `parent`, which must be a `T`.
fn child_of<T: WidgetAny>(parent: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    let children = parent.children();
    assert_eq!(children.len(), 1, "expected one child to downcast");
    let built = children[0].build();
    assert!(
        built.as_any().is::<T>(),
        "expected {}, got something else",
        std::any::type_name::<T>()
    );
    built
}

/// The surface, from the button down: `Ripple -> Pressable -> Surface`.
fn surface_of(button: &IconButton) -> Box<dyn WidgetAny> {
    let ripple = root(button);
    let ripple = ripple
        .as_any()
        .downcast_ref::<Ripple>()
        .expect("the ripple is outermost");
    let pressable = child_of::<Pressable>(ripple as &dyn WidgetAny);
    child_of::<Surface>(pressable.as_ref())
}

/// The icon node, from the button down.
fn icon_node(button: &IconButton) -> Box<dyn WidgetAny> {
    let surface = surface_of(button);
    let surface = surface
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the surface is innermost");
    let children = surface.children();
    assert_eq!(children.len(), 1, "the surface holds one child: the icon");
    children[0].build()
}

// --- the composition ------------------------------------------------------

/// The whole point of the port: `Ripple(Pressable(Surface(icon)))`, outermost
/// first. Each step fails if its wrapper moves inward, and none of them could be
/// satisfied by a widget that paints the icon itself.
#[test]
fn the_subtree_is_ripple_pressable_surface_icon() {
    let button = button();

    let ripple = root(&button);
    assert!(
        ripple.as_any().is::<Ripple>(),
        "the ripple is outermost: it is what clips the ink to the box"
    );

    let pressable = child_of::<Pressable>(ripple.as_ref());
    assert!(
        pressable.as_any().is::<Pressable>(),
        "the pressable is inside the ripple, so the whole box is the click target"
    );

    let surface = child_of::<Surface>(pressable.as_ref());
    assert!(
        surface.as_any().is::<Surface>(),
        "and the surface is innermost, holding the icon"
    );

    let icon = child_of::<IconButtonIcon>(surface.as_ref());
    assert!(
        icon.as_any().is::<IconButtonIcon>(),
        "and the icon node is the innermost of all"
    );
}

/// The control for the walk above: each step really did descend, so a suite that
/// asserted against the same node four times would be caught here.
#[test]
fn each_step_of_the_walk_descends() {
    let button = button();
    let ripple = root(&button);
    let ripple_ref = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let pressable = child_of::<Pressable>(ripple_ref);
    let pressable_ref = pressable.as_any().downcast_ref::<Pressable>().unwrap();
    let surface = child_of::<Surface>(pressable_ref);
    let surface_ref = surface.as_any().downcast_ref::<Surface>().unwrap();
    let icon = child_of::<IconButtonIcon>(surface_ref);

    // Four different node types at four depths: a walk that had gone in circles
    // would have hit the first assertion's `is::<Ripple>()` check.
    assert!(icon.as_any().is::<IconButtonIcon>());
    assert_ne!(
        std::any::TypeId::of::<IconButtonIcon>(),
        std::any::TypeId::of::<Surface>()
    );
}

// --- the box --------------------------------------------------------------

/// v1 asked for `.size(btn_size, btn_size)`, so the surface's own request pins
/// both dimensions, and the icon sits inside it centred.
#[test]
fn the_box_is_square_and_centres_its_icon() {
    let surface = surface_of(&button());
    let surface = surface
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the surface")
        .clone();
    let request = surface
        .layout_request()
        .expect("a pinned box has a request");

    assert_eq!(request.width, Some(Length::Px(40.0)), "40 wide");
    assert_eq!(request.height, Some(Length::Px(40.0)), "and 40 tall");
    assert_eq!(
        request.main_align,
        Some(Align::Center),
        "v1's `justify_center`"
    );
    assert_eq!(
        request.cross_align,
        Some(Align::Center),
        "v1's `items_center`"
    );
}

/// The control for the centring: the icon is smaller than the box it is centred
/// in, so centring is visible at all. Equal sizes would make it a no-op.
#[test]
fn the_icon_is_smaller_than_the_box_it_is_centred_in() {
    let s = style();
    assert_eq!(s.size, 40.0);
    assert_eq!(s.icon_size, 24.0);
    assert!(
        s.icon_size < s.size,
        "an icon the same size as its box would make the centring invisible"
    );
}

// --- the variants ---------------------------------------------------------

#[test]
fn each_variant_has_its_own_paint() {
    let s = style();
    assert_eq!(
        s.box_colors(IconButtonVariant::Contained, false),
        (Color::rgb(0, 0, 0xFF), Color::rgb(0, 0, 0xFF)),
        "contained: filled *and* outlined in the primary"
    );
    assert_eq!(
        s.box_colors(IconButtonVariant::Outlined, false),
        (Color::rgba(0, 0, 0, 0), Color::rgb(0xFF, 0, 0)),
        "outlined: no fill, a visible outline"
    );
    assert_eq!(
        s.box_colors(IconButtonVariant::Text, false),
        (Color::rgba(0, 0, 0, 0), Color::rgba(0, 0, 0, 0)),
        "text: neither"
    );

    let inks = [
        s.paint(IconButtonVariant::Text).ripple_ink,
        s.paint(IconButtonVariant::Contained).ripple_ink,
        s.paint(IconButtonVariant::Outlined).ripple_ink,
    ];
    assert!(
        inks[0] != inks[1] && inks[1] != inks[2] && inks[0] != inks[2],
        "v1 gives each variant its own ink; three identical inks would hide a \\
         variant read that always returned the same arm"
    );
}

/// The ink reaches the ripple, which is what clips it. Read off the built ripple
/// rather than the style, because the style only says what the kit *resolved*.
#[test]
fn the_variant_reaches_the_ripples_ink() {
    for (variant, expected) in [
        (IconButtonVariant::Contained, Color::rgba(0, 0xFF, 0, 24)),
        (IconButtonVariant::Outlined, Color::rgba(0, 0, 0xBB, 24)),
        (IconButtonVariant::Text, Color::rgba(0xAA, 0, 0, 24)),
    ] {
        let button = button().variant(variant);
        let ripple = root(&button);
        let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
        assert_eq!(
            ripple.ink_color(),
            expected,
            "{variant:?}: the ink that variant resolves to"
        );
        assert_eq!(
            ripple.clip_radius(),
            f64::from(style().radius),
            "{variant:?}: and the ink is clipped to the box's own radius"
        );
    }
}

// --- disabled -------------------------------------------------------------

/// v1 replaced the variant's fill and outline outright rather than blending, so a
/// disabled *outlined* button does not keep the outline colour. The difference is
/// visible, which is why this is a rule and not an accident.
#[test]
fn disabled_replaces_the_variants_paint_rather_than_blending() {
    let s = style();
    for variant in [
        IconButtonVariant::Text,
        IconButtonVariant::Contained,
        IconButtonVariant::Outlined,
    ] {
        let (background, border) = s.box_colors(variant, true);
        assert_eq!(
            (background, border),
            (s.disabled_background, s.disabled_border),
            "{variant:?}: the disabled values, not the variant's"
        );
        assert_eq!(background, Color::rgba(0x11, 0x22, 0x33, 12), "fill at 12");
        assert_eq!(border, Color::rgba(0x11, 0x22, 0x33, 38), "outline at 38");
    }
}

/// The icon's colour follows `disabled`, and the variant does not touch it. Wired
/// to the variant instead, a disabled contained button would show a dark glyph on
/// a dark fill.
#[test]
fn the_icon_colour_follows_disabled_and_ignores_the_variant() {
    let s = style();
    for variant in [
        IconButtonVariant::Text,
        IconButtonVariant::Contained,
        IconButtonVariant::Outlined,
    ] {
        assert_eq!(
            s.icon_color(false),
            s.foreground,
            "{variant:?}: enabled takes the foreground"
        );
        assert_eq!(
            s.icon_color(true),
            s.disabled_foreground,
            "{variant:?}: disabled takes the disabled foreground, whatever the variant"
        );
    }
    assert_ne!(s.foreground, s.disabled_foreground, "the two differ");
}

/// The reason [`IconButtonIcon`] is a node and not a built value.
///
/// A slot is replayed once per key, so a subtree rebuilt from a builder keeps the
/// icon it first built. Had the icon been built inside the builder, toggling
/// `disabled` would change the box and leave the glyph stale — and a test that
/// only looked at the box would still have passed.
#[test]
fn the_icon_is_a_node_so_a_disabled_toggle_can_restyle_it() {
    let enabled = icon_node(&button());
    let disabled = icon_node(&button().disabled(true));

    assert_eq!(
        enabled
            .as_any()
            .downcast_ref::<IconButtonIcon>()
            .unwrap()
            .pending_color(),
        Some(style().foreground),
        "an enabled button asks its icon for the foreground"
    );
    assert_eq!(
        disabled
            .as_any()
            .downcast_ref::<IconButtonIcon>()
            .unwrap()
            .pending_color(),
        Some(style().disabled_foreground),
        "and a disabled one for the disabled foreground"
    );

    assert_eq!(
        drive_update(&enabled).fill(),
        style().foreground,
        "CONTROL: after one update the glyph really is the foreground"
    );
    assert_eq!(
        drive_update(&disabled).fill(),
        style().disabled_foreground,
        "and a disabled button's glyph is the disabled colour — not the one the \\
         kit handed in, which is what a build-time icon would still be showing"
    );
}

/// The control for the assertion above: the icon the *kit* built is none of the
/// three colours in play, so the update is demonstrably what changes it.
#[test]
fn the_icon_the_kit_built_is_neither_resolved_colour() {
    let as_built = icon().fill();
    assert_ne!(
        as_built,
        style().foreground,
        "the kit's icon colour is not the style's foreground"
    );
    assert_ne!(
        as_built,
        style().disabled_foreground,
        "nor its disabled foreground — so a build-time icon would keep painting \
         a colour the style never resolved, on both paths"
    );
    assert_eq!(
        as_built,
        Color::rgb(0x99, 0x99, 0x99),
        "CONTROL: it is the colour the kit actually built with, which is the point"
    );
}

/// Run one `update` over the icon node, the way the host does each frame, and read
/// the icon back.
fn drive_update(node: &Box<dyn WidgetAny>) -> VectorIcon {
    let mut owned = node.as_any().downcast_ref::<IconButtonIcon>().unwrap();
    // A `WidgetAny` is only reachable immutably through `children()[0].build()`'s
    // box, so the update runs through a mutable borrow of a fresh clone of the
    // node's own data. `IconButtonIcon` is `Clone` precisely so a host can hold a
    // second handle; here it lets the test drive one frame.
    let mut handle = owned.clone();
    let id = handle.create(&mut AllocatingCreateContext::new());
    handle.update(&mut RecordingUpdateContext::new(), id);
    handle.icon().clone()
}

// --- painting -------------------------------------------------------------

/// The control group: the surface really paints a filled, rounded box.
#[test]
fn the_box_paints() {
    let surface = surface_of(&button().variant(IconButtonVariant::Contained));
    let surface = surface.as_any().downcast_ref::<Surface>().unwrap().clone();
    let id = surface.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&surface, id, BOX);
    assert!(painted.contains("RoundedRect"), "a rounded box: {painted}");
    assert!(
        painted.contains("#0000FFFF"),
        "filled with the contained variant's blue: {painted}"
    );
}

/// The icon is painted by the node under the surface, and it is the only thing
/// there that paints a glyph — so a missing icon is a missing glyph, not a
/// missing box.
#[test]
fn the_icon_paints_inside_the_box() {
    let node = icon_node(&button());
    let id = node.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(node.as_ref(), id, BOX);
    assert!(!painted.is_empty(), "the glyph paints: {painted}");
    assert!(
        !painted.contains("RoundedRect"),
        "and what it paints is the glyph, not another box: {painted}"
    );
}

// --- semantics ------------------------------------------------------------

/// An addition, not a translation: v1's icon button was a styled box with an
/// action and no accessibility story, so a screen reader could neither name it nor
/// learn that it was disabled. `Pressable` publishes the role; the icon carries no
/// label, so it stays decorative and the button is the thing being named.
#[test]
fn the_button_publishes_its_role_and_the_icon_stays_decorative() {
    let ripple = root(&button());
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let pressable = child_of::<Pressable>(ripple);
    let pressable = pressable.as_any().downcast_ref::<Pressable>().unwrap();

    let hint = pressable.semantics_hint();
    assert_eq!(
        hint.role,
        SemanticsRole::Button,
        "an icon button is a button to a screen reader"
    );
    assert!(
        !hint.state.contains(SemanticsState::DISABLED),
        "and this one is enabled: {:?}",
        hint.state
    );

    let icon_hint = icon().semantics_hint();
    assert_eq!(
        icon_hint.role,
        SemanticsRole::None,
        "the glyph itself stays decorative — a button is named by its label, not \
         by an unlabelled image: {icon_hint:?}"
    );
}

/// The same addition, on the disabled button: `Pressable` reports `DISABLED`, which
/// v1 could not do at all.
#[test]
fn a_disabled_button_reports_itself_as_disabled() {
    let ripple = root(&button().disabled(true));
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let pressable = child_of::<Pressable>(ripple);
    let pressable = pressable.as_any().downcast_ref::<Pressable>().unwrap();
    assert!(
        pressable
            .semantics_hint()
            .state
            .contains(SemanticsState::DISABLED),
        "a screen reader is told the button is disabled"
    );
}

/// A button with no callback is still a button: the role is published whether or
/// not anything is wired to it. v1 could say neither.
#[test]
fn a_button_with_no_callback_still_publishes_its_role() {
    let ripple = root(&button());
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let pressable = child_of::<Pressable>(ripple);
    let pressable = pressable.as_any().downcast_ref::<Pressable>().unwrap();
    assert_eq!(
        pressable.semantics_hint().role,
        SemanticsRole::Button,
        "the role is not conditional on the callback"
    );
}

// --- clicking -------------------------------------------------------------

fn click(pressable: &mut Pressable, id: floom_core::ElementId) -> bool {
    let at = Point::new(20.0, 20.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    let mut consumed = pressable.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    consumed |= pressable.event(&mut up, id);
    consumed
}

#[test]
fn an_enabled_button_fires_its_callback() {
    let hits = Arc::new(AtomicUsize::new(0));
    let button = button().on_click({
        let hits = hits.clone();
        move || {
            hits.fetch_add(1, Ordering::SeqCst);
        }
    });
    let ripple = root(&button);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let mut pressable = child_of::<Pressable>(ripple);
    let pressable_mut = pressable.as_any_mut().downcast_mut::<Pressable>().unwrap();

    let id = pressable_mut.create(&mut AllocatingCreateContext::new());
    assert!(click(pressable_mut, id), "the press is consumed");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "CONTROL: an enabled button's callback ran"
    );
}

/// v1 wrote `if !disabled { .. }` inside the action closure, which guarded the
/// pointer but not the keyboard. 2.0 puts the gate in `Pressable`, which guards
/// both — so this is an improvement rather than a translation, and the test is
/// what keeps it true.
#[test]
fn a_disabled_button_is_inert() {
    let hits = Arc::new(AtomicUsize::new(0));
    let button = button().disabled(true).on_click({
        let hits = hits.clone();
        move || {
            hits.fetch_add(1, Ordering::SeqCst);
        }
    });
    let ripple = root(&button);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let mut pressable = child_of::<Pressable>(ripple);
    let pressable_mut = pressable.as_any_mut().downcast_mut::<Pressable>().unwrap();

    let id = pressable_mut.create(&mut AllocatingCreateContext::new());
    let consumed = click(pressable_mut, id);
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "a disabled button does not fire"
    );
    assert!(
        !consumed,
        "and does not consume the press, so a parent still sees it"
    );
}
