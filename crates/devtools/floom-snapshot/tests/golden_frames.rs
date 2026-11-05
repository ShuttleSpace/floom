// SPDX-License-Identifier: MIT
//! MOTION-GAP B9's other half: **per-frame** pixel goldens.
//!
//! Everything the 2.0 work has added to the pixel layer — gradients (A3),
//! shadows (A4), filters (S7), subtree transforms (S3), presence transitions
//! (S5) — is currently pinned by tests that sample *one or two pixels* and
//! assert a relation: ink present outside the silhouette, alpha going down, a
//! blurred edge greyer than a sharp one. Those are the right shape for proving
//! a claim once, and the wrong shape for keeping it: each one encodes what the
//! author thought to measure, and a change that moves the rest of the frame
//! passes all of them.
//!
//! A golden frame pins the *whole* frame at *several* points in an animation,
//! so the thing being held is the trajectory. Two design decisions carry that:
//!
//! * **A digest, not a hash.** [`GoldenFrames`] reduces each frame to a lattice
//!   of alpha-weighted block averages and compares cell by cell with a
//!   tolerance, so a one-level rasteriser difference does not turn the suite
//!   red (and does not get "fixed" by re-baselining), while a real regression
//!   reports where and by how much. See `src/frames.rs`.
//! * **A frozen sequence is a failure.** Frames that never differ would match
//!   their golden forever, so `assert` refuses to record them. An animation
//!   that stops animating has to fail here.
//!
//! Every fixture also carries a relation of its own — a card's interior alpha has
//! to rise on a fade-in and fall on an exit, a crossfade's has to stay above
//! zero through the swap — because a golden file alone cannot tell you *why* it
//! is the picture it is, and a relation alone cannot tell you the rest of the
//! frame did not move.
//!
//! Regenerate with
//! `UPDATE_SNAPSHOTS=1 cargo test -p floom-snapshot --test golden_frames`.
//!
//! Font caveat, same as `tests/widgets.rs`: **no fixture here paints glyphs.**
//! A pixel golden over shaped text is pinned to the host's font stack.

use floom_animation::{AnimKind, CompositorProp, Tween};
use floom_app::{NodeId, WidgetHost};
use floom_core::{Child, ChildKey, LayoutRequest, Length};
use floom_layout::LayoutStyle;
use floom_snapshot::{
    FrameDigest, GoldenFrames, PixelSnapshot, ReferencePixels, snapshots_dir,
};
use floom_types::{Brush, Color, PresenceTransition, Size};
use floom_widgets::{Conditional, Container, Crossfade, SharedTransitionLayout, WidgetAny};

/// The frame: 120×60.
const FRAME: Size = Size::new(120.0, 60.0);

/// The card inside it: 80×40, deliberately smaller than the frame.
///
/// A card that fills its frame digests to 144 identical cells — the golden
/// would then pin an average and nothing else, and a change that moves half
/// the card while keeping the mean would still pass. Leaving a margin puts an
/// edge, a corner and a background into the lattice, so a regression has
/// somewhere to show up.
const CARD_W: f32 = 80.0;
const CARD_H: f32 = 40.0;

const INK: Color = Color::rgb(20, 30, 40);
const HOT: Color = Color::rgb(220, 60, 40);

/// The card's own box: a fixed size, so it does not stretch to the viewport.
fn card_style() -> LayoutStyle {
    LayoutStyle {
        width: Some(Length::Px(CARD_W)),
        height: Some(Length::Px(CARD_H)),
        ..LayoutStyle::default()
    }
}

/// Mount `widget` as the root of a host of `size` and run one frame.
fn mounted(
    size: Size,
    style: LayoutStyle,
    widget: impl WidgetAny + 'static,
) -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(size);
    let root = host.mount_root(style, widget);
    host.frame();
    (host, root)
}

/// The host's scene, rasterised. The registry is passed so a scene holding
/// resources resolves them; these fixtures are deliberately glyph-free, but
/// passing it is the habit that keeps a future fixture from silently skipping
/// nodes.
fn pixels(host: &WidgetHost, size: Size) -> PixelSnapshot {
    PixelSnapshot::of(host.scene(), Some(host.resources()), size, 1.0)
}

