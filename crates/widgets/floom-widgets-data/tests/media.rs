// SPDX-License-Identifier: MIT
//! Media-control tests: transport state, time formatting, the accessible
//! parts, and pointer interaction.

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{ElementId, KeyPhase, Point, Rect, Widget, WidgetAny};
use floom_types::{Key, KeyEvent, NamedKey, PointerEvent};
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, dispatch, semantics_snapshot,
};
use floom_widgets_data::media::{MediaControls, MediaKind, MediaState, format_time};

const BAR: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 480.0,
    y1: 40.0,
};

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn track(duration: f64) -> MediaState {
    MediaState {
        kind: MediaKind::Audio,
        duration,
        ..MediaState::default()
    }
}

#[test]
fn time_formatting_reads_like_a_transport_display() {
    assert_eq!(format_time(0.0), "0:00");
    assert_eq!(format_time(5.0), "0:05");
    assert_eq!(format_time(65.0), "1:05");
    assert_eq!(format_time(600.0), "10:00");
    // Past an hour the hour field appears, with the minutes still padded.
    assert_eq!(format_time(3725.0), "1:02:05");
    assert_eq!(format_time(3600.0), "1:00:00");
    // Nonsense in, a usable display out — never "-0:01" or "NaN:NaN".
    assert_eq!(format_time(-4.0), "0:00");
    assert_eq!(format_time(f64::NAN), "0:00");
    assert_eq!(format_time(f64::INFINITY), "0:00");
    // Fractional seconds truncate, so the display never reads ahead of the
    // position it is describing.
    assert_eq!(format_time(9.99), "0:09");
}

#[test]
fn a_default_transport_is_paused_at_the_start_at_full_volume() {
    let s = MediaState::default();
    assert!(!s.playing);
    assert_eq!(s.position, 0.0);
    assert_eq!(s.duration, 0.0);
    assert_eq!(s.volume, 1.0);
    assert!(!s.muted);
    assert_eq!(s.rate, 1.0);
    assert!(!s.looping);
    assert!(!s.is_seekable(), "an unknown duration cannot be seeked");
    assert_eq!(s.progress(), 0.0);
}

#[test]
fn play_pause_toggles_and_reports_whether_it_changed() {
    // The bool is "did anything change", like `seek`/`set_volume` — *not* "is it
    // playing now". The event handlers return it verbatim as their handled
    // flag, so pausing returning `false` would stop the control repainting.
    let mut s = track(100.0);
    assert!(s.toggle_play());
    assert!(s.playing);
    assert!(s.toggle_play(), "pausing is every bit as much a change");
    assert!(!s.playing);

    // A finished track restarts rather than staying stuck at the end.
    s.position = 100.0;
    assert!(s.toggle_play());
    assert_eq!(s.position, 0.0);
    assert!(s.playing);
}

#[test]
fn seeking_is_fractional_and_clamped() {
    let mut s = track(200.0);
    assert!(s.seek(0.5));
    assert_eq!(s.position, 100.0);
    assert_eq!(s.progress(), 0.5);

    assert!(s.seek(1.5));
    assert_eq!(s.position, 200.0, "clamped to the end");
    assert!(s.seek(-1.0));
    assert_eq!(s.position, 0.0, "clamped to the start");

    // Seeking to where we already are is not a change.
    assert!(!s.seek(0.0));
}

#[test]
fn seeking_relative_to_the_current_position_walks_the_track() {
    let mut s = track(100.0);
    s.seek(0.5);
    assert!(s.seek_by(10.0));
    assert_eq!(s.position, 60.0);
    assert!(s.seek_by(-1000.0));
    assert_eq!(s.position, 0.0);
}

#[test]
fn an_unseekable_track_refuses_seeks_without_changing_state() {
    // A live stream reports duration 0; seek controls must be inert rather
    // than silently computing a position out of a zero denominator.
    let mut s = track(0.0);
    assert!(!s.seek(0.5));
    assert!(!s.seek_by(10.0));
    assert_eq!(s.position, 0.0);
}

#[test]
fn ticking_advances_by_the_rate_and_stops_at_the_end() {
    let mut s = track(10.0);
    s.playing = true;
    assert!(s.tick(1.0));
    assert_eq!(s.position, 1.0);
    s.rate = 2.0;
    assert!(s.tick(1.0));
    assert_eq!(s.position, 3.0);

    s.tick(100.0);
    assert_eq!(s.position, 10.0);
    assert!(!s.playing, "playback stops at the end");
    // Once stopped, further ticks do nothing at all.
    assert!(!s.tick(1.0));
    assert_eq!(s.position, 10.0);
}

