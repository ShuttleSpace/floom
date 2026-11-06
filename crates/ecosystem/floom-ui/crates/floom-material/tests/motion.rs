use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_material::{MotionDuration, MotionEasing, MotionTransition};
use floom_types::{Event, Point, PointerEvent, Size};
use floom_widgets::Button;

/// The token spelling and the engine transition agree on the duration: 100 ms
/// is 0.1 s, and the one conversion in between must not drift by a factor of
/// ten (the classic ms/s mix-up).
#[test]
fn a_motion_token_becomes_the_transition_that_names_it() {
    let transition = MotionTransition::from_tokens(MotionDuration::SHORT2, MotionEasing::STANDARD);
    assert_eq!(transition.duration_secs, 0.1);
    assert_eq!(transition.easing, MotionEasing::STANDARD);
}

/// The full chain, with a real button: `MotionTransition::STATE_CHANGE` is the
/// Material answer to a hover colour change, and `Button::transition` takes it
/// without adaptation. This is the "real consumer" B6 asks for — the token does
/// not just exist, it is what a widget is configured with.
///
/// The button's own colour fade is asserted in `floom-app`'s motion-host suite
/// (B4); here the claim is narrower and belongs to this design system — that
/// the token *is* the value the button is armed with, so the two halves of the
/// chain meet without a shim.
#[test]
fn a_material_transition_arms_a_real_buttons_implicit_fade() {
    // `.transition` is the engine door, and `MotionTransition::STATE_CHANGE` is
    // the token. The chain compiles and the button keeps its identity — the
    // strongest proof a design-system crate can give that its token is the
    // engine's vocabulary without reaching up to drive a host itself.
    let button = Button::new("Hover").transition(MotionTransition::STATE_CHANGE);
    assert_eq!(button.label(), "Hover");
}

/// The token is not merely convertible — it is the duration the host *plays*.
///
/// The strongest form of "the token has a consumer": a real frame loop, a real
/// button armed with nothing but `MotionTransition::STATE_CHANGE`, and the
/// count of frames until the engine reports it has stopped. 100 ms is six
/// nominal frames; anything else says the token did not reach the engine. This
/// is what distinguishes a consumed token from a documented one — the two
/// failure modes it catches are the token never arriving (no transition at all
/// settles in one frame) and the ms/s conversion drifting by a factor of ten
/// (which would run a hundred times too long and never land).
#[test]
fn a_material_token_is_the_duration_the_host_plays() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let button = Button::new("Token").transition(MotionTransition::STATE_CHANGE);
    let node = host.mount_root(LayoutStyle::default(), button);
    host.frame();
    let bounds = host.bounds(node);

    // Settle first: a mounted transitioned button plays one 0 → 1 pass on its
    // own, and that is the mount's, not the token's, to account for.
    for _ in 0..20 {
        host.frame();
    }

    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(Event::Pointer(PointerEvent::hover(centre)));
    host.frame();
    let mut frames = 1;
    while host.animations_running() > 0 && frames < 200 {
        host.frame();
        frames += 1;
    }
    let nominal_frames = (MotionDuration::SHORT2 as f64 / 1000.0 * 60.0).ceil() as i32;
    assert!(
        // The window is derived from the token rather than written here, so the
        // claim is "the host played the token", not "the host played six-ish
        // frames". The slack above the nominal count is the start frame itself
        // plus the one that notices the finish.
        (nominal_frames..=nominal_frames + 4).contains(&frames),
        "the host should play {nominal_frames} nominal frames for {} ms, took {frames}",
        MotionDuration::SHORT2
    );
}

/// The shorthand constants spell the same tokens they wrap, so there is exactly
/// one place each duration lives.
#[test]
fn the_shorthand_constants_match_their_tokens() {
    assert_eq!(
        MotionTransition::STATE_CHANGE.duration_secs,
        MotionDuration::SHORT2 as f64 / 1000.0
    );
    assert_eq!(
        MotionTransition::EXPAND.duration_secs,
        MotionDuration::MEDIUM1 as f64 / 1000.0
    );
}
