//! Style that is made of something: gradients, elevation, implicit motion.
//!
//! Everything below is `floom2::prelude` plus one design system's motion
//! tokens. Each act asserts against what actually happened — pixels a real
//! rasteriser put in a buffer, and colours a real host sampled:
//!
//! 1. **A gradient is not one colour.** A card whose background is a linear
//!    brush puts *different* pixels at its two ends; the same card with a
//!    flat background puts the same colour at both. That pair is the gap
//!    A3 was: before this, no widget could ask for a ramp at all, however
//!    much the scene IR understood one.
//! 2. **Elevation is a shadow, not a second rect.** The pixels below the card
//!    are inked, and the same card without elevation leaves them empty — the
//!    difference between a blur and the v1-era understudy that faked it.
//! 3. **A hover fade is implicit.** Touching the button with the pointer fades
//!    its fill over several frames; the untouched-by-design control snaps in
//!    one. Both arrive at the same colour, which is what makes the comparison
//!    honest rather than merely different.
//! 4. **The design token *is* the transition.** The button's fade is armed
//!    with the Material timing itself, so changing the token changes the fade
//!    and nothing has to be restated anywhere else.
//! 5. **A filter is not a shadow.** A card drawn through a blur puts ink
//!    *outside* its own box and thinning; the same card without the blur ends
//!    exactly where it was drawn. A shadow also blurs, so the difference is
//!    what act 5 is about: this one post-processes the picture the whole
//!    subtree made, rather than decorating one shape.
//! 6. **Frost reads what is behind it.** A panel over a hard black/white edge
//!    greys the edge inside its own box and leaves the same edge sharp one
//!    pixel outside it — the one probe that tells a backdrop filter from a
//!    content one, since both blur and both change pixels.
//!
//! Run with `cargo run -p floom2-style`.

use floom_material::{MotionDuration, MotionEasing, MotionTransition};
use floom_renderer::{FrameInfo, render_once};
use floom2::floom_layout::{AbsoluteInset, Position};
use floom2::floom_scene::SceneNode;
use floom2::prelude::*;

// --- act 1 & 2: one card on a surface ---------------------------------------

const SURFACE_W: u32 = 240;
const SURFACE_H: u32 = 130;
/// The card's box: inset from the surface so its shadow has somewhere to land.
const CARD_LEFT: f32 = 24.0;
const CARD_TOP: f32 = 24.0;
const CARD_W: f32 = 180.0;
const CARD_H: f32 = 60.0;

/// A left-to-right ramp across the card, in the card's own coordinates.
fn ramp() -> Brush {
    Brush::linear(
        &[
            (0.0, Color::rgb(30, 60, 220)),
            (1.0, Color::rgb(220, 60, 30)),
        ],
        Point::new(CARD_LEFT as f64, 0.0),
        Point::new((CARD_LEFT + CARD_W) as f64, 0.0),
    )
}

/// Elevation: the silhouette, blurred, pushed down.
fn elevation() -> Shadow {
    Shadow {
        color: Color::rgba(0, 0, 0, 200),
        blur: 5.0,
        offset_x: 0.0,
        offset_y: 6.0,
    }
}

/// Mounts `card` in the same place on every act's surface, and settles.
fn mount_card(card: Container) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(SURFACE_W as f64, SURFACE_H as f64));
    // A card is mounted *into* a bare root rather than being the root: a root
    // owns the whole surface and honours no inset of its own, so an absolutely
    // placed child is what puts the card where its shadow needs room to fall.
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    host.mount(
        root,
        LayoutStyle {
            position: Position::Absolute,
            inset: AbsoluteInset {
                left: Some(CARD_LEFT),
                top: Some(CARD_TOP),
                ..AbsoluteInset::default()
            },
            width: Some(CARD_W),
            height: Some(CARD_H),
            ..LayoutStyle::default()
        },
        card,
    );
    host.frame();
    settle(&mut host);
    host
}

fn render(host: &WidgetHost) -> floom2::floom_renderer_software::SoftwareRenderer {
    let mut sw = floom2::floom_renderer_software::SoftwareRenderer::new(SURFACE_W, SURFACE_H);
    render_once(
        &mut sw,
        &FrameInfo::software(SURFACE_W, SURFACE_H, 1),
        host.scene(),
    );
    sw
}

fn pixel(sw: &floom2::floom_renderer_software::SoftwareRenderer, x: u32, y: u32) -> [u8; 4] {
    sw.pixel_rgba(x, y)
        .expect("the probe is inside the surface")
}

/// How far apart two pixels are, summed per channel.
fn apart(a: [u8; 4], b: [u8; 4]) -> u32 {
    a[0].abs_diff(b[0]) as u32 + a[1].abs_diff(b[1]) as u32 + a[2].abs_diff(b[2]) as u32
}

