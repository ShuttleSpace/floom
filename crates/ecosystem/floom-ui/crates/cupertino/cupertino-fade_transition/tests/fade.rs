//! Cupertino fade transition: the transition it plays, and the flag that
//! drives it.

use floom_core::{ElementId, Widget};
use floom_types::{PresenceChannel, PresenceTransition};
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_fade_transition::{
    CupertinoFadeTransition, CupertinoFadeTransitionStyle,
};

/// The Cupertino theme, with Cupertino selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Cupertino's own values has to
/// select Cupertino, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn tokens() -> floom_core_theme::component_tokens::FadeTransitionTokens {
    let theme = theme();
    match theme.component(ComponentId::FadeTransition) {
        ComponentTokens::FadeTransition(t) => t,
        _ => unreachable!(),
    }
}

/// The style resolves the token's own numbers.
#[test]
fn the_style_resolves_the_tokens() {
    let theme = theme();
    let style: CupertinoFadeTransitionStyle = CupertinoFadeTransition::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "cupertino");
    assert_eq!(style.duration_ms, tokens().duration_ms);
    let _ = theme;
}

/// The transition is the one this kit plays.
#[test]
fn the_transition_is_the_one_this_kit_plays() {
    let kit = CupertinoFadeTransition::new().with_theme(theme());
    let transition = kit.transition().expect("this kit plays a transition");
    assert_eq!(
        transition.channel,
        PresenceChannel::Fade,
        "v1's fade was an opacity ramp"
    );
    assert_eq!(
        transition.duration_secs,
        f64::from(tokens().duration_ms) / 1000.0,
        "the token's duration, in seconds"
    );
}

/// The content is mounted only while the flag says so.
#[test]
fn the_content_follows_the_flag() {
    let kit = CupertinoFadeTransition::new()
        .with_theme(theme())
        .child(|| Box::new(floom_widgets::Text::new("body")));
    let flag = kit.show_signal();

    floom_widgets::text_input::write_flag(&flag, true);
    let mut conditional = kit.view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = conditional.create(&mut create);
    let mut ctx = RecordingUpdateContext::default();
    conditional.update(&mut ctx, id);
    assert_eq!(
        conditional.children().len(),
        1,
        "the flag is true, so the content is mounted"
    );

    // The flag goes back to false: the *exit* starts, and the content stays
    // mounted while it plays -- a departure that vanished at once would not be
    floom_widgets::text_input::write_flag(&flag, false);
    conditional.update(&mut ctx, id);
    assert_eq!(
        conditional.children().len(),
        1,
        "the content stays mounted while its exit plays"
    );

    // And it is gone once the host says the exit has finished.
    conditional.animation_finished(&mut ctx, id);
    conditional.update(&mut ctx, id);
    assert_eq!(
        conditional.children().len(),
        0,
        "and it unmounts once the exit has played"
    );
}
