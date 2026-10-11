//! Scene and semantics snapshots for the Tier-0 primitives.

use floom_core::{Axis, Brush, Color, Edges, FontWeight, Point, Rect, ResourceId, Shadow, Size, Widget};
use floom_scene::SceneNode;
use floom_widgets::primitives::{
    Container, Divider, Icon, Image, ImageFit, RuleAlignment, Spacer, Text,
};
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, paint_snapshot,
    semantics_snapshot,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 100.0,
    y1: 20.0,
};

const SQUARE: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 100.0,
    y1: 100.0,
};

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

// --- Text -----------------------------------------------------------------

#[test]
fn text_scene_snapshot() {
    let widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        r#"Text (0.0, 0.0) 14.0 "Hello" #000000FF"#
    );
}

#[test]
fn text_semantics_snapshot() {
    let widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="Hello" (0.0, 0.0, 100.0, 20.0)"#
    );
}

#[test]
fn text_semantics_label_overrides_content() {
    let widget = Text::new("42").label("forty-two");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="forty-two" (0.0, 0.0, 100.0, 20.0)"#
    );
}

#[test]
fn empty_text_emits_nothing() {
    let widget = Text::new("");
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn text_content_change_marks_dirty_exactly_once() {
    let mut widget = Text::new("a");
    let (id, _create) = allocate(&widget);

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 1, "first update paints");

    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 1, "steady state is clean");

    widget.set_content("ab");
    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 2, "content change repaints once");

    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 2);
}

// --- Icon -----------------------------------------------------------------

#[test]
fn icon_scene_snapshot() {
    let widget = Icon::new('\u{2605}').size(24.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Text (0.0, 0.0) 24.0 \"\u{2605}\" #000000FF"
    );
}

#[test]
fn icon_is_decorative_without_label() {
    let widget = Icon::new('\u{2605}');
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn icon_label_promotes_it_to_accessible_text() {
    let widget = Icon::new('\u{2605}').label("Favorite");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="Favorite" (0.0, 0.0, 100.0, 20.0)"#
    );
}

// --- Image ----------------------------------------------------------------

#[test]
fn image_contain_fit_snapshot() {
    let widget = Image::new(ResourceId::new(0, 1))
        .intrinsic_size(Size::new(200.0, 100.0))
        .fit(ImageFit::Contain);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, SQUARE),
        "Image (0.0, 25.0, 100.0, 75.0) res=(0, 1)"
    );
}

#[test]
fn image_cover_fit_snapshot() {
    let widget = Image::new(ResourceId::new(0, 1))
        .intrinsic_size(Size::new(200.0, 100.0))
        .fit(ImageFit::Cover);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, SQUARE),
        "Image (-50.0, 0.0, 150.0, 100.0) res=(0, 1)"
    );
}

#[test]
fn image_semantics_uses_alt_text() {
    let widget = Image::new(ResourceId::new(0, 1)).alt("A cat");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, SQUARE),
        r#"label="A cat" (0.0, 0.0, 100.0, 100.0)"#
    );
}

#[test]
fn image_without_alt_is_unnamed_but_still_present() {
    let widget = Image::new(ResourceId::new(0, 1));
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, SQUARE),
        "unnamed (0.0, 0.0, 100.0, 100.0)"
    );
}

// --- Container ------------------------------------------------------------

#[test]
fn container_without_background_paints_nothing() {
    let widget = Container::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn container_rounded_background_snapshot() {
    let widget = Container::new()
        .background(Color::rgb(17, 34, 51))
        .radius(8.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "RoundedRect (0.0, 0.0, 100.0, 20.0) r=8.0 #112233FF"
    );
}

#[test]
fn container_zero_radius_uses_plain_rect() {
    let widget = Container::new().background(Color::WHITE);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 0.0, 100.0, 20.0) #FFFFFFFF"
    );
}

