// SPDX-License-Identifier: MIT
use floom_layout::{
    FlexDirection, JustifyContent, LayoutEngine, LayoutStyle, MeasureFn,
};
use floom_types::{Insets, Length, Point, Size};

#[test]
fn flex_row_places_children() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    let b = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(60.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Row,
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(40.0)),
            gap: 10.0,
            ..Default::default()
        },
        &[a, b],
    );

    eng.compute(root, Size::new(200.0, 40.0));
    let la = eng.layout(a).rect;
    let lb = eng.layout(b).rect;
    assert!((la.x0 - 0.0).abs() < 0.01);
    assert!((lb.x0 - 50.0).abs() < 0.01); // 40 + gap 10
    assert!((la.width() - 40.0).abs() < 0.01);
}

#[test]
fn column_stack_and_center() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(30.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            width: Some(Length::Px(100.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[child],
    );
    eng.compute(root, Size::new(100.0, 100.0));
    let c = eng.layout(child).rect;
    assert!((c.y0 - 45.0).abs() < 0.5, "y0={}", c.y0);
}

#[test]
fn measure_fn_leaf() {
    let mut eng = LayoutEngine::new();
    let measure: MeasureFn = Box::new(|_avail| Size::new(55.0, 33.0));
    let leaf = eng.new_leaf_with_measure(LayoutStyle::default(), measure);
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            align_items: floom_layout::AlignItems::Start,
            ..Default::default()
        },
        &[leaf],
    );
    eng.compute(root, Size::new(200.0, 100.0));
    let r = eng.layout(leaf).rect;
    assert!((r.width() - 55.0).abs() < 0.01, "w={}", r.width());
    assert!((r.height() - 33.0).abs() < 0.01, "h={}", r.height());
}

#[test]
fn public_api_exports_opaque_layout_node_only() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    assert!(src.contains("LayoutNode"));
    assert!(!src.contains("pub use taffy::"));
    assert!(!src.contains("NodeId"));
}

#[test]
fn absolute_child_escapes_parent_flow() {
    let mut eng = LayoutEngine::new();
    // In-flow sibling that would otherwise sit beside the overlay.
    let sibling = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    // Overlay anchored to all four sides of the containing block.
    let overlay = eng.new_leaf(LayoutStyle {
        position: floom_layout::Position::Absolute,
        inset: floom_layout::AbsoluteInset::fill(),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[sibling, overlay],
    );
    eng.compute(root, Size::new(200.0, 100.0));

    let sibling_rect = eng.layout(sibling).rect;
    assert_eq!(sibling_rect.x0, 0.0, "in-flow child still starts at 0");
    assert_eq!(sibling_rect.width(), 40.0);

    let r = eng.layout(overlay).rect;
    assert_eq!(r.origin(), Point::ZERO);
    assert_eq!(r.size(), Size::new(200.0, 100.0));
}

#[test]
fn absolute_child_uses_parent_alignment_as_static_position() {
    let mut eng = LayoutEngine::new();
    let panel = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(120.0)),
        height: Some(Length::Px(60.0)),
        position: floom_layout::Position::Absolute,
        ..Default::default()
    });
    // With every inset `auto` the box falls back to its static position, which
    // CSS defines by the parent's alignment — not by centring on its own.
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            justify_content: JustifyContent::Center,
            align_items: floom_layout::AlignItems::Center,
            ..Default::default()
        },
        &[panel],
    );
    eng.compute(root, Size::new(200.0, 100.0));

    let r = eng.layout(panel).rect;
    assert!((r.x0 - 40.0).abs() < 0.5, "x0={}", r.x0);
    assert!((r.y0 - 20.0).abs() < 0.5, "y0={}", r.y0);
}