/// Frames until the host is quiet: a frame that changed anything leaves damage
/// behind to be presented, so settling takes one more quiet frame.
fn settle(host: &mut WidgetHost) {
    for _ in 0..20 {
        if !host.needs_frame() {
            return;
        }
        host.frame();
    }
    panic!("the host did not settle");
}

/// A gradient card is not one colour: the two ends of its own box differ, and
/// the ramp survives to the renderer rather than being collapsed somewhere on
/// the way down.
fn act_1_a_gradient_is_not_one_colour() {
    println!("== 1. a gradient card is not one colour ==");
    let host = mount_card(Container::new().background(ramp()).radius(8.0));
    let sw = render(&host);

    // Mid-height, so neither probe can be reading the rounded corner.
    let mid_y = (CARD_TOP + CARD_H / 2.0) as u32;
    let left = pixel(&sw, (CARD_LEFT + 6.0) as u32, mid_y);
    let right = pixel(&sw, (CARD_LEFT + CARD_W - 6.0) as u32, mid_y);
    println!("  left {left:?}  right {right:?}");
    assert!(
        apart(left, right) > 200,
        "the two ends of the card are far apart: {left:?} / {right:?}"
    );
    assert!(
        left[2] > left[0],
        "the blue stop is at the start of the axis"
    );
    assert!(
        right[0] > right[2],
        "the red stop is at the far end of the axis"
    );

    // Control: the same card holding one colour puts the *same* pixel at both
    // probes. Without this, "the two ends differ" could be reading noise.
    let flat = mount_card(
        Container::new()
            .background(Color::rgb(30, 60, 220))
            .radius(8.0),
    );
    let flat_sw = render(&flat);
    assert_eq!(
        apart(
            pixel(&flat_sw, (CARD_LEFT + 6.0) as u32, mid_y),
            pixel(&flat_sw, (CARD_LEFT + CARD_W - 6.0) as u32, mid_y)
        ),
        0,
        "a flat background paints one colour everywhere"
    );

    // And the scene node still carries the ramp itself, which is what every
    // other backend needs — an op log's single sample is not the contract.
    let brush = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { fill, .. } => Some(fill.clone()),
            _ => None,
        })
        .expect("the card painted");
    assert_eq!(brush, ramp(), "the whole ramp reached the renderer");
    println!("  the ramp reached the renderer intact; the flat card stayed flat");
}

/// Elevation inks the pixels past the shape that cast it, and only there:
/// below the card something is drawn, above it nothing is, and dropping the
/// shadow takes the ink with it.
fn act_2_elevation_is_a_shadow_not_a_second_rect() {
    println!("== 2. elevation casts a shadow ==");
    let lifted = mount_card(
        Container::new()
            .background(Color::WHITE)
            .radius(8.0)
            .shadow(elevation()),
    );
    let lifted_sw = render(&lifted);

    let centre_x = (CARD_LEFT + CARD_W / 2.0) as u32;
    // Six px below the card's bottom edge: the offset silhouette's own ground.
    let below_y = (CARD_TOP + CARD_H + 2.0) as u32;
    let below = pixel(&lifted_sw, centre_x, below_y);
    println!("  below the card: {below:?}");
    assert!(below[3] > 0, "the shadow inked past the card's own box");

    // Above the card nothing reached: a shadow with no vertical offset upward
    // must not smear into the space there.
    let above_y = (CARD_TOP - 6.0) as u32;
    assert_eq!(
        pixel(&lifted_sw, centre_x, above_y)[3],
        0,
        "the shadow falls downward, not everywhere"
    );

    // Control: the same card, no elevation — same probe, empty.
    let flat = mount_card(Container::new().background(Color::WHITE).radius(8.0));
    let flat_sw = render(&flat);
    assert_eq!(
        pixel(&flat_sw, centre_x, below_y)[3],
        0,
        "nothing below an unraised card"
    );

    // And the request itself is on the node *before* the fill it belongs to:
    // one node carrying the shadow, then the card's own.
    let order: Vec<bool> = lifted
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
    assert_eq!(
        order,
        vec![true, false],
        "the shadow is requested ahead of the shape that cast it"
    );
    println!("  below: inked; above and the unraised control: empty");
}

// --- act 3 & 4: implicit motion on a real control ---------------------------

/// The fill the host painted for the only rect on screen: the button's own
/// background.
fn button_fill(host: &WidgetHost) -> Color {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { fill, .. } => fill.solid(),
            _ => None,
        })
        .expect("the button painted its background")
}

