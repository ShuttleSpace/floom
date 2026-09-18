//! Phase 29.1 — the `floem2::prelude` surface is pinned by name.
//!
//! Every name a beginner is promised must resolve from a single glob import.
//! If a surface item is removed or renamed, this file stops compiling — that
//! is the stability contract. The *negative* surface (what must NOT be here:
//! the v1 reactive graph, the v1 `View` lifecycle, type-erasure internals) is
//! pinned by the compile-fail doctests in the crate docs.

use floem2::prelude::*;

// `AppRunner` is generic over the presenter backend (a concrete type only
// exists behind a renderer feature), so its surface presence is pinned as a
// *path*: if the re-export disappears, this import stops resolving.
#[allow(unused_imports)]
use floem2::prelude::AppRunner;

fn exists<T: ?Sized>() {}

#[test]
fn prelude_exports_the_semantic_widget_surface() {
    // Tier-0 primitives.
    exists::<Container>();
    exists::<Divider>();
    exists::<Icon>();
    exists::<Image>();
    exists::<ImageFit>();
    exists::<Spacer>();
    exists::<Text>();

    // Controls.
    exists::<Button>();
    exists::<Checkbox>();
    exists::<Progress>();
    exists::<Radio>();
    exists::<Select>();
    exists::<Slider>();
    exists::<Switch>();
    exists::<TextField>();
    exists::<InteractionState>();

    // Composites.
    exists::<Stack>();
    exists::<List<String>>();
    exists::<Grid<String>>();
    exists::<Tabs>();
    exists::<Conditional>();
    exists::<Dialog>();
    exists::<Overlay>();
    exists::<Popup>();
    exists::<BottomSheet>();

    // Navigation.
    exists::<NavHandle>();
    exists::<NavHost>();
    exists::<NavDestination>();
    exists::<NavigationBar>();
    exists::<NavigationDrawer>();
    exists::<NavigationRail>();

    // Labelled + native.
    exists::<Labeled>();
    exists::<NativeSurface>();
}

#[test]
fn prelude_exports_the_runtime_primitives() {
    // Geometry + layout values.
    exists::<Point>();
    exists::<Rect>();
    exists::<Size>();
    exists::<Align>();
    exists::<Axis>();
    exists::<LayoutStyle>();
    exists::<LayoutRequest>();

    // Tree composition: the builder vocabulary beginners need.
    exists::<Child>();
    exists::<ChildKey>();

    // Input + shell.
    exists::<Event>();
    exists::<PointerEvent>();
    exists::<ScrollEvent>();
    exists::<WidgetHost>();
    exists::<NodeId>();
    exists::<RunStats>();
    exists::<FrameStats>();
}

#[test]
fn advanced_surface_is_explicit_not_global() {
    // The advanced tier exists, but under `floem2::advanced`, never in the
    // prelude (asserted negatively by the compile-fail doctests).
    use floem2::advanced;

    // Type-erasure + authoring contracts (traits).
    exists::<dyn advanced::WidgetAny>();
    exists::<dyn advanced::Widget>();
    exists::<dyn advanced::CreateContext>();
    exists::<dyn advanced::UpdateContext>();
    exists::<dyn advanced::EventContext>();
    exists::<dyn advanced::PaintContext>();

    // Value types.
    exists::<advanced::ElementId>();
    exists::<advanced::ResourceId>();
    exists::<advanced::SemanticsRole>();
    exists::<advanced::SemanticsHint>();
    exists::<advanced::LayoutEngine>();
    exists::<advanced::MeasureFn>();

    // The accessibility bridge (functions).
    let _ = advanced::to_core_role;
    let _ = advanced::to_core_state;
}

#[test]
fn a_hello_app_mounts_and_paints_through_the_prelude_alone() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 100.0));

    let root = Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("hello")),
    ));
    let root = root.child(Child::new(ChildKey::index(1), LayoutRequest::new(), || {
        Box::new(Button::new("click me"))
    }));
    host.mount_root(LayoutStyle::default(), root);

    let stats = host.frame();
    assert!(stats.painted > 0, "the hello app must paint something");
}

#[test]
fn prelude_widgets_participate_in_input_routing() {
    // A beginner's first interaction: click a button through the prelude-only
    // surface and observe the callback fire exactly once.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    // `Child` builders are `Send + Sync` (they may replay on any thread), so
    // the shared counter is an `Arc<AtomicU32>` — while `on_click` is a plain
    // `Box<dyn Fn()>` and happily takes the `Arc` too.
    let clicks = Arc::new(AtomicU32::new(0));

    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 100.0));

    let counter = clicks.clone();
    let root = Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || {
            let counter = counter.clone();
            Box::new(Button::new("press").on_click(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }))
        },
    ));
    host.mount_root(LayoutStyle::default(), root);
    host.frame();

    // Find the button node through the host's public surface.
    let mut button_id = None;
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if host.widget::<Button>(node).is_some() {
            button_id = Some(node);
            break;
        }
    }
    let button_id = button_id.expect("mounted button");

    let bounds = host.bounds(button_id);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(PointerEvent::primary_down(centre).into());
    host.dispatch(PointerEvent::primary_up(centre).into());
    host.frame();

    assert_eq!(
        clicks.load(Ordering::SeqCst),
        1,
        "one press/release cycle must fire on_click exactly once"
    );

    // A release without a press must not fire the callback.
    host.dispatch(PointerEvent::primary_up(centre).into());
    host.frame();
    assert_eq!(
        clicks.load(Ordering::SeqCst),
        1,
        "a stray release is not a click"
    );
}
