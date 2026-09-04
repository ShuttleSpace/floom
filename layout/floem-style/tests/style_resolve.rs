use floem_style::{StyleMap, StyleProps, StyleSet, StyleState};
use floem_types::Color;

#[test]
fn state_precedence_pressed_over_hover() {
    let mut set = StyleSet::new();
    set.set(
        StyleState::NORMAL,
        StyleProps {
            background: Some(Color::rgba(0, 0, 0, 255)),
            ..Default::default()
        },
    );
    set.set(
        StyleState::HOVER,
        StyleProps {
            background: Some(Color::rgba(1, 0, 0, 255)),
            ..Default::default()
        },
    );
    set.set(
        StyleState::PRESSED,
        StyleProps {
            background: Some(Color::rgba(2, 0, 0, 255)),
            ..Default::default()
        },
    );

    let r = set.resolve(StyleState::HOVER | StyleState::PRESSED);
    assert_eq!(r.background, Some(Color::rgba(2, 0, 0, 255)));
}

#[test]
fn typography_inherits_only() {
    let parent = StyleProps {
        font_size: Some(18.0),
        foreground: Some(Color::rgba(10, 10, 10, 255)),
        background: Some(Color::rgba(255, 255, 255, 255)),
        padding: Some(8.0),
        ..Default::default()
    };
    let mut child = StyleProps::default();
    child.inherit_typography(&parent);
    assert_eq!(child.font_size, Some(18.0));
    assert_eq!(child.foreground, Some(Color::rgba(10, 10, 10, 255)));
    assert!(child.background.is_none());
    assert!(child.padding.is_none());
}

#[test]
fn all_states_resolvable() {
    let mut set = StyleSet::new();
    for state in [
        StyleState::NORMAL,
        StyleState::HOVER,
        StyleState::PRESSED,
        StyleState::FOCUSED,
        StyleState::DISABLED,
        StyleState::SELECTED,
    ] {
        set.set(
            state,
            StyleProps {
                opacity: Some(state.bits() as f32 / 64.0),
                ..Default::default()
            },
        );
    }
    assert!(set.resolve(StyleState::DISABLED).opacity.is_some());
    assert!(set.resolve(StyleState::FOCUSED).opacity.is_some());
    assert!(set.resolve(StyleState::SELECTED).opacity.is_some());
}

#[test]
fn style_map_invalidation_counter() {
    let mut map = StyleMap::default();
    map.insert(1, StyleSet::new());
    map.invalidate(1);
    assert_eq!(map.invalidate_count, 2);
}