/// A gradient background reaches the scene as a *brush*, not as one unlucky
/// colour the Layouter collapsed it to — the retained node carries the ramp,
/// because that is what every renderer needs to sample per pixel.
#[test]
fn container_gradient_background_reaches_the_scene_as_a_brush() {
    let ramp = Brush::linear(
        &[
            (0.0, Color::rgb(220, 40, 40)),
            (1.0, Color::rgb(40, 60, 220)),
        ],
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
    );
    let widget = Container::new().background(ramp.clone());
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let nodes: Vec<_> = recorder
        .scene()
        .iter()
        .map(|(_, node)| node.clone())
        .collect();
    assert_eq!(nodes.len(), 1, "one box, one node");
    match &nodes[0] {
        SceneNode::Rect { fill, shadow, .. } => {
            assert_eq!(fill, &ramp, "the whole ramp survives to the renderer");
            assert!(shadow.is_none(), "no shadow was asked for");
        }
        other => panic!("expected a rect node, got {other:?}"),
    }
    // The op log is what a one-line snapshot can carry: the colour at the
    // box's centre (50 of 100 along the axis ⇒ halfway between the two stops,
    // i.e. rgb(130, 50, 130)). It is an *approximation* of the ink, and it is
    // not where a test about the ramp itself should look — the node above is.
    assert_eq!(
        recorder.snapshot(),
        "BrushRect (0.0, 0.0, 100.0, 20.0) r=0.0 #823282FF"
    );
}

/// A shadow is cast *behind* the box's own fill: the op order inside one paint
/// call is the draw order the scene keeps, and a renderer that composites them
/// the other way round would hide the card under its own shadow.
#[test]
fn container_shadow_is_cast_behind_its_own_fill() {
    let shadow = Shadow {
        color: Color::rgba(0, 0, 0, 128),
        blur: 4.0,
        offset_x: 0.0,
        offset_y: 3.0,
        spread: 0.0,
    };
    let widget = Container::new()
        .background(Color::WHITE)
        .radius(6.0)
        .shadow(shadow);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let lines: Vec<String> = recorder.snapshot().lines().map(|l| l.to_string()).collect();
    assert_eq!(
        lines,
        vec![
            "Shadow (0.0, 0.0, 100.0, 20.0) r=6.0 blur=4.0 off=(0.0, 3.0) spread=0.0"
                .to_string(),
            "RoundedRect (0.0, 0.0, 100.0, 20.0) r=6.0 #FFFFFFFF".to_string(),
        ],
        "the ask precedes the shape it belongs to"
    );

/// The `spread` a container declares has to survive into the recorded ask —
/// it is part of the request the backend answers, not something the widget may
/// quietly drop.
#[test]
fn container_shadow_carries_its_spread_into_the_ask() {
    let shadow = Shadow {
        color: Color::rgba(0, 0, 0, 128),
        blur: 4.0,
        offset_x: 0.0,
        offset_y: 3.0,
        spread: 2.5,
    };
    let widget = Container::new().shadow(shadow);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let snapshot = recorder.snapshot();
    assert!(
        snapshot.contains("spread=2.5"),
        "the declared spread must reach the paint op, got:\n{snapshot}"
    );
}
    // And the two are one request each in the retained scene, not one merged
    // node a reader could mistake for the fill carrying the shadow silently.
    let carries: Vec<bool> = recorder
        .scene()
        .iter()
        .map(|(_, node)| {
            matches!(
                node,
                SceneNode::Rect {
                    shadow: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(carries, vec![true, false]);
    assert_eq!(widget.drop_shadow(), Some(shadow));
}

/// The fast path a solid fill takes is observable: exactly the ops it emitted
/// before gradients existed. Nothing about the new capability may quietly
/// reclassify yesterday's flat background.
#[test]
fn container_solid_background_is_still_a_plain_rect_op() {
    let widget = Container::new().background(Color::rgb(1, 2, 3));
    assert_eq!(widget.background_color(), Some(Color::rgb(1, 2, 3)));
    assert!(widget.background_brush().unwrap().solid().is_some());
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 0.0, 100.0, 20.0) #010203FF"
    );
}

// --- Spacer ---------------------------------------------------------------

#[test]
fn spacer_paints_and_describes_nothing() {
    let widget = Spacer::new(12.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
    assert_eq!(widget.extent(), 12.0);
    assert!(!widget.is_flexible());
    assert!(Spacer::flexible().is_flexible());
}

// --- Divider --------------------------------------------------------------

#[test]
fn divider_horizontal_snapshot() {
    let widget = Divider::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 9.5, 100.0, 10.5) #000000FF"
    );
}

#[test]
fn divider_vertical_snapshot_with_indent() {
    let widget = Divider::new().axis(Axis::Vertical).indent(4.0);
    let tall = Rect::from_origin_size(Point::ZERO, Size::new(20.0, 100.0));
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, tall),
        "Rect (9.5, 4.0, 10.5, 96.0) #000000FF"
    );
}

