use floom_app::{NodeId, WidgetHost};
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_ui_composites::{NavHandle, NavHost};
use floom_widgets::Text;

fn host_with(nav: NavHandle) -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 320.0));
    let nav_host = NavHost::new(nav, |page| {
        // A distinct widget per page: its text is the page path, which lets us
        // find nodes and check identity across navigation.
        Box::new(Text::new(page.path.clone())) as Box<dyn floom_core::WidgetAny>
    });
    let root = host.mount_root(LayoutStyle::default(), nav_host);
    host.frame();
    (host, root)
}

fn find_text(host: &WidgetHost, label: &str) -> NodeId {
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if let Some(text) = host.widget::<Text>(node) {
            if text.content() == label {
                return node;
            }
        }
    }
    panic!("no text labelled {label:?}");
}

#[test]
fn push_mounts_exactly_one_new_page() {
    let nav = NavHandle::new("home");
    let (mut host, _root) = host_with(nav.clone());

    let base = host.last_stats().painted;
    assert_eq!(base, 2, "NavHost + root page");

    nav.push("detail");
    host.frame();
    assert_eq!(
        host.last_stats().painted,
        base + 1,
        "a push mounts one page widget, reusing the rest"
    );
}

#[test]
fn page_widget_is_reused_not_remounted_across_a_push() {
    let nav = NavHandle::new("home");
    let (mut host, _root) = host_with(nav.clone());

    let home_before = find_text(&host, "home");
    nav.push("detail");
    host.frame();

    let home_after = find_text(&host, "home");
    assert_eq!(
        home_before, home_after,
        "the root page keeps its NodeId across a sibling push"
    );
    assert!(host.widget::<Text>(home_before).is_some());
}

#[test]
fn pop_keeps_the_page_mounted_until_finish_exit() {
    let nav = NavHandle::new("home");
    let (mut host, _root) = host_with(nav.clone());
    nav.push("detail");
    host.frame();
    let with_detail = host.last_stats().painted;
    assert_eq!(with_detail, 3);

    assert!(nav.pop());
    host.frame();
    assert_eq!(
        host.last_stats().painted,
        with_detail,
        "the popped page stays mounted for its exit transition"
    );
    assert!(nav.top().path == "home");

    nav.finish_exit();
    host.frame();
    assert_eq!(
        host.last_stats().painted,
        with_detail - 1,
        "finish_exit unmounts the exiting page"
    );
}

#[test]
fn pop_to_root_then_finish_leaves_only_the_root() {
    let nav = NavHandle::new("home");
    let (mut host, _root) = host_with(nav.clone());
    nav.push("a");
    nav.push("b");
    host.frame();
    assert_eq!(host.last_stats().painted, 4, "NavHost + three pages");

    assert_eq!(nav.pop_to_root(), 2);
    nav.finish_exit();
    host.frame();
    assert_eq!(host.last_stats().painted, 2, "NavHost + home");
}
