
use floom_core::{ElementId, Widget};
use floom_types::{PresenceChannel, PresenceTransition};
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_scale_transition::{MaterialScaleTransition, MaterialScaleTransitionStyle};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn tokens() -> floom_core_theme::component_tokens::ScaleTransitionTokens {
    let theme = theme();
    match theme.component(ComponentId::ScaleTransition) {
        ComponentTokens::ScaleTransition(t) => t,
        _ => unreachable!(),
    }
}

/// The style resolves the token's own numbers.
#[test]
fn the_style_resolves_the_tokens() {
    let theme = theme();
    let style: MaterialScaleTransitionStyle = MaterialScaleTransition::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "material");
    assert_eq!(style.duration_ms, tokens().duration_ms);
    assert_eq!(style.min_scale, tokens().min_scale);
    let _ = theme;
}

/// The transition is the one this kit plays.
#[test]
fn the_transition_is_the_one_this_kit_plays() {
    let kit = MaterialScaleTransition::new().with_theme(theme());
    let transition = kit.transition().expect("this kit plays a transition");
    assert_eq!(
        transition.channel,
        PresenceChannel::Grow {
            from: tokens().min_scale
        },
        "v1's scale grew from `min_scale` to natural size"
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
    let kit = MaterialScaleTransition::new()
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

    floom_widgets::text_input::write_flag(&flag, false);
    conditional.update(&mut ctx, id);
    assert_eq!(
        conditional.children().len(),
        1,
        "the content stays mounted while its exit plays"
    );

    conditional.animation_finished(&mut ctx, id);
    conditional.update(&mut ctx, id);
    assert_eq!(
        conditional.children().len(),
        0,
        "and it unmounts once the exit has played"
    );
}