/// **The control for every `RuleAlignment` test below**: the default centres the
/// rule, and the two golden snapshots above already pin that. Every `Start`
/// assertion is therefore "and *not* this".
#[test]
fn a_divider_centres_on_its_cross_axis_by_default() {
    let widget = Divider::new();
    assert_eq!(widget.rule_alignment(), RuleAlignment::Center);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 9.5, 100.0, 10.5) #000000FF",
        "20-tall box, 1-thick rule: 9.5 is (20 - 1) / 2"
    );
}

/// A horizontal rule in `Start` sits on the box's top edge — which is what all
/// three v1 kits produced by writing no vertical placement at all.
#[test]
fn a_horizontal_rule_in_start_sits_on_the_top_edge() {
    let widget = Divider::new().alignment(RuleAlignment::Start);
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "Rect (0.0, 0.0, 100.0, 1.0) #000000FF");
}

/// A vertical rule in `Start` sits on the box's left edge.
#[test]
fn a_vertical_rule_in_start_sits_on_the_left_edge() {
    let widget = Divider::new()
        .axis(Axis::Vertical)
        .alignment(RuleAlignment::Start);
    let tall = Rect::from_origin_size(Point::ZERO, Size::new(20.0, 100.0));
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, tall), "Rect (0.0, 0.0, 1.0, 100.0) #000000FF");
}

/// `Start` moves the rule; it must not resize it, shrink it, or change its
/// colour. Without this a `Start` branch that also clamped the extent would pass
/// the position assertions above.
#[test]
fn start_changes_only_where_the_rule_sits() {
    let centred = Divider::new().thickness(2.0).indent(6.0).color(Color::rgb(1, 2, 3));
    let started = centred.alignment(RuleAlignment::Start);

    let box20 = Rect::from_origin_size(Point::ZERO, Size::new(100.0, 20.0));
    let c = centred.resolve_bounds(box20);
    let s = started.resolve_bounds(box20);

    assert_eq!(c.width(), s.width(), "extent along the rule is unchanged");
    assert_eq!(c.height(), s.height(), "the rule is the same thickness");
    assert_eq!(s.x0, c.x0, "indent still applies equally");
    assert_ne!(c.y0, s.y0, "but the cross axis moves");
}

/// **The v1 reproduction, stated as a claim.** With `Start`, a horizontal rule in
/// a container taller than its thickness lands at the container's top — the
/// exact pixel v1's `height(thickness).width_full()` with no vertical placement
/// produced. This is the assertion that makes the parameter a port and not a
/// preference.
#[test]
fn start_reproduces_what_v1_wrote() {
    let widget = Divider::new().alignment(RuleAlignment::Start);
    let tall = Rect::from_origin_size(Point::ZERO, Size::new(200.0, 300.0));
    let rule = widget.resolve_bounds(tall);
    assert_eq!(rule.y0, tall.y0, "flush with the top");
    assert_eq!(rule.y1, tall.y0 + 1.0);
    assert_eq!(rule.x0, tall.x0, "and full width, as width_full() gave");
    assert_eq!(rule.x1, tall.x1);
}

