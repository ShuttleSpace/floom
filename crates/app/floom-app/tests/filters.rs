// SPDX-License-Identifier: MIT
//! A4's back half through the real host: what a control asks for is what the
//! retained scene carries, in the order the draw made it.
//!
//! The software renderer proves the *pixels*
//! (`floom-renderer-software/tests/filters.rs`); this file proves the plumbing
//! a control can actually reach. Two different doors, and the distinction is
//! the whole slice:
//!
//! * a **content filter** is a *declaration* — `Container::blur` — because
//!   what it filters is the subtree composited, and no paint call can bracket
//!   a subtree it does not paint. The host writes the markers, so the test
//!   asserts they land *around* the picture rather than inside it.
//! * a **backdrop filter** is an *op* — `Container::backdrop_blur` — because
//!   its input is whatever is already behind, which no bracket can express.
//!
//! Order is asserted everywhere, not just presence: a `FilterPush` appended
//! after the content it should wrap is the bug this slice is most likely to
//! ship, because slot order is draw order and a filter declared mid-life is
//! minted after the picture already exists.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_types::{Color, Filter, Length, Size};
use floom_widgets::Container;

const INK: Color = Color::rgb(20, 30, 40);

fn box_style(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle {
        width: Some(Length::Px(width)),
        height: Some(Length::Px(height)),
        ..LayoutStyle::default()
    }
}

/// Mounts a 120×80 card (optionally with a filter) holding one child card, and
/// settles the frame loop. Returns the host with the card's own id, so a test
/// can mutate it and watch the scene change.
fn host_with_card(card: Container) -> (WidgetHost, floom_app::NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 80.0));
    let root = host.mount_root(box_style(120.0, 80.0), card);
    // A child the root does not paint: the whole reason the host has to write
    // the bracket.
    let _child = host.mount(
        root,
        box_style(40.0, 20.0),
        Container::new().background(Color::rgb(200, 10, 10)),
    );
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
    (host, root)
}

/// The scene's node kinds, in draw order.
fn kinds(host: &WidgetHost) -> Vec<&'static str> {
    host.scene()
        .iter()
        .map(|(_, node)| match node {
            SceneNode::Rect { .. } => "Rect",
            SceneNode::Path { .. } => "Path",
            SceneNode::Image { .. } => "Image",
            SceneNode::Text { .. } => "Text",
            SceneNode::ClipPush { .. } => "ClipPush",
            SceneNode::ClipPop => "ClipPop",
            SceneNode::GroupPush { .. } => "GroupPush",
            SceneNode::GroupPop => "GroupPop",
            SceneNode::FilterPush { .. } => "FilterPush",
            SceneNode::FilterPop => "FilterPop",
            SceneNode::BackdropFilter { .. } => "BackdropFilter",
            SceneNode::Shader { .. } => "Shader",
        })
        .collect()
}

/// The index of the first node of `kind`, if any.
fn index_of(kinds: &[&'static str], kind: &str) -> Option<usize> {
    kinds.iter().position(|k| *k == kind)
}

/// Every filter marker pair the scene holds, in draw order.
fn filter_brackets(host: &WidgetHost) -> Vec<Filter> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::FilterPush { filter } => Some(*filter),
            _ => None,
        })
        .collect()
}

#[test]
fn a_blurred_card_brackets_its_own_paint_and_its_childs() {
    // The bracket has to sit *outside* both rects: the filter sees the
    // composite of the card and its child, so a push minted between them
    // would blur one and not the other.
    let (host, _root) = host_with_card(Container::new().background(INK).blur(4.0));
    let kinds = kinds(&host);
    let push = index_of(&kinds, "FilterPush").expect("a blurred card opens a scope");
    let pop = index_of(&kinds, "FilterPop").expect("and closes it");
    let rects: Vec<usize> = kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == "Rect")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(rects.len(), 2, "the card and its child both painted");
    assert!(push < rects[0], "the push precedes everything it filters");
    assert!(pop > rects[1], "and the pop follows all of it");
    assert_eq!(filter_brackets(&host), vec![Filter::blur(4.0)]);
}