#[test]
fn absolute_inset_pins_to_one_edge() {
    let mut eng = LayoutEngine::new();
    // A bottom-anchored sheet: pinned down, stretched sideways.
    let sheet = eng.new_leaf(LayoutStyle {
        height: Some(Length::Px(50.0)),
        position: floom_layout::Position::Absolute,
        inset: floom_layout::AbsoluteInset {
            left: Some(0.0),
            right: Some(0.0),
            bottom: Some(0.0),
            top: None,
        },
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(300.0)),
            height: Some(Length::Px(200.0)),
            ..Default::default()
        },
        &[sheet],
    );
    eng.compute(root, Size::new(300.0, 200.0));

    let r = eng.layout(sheet).rect;
    assert_eq!(r.width(), 300.0);
    assert_eq!(r.height(), 50.0);
    assert!(
        (r.y0 - 150.0).abs() < 0.5,
        "sheet must sit at the bottom: y0={}",
        r.y0
    );
}

#[test]
fn removing_a_child_updates_the_flow() {
    let mut eng = LayoutEngine::new();
    let keep = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    let drop = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        &[keep, drop],
    );
    eng.compute(root, Size::new(200.0, 100.0));
    assert!((eng.layout(drop).rect.y0 - 20.0).abs() < 0.5);

    eng.remove_child(root, drop);
    eng.remove(drop);
    eng.compute(root, Size::new(200.0, 100.0));

    assert!(
        !eng.contains(drop),
        "removed handles are stale, not dangling"
    );
    assert_eq!(eng.node_count(), 2, "root and the surviving child remain");

    // The remaining child still lays out as the only row item.
    assert_eq!(eng.layout(keep).rect.y0, 0.0);
}

#[test]
fn insert_child_at_index_preserves_order() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(10.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let b = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(10.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let c = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(10.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(LayoutStyle::column(), &[a, b]);
    // `c` goes in the middle rather than after `b`.
    eng.insert_child_at_index(root, 1, c);
    eng.compute(root, Size::new(200.0, 100.0));

    let expected = [(a, 0.0), (c, 10.0), (b, 20.0)];
    for (node, y) in expected {
        let rect = eng.layout(node).rect;
        assert!((rect.y0 - y).abs() < 0.5, "y0={} expected {}", rect.y0, y);
    }
}

#[test]
fn removed_slots_are_reused() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle::default());
    let before = eng.node_count();
    eng.remove(a);
    let b = eng.new_leaf(LayoutStyle::default());
    assert_eq!(b, a, "freed slot comes back first");
    // Reusing the slot keeps the live count bounded instead of growing forever.
    assert_eq!(eng.node_count(), before);
    assert!(eng.contains(b));
}

/// Measure nodes must re-measure under the space their parent actually gave
/// them — not the root viewport captured once at compute time. Shrinking the
/// fixed box that holds a text-like leaf must re-run the measure fn with the
/// tighter width so the leaf can wrap (grow taller).
#[test]
fn measure_nodes_remeasure_under_per_node_constraints() {
    use std::sync::Arc;
    use std::sync::Mutex;

    let mut eng = LayoutEngine::new();
    let seen = Arc::new(Mutex::new(f64::NAN));

    let measure_seen = seen.clone();
    let measure: MeasureFn = Box::new(move |avail| {
        *measure_seen.lock().unwrap() = avail.width;
        // Fake "text": single line 100x20, wrapping to 60 tall under 50 wide.
        let width = avail.width.min(100.0);
        let height = if avail.width < 50.0 { 60.0 } else { 20.0 };
        Size::new(width, height)
    });

    let text = eng.new_leaf_with_measure(LayoutStyle::default(), measure);
    let holder = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(100.0)),
            height: Some(Length::Px(100.0)),
            ..LayoutStyle::column()
        },
        &[text],
    );
    let root = eng.new_with_children(LayoutStyle::default(), &[holder]);

    eng.compute(root, Size::new(200.0, 200.0));
    let wide = eng.layout(text).rect;
    assert!(
        (wide.height() - 20.0).abs() < 0.5,
        "single line in a 100-wide box, got {}",
        wide.height()
    );
    assert!(
        (*seen.lock().unwrap() - 100.0).abs() < 0.5,
        "measure saw the holder's box"
    );

    // Shrink the holder: same measure fn, tighter constraint, taller leaf.
    eng.set_style(
        holder,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(100.0)),
            ..LayoutStyle::column()
        },
    );
    eng.compute(root, Size::new(200.0, 200.0));
    let narrow = eng.layout(text).rect;
    assert!(
        (narrow.height() - 60.0).abs() < 0.5,
        "wrapped in the 40-wide box, got {}",
        narrow.height()
    );
    assert!(
        (*seen.lock().unwrap() - 40.0).abs() < 0.5,
        "measure saw the new constraint"
    );
}

