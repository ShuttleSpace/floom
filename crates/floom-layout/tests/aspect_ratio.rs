use floom_layout::{AlignItems, LayoutEngine, LayoutStyle};
use floom_types::{Length, Size};

/// A box that states a ratio rather than a height gets the height the ratio
/// implies.
///
/// This is the capability `LayoutRequest::aspect_ratio` was added for: a
/// responsive media box cannot say "width 200, height 112.5" without knowing
/// its width first, and a fixed height is wrong at every width but one.
///
/// The root does not stretch its child, because a stretched child is given the
/// parent's cross-axis extent and the ratio is moot.
#[test]
fn an_aspect_ratio_derives_the_other_dimension() {
    let mut eng = LayoutEngine::new();
    let media = eng.new_leaf(LayoutStyle {
        width: Some(Length::Pct(100.0)),
        aspect_ratio: Some(16.0 / 9.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            align_items: AlignItems::Start,
            width: Some(Length::Px(320.0)),
            height: Some(Length::Px(320.0)),
            ..Default::default()
        },
        &[media],
    );

    eng.compute(root, Size::new(320.0, 320.0));
    let rect = eng.layout(media).rect;
    assert!(
        (rect.width() - 320.0).abs() < 0.01,
        "the width is as stated"
    );
    assert!(
        (rect.height() - 180.0).abs() < 0.01,
        "and the height follows from the ratio: {:?}",
        rect
    );
}

/// Without a ratio the box keeps whatever height it was given -- the capability
/// is additive, not a change of the default.
#[test]
fn without_a_ratio_the_height_is_what_it_said() {
    let mut eng = LayoutEngine::new();
    let box_ = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(100.0)),
        height: Some(Length::Px(40.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            align_items: AlignItems::Start,
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(200.0)),
            ..Default::default()
        },
        &[box_],
    );

    eng.compute(root, Size::new(200.0, 200.0));
    let rect = eng.layout(box_).rect;
    assert!((rect.height() - 40.0).abs() < 0.01);
}