/// The alpha of the card's interior, taken as the **max** over four cells that
/// are deep inside the 80×40 card no matter how it lands in the 120×60 frame:
/// the card sits at the top-left (0..80, 0..40) here, but these cells are also
/// inside it if it were centred (20..100, 10..50). Grid 12 over 120×60 makes
/// each cell 10×5, so (3,3),(3,4),(4,3),(4,4) cover x 30..50, y 15..25.
///
/// Read from the digest rather than from the raw buffer on purpose: it is the
/// same reduction the golden compares, so a relation asserted here and a golden
/// asserted on the file cannot disagree about what the frame contains. Not the
/// mean over the whole frame — the card covers 44 % of it, so a whole-frame
/// mean tops out at 113 however opaque the card is, and every threshold would
/// be a number quietly divided by a coverage nobody wrote down. Max, not one
/// cell, so a half-collapsed crossfade (a card that drifted off the corner)
/// still reports what it is instead of averaging the background in.
fn card_alpha(g: &GoldenFrames, index: usize) -> u8 {
    let cells = &g.frames()[index].1.cells;
    let deep = [3 * 12 + 3, 3 * 12 + 4, 4 * 12 + 3, 4 * 12 + 4];
    deep.iter()
        .map(|&i| cells[i][3])
        .max()
        .expect("four interior cells")
}

/// A linear opacity tween from `from` to `to` over `secs`.
fn fade(from: f64, to: f64, secs: f64) -> AnimKind {
    AnimKind::Tween(Tween::new(from, to, secs).with_compositor(CompositorProp::Opacity(0.0)))
}

/// Record the frames at `ticks`, advancing the host one nominal frame at a time.
///
/// The host's clock is a pure accumulator (`anim_clock += NOMINAL_FRAME_SECS`),
/// never a wall clock, so frame `n` is always `n/60 s` into the animation and a
/// golden written on one machine means the same instant on another.
fn record(g: &mut GoldenFrames, host: &mut WidgetHost, size: Size, ticks: &[usize]) {
    let last = *ticks.iter().max().expect("at least one tick");
    for n in 1..=last {
        host.frame();
        if ticks.contains(&n) {
            g.frame(&format!("f{n:02}"), &pixels(host, size));
        }
    }
}

#[test]
fn a_fading_card_is_golden_frame_by_frame() {
    let (mut host, root) = mounted(FRAME, card_style(), Container::new().background(INK));
    host.animate(root, fade(0.0, 1.0, 0.3));

    let mut g = GoldenFrames::new();
    // 0.3 s at the nominal step is 18 frames; six samples across it.
    record(&mut g, &mut host, FRAME, &[1, 4, 7, 10, 13, 18]);

    // The relation the golden file cannot state: the card is arriving, so every
    // sampled frame is more opaque than the one before it.
    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.windows(2).all(|p| p[0] < p[1]),
        "a fade-in has to get more visible every sample: {alphas:?}"
    );
    assert!(
        alphas[0] < 64,
        "the first sample is early in the fade, not the settled card: {alphas:?}"
    );
    assert!(
        alphas.last().copied().unwrap_or(0) >= 250,
        "and the last one is all but opaque: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_card_fade_in");
}

#[test]
fn a_fading_gradient_card_is_golden_frame_by_frame() {
    // Same animation, a card made of something: a linear gradient fill plus a
    // shadow. The pixels now depend on A3 and A4 as well as on the clock, so a
    // regression in brush sampling or shadow blur moves these frames even
    // though the fade itself is unchanged.
    let card = Container::new()
        .background(Brush::Linear {
            stops: vec![(0.0, HOT), (1.0, INK)],
            from: floom_types::Point::new(0.0, 0.0),
            to: floom_types::Point::new(CARD_W as f64, CARD_H as f64),
        })
        .shadow(floom_types::Shadow {
            color: Color::rgba(0, 0, 0, 160),
            blur: 6.0,
            offset_x: 3.0,
            offset_y: 3.0,
            spread: 0.0,
        });

    let (mut host, root) = mounted(FRAME, card_style(), card);
    host.animate(root, fade(0.0, 1.0, 0.3));

    let mut g = GoldenFrames::new();
    record(&mut g, &mut host, FRAME, &[1, 7, 13, 18]);

    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.windows(2).all(|p| p[0] < p[1]),
        "the styled card fades in too: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_gradient_card_fade_in");
}

#[test]
fn a_fading_blurred_card_is_golden_frame_by_frame() {
    // S7's content blur, through the same clock. The blur's reach means the
    // frame changes shape as the alpha rises — ink crossing the card's own edge
    // by an amount that depends on the radius — so these frames pin the filter
    // and the animation together.
    let (mut host, root) = mounted(
        FRAME,
        card_style(),
        Container::new().background(INK).blur(4.0),
    );
    host.animate(root, fade(0.0, 1.0, 0.3));

    let mut g = GoldenFrames::new();
    record(&mut g, &mut host, FRAME, &[1, 7, 13, 18]);

    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.windows(2).all(|p| p[0] < p[1]),
        "a blurred card fades in: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_blurred_card_fade_in");
}

