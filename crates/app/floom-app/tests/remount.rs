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
