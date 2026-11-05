// SPDX-License-Identifier: MIT
//! Phase 28.3 — idle and power: a static app must not burn frames, and a
//! suspended one must not burn any work at all.
//!
//! "Idle CPU" is a claim about *absence of work*: no frame requests for a
//! tree that stopped changing, no animation progress while the OS has the app
//! suspended, no presents into a surface that is gone. Each test pins one of
//! those absences with a positive control — the work happens again the moment
//! it should.

use floom_app::{AppRunner, NOMINAL_FRAME_SECS, RunStats, WidgetHost};
use floom_layout::LayoutStyle;
use floom_platform::{FakePlatform, Platform, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_widgets::Text;

fn small_window() -> WindowConfig {
    WindowConfig {
        title: "idle-test".into(),
        size: Size::new(64.0, 48.0),
        resizable: true,
    }
}

use floom_types::Size;

/// Pumps until the queue drains and requested frames are presented.
fn settle(platform: &mut FakePlatform, runner: &mut AppRunner<SoftwarePresenter>) -> RunStats {
    let mut stats = RunStats::default();
    for _ in 0..4 {
        stats = runner.pump(platform);
    }
    stats
}

/// A settled tree stops asking the platform for frames: after the content
/// cascade drains, pumping delivers no events, so no new redraw requests may
/// appear — an idle app asks for nothing.
#[test]
fn a_settled_tree_stops_requesting_redraws() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    let label = host.mount_root(LayoutStyle::default(), Text::new("static"));
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    // One out-of-band edit drives a cascade (frame loop asks on its own)…
    if let Some(text) = runner.host_mut().widget_mut::<Text>(label) {
        text.set_content("settled");
    }
    runner.request_frame(&mut platform);
    settle(&mut platform, &mut runner);

    // …and then the requests stop: eight quiet pumps add zero.
    let quiet_before = runner.stats().redraws_requested;
    for _ in 0..8 {
        runner.pump(&mut platform);
    }
    assert_eq!(
        runner.stats().redraws_requested,
        quiet_before,
        "an idle app must not request frames"
    );
    assert!(!runner.host().needs_frame());
}

/// While the platform reports the app suspended, frames are dropped (counted,
/// not painted) and the animation clock is frozen. Resuming repaints and
/// unfreezes the clock — the positive control proving the skip was the
/// suspension, not a broken pipeline.
#[test]
fn a_suspended_runner_skips_frames_and_freezes_animations() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    let _label = host.mount_root(LayoutStyle::default(), Text::new("running"));
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    let frames_before = runner.stats().frames;
    let clock_before = runner.host().anim_clock();
    assert!(
        frames_before >= 1,
        "control: the window painted before suspension"
    );
    assert!(!runner.host().is_paused());

    platform.suspend();
    settle(&mut platform, &mut runner);
    // Something asks for a frame while suspended (an animation callback, a
    // background service): the request must be dropped, not painted.
    runner.request_frame(&mut platform);
    settle(&mut platform, &mut runner);
    assert_eq!(
        runner.stats().frames,
        frames_before,
        "no frames are presented while suspended"
    );
    assert!(
        runner.stats().frames_skipped_suspended >= 1,
        "the dropped frame requests are counted, not silently lost"
    );
    assert_eq!(
        runner.host().anim_clock(),
        clock_before,
        "the animation clock is frozen while suspended"
    );
    assert!(runner.host().is_paused());
    assert!(!runner.presenter_ready(), "the surface was released");

    platform.resume();
    settle(&mut platform, &mut runner);
    assert!(runner.stats().frames > frames_before, "resuming repaints");
    assert!(
        runner.host().anim_clock() > clock_before,
        "animation resumes from where the pause froze it"
    );
    assert!(!runner.host().is_paused());
}

/// A paused host runs frames (a suspended app must still be able to answer a
/// platform-driven repaint with settled pixels) but its animation clock makes
/// no progress — and unpausing resumes from exactly the frozen point.
#[test]
fn a_paused_host_presents_but_does_not_animate() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(64.0, 48.0));
    host.mount_root(LayoutStyle::default(), Text::new("paused"));
    host.frame();
    let clock_before = host.anim_clock();
    assert!(
        clock_before > 0.0,
        "control: an unpaused frame advances the clock"
    );

    host.set_paused(true);
    for _ in 0..5 {
        let stats = host.frame();
        assert_eq!(stats.updated, 0, "a static paused tree stays static");
    }
    assert_eq!(
        host.anim_clock(),
        clock_before,
        "five frames made zero animation progress"
    );

    host.set_paused(false);
    host.frame();
    assert!(
        (host.anim_clock() - (clock_before + NOMINAL_FRAME_SECS)).abs() < 1e-9,
        "unpausing resumes the clock from the frozen point"
    );
}