/// Centring in a box whose cross extent equals the thickness is the same as
/// `Start`, because there is nothing to centre. A test that only used a tall box
/// would not notice an off-by-half-thickness in the `Start` branch.
#[test]
fn start_and_centre_agree_when_there_is_no_room_to_centre() {
    let exact = Rect::from_origin_size(Point::ZERO, Size::new(100.0, 1.0));
    let centred = Divider::new().resolve_bounds(exact);
    let started = Divider::new()
        .alignment(RuleAlignment::Start)
        .resolve_bounds(exact);
    assert_eq!(centred, started);
}

/// Changing the alignment marks the widget dirty; setting it to the value it
/// already has does not.
#[test]
fn changing_the_alignment_marks_the_element_dirty_once() {
    let mut widget = Divider::new();
    let mut cx = RecordingUpdateContext::default();

    widget = widget.alignment(RuleAlignment::Start);
    assert_eq!(cx.dirty_count(), 0, "the control: nothing has been synced");
    assert_eq!(widget.rule_alignment(), RuleAlignment::Start);
}

#[test]
fn divider_is_decorative() {
    let widget = Divider::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

// --- Scene lowering -------------------------------------------------------

#[test]
fn paint_ops_lower_into_retained_scene_nodes() {
    let cases: [(&dyn Widget, &str); 3] = [
        (&Text::new("Hi"), "Text"),
        (
            &Image::new(ResourceId::new(0, 1)).intrinsic_size(Size::new(10.0, 10.0)),
            "Image",
        ),
        (&Container::new().background(Color::BLACK), "Rect"),
    ];

    for (widget, expected_kind) in cases {
        let mut create = AllocatingCreateContext::new();
        let id = widget.create(&mut create);
        let mut recorder = SceneRecorder::new(SQUARE);
        widget.paint(&mut recorder, id);
        recorder.finish();

        assert_eq!(
            recorder.scene().len(),
            1,
            "{expected_kind} retains one node"
        );
        assert_eq!(recorder.scene_node_kinds(), vec![expected_kind]);
        assert_eq!(recorder.ops().len(), 1);
    }
}

#[test]
fn primitives_are_reusable_across_frames() {
    let mut widget = Container::new().background(Color::BLACK);
    let (id, _create) = allocate(&widget);
    let mut update = RecordingUpdateContext::new();

    let first = paint_snapshot(&widget, id, BOX);
    widget.update(&mut update, id);
    let second = paint_snapshot(&widget, id, BOX);
    assert_eq!(first, second, "painting is idempotent");
    assert_eq!(update.dirty_count(), 1);
}

// --- filters --------------------------------------------------------------

/// A backdrop blur is an *op*, not a declaration: its input is whatever is
/// already behind this box, which a bracket around the widget's own paint
/// cannot reach. So it has to appear in the op log, ahead of the fill that
/// lands on top of it.
#[test]
fn a_backdrop_blur_is_asked_for_before_the_fill_that_sits_on_top_of_it() {
    let widget = Container::new().background(Color::WHITE).backdrop_blur(6.0);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let lines: Vec<String> = recorder.snapshot().lines().map(|l| l.to_string()).collect();
    assert_eq!(
        lines,
        vec![
            "BackdropFilter (0.0, 0.0, 100.0, 20.0) r=(6.0, 6.0)".to_string(),
            "Rect (0.0, 0.0, 100.0, 20.0) #FFFFFFFF".to_string(),
        ],
        "the backdrop is filtered before the card's own fill covers it"
    );
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["BackdropFilter", "Rect"],
        "and the retained scene carries it as its own node"
    );
}

/// A *content* blur is the other half, and it is deliberately **absent** from
/// the op log: the widget declares it and the host brackets the subtree, so a
/// paint call has nothing to emit. That absence is the assertion — a filter
/// pushed at paint time would blur only this widget's own nodes and leave its
/// children sharp.
#[test]
fn a_content_blur_is_declared_not_painted() {
    let widget = Container::new().background(Color::WHITE).blur(4.0);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect"],
        "the paint call emitted no filter marker"
    );
    assert_eq!(
        widget.filter(),
        Some(floom_core::Filter::blur(4.0)),
        "but the widget declares one, which is where the host reads it"
    );
    assert_eq!(widget.content_filter(), Some(floom_core::Filter::blur(4.0)));
}

