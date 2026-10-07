//! The ported Cupertino list item.
//!
//! The engine suite (`floom-widgets/tests/list_item.rs`) owns the *shape*; this
//! suite owns the *translation*, which is all that is left in this crate. So each
//! assertion is one v1 read: the token or literal it came from, and where it went.

use floom_core::Widget;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_types::FontWeight;
use floom_widgets::{Pressable, Ripple, Surface};

use floom_widget_cupertino_list_item::{CupertinoListItem, to_engine_weight};

/// Activate Cupertino and hand back a theme that will read it.
///
/// `MultiTheme::new_for(ds)` does **not** select `ds` — it reads a process-global at
/// every access (the porting plan's correction 9) — so a test that wants Cupertino's
/// numbers has to say so.
fn cupertino() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn item() -> CupertinoListItem {
    let _ = cupertino();
    CupertinoListItem::new().title("Wi-Fi")
}

fn list_tokens(theme: &MultiTheme) -> floom_core_theme::component_tokens::ListItemTokens {
    match theme.component(ComponentId::ListItem) {
        ComponentTokens::ListItem(t) => t,
        _ => unreachable!("ComponentId::ListItem resolves to ListItemTokens"),
    }
}

// --- the tokens reach the engine ------------------------------------------

/// Cupertino is the kit that **does not** read its own token table for three of the five
/// numbers. v1 hard-coded them, and this test pins the hard-coded values *and* the
/// disagreement with `ListItemTokens` — because a port that "helpfully" routed all
/// three through the tokens would change Cupertino's rows from 44.0 to 56.0 tall, which
/// is a design change wearing a refactor's clothes.
///
/// So this is the one assertion in the whole port that must **fail** if someone
/// makes the three kits consistent, and the engine widget's doc comment says why
/// that would be wrong.
#[test]
fn the_hard_coded_numbers_are_preserved_and_disagree_with_the_tokens() {
    let theme = cupertino();
    let tokens = list_tokens(&theme);
    let style = item().resolved_style();

    assert_eq!(style.min_height, 44.0, "v1's literal row floor");
    assert_eq!(style.title_size, 17.0, "v1's literal title size");
    assert_eq!(style.subtitle_size, Some(15.0), "v1's literal subtitle size");

    assert_eq!(
        tokens.min_height, 56.0,
        "CONTROL: `ListItemTokens::default()` says 56.0 for every kit"
    );
    assert_ne!(
        style.min_height, tokens.min_height,
        "and Cupertino ignores it — which is the whole point of this test"
    );
    assert_ne!(style.title_size, tokens.title_font_size);
    assert_ne!(style.subtitle_size, Some(tokens.subtitle_font_size));

    // The two this kit *does* read are still read, so the change above did not turn
    // into "stop reading tokens at all".
    assert_eq!(
        style.padding_horizontal,
        f64::from(tokens.padding_horizontal),
        "the horizontal padding still comes from the token"
    );
    assert_eq!(style.gap, tokens.gap, "and so does the gap");
}

/// v1 wrote `.gap(4.0)` at the call site rather than reading a token, and the token
/// table has no field for it. Kept as a literal and made visible.
#[test]
fn the_column_gap_is_the_literal_v1_wrote() {
    assert_eq!(
        item().resolved_style().text_gap,
        4.0,
        "v1's `.gap(4.0)`, not a token — so this is not covered by the test above"
    );
}

/// The title's weight comes from the **typography** scale in v1, not from the list
/// item's tokens, and it is the one field the engine cannot resolve for a kit.
#[test]
fn the_title_weight_comes_from_the_typography_scale() {
    let theme = cupertino();
    let style = item().resolved_style();
    let expected = theme.typography().title.medium.font_weight;

    assert_eq!(
        style.title_weight,
        to_engine_weight(expected),
        "v1's `typography().title.medium.font_weight`"
    );
    assert_eq!(
        style.title_weight,
        to_engine_weight(expected),
        "and the mounted row carries the same weight, through the crate's own \
         mapping rather than v1's `to_floom()`"
    );
}

// --- the subtitle ---------------------------------------------------------

