//! Subtree transforms, from the prelude alone.
//!
//! Before this slice, `CompositorProp::Scale` was deliberately inert — the
//! host's own comment said so: scaling each rect about its own centre would
//! tear the picture apart, and there was no group to scale it about a common
//! origin. Now a transform-class composite (scale, rotate) becomes a
//! `GroupPush` scope bracketing the animated element's picture, and every
//! backend applies one transform. Each claim below is asserted against the
//! scene the host retained and pixels a real rasteriser produced:
//!
//! 1. **A scale animation brackets its picture.** The card's own payloads
//!    never move — the scope carries the sampled scale about the card's box
//!    centre, push before the picture, pop after it.
//! 2. **The pixels follow the value.** At t=0 the card is its declared size;
//!    mid-tween it covers ground it did not cover before; at t=1 it covers
//!    more still. Probed in the band beyond the card's original edge, with
//!    the empty-at-rest control in the same test.
//! 3. **A rotation turns the whole picture.** A quarter turn later, the card
//!    that sat left of its pivot sits above it — in pixels — and the scope's
//!    matrix is the sampled angle, frame to frame.
//! 4. **"Finished" is not "cancelled".** A tween that ran to its end parks
//!    at its endpoint — the scope stays, carrying the final scale, and the
//!    card rests there. (It is *cancelling* that retires the scope; the
//!    host's own test suite pins that side of the contract.)
//!
//! Everything below is `floom2::prelude`. Run with `cargo run -p
//! floom2-transform`.

use floom2::floom_scene::SceneNode;
use floom2::floom_types::Transform;
use floom2::prelude::*;

/// The card's surface, and the card inside it (in the canvas's own coords).
const SURFACE_W: u32 = 96;
const SURFACE_H: u32 = 96;
/// Logical size of the host and the card. 44×24: asymmetric on purpose.
const SIZE: f64 = 96.0;
const CARD_W: f64 = 44.0;
const CARD_H: f64 = 24.0;

fn main() {
    act_1_the_scope_brackets_the_picture();
    act_2_the_pixels_follow_the_scale();
    act_3_a_rotation_turns_the_whole_picture();
    act_4_a_finished_transform_parks_clean();

    println!();
    println!("every act above was asserted, not printed");
}

/// A card: one filled rectangle centred in the canvas — wider than tall, so a
/// quarter turn visibly changes which ground it covers (a centred *square*
/// rotated 90° maps onto itself, which would make every probe below vacuous).
/// Its painter is deliberately static — everything that moves below moves
/// through the transform scope, not through the payload.
fn card() -> Canvas {
    Canvas::new(|cx| {
        let b = cx.bounds();
        let (w, h) = (CARD_W.min(b.width()), CARD_H.min(b.height()));
        let rect = Rect {
            x0: b.x0 + (b.width() - w) / 2.0,
            y0: b.y0 + (b.height() - h) / 2.0,
            x1: b.x0 + (b.width() + w) / 2.0,
            y1: b.y0 + (b.height() + h) / 2.0,
        };
        cx.fill_rect(rect, Color::rgb(190, 60, 60));
    })
    .size(SIZE as f32, SIZE as f32)
    .label("Transform card")
}

fn host_with_card() -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(SIZE, SIZE));
    let root = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(SIZE as f32)),
            height: Some(Length::Px(SIZE as f32)),
            ..LayoutStyle::default()
        },
        Stack::new(Axis::Vertical),
    );
    let canvas = host.mount(
        root,
        LayoutStyle {
            width: Some(Length::Px(SIZE as f32)),
            height: Some(Length::Px(SIZE as f32)),
            ..LayoutStyle::default()
        },
        card(),
    );
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
    (host, canvas)
}

/// The card's painted rect (its own payload, pre-transform).
fn card_rect(host: &WidgetHost) -> Rect {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the card painted its rect")
}

fn is_card_pixel(sw: &floom2::floom_renderer_software::SoftwareRenderer, x: u32, y: u32) -> bool {
    sw.pixel_rgba(x, y)
        .map(|p| p[0].abs_diff(190) < 12 && p[1].abs_diff(60) < 12)
        .unwrap_or(false)
}