/// A child box with a size of its own.
///
/// A `Container` asked for with `LayoutRequest::default()` measures 0×0 — it
/// has no content to give it an intrinsic width — so a presence fixture built
/// that way fades in an *empty* frame and every digest comes back zeroed. The
/// fade still runs, the alpha still rises, and nothing is visible; a single
/// pixel probe would not have caught it either.
fn sized_card(color: Color, index: usize) -> Child {
    Child::new(
        ChildKey::index(index),
        LayoutRequest {
            width: Some(Length::Px(CARD_W)),
            height: Some(Length::Px(CARD_H)),
            ..LayoutRequest::default()
        },
        move || Box::new(Container::new().background(color)),
    )
}

/// A `Conditional` holding one solid card, with a fade on both ends.
fn presence_host(when: bool) -> (WidgetHost, NodeId) {
    let child = sized_card(INK, 1);
    let conditional = Conditional::new(when)
        .show(child)
        .enter(PresenceTransition::fade(0.2))
        .exit(PresenceTransition::fade(0.2));
    mounted(FRAME, LayoutStyle::default(), conditional)
}

#[test]
fn a_presence_child_entering_is_golden_frame_by_frame() {
    let (mut host, _root) = presence_host(true);

    let mut g = GoldenFrames::new();
    // 0.2 s is 12 frames.
    record(&mut g, &mut host, FRAME, &[1, 4, 8, 12]);

    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.windows(2).all(|p| p[0] < p[1]),
        "an entering child is arriving, not popping: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_presence_enter");
}

#[test]
fn a_presence_child_leaving_is_golden_frame_by_frame() {
    // The half that is easy to get wrong and impossible to see in a single
    // frame: the child has to stay *mounted* and keep dimming, so the golden
    // has to hold a frame at every step of the way out. A single "is it gone
    // yet" assertion would pass for an exit that unmounts immediately.
    let (mut host, root) = presence_host(true);
    // Settle the enter first.
    for _ in 0..20 {
        host.frame();
    }

    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);

    let mut g = GoldenFrames::new();
    record(&mut g, &mut host, FRAME, &[1, 4, 8, 12]);

    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.windows(2).all(|p| p[0] > p[1]),
        "a leaving child has to keep dimming, not vanish on the first frame: {alphas:?}"
    );
    assert!(
        alphas.last().copied().unwrap_or(255) < 40,
        "and the last sample is nearly gone: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_presence_exit");
}

#[test]
fn a_crossfade_swap_is_golden_frame_by_frame() {
    let a = sized_card(INK, 1);
    let b = sized_card(HOT, 2);
    let (mut host, root) = mounted(
        FRAME,
        LayoutStyle::default(),
        Crossfade::new(true)
            .a(a)
            .b(b)
            .presence(PresenceTransition::fade(0.2)),
    );
    // Let A rest fully shown before the swap.
    for _ in 0..20 {
        host.frame();
    }

    host.widget_mut::<Crossfade>(root)
        .expect("the root is a crossfade")
        .set_show_a(false);

    let mut g = GoldenFrames::new();
    record(&mut g, &mut host, FRAME, &[1, 4, 8, 12]);

    // Both sides are on screen through the swap, so the frame never goes empty:
    // the composite of a leaving opaque card and an arriving one keeps the
    // mean alpha up even at the crossover. That is the property a plain
    // Conditional swap does not have, and it is what these frames pin.
    let alphas: Vec<u8> = (0..g.frames().len()).map(|i| card_alpha(&g, i)).collect();
    assert!(
        alphas.iter().all(|&a| a >= 180),
        "a crossfade never blanks the frame mid-swap: the composite of a leaving          opaque card and an arriving one stays well above zero, whereas a          sequential swap would pass through a transparent frame: {alphas:?}"
    );

    g.assert(snapshots_dir!(), "frames_crossfade_swap");
}

/// A small solid square child built directly (no glyphs — a golden over shaped
/// text is pinned to the host's font stack).
fn shared_card(color: Color, index: usize) -> Child {
    Child::new(
        ChildKey::index(index),
        LayoutRequest {
            width: Some(Length::Px(20.0)),
            height: Some(Length::Px(20.0)),
            ..LayoutRequest::default()
        },
        move || Box::new(Container::new().background(color)),
    )
}