/// A subtitle is an `Option`, and the props' own `Option<String>` is what decides.
#[test]
fn no_subtitle_leaves_the_engine_field_empty() {
    // The kit's own number, not the token's: this kit hard-codes it, which is what
    // `the_hard_coded_numbers_are_preserved_and_disagree_with_the_tokens` is about.
    assert_eq!(item().resolved_style().subtitle_size, Some(15.0));
    assert!(!item().view().has_subtitle(), "and the row has one line");

    let with = item().subtitle("Home network").view();
    assert!(with.has_subtitle(), "a subtitle makes two");
}

// --- the assembly ---------------------------------------------------------

/// The composition, read through the kit: `Ripple(Pressable(Surface(row)))`.
#[test]
fn the_kit_hands_over_a_pressable_band() {
    let view = item().view();
    let children = view.children();
    assert_eq!(children.len(), 1);

    let ripple = children[0].build();
    assert!(ripple.as_any().is::<Ripple>(), "the ripple is outermost");
    let pressable = ripple
        .as_any()
        .downcast_ref::<Ripple>()
        .unwrap()
        .children()[0]
        .build();
    assert!(
        pressable.as_any().is::<Pressable>(),
        "the pressable is the click target"
    );
    let surface = pressable
        .as_any()
        .downcast_ref::<Pressable>()
        .unwrap()
        .children()[0]
        .build();
    assert!(surface.as_any().is::<Surface>(), "and the surface is the band");
}

/// The title travels into the row, and the subtitle is optional.
#[test]
fn the_title_and_subtitle_travel_into_the_row() {
    assert_eq!(item().view().title(), "Wi-Fi");
    assert_eq!(item().subtitle("Home").view().title(), "Wi-Fi");
}

// --- disabled ------------------------------------------------------------

#[test]
fn disabled_reaches_the_row_and_its_ripple() {
    let view = item().disabled(true).view();
    assert!(view.is_disabled(), "the row knows");

    let ripple = view.children()[0].build();
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    assert!(ripple.is_disabled(), "and v1's `.disabled(disabled)` reached the ink");
}

// --- the props -----------------------------------------------------------

/// `nav` is stored and never read — v1 read it nowhere in `view()`.
#[test]
fn nav_is_carried_but_unread() {
    let item = item().nav(true);
    assert!(item.props().nav, "stored on the props, as v1 stored it");
    assert!(
        item.view().children().len() == 1,
        "and the row is the same shape either way — which is the whole of v1's \\
         behaviour for this prop"
    );
}

/// The click callback is stored twice — a v1 `Box` for the props and an
/// `Arc<dyn Fn() + Send + Sync>` for the engine — and both come from one closure.
#[test]
fn the_props_report_the_click_callback() {
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let item = {
        let hits = hits.clone();
        item().on_click(move || {
            hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
    };
    assert!(
        item.props().on_click.is_some(),
        "the v1-shaped copy is set, so `props()` reports what v1 reported"
    );

    // And the engine's copy fires: click the pressable.
    let view = item.view();
    let ripple = view.children()[0].build();
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let mut pressable = ripple.children()[0].build();
    let pressable = pressable.as_any_mut().downcast_mut::<Pressable>().unwrap();

    use floom_types::{Point, PointerEvent, Rect};
    let bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 56.0,
    };
    let id = pressable.create(&mut floom_widgets::testkit::AllocatingCreateContext::new());
    let at = Point::new(150.0, 28.0);
    let mut down = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_down(at).into(),
    );
    pressable.event(&mut down, id);
    let mut up = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_up(at).into(),
    );
    pressable.event(&mut up, id);

    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "CONTROL: the engine's copy fired"
    );
}

/// The slots are optional builders, so a caller with nothing to put there does not
/// pass one at all — and the row is one child either way.
#[test]
fn the_slots_default_to_absent() {
    assert_eq!(item().view().children().len(), 1);

    let with_leading = item().leading(|| Box::new(floom_widgets::Text::new("L")));
    assert_eq!(with_leading.view().children().len(), 1, "and still assembles");
}

/// Design-system colour -> engine colour, the widening the crate does.
fn to_paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