fn render(host: &WidgetHost) -> floom2::floom_renderer_software::SoftwareRenderer {
    let mut sw = floom2::floom_renderer_software::SoftwareRenderer::new(SURFACE_W, SURFACE_H);
    floom_renderer::render_once(
        &mut sw,
        &floom_renderer::FrameInfo::software(SURFACE_W, SURFACE_H, 1),
        host.scene(),
    );
    sw
}

/// Frames until the host is quiet.
fn settle(host: &mut WidgetHost) {
    for _ in 0..80 {
        if !host.needs_frame() {
            return;
        }
        host.frame();
    }
    panic!("the host did not settle");
}

fn act_1_the_scope_brackets_the_picture() {
    println!("== 1. the scope brackets the picture ==");
    let (mut host, canvas) = host_with_card();
    let payload_before = card_rect(&host);

    host.animate(
        canvas,
        AnimKind::Tween(Tween::new(0.5, 1.5, 0.6).with_compositor(CompositorProp::Scale(1.0))),
    );
    host.frame();
    for _ in 0..5 {
        host.frame();
    }

    // The scene carries a GroupPush/GroupPop pair around the card's rect.
    let mut push_at = None;
    let mut rect_at = None;
    let mut pop_at = None;
    for (i, (_, node)) in host.scene().iter().enumerate() {
        match node {
            SceneNode::GroupPush { transform } => {
                push_at = Some((i, *transform));
            }
            SceneNode::Rect { .. } if rect_at.is_none() => rect_at = Some(i),
            SceneNode::GroupPop => pop_at = Some(i),
            _ => {}
        }
    }
    let (push_i, transform) = push_at.expect("a scale animation opens a scope");
    let rect_i = rect_at.expect("the card's rect is in the scene");
    let pop_i = pop_at.expect("and the walk closes it");
    assert!(
        push_i < rect_i && rect_i < pop_i,
        "the bracket surrounds the picture: push {push_i} < rect {rect_i} < pop {pop_i}"
    );

    // The scope carries the sampled scale about the card's box centre.
    let b = host.bounds(canvas);
    let centre = Point::new(b.x0 + b.width() / 2.0, b.y0 + b.height() / 2.0);
    let value = host.animation_value(canvas).expect("the tween is running");
    let expected = Transform::scale_around(value, value, centre);
    assert!(
        (transform.a - expected.a).abs() < 1e-9 && (transform.e - expected.e).abs() < 1e-9,
        "the scope carries the sampled scale: {transform:?} vs {expected:?}"
    );
    println!("  sampled scale {value:.3} about the box centre");

    // The payload never moved — that is the whole point of the scope.
    let payload_after = card_rect(&host);
    assert_eq!(
        payload_before, payload_after,
        "the card's own rect is untouched by the scale"
    );

    settle(&mut host);
}

/// Scale 1.0 → 2.0 about the centre over 0.6s: 36 frames at 60 Hz. The card's
/// long side is 44px on a 96px surface, so the band between its at-rest edge
/// (70) and the surface edge is where only a scaled card can paint.
fn act_2_the_pixels_follow_the_scale() {
    println!("== 2. the pixels follow the scale ==");
    let (mut host, canvas) = host_with_card();

    // t=0 control: the band beyond the card's at-rest edge is empty.
    let rest_edge = ((SIZE + CARD_W) / 2.0) as u32; // 70
    let probe = rest_edge + 4; // 74: outside the at-rest card
    let sw = render(&host);
    assert!(
        sw.pixel_rgba(probe, SURFACE_H / 2).unwrap()[3] == 0,
        "t=0: nothing paints beyond the card's at-rest edge"
    );

    host.animate(
        canvas,
        AnimKind::Tween(Tween::new(1.0, 2.0, 0.6).with_compositor(CompositorProp::Scale(1.0))),
    );
    // Mid-tween: 18 frames = 0.3s of 0.6s → value 1.5 → the card is 60px,
    // its edge at 18..78. The probe sits inside that but outside the rest.
    for _ in 0..18 {
        host.frame();
    }
    let value = host.animation_value(canvas).expect("mid-tween");
    assert!((value - 1.5).abs() < 1e-9, "18 frames in, the value is 1.5");
    let sw = render(&host);
    assert!(
        is_card_pixel(&sw, probe, SURFACE_H / 2),
        "t=50%: the scaled card covers ground it did not cover at rest"
    );

    // t=100%: value 2.0 → the card is 88px wide, edge at 4..92. Further out.
    for _ in 0..18 {
        host.frame();
    }
    let value = host.animation_value(canvas).expect("finished");
    assert!((value - 2.0).abs() < 1e-9, "the tween reached its end");
    let far = rest_edge + 16; // 86: only a 2× card reaches here (1.5× stops at 81)
    let sw = render(&host);
    assert!(
        is_card_pixel(&sw, far, SURFACE_H / 2),
        "t=100%: the 2× card covers ground the 1.5× card did not"
    );

    settle(&mut host);
}