/// The vertical band of the highest inked row, as a cell index (rows of the
/// 12-lattice; each row is 5 px in a 60 px-tall frame). Measures where the top
/// edge of the card is, which is the coordinate a flight actually changes.
fn top_true(d: &FrameDigest) -> usize {
    (0..12)
        .find(|row| (0..12).any(|col| d.cells[row * 12 + col][3] > 8))
        .unwrap_or(12)
}

#[test]
fn a_shared_element_flight_is_golden_frame_by_frame() {
    // Source square pinned to the layout's top-left, destination 20 px below.
    // Grid 12 over a 60 px frame makes 20 px exactly 4 rows, so a flight over
    // the whole `delta` moves the top edge from cell-row near 0 to near 4.
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let fly = Arc::new(AtomicBool::new(false));
    let layout = SharedTransitionLayout::new(7, 20.0)
        .duration(0.4)
        .fly_signal(fly.clone())
        .source(shared_card(INK, 1))
        .dest(shared_card(HOT, 2));
    let (mut host, _root) = mounted(FRAME, LayoutStyle::default(), layout);

    // Both boxes are laid out but only the source shows at rest (the destination
    // is hidden until the flight lands) — settle that first.
    for _ in 0..5 {
        host.frame();
    }
    assert_eq!(top_true(&host_digest(&host)), 0, "source rests at the top");

    fly.store(true, Ordering::Relaxed);

    let mut g = GoldenFrames::new();
    // 0.4 s at the nominal step is 24 frames; five samples across it, plus one
    // past the landing so the destination's own content is revealed.
    record(&mut g, &mut host, FRAME, &[1, 7, 13, 19, 24, 26]);

    // The flight is a *trajectory*: the top edge only ever travels downward,
    // and it ends up at the destination box's row (≈4), not just somewhere
    // lower.
    let rows: Vec<usize> = (0..g.frames().len())
        .map(|i| top_true(&g.frames()[i].1))
        .collect();
    assert!(
        rows.windows(2).all(|p| p[0] <= p[1]),
        "a one-way flight never travels upward: {rows:?}"
    );
    assert!(
        rows[0] < rows[rows.len() - 1],
        "the top edge has to actually move down across the flight: {rows:?}"
    );
    assert!(
        rows[rows.len() - 1] >= 3,
        "and it lands on the destination's box (row ≈4): {rows:?}"
    );

    // And the destination reveals *its own* content once the flight lands —
    // the source card (INK) is replaced by the destination card (HOT), which is
    // what makes this a shared element rather than a card that walked somewhere.
    let last_cells = &g.frames()[g.frames().len() - 1].1.cells;
    let dest_cell = last_cells[4 * 12];
    assert!(
        dest_cell[0] > 0x80 && dest_cell[2] < 0x60,
        "the landed frame must show the hot destination (r high, b low), not \
         the inky source: {dest_cell:?}"
    );

    g.assert(snapshots_dir!(), "frames_shared_element_flight");
}

/// A one-frame digest helper reused by the shared-flight control assertion.
fn host_digest(host: &WidgetHost) -> FrameDigest {
    FrameDigest::of(&pixels(host, FRAME), 12)
}

#[test]
fn every_frame_golden_is_accounted_for() {
    // `UPDATE_SNAPSHOTS=1` writes what it is asked for and deletes nothing, so
    // a renamed test leaves an orphan behind that no golden points at — and
    // nothing else in the suite would notice.
    if std::env::var("UPDATE_SNAPSHOTS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return;
    }
    let dir = std::path::Path::new(snapshots_dir!()).join("snapshots");
    let owned: Vec<String> = std::fs::read_dir(&dir)
        .expect("snapshots dir must exist")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("frames_"))
        .collect();
    let expected = [
        "frames_card_fade_in.snap",
        "frames_gradient_card_fade_in.snap",
        "frames_blurred_card_fade_in.snap",
        "frames_presence_enter.snap",
        "frames_presence_exit.snap",
        "frames_crossfade_swap.snap",
        "frames_shared_element_flight.snap",
    ];
    for want in expected {
        assert!(
            owned.iter().any(|n| n == want),
            "missing frame golden `{want}` — run with UPDATE_SNAPSHOTS=1"
        );
    }
    assert_eq!(
        owned.len(),
        expected.len(),
        "orphaned frame goldens: {owned:?}"
    );
}