/// A margin is space *outside* a box, inside its parent — which is how a
/// decorated box spells `Container::new(child).style(|s| s.padding(..))`. The
/// inset has to push the child off the parent's edge, and it has to be counted
/// in the parent's own size rather than clipped away.
#[test]
fn a_margin_insets_the_child_inside_its_parent() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        margin: Insets::uniform(16.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 100.0));
    assert!(
        (eng.layout(child).rect.x0 - 16.0).abs() < 0.01,
        "a 16px margin pushes the child 16px from the parent's edge"
    );
}

/// The control for the wrong reading: the inset must not *shrink* the child's
/// own box. `child.width = 40` with a 16px margin is a 40px box 16px in — not a
/// 40px box reduced to 8px, which is what treating a margin as padding-inverted
/// would produce.
#[test]
fn a_margin_does_not_shrink_the_children_box() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        margin: Insets::uniform(16.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 100.0));
    let rect = eng.layout(child).rect;
    assert!(
        (rect.width() - 40.0).abs() < 0.01,
        "the box keeps its width"
    );
    assert!((rect.x0 - 16.0).abs() < 0.01);
}

/// `Insets` per side, because `padding_horiz` / `padding_vert` are the common
/// call and a single number cannot express the expansion panels' header inset
/// (horizontal only).
#[test]
fn a_margin_is_per_side() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(10.0)),
        height: Some(Length::Px(10.0)),
        margin: Insets {
            left: 4.0,
            top: 8.0,
            right: 0.0,
            bottom: 0.0,
        },
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 100.0));
    let rect = eng.layout(child).rect;
    assert!((rect.x0 - 4.0).abs() < 0.01);
    assert!((rect.y0 - 8.0).abs() < 0.01);
}

/// Padding is the other half of the box model and the reason it cannot be
/// substituted for a margin: it insets the children *inside* the box, so the
/// box's own size grows by it and the child's box does not.
#[test]
fn padding_insets_the_children_inside_the_box() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(40.0)),
        height: Some(Length::Px(20.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            padding: Insets::uniform(16.0),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 100.0));
    let rect = eng.layout(child).rect;
    assert!((rect.x0 - 16.0).abs() < 0.01, "inset from the inside");
    assert!(
        (rect.width() - 40.0).abs() < 0.01,
        "and the child's own box is unchanged, unlike a margin"
    );
}

