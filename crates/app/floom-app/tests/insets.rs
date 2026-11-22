// SPDX-License-Identifier: MIT
//! Area the OS owns must push content in, not get painted over.
//!
//! The insets arrive from the host (`floom_ios_set_safe_area_insets` on iOS).
//! Applying them as root padding keeps the app's backgrounds edge to edge while
//! its widgets stay clear of the notch and the home indicator.

use floom_accessibility::SemanticsRole;
use floom_app::WidgetHost;
use floom_core::{Axis, Child, ChildKey, LayoutRequest, Length};
use floom_layout::LayoutStyle;
use floom_types::{Insets, Size};
use floom_widgets::compose::Stack;
use floom_widgets::controls::Button;

fn mounted(insets: Insets) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 400.0));
    host.set_content_insets(insets);
    host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(400.0)),
            ..LayoutStyle::default()
        },
        Stack::new(Axis::Vertical).child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Button::new("Save")),
        )),
    );
    host.frame();
    host
}

#[test]
fn a_safe_area_moves_content_clear_of_the_system_chrome() {
    let host = mounted(Insets {
        top: 44.0,
        right: 0.0,
        bottom: 34.0,
        left: 8.0,
    });
    let (_, node) = host
        .semantics()
        .iter()
        .find(|(_, node)| node.role == SemanticsRole::Button)
        .expect("the button publishes");
    assert_eq!(node.bounds.y0, 44.0, "content starts below the status bar");
    assert_eq!(node.bounds.x0, 8.0, "and inside the left inset");
}

#[test]
fn no_insets_leaves_the_root_at_the_origin() {
    let host = mounted(Insets::ZERO);
    let (_, node) = host
        .semantics()
        .iter()
        .find(|(_, node)| node.role == SemanticsRole::Button)
        .expect("the button publishes");
    assert_eq!(node.bounds.y0, 0.0);
    assert_eq!(node.bounds.x0, 0.0);
}
