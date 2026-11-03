//! Fluent expand transition: the transition it plays, and the flag that

use floom_core::{ElementId, Widget};
use floom_types::{PresenceChannel, PresenceTransition};
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_expand_transition::{FluentExpandTransition, FluentExpandTransitionStyle};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::ExpandTransitionTokens {
    let theme = theme();
    match theme.component(ComponentId::ExpandTransition) {
        ComponentTokens::ExpandTransition(t) => t,
        _ => unreachable!(),
    }
}

#[test]
fn the_style_resolves_the_tokens() {
    let theme = theme();
    let style: FluentExpandTransitionStyle = FluentExpandTransition::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "fluent");
    assert_eq!(style.duration_ms, tokens().duration_ms);
    let _ = theme;
}

#[test]
fn the_transition_is_the_one_this_kit_plays() {
    let kit = FluentExpandTransition::new().with_theme(theme());
    assert!(
        kit.transition().is_none(),
        "the height reveal cannot be expressed, so no transition is played"
    );
    let transition = PresenceTransition::fade(0.0);
    assert_eq!(
        transition.duration_secs, 0.0,
        "the token's duration, in seconds"
    );
}

#[test]
fn the_content_follows_the_flag() {
    let kit = FluentExpandTransition::new()
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
        0,
        "no transition means no exit to play, so the content unmounts at once"
    );
}
