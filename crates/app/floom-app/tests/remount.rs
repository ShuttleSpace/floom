//! A rebuild reruns the parent's source; the widget is what remembers.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_widgets::Text;

fn host_with(text: &str) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    host.mount_root(LayoutStyle::default(), Text::new(text));
    host.frame();
    host
}

#[test]
fn a_rebuild_keeps_the_text_the_app_wrote() {
    let mut host = host_with("Count: 0");
    let root = host.node_at(0).expect("a mounted root");
    host.widget_mut::<Text>(root)
        .expect("the root is a label")
        .set_content("Count: 7");

    // The parent's source still says 0 -- that is exactly the case a rebuild has
    // to survive, because the source did not change, the code did.
    host.remount(|host| {
        host.mount_root(LayoutStyle::default(), Text::new("Count: 0"));
    });

    let root = host.node_at(0).expect("a mounted root");
    assert_eq!(
        host.widget_mut::<Text>(root)
            .expect("the root is a label")
            .content(),
        "Count: 7"
    );
}

#[test]
fn a_rebuild_asks_for_a_frame() {
    // Nothing carried, nothing drawn: the new tree has never been laid out, so
    // the next frame has to be requested by the host rather than by luck.
    let mut host = host_with("Count: 0");
    host.frame();
    host.remount(|host| {
        host.mount_root(LayoutStyle::default(), Text::new("Count: 0"));
    });
    assert!(host.needs_frame());
}

#[test]
fn a_rebuild_that_mounts_nothing_carries_nothing() {
    let mut host = host_with("Count: 0");
    host.remount(|_| {});
    assert_eq!(host.node_at(0).map(|_| ()), None);
}

/// The other half of a rebuild: the tree is not just re-bound, it is laid out
/// and published again at whatever the new source says. Without this, "hot
/// reload" would mean "state survives, layout does not".
#[test]
fn a_rebuild_that_changes_a_size_lays_out_at_the_new_size() {
    use floom_accessibility::SemanticsRole;
    use floom_core::{Axis, Child, ChildKey, LayoutRequest};
    use floom_widgets::{Button, Stack};

    fn button_width(host: &WidgetHost) -> f64 {
        let (id, _) = host
            .semantics()
            .iter()
            .find(|(_, node)| node.role == SemanticsRole::Button)
            .expect("the button publishes a node");
        let node = host.node_at(id.index() as usize).expect("the node exists");
        host.bounds(node).width()
    }

    let build = |width: f32| {
        move |host: &mut WidgetHost| {
            host.mount_root(
                LayoutStyle::default(),
                Stack::new(Axis::Vertical).child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width(width).height(40.0),
                    || Box::new(Button::new("Increment")),
                )),
            );
        }
    };

    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    build(100.0)(&mut host);
    host.frame();
    let first = button_width(&host);

    host.remount(build(220.0));
    host.frame();
    let second = button_width(&host);
    assert!(first > 0.0, "first build: {first}");
    assert!(
        (second - first).abs() > 100.0,
        "the rebuild has to change the laid-out box: {first} then {second}"
    );
}