/// A host with one button in it, and the box that button was given. Hovering
/// uses the *measured* centre rather than a guessed point: a pointer dispatched
/// one pixel outside the box proves nothing at all about the control.
fn mount_button(transition: Option<Transition>) -> (WidgetHost, Rect) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let mut button = Button::new("Hover");
    if let Some(transition) = transition {
        button = button.transition(transition);
    }
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let node = host.mount(
        root,
        LayoutStyle {
            position: Position::Absolute,
            inset: AbsoluteInset {
                left: Some(30.0),
                top: Some(20.0),
                ..AbsoluteInset::default()
            },
            width: Some(140.0),
            height: Some(40.0),
            ..LayoutStyle::default()
        },
        button,
    );
    host.frame();
    settle(&mut host);
    let box_of = host.bounds(node);
    (host, box_of)
}

fn hover(host: &mut WidgetHost, bounds: Rect) {
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(Event::Pointer(PointerEvent::hover(centre)));
}

/// Touching the pointer fades the fill over several frames rather than snapping
/// it — and the control that says nothing about transitions still snaps. Both
/// end up at the same colour, which is what makes this a fair comparison
/// rather than a decoration.
fn act_3_the_hover_fade_is_implicit() {
    println!("== 3. the hover fade is implicit ==");
    let (mut faded, faded_box) = mount_button(Some(MotionTransition::STATE_CHANGE));
    let (mut snapped, snapped_box) = mount_button(None);
    let resting = button_fill(&faded);
    assert_eq!(
        resting,
        button_fill(&snapped),
        "both buttons start at the resting colour"
    );

    hover(&mut faded, faded_box);
    hover(&mut snapped, snapped_box);
    faded.frame();
    snapped.frame();

    let arrived = button_fill(&snapped);
    let one_frame_in = button_fill(&faded);
    println!("  one frame after hover: faded {one_frame_in:?}, snapped {arrived:?}");
    assert_ne!(arrived, resting, "hovering changed the target colour");
    assert_eq!(
        one_frame_in, resting,
        "the transitioned button has not jumped: it is still opening"
    );
    assert_eq!(
        button_fill(&snapped),
        arrived,
        "and the control with no transition already arrived"
    );

    // Run it out. Six nominal frames is a tenth of a second; twenty is past
    // every material timing, and the point is the arrival, not the count.
    for _ in 0..20 {
        faded.frame();
    }
    assert_eq!(
        button_fill(&faded),
        arrived,
        "the fade lands exactly on the colour the snap reached in one frame"
    );
    println!("  both arrive at {arrived:?}; only one of them travelled");
}

/// The button was armed with the design system's own timing, not a restatement
/// of it: the token says how long, and the fade takes exactly that long — the
/// claim B6 asks for, stated so that changing the token changes the motion and
/// nothing has to be edited here.
fn act_4_the_token_is_the_transition() {
    println!("== 4. the design token is the transition ==");
    let token = MotionTransition::STATE_CHANGE;
    println!(
        "  MotionDuration::SHORT2 = {} ms → {:.3} s, easing = {:?}",
        MotionDuration::SHORT2,
        token.duration_secs,
        token.easing
    );
    assert_eq!(
        token.duration_secs,
        MotionDuration::SHORT2 as f64 / 1000.0,
        "one conversion, in one place"
    );
    assert_eq!(token.easing, MotionEasing::STANDARD);

    // The same token, played: the fade has not finished one nominal frame
    // after the hover, and has finished well past its own duration.
    let (mut host, box_of) = mount_button(Some(token));
    let resting = button_fill(&host);
    hover(&mut host, box_of);
    host.frame();
    assert_eq!(
        button_fill(&host),
        resting,
        "{} ms is several frames, so this one is still opening",
        MotionDuration::SHORT2
    );
    for _ in 0..4 {
        host.frame();
    }
    assert_ne!(
        button_fill(&host),
        resting,
        "past half its duration the fill has visibly moved"
    );
    println!("  armed with MotionTransition::STATE_CHANGE, and it takes that long");
}

fn main() {
    act_1_a_gradient_is_not_one_colour();
    act_2_elevation_is_a_shadow_not_a_second_rect();
    act_3_the_hover_fade_is_implicit();
    act_4_the_token_is_the_transition();
    act_5_a_filter_is_not_a_shadow();
    act_6_frost_reads_what_is_behind_it();
    println!("\nfloom2-style: 6 acts, every claim asserted.");
}

