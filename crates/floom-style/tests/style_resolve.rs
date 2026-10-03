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

#[test]
fn shape_props_survive_state_resolution() {
    // A variant's identity lives in its shape as much as its colour: an
    // outlined button is `border_width > 0` and an elevated one is
    // `elevation > 0`, and those have to survive a state overlay.
    let mut set = StyleSet::new();
    set.set(
        StyleState::NORMAL,
        StyleProps {
            radius: Some(8.0),
            border_width: Some(1.0),
            border_color: Some(Color::rgba(1, 1, 1, 255)),
            elevation: Some(2.0),
            ..Default::default()
        },
    );
    set.set(
        StyleState::HOVER,
        StyleProps {
            border_color: Some(Color::rgba(2, 2, 2, 255)),
            ..Default::default()
        },
    );

    let normal = set.resolve(StyleState::NORMAL);
    assert_eq!(normal.border_width, Some(1.0));
    assert_eq!(normal.elevation, Some(2.0));

    let hover = set.resolve(StyleState::HOVER);
    assert_eq!(hover.border_color, Some(Color::rgba(2, 2, 2, 255)));
    // The overlay must not wipe the properties it does not mention.
    assert_eq!(hover.border_width, Some(1.0), "overlay erased a shape prop");
    assert_eq!(hover.radius, Some(8.0), "overlay erased the radius");
}

#[test]
fn shape_props_do_not_inherit() {
    // Only typography inherits. A child that inherits its parent's radius
    // would get a rounded input inside a square parent for no stated reason.
    let parent = StyleProps {
        radius: Some(12.0),
        border_width: Some(2.0),
        elevation: Some(4.0),
        font_size: Some(18.0),
        ..Default::default()
    };
    let mut child = StyleProps::default();
    child.inherit_typography(&parent);
    assert_eq!(child.font_size, Some(18.0));
    assert!(child.radius.is_none(), "radius must not inherit");
    assert!(child.border_width.is_none(), "border must not inherit");
    assert!(child.elevation.is_none(), "elevation must not inherit");
}

#[test]
fn an_undecorated_style_reports_no_shape() {
    // `None` means "unset", not "zero": a widget that is told nothing about its
    // shape keeps its own default rather than being flattened to a square.
    let mut set = StyleSet::new();
    set.set(
        StyleState::NORMAL,
        StyleProps {
            background: Some(Color::rgba(9, 9, 9, 255)),
            ..Default::default()
        },
    );
    let r = set.resolve(StyleState::NORMAL);
    assert!(r.radius.is_none());
    assert!(r.border_width.is_none());
    assert!(r.border_color.is_none());
    assert!(r.elevation.is_none());
}