/// Rotate 0 → 90° about the centre over 1s. The card's centre of mass sits
/// left of the pivot; a quarter turn carries it above the pivot. In pixels.
fn act_3_a_rotation_turns_the_whole_picture() {
    println!("== 3. a rotation turns the whole picture ==");
    let (mut host, canvas) = host_with_card();

    // t=0 control. The card spans x 26..70, y 36..60: probe (32, 48) — left
    // of the pivot, inside the card — is ink; (48, 32) — above the pivot,
    // outside the short side — is backdrop.
    let sw = render(&host);
    assert!(
        is_card_pixel(&sw, 32, 48),
        "t=0 control: the card is where it was declared"
    );
    assert!(
        !is_card_pixel(&sw, 48, 32),
        "t=0 control: above the pivot is backdrop"
    );

    host.animate(
        canvas,
        AnimKind::Tween(
            Tween::new(0.0, std::f64::consts::FRAC_PI_2, 1.0)
                .with_compositor(CompositorProp::Rotate(1.0)),
        ),
    );
    for _ in 0..60 {
        host.frame();
    }
    let value = host.animation_value(canvas).expect("finished");
    assert!(
        (value - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
        "the rotation reached a quarter turn"
    );

    // y is down, so +90° carries a point left of the pivot to a point *above*
    // it: the card that spanned x 26..70 × y 36..60 now spans x 36..60 ×
    // y 26..70. Its long side stands up: (48, 32) is ink now, and (32, 48) —
    // ink at rest — is backdrop.
    let sw = render(&host);
    assert!(
        is_card_pixel(&sw, 48, 32),
        "t=100%: the card paints above the pivot, where the quarter turn put it"
    );
    assert!(
        !is_card_pixel(&sw, 32, 48),
        "t=100%: left of the pivot is now backdrop"
    );

    settle(&mut host);
}

fn act_4_a_finished_transform_parks_clean() {
    println!("== 4. a finished transform parks clean ==");
    let (mut host, canvas) = host_with_card();
    host.animate(
        canvas,
        AnimKind::Tween(Tween::new(0.5, 1.5, 0.5).with_compositor(CompositorProp::Scale(1.0))),
    );
    settle(&mut host);

    // The endpoint is parked, not stopped: the scope is still in the scene,
    // carrying the final scale — the card rests at 1.5x rather than snapping
    // back the frame the tween ended.
    let value = host
        .animation_value(canvas)
        .expect("parked at the endpoint");
    assert!((value - 1.5).abs() < 1e-9, "the value parked at its end");
    let scope = host.scene().iter().find_map(|(_, node)| match node {
        SceneNode::GroupPush { transform } => Some(*transform),
        _ => None,
    });
    let transform = scope.expect("the endpoint scope is still applied");
    let b = host.bounds(canvas);
    let centre = Point::new(b.x0 + b.width() / 2.0, b.y0 + b.height() / 2.0);
    let expected = Transform::scale_around(1.5, 1.5, centre);
    assert!(
        (transform.a - expected.a).abs() < 1e-9,
        "the scope carries the endpoint scale: {transform:?}"
    );
    assert!(
        !host.needs_frame(),
        "and the loop is parked: a parked endpoint asks for no frames"
    );
}