#[test]
fn ticking_a_looping_track_wraps_instead_of_stopping() {
    let mut s = track(10.0);
    s.playing = true;
    s.looping = true;
    s.position = 9.0;
    assert!(s.tick(3.0));
    assert!(s.playing);
    assert!((s.position - 2.0).abs() < 1e-9, "wrapped to {}", s.position);
}

#[test]
fn a_paused_track_does_not_tick() {
    let mut s = track(10.0);
    s.position = 4.0;
    assert!(!s.tick(1.0));
    assert_eq!(s.position, 4.0);
}

#[test]
fn volume_is_clamped_and_muting_preserves_the_level() {
    let mut s = track(10.0);
    assert!(s.set_volume(0.5));
    assert_eq!(s.volume, 0.5);
    assert!(s.set_volume(4.0));
    assert_eq!(s.volume, 1.0);
    assert!(s.set_volume(-1.0));
    assert_eq!(s.volume, 0.0);
    assert!(
        !s.set_volume(0.0),
        "setting the current value is not a change"
    );

    // Muting is a separate flag, so unmuting restores the level the user
    // chose rather than whatever the slider was left at.
    s.set_volume(0.8);
    assert!(s.set_muted(true));
    assert_eq!(s.volume, 0.8);
    assert_eq!(s.effective_volume(), 0.0);
    assert!(s.set_muted(false));
    assert_eq!(s.effective_volume(), 0.8);
}

#[test]
fn the_control_cluster_describes_its_parts() {
    let mut controls = MediaControls::new(track(200.0));
    let parts = controls.parts();
    assert_eq!(parts.len(), 4, "play, time, seek, volume");

    assert_eq!(parts[0].role, SemanticsRole::Button);
    assert_eq!(parts[0].label, "Play");
    assert!(parts[0].state.contains(SemanticsState::FOCUSABLE));

    assert_eq!(parts[1].role, SemanticsRole::StaticText);
    assert!(
        parts[1].label.contains("of 3:20"),
        "got {:?}",
        parts[1].label
    );

    assert_eq!(parts[2].role, SemanticsRole::Slider);
    assert!(parts[2].state.contains(SemanticsState::ENABLED));

    assert_eq!(parts[3].role, SemanticsRole::Slider);
    assert!(parts[3].label.contains("100%"), "got {:?}", parts[3].label);

    controls.state_mut().playing = true;
    assert_eq!(controls.parts()[0].label, "Pause");

    controls.state_mut().muted = true;
    let volume = &controls.parts()[3];
    assert!(volume.label.contains("muted"), "got {:?}", volume.label);
}

#[test]
fn an_unseekable_track_disables_the_seek_slider() {
    let controls = MediaControls::new(track(0.0));
    let seek = &controls.parts()[2];
    assert!(seek.state.contains(SemanticsState::DISABLED));
    assert!(!seek.state.contains(SemanticsState::FOCUSABLE));

    let live = MediaControls::new(track(0.0));
    // A live stream still shows elapsed time; it just has no total.
    assert!(
        !live.parts()[1].label.contains(" of "),
        "got {:?}",
        live.parts()[1].label
    );
}

#[test]
fn the_cluster_summarizes_its_state_for_assistive_tech() {
    let mut controls = MediaControls::new(track(200.0));
    let label = controls.label();
    assert!(label.contains("Audio player"), "got {label:?}");
    assert!(label.contains("paused"), "got {label:?}");
    assert!(label.contains("0:00 of 3:20"), "got {label:?}");

    controls.state_mut().kind = MediaKind::Video;
    controls.state_mut().playing = true;
    let label = controls.label();
    assert!(label.contains("Video player"), "got {label:?}");
    assert!(label.contains("playing"), "got {label:?}");

    let id = allocate(&controls);
    let snap = semantics_snapshot(&controls, id, BAR);
    assert!(snap.contains("Video player"), "got {snap:?}");
}

#[test]
fn the_layout_partitions_its_box_without_overlap() {
    let controls = MediaControls::new(track(200.0));
    let l = controls.layout(BAR);
    // Order matters: transport, time, seek, volume.
    assert!(l.play.x0 < l.time.x0);
    assert!(l.time.x0 < l.seek.x0);
    assert!(l.seek.x0 < l.volume.x0);
    // No overlaps, and everything inside the box.
    assert!(l.play.x1 <= l.time.x0);
    assert!(l.time.x1 <= l.seek.x0);
    assert!(l.seek.x1 <= l.volume.x0);
    assert!(l.volume.x1 <= BAR.x1);
    assert_eq!(l.play.x0, BAR.x0);
    // The seek bar absorbs whatever is left, which is what makes the layout
    // responsive without a per-width table of constants.
    assert!(l.seek.width() > 100.0);

    let wide = controls.layout(Rect { x1: 900.0, ..BAR });
    assert!((wide.seek.width() - l.seek.width() - 420.0).abs() < 1e-6);
}

