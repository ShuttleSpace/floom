//! The ported Material parallax -- a layer that moves more slowly than the
//! page it sits on.
//!
//! The shape is a kit-local widget, because the effect needs a translate and
//! the engine has none in its layout vocabulary. So what is tested here is the
//! translation and the motion: the factor reaches the style, the layer
//! declares the translate v1 wrote, and a moved offset is a new declaration.
//!
//! ## The tripwires
//!
//! `the_layer_moves_the_content_by_the_scroll_times_the_factor` and
//! `a_moved_offset_re_declares_the_motion` would fail if this kit stopped
//! being this kit.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Widget};
use floom_types::CompositorProp;

use floom_widgets::testkit::RecordingUpdateContext;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_parallax::{
    MaterialParallax, MaterialParallaxStyle, read_scroll, shared_scroll, write_scroll,
};

const ID: ElementId = ElementId::new(1, 1);

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialParallax {
    MaterialParallax::new().with_theme(theme())
}

/// The parallax token's factor.
fn token_factor() -> f32 {
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::Parallax) {
            floom_core_theme::component_tokens::ComponentTokens::Parallax(tokens) => tokens,
            _ => unreachable!(),
        };
    tokens.factor
}

// --- the translation ------------------------------------------------------

/// The factor is the caller's when it is above zero.
#[test]
fn the_factor_is_the_callers_when_it_is_positive() {
    let style: MaterialParallaxStyle = kit().factor(0.75).resolved_style();
    assert_eq!(style.factor, 0.75, "the caller's factor");
}

/// A factor that is not positive falls back to the token.
#[test]
fn a_factor_that_is_not_positive_falls_back_to_the_token() {
    assert_eq!(
        kit().factor(0.0).resolved_style().factor,
        token_factor(),
        "the token's factor"
    );
    assert_eq!(
        kit().factor(-1.0).resolved_style().factor,
        token_factor(),
        "and a negative one, too"
    );
}

/// v1's props default already carries a positive factor, so the rule never
/// reaches the token.
#[test]
fn the_default_factor_never_reaches_the_token() {
    let kit = kit();
    assert_eq!(
        kit.resolved_style().factor,
        kit.props().factor,
        "the props' own default is the factor"
    );
    assert!(
        kit.props().factor > 0.0,
        "and it is positive, so the token's number is unreachable -- which is \\
         why it is named in the module note rather than relied on"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

// --- the motion -----------------------------------------------------------

/// The layer declares the translate v1 wrote: `-scroll * factor` on Y.
#[test]
fn the_layer_moves_the_content_by_the_scroll_times_the_factor() {
    let scroll = shared_scroll(100.0);
    let mut layer = kit()
        .factor(0.5)
        .scroll_signal(scroll.clone())
        .view();

    let mut cx = RecordingUpdateContext::new();
    layer.update(&mut cx, ID);
    assert_eq!(cx.dirty_count(), 1, "the first read is a change");

    let spec = layer.animation().expect("the layer declares its motion");
    assert_eq!(spec.channels.len(), 1, "one channel");
    match spec.channels[0].prop {
        CompositorProp::TranslateY(_) => {}
        other => panic!("the layer translates on Y, not {other:?}"),
    }
    assert_eq!(
        spec.channels[0].from, -50.0,
        "v1's `translate_y(-scroll * factor)`: -100 * 0.5"
    );
    assert_eq!(spec.channels[0].to, -50.0, "and it holds there");
}

/// A moved offset is a new declaration, so the host restarts the motion
/// rather than leaving the old one parked.
#[test]
fn a_moved_offset_re_declares_the_motion() {
    let scroll = shared_scroll(100.0);
    let mut layer = kit()
        .factor(0.5)
        .scroll_signal(scroll.clone())
        .view();

    let mut cx = RecordingUpdateContext::new();
    layer.update(&mut cx, ID);
    let before = layer.animation().expect("the layer declares its motion");

    write_scroll(&scroll, 200.0);
    let mut cx = RecordingUpdateContext::new();
    layer.update(&mut cx, ID);
    assert_eq!(cx.dirty_count(), 1, "a moved offset is a change");

    let after = layer.animation().expect("the layer declares its motion");
    assert_eq!(after.channels[0].from, -100.0, "-200 * 0.5");
    assert_ne!(
        after.key, before.key,
        "and it is a *different* declaration, which is what makes the host \\
         restart it instead of leaving the old value parked"
    );
}

/// An offset that did not move is not a change, so the frame is not asked to
/// repaint.
#[test]
fn an_offset_that_did_not_move_is_not_a_change() {
    let scroll = shared_scroll(100.0);
    let mut layer = kit().scroll_signal(scroll.clone()).view();

    let mut cx = RecordingUpdateContext::new();
    layer.update(&mut cx, ID);
    assert_eq!(cx.dirty_count(), 1, "the first read is a change");

    let mut cx = RecordingUpdateContext::new();
    layer.update(&mut cx, ID);
    assert_eq!(
        cx.dirty_count(),
        0,
        "and a settled offset asks for no repaint"
    );
}

/// The shared offset is the value the host writes.
#[test]
fn the_shared_offset_is_the_value_the_host_writes() {
    let kit = kit();
    let scroll = kit.scroll_offset();
    assert_eq!(read_scroll(&scroll), 0.0, "no scroll to begin with");
    assert!(write_scroll(&scroll, 42.0), "the host wrote it");
    assert_eq!(read_scroll(&scroll), 42.0);
    assert!(
        !write_scroll(&scroll, 42.0),
        "and writing the same number changes nothing"
    );
}

/// The content is mounted under the layer.
#[test]
fn the_content_is_mounted_under_the_layer() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("background"))
    });
    let layer = kit().child(child).view();
    assert_eq!(layer.children().len(), 1, "the content");
}

// --- the props ------------------------------------------------------------

/// The props report the factor.
#[test]
fn the_props_report_the_factor() {
    assert_eq!(kit().factor(0.4).props().factor, 0.4);
}

// --- the theme ------------------------------------------------------------

/// The theme: v1's Material parallax read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_eq!(
        MaterialParallax::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material parallax read the app-wide signal, not a snapshot"
    );
}