#[test]
fn an_unblurred_card_writes_no_markers_at_all() {
    // The control for the bracket above, and the cost claim: a card with no
    // filter must not make every backend open a layer.
    let (host, _root) = host_with_card(Container::new().background(INK));
    let kinds = kinds(&host);
    assert_eq!(index_of(&kinds, "FilterPush"), None);
    assert_eq!(index_of(&kinds, "FilterPop"), None);
    assert_eq!(
        kinds.iter().filter(|k| **k == "Rect").count(),
        2,
        "both cards still painted"
    );
}

#[test]
fn a_blur_declared_after_the_first_paint_lands_ahead_of_the_picture() {
    // Slot order is draw order, so a card that gains a blur mid-life would
    // otherwise have its marker appended *after* the picture it has to wrap —
    // a filter that does nothing, with no error anywhere.
    let (mut host, root) = host_with_card(Container::new().background(INK));
    assert_eq!(index_of(&kinds(&host), "FilterPush"), None);
    host.widget_mut::<Container>(root)
        .expect("the root is a container")
        .set_blur(6.0);
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }

    let kinds = kinds(&host);
    let push = index_of(&kinds, "FilterPush").expect("the blur is applied");
    let first_rect = index_of(&kinds, "Rect").expect("the card painted");
    assert!(
        push < first_rect,
        "the marker was minted ahead of the picture it filters: {kinds:?}"
    );
}

#[test]
fn cancelling_a_blur_retires_both_markers() {
    // A widget that stops filtering has to stop paying for a layer, and a
    // `FilterPop` left behind would close some *other* scope — or none.
    let (mut host, root) = host_with_card(Container::new().background(INK).blur(4.0));
    assert_eq!(filter_brackets(&host).len(), 1);
    host.widget_mut::<Container>(root)
        .expect("the root is a container")
        .set_blur(0.0);
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert_eq!(filter_brackets(&host).len(), 0, "no scope was left open");
    let kinds = kinds(&host);
    assert_eq!(index_of(&kinds, "FilterPop"), None);
}

#[test]
fn a_backdrop_blur_reaches_the_scene_before_the_card_that_asked_for_it() {
    // A frosted panel reads what is behind it, so its node has to come before
    // the tint the panel draws on top — otherwise the panel would be frosting
    // its own background.
    let (host, _root) = host_with_card(Container::new().background(INK).backdrop_blur(5.0));
    let mut backdrop = None;
    let mut first_rect = None;
    for (i, (_, node)) in host.scene().iter().enumerate() {
        match node {
            SceneNode::BackdropFilter { rect, filter } => backdrop = Some((i, *rect, *filter)),
            SceneNode::Rect { .. }
                if first_rect.is_none() => {
                    first_rect = Some(i);
                }
            _ => {}
        }
    }
    let (at, _rect, filter) = backdrop.expect("the backdrop node is in the scene");
    assert_eq!(filter, Filter::blur(5.0));
    assert!(
        at < first_rect.expect("the card painted"),
        "the backdrop runs before the card's own fill"
    );
    // And it is a node, not a bracket: nothing opened a filter scope for it.
    let kinds = kinds(&host);
    assert_eq!(index_of(&kinds, "FilterPush"), None);
    assert_eq!(index_of(&kinds, "FilterPop"), None);
}

#[test]
fn an_identity_backdrop_is_not_written_at_all() {
    // Zero radius is "no filter", and the host must not make a backend pay for
    // a layer (or a frost that does nothing) to honour it.
    let (host, _root) = host_with_card(Container::new().background(INK).backdrop_blur(0.0));
    assert_eq!(index_of(&kinds(&host), "BackdropFilter"), None);
}