#[test]
fn clicking_the_play_button_toggles_transport() {
    let mut controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let l = controls.layout(BAR);
    let x = (l.play.x0 + l.play.x1) / 2.0;

    assert!(dispatch(
        &mut controls,
        id,
        BAR,
        PointerEvent::primary_down(Point::new(x, 20.0)).into()
    ));
    assert!(controls.state().playing);
    dispatch(
        &mut controls,
        id,
        BAR,
        PointerEvent::primary_down(Point::new(x, 20.0)).into(),
    );
    assert!(!controls.state().playing);
}

#[test]
fn clicking_the_seek_bar_seeks_proportionally() {
    let mut controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let l = controls.layout(BAR);
    let x = l.seek.x0 + l.seek.width() / 4.0;
    assert!(dispatch(
        &mut controls,
        id,
        BAR,
        PointerEvent::primary_down(Point::new(x, 20.0)).into()
    ));
    assert!(
        (controls.state().position - 50.0).abs() < 1.0,
        "got {}",
        controls.state().position
    );
}

#[test]
fn clicking_an_unseekable_bar_does_nothing() {
    let mut controls = MediaControls::new(track(0.0));
    let id = allocate(&controls);
    let l = controls.layout(BAR);
    let x = l.seek.x0 + l.seek.width() / 2.0;
    assert!(!dispatch(
        &mut controls,
        id,
        BAR,
        PointerEvent::primary_down(Point::new(x, 20.0)).into()
    ));
    assert_eq!(controls.state().position, 0.0);
}

#[test]
fn clicking_the_volume_bar_sets_the_level() {
    let mut controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let l = controls.layout(BAR);
    let x = l.volume.x0 + l.volume.width() * 0.25;
    assert!(dispatch(
        &mut controls,
        id,
        BAR,
        PointerEvent::primary_down(Point::new(x, 20.0)).into()
    ));
    assert!(
        (controls.state().volume - 0.25).abs() < 0.02,
        "got {}",
        controls.state().volume
    );
}

#[test]
fn space_toggles_playback_and_the_arrow_keys_walk_the_track() {
    let mut controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let key = |controls: &mut MediaControls, k: Key| {
        let ev = KeyEvent {
            phase: KeyPhase::Down,
            key: k,
            modifiers: floom_types::Modifiers::NONE,
        };
        dispatch(controls, id, BAR, ev.into())
    };

    assert!(key(&mut controls, Key::Char(' ')));
    assert!(controls.state().playing);

    key(&mut controls, Key::Named(NamedKey::ArrowRight));
    assert!(
        (controls.state().position - 5.0).abs() < 1e-6,
        "step forward"
    );
    key(&mut controls, Key::Named(NamedKey::ArrowLeft));
    assert!((controls.state().position - 0.0).abs() < 1e-6, "step back");

    key(&mut controls, Key::Named(NamedKey::Home));
    assert_eq!(controls.state().position, 0.0);
    key(&mut controls, Key::Named(NamedKey::End));
    assert_eq!(controls.state().position, 200.0);

    key(&mut controls, Key::Named(NamedKey::ArrowUp));
    assert!(
        (controls.state().volume - 1.0).abs() < 1e-6,
        "volume already maxed"
    );
    key(&mut controls, Key::Named(NamedKey::ArrowDown));
    assert!(
        (controls.state().volume - 0.95).abs() < 1e-6,
        "got {}",
        controls.state().volume
    );
}

#[test]
fn update_repaints_only_when_the_state_moved() {
    let mut controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let mut cx = RecordingUpdateContext::new();
    controls.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 1);

    let mut cx = RecordingUpdateContext::new();
    controls.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 0);

    controls.state_mut().playing = true;
    let mut cx = RecordingUpdateContext::new();
    controls.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 1);
}

#[test]
fn the_cluster_paints_its_transport() {
    let controls = MediaControls::new(track(200.0));
    let id = allocate(&controls);
    let mut rec = SceneRecorder::new(BAR);
    controls.paint(&mut rec, id);
    rec.finish();

    let ops = rec.ops().len();
    assert!(ops >= 5, "expected a transport, got {ops} ops");
    assert!(
        ops < 40,
        "the transport should be a handful of ops, got {ops}"
    );
    let texts: Vec<String> = rec
        .ops()
        .iter()
        .filter_map(|op| match op {
            floom_widgets::testkit::PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "0:00 of 3:20"), "got {texts:?}");
}

#[test]
fn a_media_control_is_type_erasable_like_every_other_widget() {
    let controls = MediaControls::new(track(1.0));
    let erased: &dyn WidgetAny = &controls;
    assert!(erased.as_any().downcast_ref::<MediaControls>().is_some());
}