/// The control that keeps the two apart, and the reason `Surface` carries real
/// padding rather than a margin per child.
///
/// With **one** child the two are geometrically identical — measured, both give
/// the child at 8px and the box at 56x36 — so a test written against a single
/// child would pass for either implementation. With **two** they diverge: a
/// margin is per child and lands between them as well, while a padding is one
/// inset around the group. Taffy does not collapse margins, so the row below ends
/// up with a 16px hole in the middle that the padded row does not have.
#[test]
fn a_margin_is_per_child_and_a_padding_is_around_the_group() {
    let two = |margin: Insets, padding: Insets| {
        let mut eng = LayoutEngine::new();
        let a = eng.new_leaf(LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(20.0)),
            margin,
            ..Default::default()
        });
        let b = eng.new_leaf(LayoutStyle {
            width: Some(Length::Px(30.0)),
            height: Some(Length::Px(20.0)),
            margin,
            ..Default::default()
        });
        let root = eng.new_with_children(
            LayoutStyle {
                flex_direction: FlexDirection::Row,
                padding,
                ..Default::default()
            },
            &[a, b],
        );
        eng.compute(root, Size::new(200.0, 100.0));
        (
            eng.layout(root).rect,
            eng.layout(a).rect,
            eng.layout(b).rect,
        )
    };

    let inset = 8.0;
    let (padded_box, padded_a, padded_b) = two(Insets::ZERO, Insets::uniform(inset));
    let (margin_box, margin_a, margin_b) = two(Insets::uniform(inset), Insets::ZERO);

    assert!(
        (padded_a.x0 - inset).abs() < 0.01,
        "the group is inset once"
    );
    assert!(
        (padded_b.x0 - padded_a.x1).abs() < 0.01,
        "and its members stay flush with each other"
    );
    assert!(
        (margin_a.x0 - inset).abs() < 0.01,
        "each margin insets its own child"
    );
    assert!(
        (margin_b.x0 - margin_a.x1 - inset * 2.0).abs() < 0.01,
        "and the second margin also lands *between* them — a hole a padded \
         group does not have"
    );
    assert!(
        (margin_box.width() - padded_box.width() - inset * 2.0).abs() < 0.01,
        "which is why the margin version's box is wider: four margins against \
         two paddings"
    );
}

/// Padding is per side, because a design system's header inset is horizontal and
/// its body inset is usually not.
#[test]
fn padding_is_per_side() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(10.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(100.0)),
            padding: Insets {
                left: 4.0,
                top: 8.0,
                right: 0.0,
                bottom: 0.0,
            },
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 100.0));
    let rect = eng.layout(child).rect;
    assert!((rect.x0 - 4.0).abs() < 0.01);
    assert!((rect.y0 - 8.0).abs() < 0.01);
}

// --- percentage sizing -----------------------------------------------------

/// The claim, and the reason `Length` exists: a percentage resolves against the
/// containing block, so `width_pct(100.0)` is a full-width row while a fixed
/// `200.0` is not. Before this, 2.0 had no spelling for the 310 `width_pct` /
/// `height_pct` calls across floem-ui.
#[test]
fn a_percentage_resolves_against_the_containing_block() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Pct(100.0)),
        height: Some(Length::Pct(50.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(300.0)),
            height: Some(Length::Px(200.0)),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(300.0, 200.0));
    let rect = eng.layout(child).rect;
    assert!((rect.width() - 300.0).abs() < 0.01, "100% of 300");
    assert!((rect.height() - 100.0).abs() < 0.01, "50% of 200");
}

/// The control for reading `100.0` as `1.0`: taffy's percentage is a fraction, so
/// a missing `/100.0` makes the box 100× its parent rather than the same size —
/// which overflows rather than looks wrong, and is easy to miss.
#[test]
fn a_percentage_is_hundred_not_one() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(Length::Pct(1.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(50.0)),
            ..Default::default()
        },
        &[child],
    );

    eng.compute(root, Size::new(200.0, 50.0));
    assert!(
        (eng.layout(child).rect.width() - 2.0).abs() < 0.01,
        "1% of 200 is 2px, not 200px"
    );
}

/// A fixed length and a percentage are different values, not two spellings of
/// one, and the difference is invisible at the parent size where they happen to
/// agree — which is why a test written at one width proves nothing.
#[test]
fn a_fixed_length_and_a_percentage_diverge_as_the_parent_grows() {
    let sized = |parent: f32, w: Option<Length>| {
        let mut eng = LayoutEngine::new();
        let child = eng.new_leaf(LayoutStyle {
            width: w,
            ..Default::default()
        });
        let root = eng.new_with_children(
            LayoutStyle {
                width: Some(Length::Px(parent)),
                height: Some(Length::Px(50.0)),
                ..Default::default()
            },
            &[child],
        );
        eng.compute(root, Size::new(f64::from(parent), 50.0));
        eng.layout(child).rect.width()
    };

    // 50px and 25% coincide at a 200px parent...
    assert!(
        (sized(200.0, Some(Length::Px(50.0))) - sized(200.0, Some(Length::Pct(25.0)))).abs() < 0.01
    );
    // ...and part company at 400, which is the difference in one line.
    assert!((sized(400.0, Some(Length::Px(50.0))) - 50.0).abs() < 0.01);
    assert!((sized(400.0, Some(Length::Pct(25.0))) - 100.0).abs() < 0.01);
    assert!(
        sized(400.0, Some(Length::Px(50.0))) < sized(400.0, Some(Length::Pct(25.0))),
        "the fixed length does not follow the parent, which is the whole point"
    );
}