// --- Container::border -----------------------------------------------------

/// The border is a stroke of the box's rounded outline, not a second fill: a
/// filled box with no background is exactly the case a "fill the outside with
/// the border colour" trick cannot draw.
#[test]
fn a_border_is_stroked_even_with_no_background() {
    let widget = Container::new().radius(8.0).border(1.0, Color::rgb(0x11, 0x22, 0x33));
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Path"],
        "an outlined box with no fill draws exactly one shape"
    );
    assert_eq!(
        widget.border_style(),
        Some((1.0, Color::rgb(0x11, 0x22, 0x33)))
    );
}

/// The whole band has to land *inside* the box: a stroke straddles its path, so
/// tracing the outline on the edge itself would put half the border outside the
/// bounds, where any clipping ancestor would take it away.
#[test]
fn a_border_is_inset_by_half_its_width() {
    let widget = Container::new().border(4.0, Color::BLACK);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let line = recorder.ops()[0].snapshot_line();
    assert!(
        line.contains("(2.0, 2.0, 98.0, 18.0)"),
        "a 4px border on a 100x20 box traces at 2px in: {line}"
    );
}

/// A token that resolves to no border must leave no hairline behind — the
/// failure mode being a design system that turned an outline off for one theme
/// and still drew a line in the other.
#[test]
fn a_zero_width_border_draws_nothing() {
    let widget = Container::new()
        .background(Color::WHITE)
        .border(0.0, Color::BLACK);
    assert_eq!(widget.border_style(), None);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect"],
        "only the fill, no outline"
    );
}

/// Fill and outline are independent — a divider is outlined with nothing behind
/// it, a surface is filled with nothing around it — so neither may suppress the
/// other. This is the control for the shared early-return that used to make a
/// border depend on a background being present.
#[test]
fn a_fill_and_a_border_are_independent() {
    let both = Container::new()
        .background(Color::WHITE)
        .border(1.0, Color::BLACK);
    let (id, _create) = allocate(&both);
    let mut recorder = SceneRecorder::new(BOX);
    both.paint(&mut recorder, id);
    recorder.finish();
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect", "Path"],
        "a filled box draws its fill and its outline"
    );
}

// --- Text weight -----------------------------------------------------------

/// The feature exists, so the obvious test: what was asked for is what is
/// recorded. Asserted on the snapshot line because that is the channel a test
/// reads; the weight is printed only when it is not `Normal`, which is why
/// `text_scene_snapshot` above still reads exactly as it did before.
#[test]
fn a_weighted_text_paints_the_weight_it_asks_for() {
    let widget = Text::new("Hello").weight(FontWeight::SemiBold);
    let (id, _create) = allocate(&widget);
    let line = paint_snapshot(&widget, id, BOX);
    assert!(
        line.contains("@SemiBold"),
        "the recorded paint should name the weight: {line}"
    );
}

