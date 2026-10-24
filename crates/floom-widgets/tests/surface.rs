//! `Surface` — the decorated box 2.0 was missing.
//!
//! v1's most common shape by far is `Container::new(child).style(|s| s
//! .background(..).padding(..))`: a decorated box with something in it. 2.0's
//! `Container` is a leaf, so that shape had no spelling at all. These tests pin
//! the two halves that matter and that a wrong implementation would get right by
//! accident: the padding has to be *inside* the box (the `md-card` bug), and the
//! decoration has to be drawn by the same code a bare `Container` uses.

use floom_core::{ChildKey, LayoutRequest, Length, Widget, WidgetAny};
use floom_types::{Color, Insets, Shadow};
use floom_widgets::primitives::Text;
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, paint_snapshot,
};
use floom_widgets::{Container, Surface};

const BOX: floom_types::Rect = floom_types::Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

fn allocate(widget: &dyn Widget) -> floom_core::ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn scene_kinds(widget: &dyn Widget, id: floom_core::ElementId) -> Vec<&'static str> {
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.scene_node_kinds()
}

// --- children --------------------------------------------------------------

/// The whole point: a decorated box that *holds* something. A leaf `Container`
/// can be filled but has nowhere to put content, which is why this type exists.
#[test]
fn a_surface_holds_its_children() {
    let surface = Surface::new()
        .background(Color::WHITE)
        .push(|| Box::new(Text::new("hello")))
        .push(|| Box::new(Text::new("world")));

    let children = surface.children();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].key, ChildKey::index(0));
    assert_eq!(children[1].key, ChildKey::index(1));
    assert!(
        children[0].build().as_any().is::<Text>(),
        "and it builds the caller's widget, untouched"
    );
}

#[test]
fn an_empty_surface_is_a_bare_decoration() {
    let surface = Surface::new().background(Color::WHITE);
    assert_eq!(surface.children().len(), 0);
    assert_eq!(surface.child_count(), 0);
    assert_eq!(
        scene_kinds(&surface, allocate(&surface)),
        vec!["Rect"],
        "a filled box with nothing in it is exactly one rect"
    );
}

#[test]
fn a_child_can_carry_its_own_layout() {
    let surface = Surface::new().push_layout(LayoutRequest::new().width(120.0), || {
        Box::new(Text::new("wide"))
    });
    let children = surface.children();
    assert_eq!(children[0].layout.width, Some(Length::Px(120.0)));
}

/// A keyed child follows its item rather than the slot, which is what lets a
/// reordered list keep each row's state — the same reason `List` and `Stack`
/// offer it.
#[test]
fn a_keyed_child_gets_a_stable_id() {
    let surface = Surface::new().push_keyed(7, || Box::new(Text::new("x")));
    assert_eq!(surface.children()[0].key, ChildKey::keyed(7));
}

// --- padding is real -------------------------------------------------------

/// The `md-card` bug, as a test. `Container` once carried a `padding` field with
/// a setter and no reader; a card built on it rendered with its text against the
/// edge, and its test passed because it only asserted the token was positive.
/// Here the padding has to come *out* as a layout request, because that is the
/// only route from a widget to the layout engine.
#[test]
fn padding_is_declared_so_the_engine_can_apply_it() {
    let surface = Surface::new().padding(Insets::uniform(16.0));
    let req = surface
        .layout_request()
        .expect("a padded box asks for its inset");
    assert_eq!(
        req.padding,
        Some(Insets::uniform(16.0)),
        "padding, not a margin: it has to land inside the box"
    );
    assert_eq!(req.margin, None, "and not as a margin on each child");
}

#[test]
fn padding_can_be_sided() {
    let surface = Surface::new().padding_horiz(24.0).padding_vert(8.0);
    assert_eq!(
        surface.padding_insets(),
        Insets {
            left: 24.0,
            right: 24.0,
            top: 8.0,
            bottom: 8.0,
        }
    );
}

/// An unpadded box asks for nothing, so it does not put a zero-valued style on
/// every node in the tree — the same "unspecified is not zero" rule the rest of
/// `LayoutRequest` follows.
#[test]
fn an_unpadded_surface_wants_no_layout() {
    assert!(
        Surface::new()
            .background(Color::WHITE)
            .layout_request()
            .is_none(),
        "no padding, no request"
    );
}

