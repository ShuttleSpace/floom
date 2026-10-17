//! Phase 29.1 — the `floom::prelude` surface is pinned by name.
//!
//! Every name a beginner is promised must resolve from a single glob import.
//! If a surface item is removed or renamed, this file stops compiling — that
//! is the stability contract. The *negative* surface (what must NOT be here:
//! the v1 reactive graph, the v1 `View` lifecycle, type-erasure internals) is
//! pinned by the compile-fail doctests in the crate docs.

use floom::prelude::*;

// `AppRunner` is generic over the presenter backend (a concrete type only
// exists behind a renderer feature), so its surface presence is pinned as a
// *path*: if the re-export disappears, this import stops resolving.
#[allow(unused_imports)]
use floom::prelude::AppRunner;

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
    exists::<Switch>();
    exists::<InteractionState>();

    // Composites.
    exists::<Stack>();
    exists::<List<String>>();
    exists::<Grid<String>>();
    exists::<Conditional>();
    exists::<Crossfade>();
    exists::<SharedTransitionLayout>();

    // Navigation and labelled-value composites are deliberately absent.
    // `Labeled`, `NavHandle`, `NavHost`, `NavDestination`, `NavigationBar`,
    // `NavigationDrawer` and `NavigationRail` were re-exported here while they
    // were `floom_widgets` modules. They moved up a layer into
    // `floom-ui-composites`, which lives in the separate `floom-ui` workspace,
    // and a shipped facade does not take a dependency on a satellite workspace
    // to keep a name it promised before the layering was right. See
    // `the_prelude_does_not_promise_the_composites` below, which is the
    // assertion; this comment is only there so the absence is not mistaken for
    // an oversight the next time somebody reads this list.
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
    // The advanced tier exists, but under `floom::advanced`, never in the
    // prelude (asserted negatively by the compile-fail doctests).
    use floom::advanced;

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

    // The declarative motion a widget returns from `Widget::animation` — only
    // nameable by someone implementing that contract, so it stays out of the
    // prelude with the rest of the authoring surface.
    exists::<advanced::AnimationSpec>();
    exists::<advanced::RepeatSpec>();

    // The accessibility bridge (functions).
    let _ = advanced::to_core_role;
    let _ = advanced::to_core_state;
}

/// The motion vocabulary is beginner surface, and it has to *work* through the
/// prelude alone.
///
/// `WidgetHost::animate` is a single call, which makes it easy to ship the
/// entry point and forget that the call takes arguments a beginner cannot
/// name: without the vocabulary in the prelude, "you can animate any node" is
/// true of the host and false of the documented surface. Naming the types is
/// the compile-time half; driving a node to its endpoint is the half that
/// catches a vocabulary that resolves but does not connect.
#[test]
fn the_prelude_can_animate_a_node_from_a_single_glob_import() {
    exists::<AnimKind>();
    exists::<Tween>();
    exists::<Easing>();
    exists::<RepeatMode>();
    exists::<CompositorProp>();
    exists::<AnimId>();

    let mut host = WidgetHost::new();
    host.set_size(Size::new(160.0, 40.0));
    let label = host.mount_root(LayoutStyle::default(), Text::new("fade"));
    host.frame();

    host.animate(
        label,
        AnimKind::Tween(
            Tween::new(1.0, 0.0, 0.2)
                .with_easing(Easing::EASE_OUT)
                .with_compositor(CompositorProp::Opacity(0.0)),
        ),
    );
    host.frame();
    assert_eq!(
        host.animations_running(),
        1,
        "the prelude's vocabulary has to drive the host, not just resolve"
    );

    for _ in 0..20 {
        host.frame();
    }
    assert_eq!(host.animations_running(), 0, "and it finishes");
    assert_eq!(
        host.animation_value(label),
        Some(0.0),
        "on the endpoint the prelude named"
    );
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

/// The sixteen names the prelude used to promise, as the negative half of the
/// stability contract above.
///
/// A *positive* assertion cannot express "this must not resolve": naming
/// `NavHost` in a type position would stop this file compiling, which is a
/// failure of the test suite rather than a statement about the prelude. So the
/// absence is checked where it is actually decidable — the facade's own
/// manifest and the source of the prelude module.
///
/// The manifest half matters as much as the source half. Re-exporting the name
/// requires the dependency, and the dependency is the thing that is wrong: it
/// would put the shipped L9 facade above a crate the root workspace `exclude`s.
#[test]
fn the_prelude_does_not_promise_the_composites() {
    const MOVED: [&str; 16] = [
        "Labeled",
        "NavHandle",
        "NavHost",
        "NavDestination",
        "NavigationBar",
        "NavigationDrawer",
        "NavigationRail",
        "Select",
        "Overlay",
        "Dialog",
        "Popup",
        "BottomSheet",
        "Tabs",
        "Slider",
        "Radio",
        "TextField",
    ];

    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let read = |name: &str| {
        std::fs::read_to_string(crate_dir.join(name))
            .unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
    };

    let manifest = read("Cargo.toml");
    assert!(
        !manifest.contains("floom-ui-composites"),
        "the facade must not depend on the satellite crate: a shipped crate does \
         not take an L9 -> excluded-workspace edge to keep a name"
    );

    // The `pub mod prelude { .. }` body, comment lines dropped: a name in a
    // comment is prose *about* the surface, not a promise made by it, and the
    // note in `src/lib.rs` is required to name all seven.
    let lib = read("src/lib.rs");
    let start = lib.find("pub mod prelude {").expect("prelude module");
    let body = &lib[start..];
    let body = &body[..body.find("\n}\n").expect("unterminated prelude module")];
    let code: String = body
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for name in MOVED {
        assert!(
            !code.contains(name),
            "`{name}` moved to `floom-ui-composites` and must not come back to \
             `floom::prelude`"
        );
    }
}