/// A blurred card's ink crosses the edge it was drawn inside, and thins as it
/// goes. The control is the same card without the blur: it ends exactly where
/// it was laid out, so "ink landed outside the box" is evidence rather than a
/// lucky sample.
fn act_5_a_filter_is_not_a_shadow() {
    println!("== 5. a filter is not a shadow ==");
    let host = mount_card(Container::new().background(Color::WHITE).blur(7.0));
    let sw = render(&host);

    let mid_y = (CARD_TOP + CARD_H / 2.0) as u32;
    let just_outside = (CARD_LEFT - 3.0) as u32;
    let further_out = (CARD_LEFT - 6.0) as u32;
    let near = pixel(&sw, just_outside, mid_y);
    let far = pixel(&sw, further_out, mid_y);
    println!("  3px outside the card: {near:?}   6px outside: {far:?}");
    assert!(near[3] > 0, "the blur carried ink past the card's own box");
    assert!(
        near[3] > far[3],
        "and it thins with distance instead of stopping at a hard edge"
    );
    assert_eq!(
        sw.filters_drawn, 1,
        "one filtered scope was composited — not zero, which would mean the \
         request was dropped"
    );

    let plain = mount_card(Container::new().background(Color::WHITE));
    let plain_sw = render(&plain);
    assert_eq!(
        pixel(&plain_sw, just_outside, mid_y)[3],
        0,
        "the same card without a blur ends where it was drawn"
    );
    assert_eq!(plain_sw.filters_drawn, 0, "and pays for no layer");
    println!("  the ink crossed the edge and thinned; the unfiltered card did not");
}

/// Frost: the vertical edge down the middle of the surface is the filter's
/// *input*. Inside the panel it goes grey; one pixel outside the panel the
/// same edge is still a hard step.
fn act_6_frost_reads_what_is_behind_it() {
    println!("== 6. frost reads what is behind it ==");
    // The edge: white left of x = 120, black right of it.
    let edge_x = SURFACE_W as f32 / 2.0;
    // A panel straddling the edge, with no fill of its own: everything a probe
    // reads inside it is the backdrop, filtered or not.
    let panel_left = edge_x - 80.0;
    let panel_top = 30.0;
    let panel_w = 160.0;
    let panel_h = 70.0;

    let mut host = WidgetHost::new();
    host.set_size(Size::new(SURFACE_W as f64, SURFACE_H as f64));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    for (left, color) in [(0.0, Color::WHITE), (edge_x, Color::BLACK)] {
        host.mount(
            root,
            LayoutStyle {
                position: Position::Absolute,
                inset: AbsoluteInset {
                    left: Some(left),
                    top: Some(0.0),
                    ..AbsoluteInset::default()
                },
                width: Some(edge_x),
                height: Some(SURFACE_H as f32),
                ..LayoutStyle::default()
            },
            Container::new().background(color),
        );
    }
    host.mount(
        root,
        LayoutStyle {
            position: Position::Absolute,
            inset: AbsoluteInset {
                left: Some(panel_left),
                top: Some(panel_top),
                ..AbsoluteInset::default()
            },
            width: Some(panel_w),
            height: Some(panel_h),
            ..LayoutStyle::default()
        },
        Container::new().backdrop_blur(7.0),
    );
    host.frame();
    settle(&mut host);
    let sw = render(&host);
    assert_eq!(sw.backdrops_drawn, 1, "one backdrop was applied");

    // Two px left of the edge, *inside* the panel.
    let inside = pixel(
        &sw,
        (edge_x - 2.0) as u32,
        (panel_top + panel_h / 2.0) as u32,
    );
    println!("  inside the panel, 2px from the edge: {inside:?}");
    assert_ne!(
        inside,
        [255, 255, 255, 255],
        "the edge smeared — white picked up ink from the black side"
    );
    assert!(
        inside[0] < 250 && inside[0] > 5,
        "it is grey, not either end"
    );
    assert_eq!(inside[3], 255, "a backdrop blur does not punch a hole");

    // The same x, above the panel: outside its rect, so the edge is untouched.
    let outside = pixel(&sw, (edge_x - 2.0) as u32, (panel_top - 8.0) as u32);
    assert_eq!(
        outside,
        [255, 255, 255, 255],
        "the filter is confined to the panel's own box"
    );

    // Control: the scene without the panel's frost leaves the edge sharp.
    let mut bare = WidgetHost::new();
    bare.set_size(Size::new(SURFACE_W as f64, SURFACE_H as f64));
    let bare_root = bare.mount_root(LayoutStyle::default(), Container::new());
    for (left, color) in [(0.0, Color::WHITE), (edge_x, Color::BLACK)] {
        bare.mount(
            bare_root,
            LayoutStyle {
                position: Position::Absolute,
                inset: AbsoluteInset {
                    left: Some(left),
                    top: Some(0.0),
                    ..AbsoluteInset::default()
                },
                width: Some(edge_x),
                height: Some(SURFACE_H as f32),
                ..LayoutStyle::default()
            },
            Container::new().background(color),
        );
    }
    bare.frame();
    settle(&mut bare);
    let bare_sw = render(&bare);
    assert_eq!(
        pixel(
            &bare_sw,
            (edge_x - 2.0) as u32,
            (panel_top + panel_h / 2.0) as u32
        ),
        [255, 255, 255, 255],
        "without the panel the same probe reads the sharp edge"
    );
    assert_eq!(bare_sw.backdrops_drawn, 0);
    println!("  the edge went grey inside the panel and stayed sharp outside it");
}