// --- the decoration is a Container, not a copy -----------------------------

/// One implementation of "fill, then outline, then shadow, then the backdrop
/// filter", so a `Surface` cannot drift from a bare `Container` about how either
/// is drawn.
#[test]
fn the_decoration_is_a_container_verbatim() {
    let surface = Surface::new()
        .background(Color::rgb(0x11, 0x22, 0x33))
        .radius(8.0)
        .border(1.0, Color::BLACK)
        .shadow(Shadow {
            color: Color::rgba(0, 0, 0, 40),
            blur: 2.0,
            offset_x: 0.0,
            offset_y: 2.0,
            spread: 0.0,
        });
    let bare = Container::new()
        .background(Color::rgb(0x11, 0x22, 0x33))
        .radius(8.0)
        .border(1.0, Color::BLACK)
        .shadow(Shadow {
            color: Color::rgba(0, 0, 0, 40),
            blur: 2.0,
            offset_x: 0.0,
            offset_y: 2.0,
            spread: 0.0,
        });

    assert_eq!(
        surface.decoration(),
        &bare,
        "a surface with no children paints exactly what the container paints"
    );
    assert_eq!(
        scene_kinds(&surface, allocate(&surface)),
        scene_kinds(&bare, allocate(&bare)),
        "and it draws the same scene nodes, in the same order"
    );
}

/// Fill and outline are independent, and adding children must not change either:
/// the child count has no business reaching the decoration.
#[test]
fn the_children_do_not_change_what_is_drawn() {
    let bare = Surface::new()
        .background(Color::WHITE)
        .border(1.0, Color::BLACK);
    let with_child = Surface::new()
        .background(Color::WHITE)
        .border(1.0, Color::BLACK)
        .push(|| Box::new(Text::new("x")));

    assert_eq!(
        scene_kinds(&bare, allocate(&bare)),
        scene_kinds(&with_child, allocate(&with_child)),
        "declaring a child adds no ink of its own"
    );
}

/// The filter has to bracket the whole subtree, so it is declared rather than
/// pushed in `paint` — a `Surface` that forgot to forward it would draw its
/// children sharp and report nothing wrong.
#[test]
fn a_content_filter_is_declared_not_painted() {
    let surface = Surface::new().background(Color::WHITE).blur(4.0);
    assert_eq!(surface.filter(), Some(floom_core::Filter::blur(4.0)));
    assert_eq!(
        surface.decoration().content_filter(),
        Some(floom_core::Filter::blur(4.0))
    );
    assert_eq!(
        scene_kinds(&surface, allocate(&surface)),
        vec!["Rect"],
        "the paint call emitted no filter marker"
    );
}

// --- invalidation ----------------------------------------------------------

/// The decoration owns the revision; `Surface` has no state of its own, so it
/// must not invent a second one and mark dirty on frames where nothing changed.
#[test]
fn a_settled_surface_marks_nothing_dirty() {
    let mut surface = Surface::new()
        .background(Color::WHITE)
        .push(|| Box::new(Text::new("x")));
    let id = allocate(&surface);

    let mut first = RecordingUpdateContext::new();
    surface.update(&mut first, id);
    let mut second = RecordingUpdateContext::new();
    surface.update(&mut second, id);

    assert_eq!(
        first.dirty_count(),
        1,
        "the first sync reports once — one revision, however many fields moved"
    );
    assert_eq!(
        second.dirty_count(),
        0,
        "an unchanged box must not invalidate again, or every frame would \\
         invalidate the whole subtree under it"
    );
}

/// Changing the fill after mount has to reach the node — this is the in-place
/// form a theme signal uses. `Surface` hands out no `&mut Container`, because
/// changing the fields that way is a way to change the box with no way to
/// invalidate it, which is exactly how `Container`'s dead `padding` field
/// happened.
#[test]
fn changing_the_decoration_after_mount_reaches_the_node() {
    let mut surface = Surface::new().background(Color::WHITE);
    let id = allocate(&surface);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);
    cx.clear();

    surface.set_background(Color::BLACK);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);

    assert_eq!(cx.dirty_count(), 1, "the new fill has to be painted");
}