// --- SpaceBetween ----------------------------------------------------------

/// The layout layer carried `SpaceBetween` from the start; only the
/// engine-neutral vocabulary was missing it, and floem-ui asks for it 38 times.
/// Two boxes with free space between them and none at either end is what makes it
/// distinguishable from `End`.
#[test]
fn space_between_pushes_the_children_apart() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(20.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let b = eng.new_leaf(LayoutStyle {
        width: Some(Length::Px(20.0)),
        height: Some(Length::Px(10.0)),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(50.0)),
            ..Default::default()
        },
        &[a, b],
    );

    eng.compute(root, Size::new(200.0, 50.0));
    let (ra, rb) = (eng.layout(a).rect, eng.layout(b).rect);
    assert!(
        (ra.x0 - 0.0).abs() < 0.01,
        "nothing at the start edge: {:?}",
        ra.x0
    );
    assert!(
        (rb.x1 - 200.0).abs() < 0.01,
        "and nothing at the end edge: {:?}",
        rb.x1
    );
}

/// **A measure leaf's box comes back rounded to whole pixels**, and that is the
/// reason the host's paint path cannot wrap text against it.
///
/// `79.078125` in, `x1: 79.0` out. The host re-shapes the same string to get
/// glyph runs and used to use this rounded box as the wrap width, so any string
/// whose natural width landed in the upper half-pixel of its own box painted one
/// word short of fitting — measured at 498 of 2997 random regular-weight
/// strings. The host now wraps against the *measured* width instead (see
/// `TextMeasure::measured` in `floom-app`).
///
/// This test pins the rounding itself rather than the workaround, so that if a
/// future taffy stops rounding, the comment above is what points at the code
/// that can then be deleted.
#[test]
fn a_measure_leaves_box_is_rounded_to_whole_pixels() {
    let mut eng = LayoutEngine::new();
    let leaf = eng.new_leaf_with_measure(
        LayoutStyle::default(),
        Box::new(|_| Size::new(79.078125, 14.0)),
    );
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(Length::Px(400.0)),
            height: Some(Length::Px(100.0)),
            ..Default::default()
        },
        &[leaf],
    );
    eng.compute(root, Size::new(400.0, 100.0));
    let rect = eng.layout(leaf).rect;
    assert!(
        (rect.x1 - 79.0).abs() < 0.001,
        "expected the box to be rounded down to 79.0, got {rect:?}"
    );
}

/// The control for the above: a box that comes out *wider* than it went in would
/// mean the rounding is a `ceil` and the host's `max` would be the wrong
/// direction. Rounding is to nearest, so a measure returning 71.59 — which rounds
/// *up* to 72 — proves both halves.
#[test]
fn the_rounding_goes_both_ways() {
    let width_for = |measured: f64| {
        let mut eng = LayoutEngine::new();
        let leaf = eng.new_leaf_with_measure(
            LayoutStyle::default(),
            Box::new(move |_| Size::new(measured, 14.0)),
        );
        let root = eng.new_with_children(
            LayoutStyle {
                width: Some(Length::Px(400.0)),
                height: Some(Length::Px(100.0)),
                ..Default::default()
            },
            &[leaf],
        );
        eng.compute(root, Size::new(400.0, 100.0));
        eng.layout(leaf).rect.x1
    };
    assert!((width_for(79.078125) - 79.0).abs() < 0.001, "rounds down");
    assert!((width_for(71.5927734375) - 72.0).abs() < 0.001, "rounds up");
}