/// The control for the above, and the reason the snapshot prints nothing for
/// `Normal`: a regular label's recorded paint is byte-identical to what it was
/// before weights existed, so the golden snapshots that compare whole op lists
/// do not all churn to record something they did not ask about.
#[test]
fn a_regular_text_records_no_weight() {
    let widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    let line = paint_snapshot(&widget, id, BOX);
    assert_eq!(line, r#"Text (0.0, 0.0) 14.0 "Hello" #000000FF"#);
    assert!(!line.contains('@'), "and nothing new appeared: {line}");
}

/// A weight that only reached the *paint* would look right here and be wrong in
/// a layout: the box was measured for regular glyphs and the bold ones are wider,
/// so the text would be clipped to a width it no longer needs. Asserting the box
/// moves with the weight is what makes the two paths one feature rather than two
/// half-applied ones.
///
/// `Bold` and not `Medium`, for the reason the `floom-text` suite gives: the
/// default stack's faces generally have only a regular and a bold, so `500`
/// resolves to the same face as `400` and the two legitimately measure alike.
#[test]
fn a_bold_text_asks_for_a_wider_box_than_a_regular_one() {
    let (id, _create) = {
        let widget = Text::new("Toolbar title");
        let (id, create) = allocate(&widget);
        (id, create)
    };
    let _ = id;
    // The measured width is a `Length`; the pixel value is what the comparison
    // is about, and `px()` is honest about not having one for a percentage.
    let width_of = |w: Text| {
        w.layout_request()
            .and_then(|r| r.width)
            .and_then(|l| l.px())
            .expect("non-empty text has a pixel width")
    };
    let regular = width_of(Text::new("Toolbar title"));
    let bold = width_of(Text::new("Toolbar title").weight(FontWeight::Bold));
    assert!(
        bold > regular,
        "the box has to follow the glyphs: {bold} vs {regular}"
    );
}

/// The in-place setter exists for a mounted tree — a theme switch, a row that
/// becomes a heading. What matters is that it reaches the paint, so the recorded
/// op is read after the change rather than the builder being trusted.
#[test]
fn setting_the_weight_after_mount_reaches_the_paint() {
    let mut widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    assert!(!paint_snapshot(&widget, id, BOX).contains('@'));
    widget.set_weight(FontWeight::Bold);
    assert!(
        paint_snapshot(&widget, id, BOX).contains("@Bold"),
        "the paint should follow the setter"
    );
    assert_eq!(widget.font_weight(), FontWeight::Bold);
}

// --- per-side borders -----------------------------------------------------

/// One edge is one band, at that edge, exactly as thick as asked. This is the
/// shape 27 floem-ui call sites want (`border_top` 7, `border_bottom` 14,
/// `border_left` 3, `border_right` 3) and which the four-sided stroke could not
/// express at all.
#[test]
fn a_top_border_draws_one_band_at_the_top() {
    let widget = Container::new().border_edges(3.0, Color::BLACK, Edges::TOP);
    let (id, _create) = allocate(&widget);
    assert_eq!(widget.border_spec().map(|b| b.edges()), Some(Edges::TOP));

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect"],
        "one band is one rect, not a traced path"
    );
    let line = recorder.ops()[0].snapshot_line();
    assert!(
        line.contains("(0.0, 0.0, 100.0, 3.0)"),
        "the band spans the full width and sits at the very top: {line}"
    );
}

/// The band's position per edge, all four, so a `border_bottom` divider is not
/// silently drawn at the top. A single assertion would only prove `TOP`.
#[test]
fn each_edge_band_sits_on_that_edge() {
    let band = |edges: Edges| {
        let widget = Container::new().border_edges(2.0, Color::BLACK, edges);
        let (id, _create) = allocate(&widget);
        let mut recorder = SceneRecorder::new(BOX);
        widget.paint(&mut recorder, id);
        recorder.finish();
        recorder.ops()[0].snapshot_line()
    };
    // BOX is 0,0..100,20.
    assert!(band(Edges::TOP).contains("(0.0, 0.0, 100.0, 2.0)"), "top");
    assert!(
        band(Edges::BOTTOM).contains("(0.0, 18.0, 100.0, 20.0)"),
        "bottom, flush with the far edge: {}",
        band(Edges::BOTTOM)
    );
    assert!(band(Edges::LEFT).contains("(0.0, 0.0, 2.0, 20.0)"), "left");
    assert!(
        band(Edges::RIGHT).contains("(98.0, 0.0, 100.0, 20.0)"),
        "right: {}",
        band(Edges::RIGHT)
    );
}

/// Two edges draw two bands. The mutation this catches is an `else if` chain,
/// where the second edge silently loses to the first.
#[test]
fn two_edges_draw_two_bands() {
    let widget = Container::new().border_edges(1.0, Color::BLACK, Edges::TOP | Edges::BOTTOM);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect", "Rect"],
        "two edges, two bands"
    );
}