/// Idempotence, the same property `Ripple`'s `set_*` has and for the same
/// reason: a theme signal fires on unrelated changes too, and a setter that
/// always bumped would repaint the whole tree on every one of them.
#[test]
fn setting_the_same_value_again_dirties_nothing() {
    let mut surface = Surface::new()
        .background(Color::WHITE)
        .padding(Insets::uniform(8.0));
    let id = allocate(&surface);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);
    cx.clear();

    surface.set_background(Color::WHITE);
    surface.set_radius(0.0);
    surface.set_border(0.0, Color::BLACK);
    surface.set_shadow(None);
    surface.set_padding(Insets::uniform(8.0));
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);

    assert_eq!(
        cx.dirty_count(),
        0,
        "re-applying the same decoration is a no-op"
    );
}

/// The padding is this widget's own field, not the decoration's, so it needs its
/// own route to the node — and a changed `layout_request` only helps if the node
/// is re-read.
#[test]
fn changing_the_padding_after_mount_reaches_the_node() {
    let mut surface = Surface::new().background(Color::WHITE);
    let id = allocate(&surface);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);
    cx.clear();

    surface.set_padding(Insets::uniform(24.0));
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);

    assert_eq!(cx.dirty_count(), 1);
    assert_eq!(
        surface.layout_request().and_then(|r| r.padding),
        Some(Insets::uniform(24.0)),
        "and the new inset is what the engine will be told"
    );
}

/// The third door, and the one this widget existed to need. A repaint redraws the
/// *previous* frame's geometry, because the host lays out only when layout is
/// dirty — so a padding that moved without saying so changes nothing a user can
/// see. This is the assertion that would have caught the original bug.
#[test]
fn a_moved_padding_asks_for_a_layout_pass() {
    let mut surface = Surface::new().padding(Insets::uniform(8.0));
    let id = allocate(&surface);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);
    cx.clear();

    surface.set_padding(Insets::uniform(24.0));
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);

    assert!(
        cx.laid_out_again(),
        "the box is a different size now, and a repaint alone would not know it"
    );
    assert_eq!(cx.layout_dirty(), &[id]);
}

/// The cost control on that door: a theme toggle changes colours and radii on
/// every surface in a tree, and asking for a re-layout each time would re-run the
/// layout pass for all of them. Only a *layout* change may use it.
#[test]
fn a_moved_colour_does_not_ask_for_a_layout_pass() {
    let mut surface = Surface::new()
        .background(Color::WHITE)
        .padding(Insets::uniform(8.0));
    let id = allocate(&surface);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);
    cx.clear();

    surface.set_background(Color::BLACK);
    surface.set_radius(4.0);
    surface.set_border(2.0, Color::BLACK);
    let mut cx = RecordingUpdateContext::new();
    surface.update(&mut cx, id);

    assert_eq!(cx.dirty_count(), 1, "the node is repainted");
    assert!(
        !cx.laid_out_again(),
        "but its geometry did not move, so layout must not re-run — this is the \
         difference between a theme change and an inset change"
    );
}

// --- it is what a ported card needs ----------------------------------------

/// The end-to-end claim, in the shape the kits use it: a filled, rounded,
/// shadowed box that also declares the content that goes inside it.
#[test]
fn a_card_shaped_surface_is_one_widget() {
    let surface = Surface::new()
        .background(Color::rgb(0xFF, 0xFF, 0xFF))
        .radius(12.0)
        .padding(Insets::uniform(16.0))
        .shadow(Shadow {
            color: Color::rgba(0, 0, 0, 32),
            blur: 8.0,
            offset_x: 0.0,
            offset_y: 4.0,
            spread: 0.0,
        })
        .push(|| Box::new(Text::new("card body")));

    assert_eq!(surface.child_count(), 1);
    assert_eq!(surface.padding_insets(), Insets::uniform(16.0));

    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let snapshot = paint_snapshot(&surface, id, BOX);
    assert!(
        snapshot.contains("RoundedRect") && snapshot.contains("Shadow"),
        "the card's fill and elevation are both there:\n{snapshot}"
    );
}