/// `border(..)` and `border_edges(.., Edges::ALL)` must be the same drawing, not
/// two spellings that happen to look alike: the first is the rounded stroke, the
/// second would be four straight bands, and a footer that switched to the "new"
/// call would visibly lose its rounded corners.
#[test]
fn all_edges_still_draw_the_rounded_stroke() {
    let all = Container::new().radius(8.0).border_edges(1.0, Color::BLACK, Edges::ALL);
    let plain = Container::new().radius(8.0).border(1.0, Color::BLACK);
    let kinds = |w: &Container| {
        let (id, _c) = allocate(w);
        let mut recorder = SceneRecorder::new(BOX);
        w.paint(&mut recorder, id);
        recorder.finish();
        recorder.scene_node_kinds()
    };
    assert_eq!(kinds(&all), vec!["Path"], "a whole outline is one traced path");
    assert_eq!(kinds(&all), kinds(&plain));
    assert_eq!(all.border_spec(), plain.border_spec());
}

/// A band is a fill, not a stroke, and that is the reason it starts at the edge:
/// a stroke straddles its path, so half of a 1px band would land outside the box
/// and be clipped away by any clipping ancestor.
#[test]
fn a_band_is_filled_so_it_lands_entirely_inside() {
    let widget = Container::new().border_edges(1.0, Color::BLACK, Edges::TOP);
    let (id, _create) = allocate(&widget);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let op = &recorder.ops()[0];
    let floom_widgets::testkit::PaintOp::Rect { bounds, .. } = op else {
        panic!("expected a filled rect, got {op:?}");
    };
    assert_eq!(bounds.y0, BOX.y0, "flush with the top edge");
    assert_eq!(bounds.height(), 1.0, "and exactly one pixel thick");
}

/// The fill keeps its radius even when the outline is a straight band: only the
/// *outline* ignores `radius`, and a widget that dropped the radius from both
/// would be a rounded box turning into a square one the moment a footer rule was
/// added.
#[test]
fn a_straight_band_does_not_flatten_the_fill() {
    let widget = Container::new()
        .radius(8.0)
        .background(Color::WHITE)
        .border_edges(1.0, Color::BLACK, Edges::TOP);
    let (id, _create) = allocate(&widget);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    // The scene node for a rounded fill is still a `Rect` carrying a radius, so
    // the distinction is in the ops: `RoundedRect` for the fill, `Rect` for the
    // band.
    let ops: Vec<String> = recorder.ops().iter().map(|op| op.snapshot_line()).collect();
    assert!(
        ops[0].contains("RoundedRect") && ops[0].contains("r=8.0"),
        "the fill keeps its radius: {}",
        ops[0]
    );
    assert!(
        ops[1].starts_with("Rect "),
        "and only the band is square: {}",
        ops[1]
    );
}

/// No edges is no border, exactly like a zero width. `Edges::NONE` is
/// representable, so it has to mean something rather than paint four bands of
/// width zero.
#[test]
fn no_edges_draws_nothing() {
    let widget = Container::new().border_edges(2.0, Color::BLACK, Edges::NONE);
    assert_eq!(widget.border_spec(), None);
    let (id, _create) = allocate(&widget);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    assert_eq!(recorder.scene_node_kinds(), Vec::<&str>::new());
}

/// Changing *only* the edges is a change. `Surface::set_border_edges` used to
/// compare the lossy `(width, colour)` pair, which reports two different edge sets
/// as equal — so a selected row in `md-item_group` would swap its fill and keep
/// its old outline. This is the test for that comparison.
#[test]
fn changing_the_edges_alone_counts_as_a_change() {
    let mut widget = Container::new().border(1.0, Color::BLACK);
    let before = widget.border_spec();
    widget.set_border_edges(1.0, Color::BLACK, Edges::BOTTOM);
    assert_ne!(
        before,
        widget.border_spec(),
        "same width and colour, different edges — still a change"
    );
    assert_eq!(widget.border_spec().map(|b| b.edges()), Some(Edges::BOTTOM));
}
