// SPDX-License-Identifier: MIT
//! P6 — peak memory across frames.
//!
//! A frame allocates: a scene, paint ops, glyph runs, layout output. None of
//! that is wrong — what is wrong is any of it *accumulating*, which is what a
//! leak looks like from the outside: peak RSS climbs frame after frame and
//! nothing ever gives it back.
//!
//! So the assertion is about growth, not about the absolute number. The
//! absolute number is printed because it is the one a competitor comparison
//! quotes, and because a regression in it deserves to be visible in the log
//! before it is visible in a bug report.
//!
//! Peak rather than current RSS is deliberate: it is what `getrusage` reports
//! without a platform API per platform, and a leak moves the peak exactly as
//! fast as it moves the resident size.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_types::{Length, Size};
use floom_widgets::Container;

/// The peak resident set size in bytes, or `None` where there is no cheap way
/// to ask. Windows has no `getrusage`, so the gate simply does not run there —
/// a test that pretends to measure on a platform it cannot see is worse than
/// one that says it skipped.
#[cfg(unix)]
fn peak_rss_bytes() -> Option<u64> {
    // SAFETY: `getrusage` fills the struct it is given and reads nothing else;
    // `RUSAGE_SELF` with a zeroed `rusage` is the documented call.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: as above — one out-parameter, valid for the call.
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if result != 0 {
        return None;
    }
    // macOS reports bytes here and Linux KiB; both are `c_long` on their target.
    let raw = usize::try_from(usage.ru_maxrss).ok()? as u64;
    #[cfg(target_os = "macos")]
    let bytes = raw;
    #[cfg(not(target_os = "macos"))]
    let bytes = raw.saturating_mul(1024);
    Some(bytes)
}

#[cfg(not(unix))]
fn peak_rss_bytes() -> Option<u64> {
    None
}

/// A tree wide enough that a per-node leak shows up in a few hundred frames.
fn wide_tree(rows: usize) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(320.0, 2_000.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    for _ in 0..rows {
        host.mount(
            root,
            LayoutStyle {
                height: Some(Length::Px(1.0)),
                ..LayoutStyle::default()
            },
            Container::new(),
        );
    }
    host
}

/// Three hundred frames of real work must not raise peak RSS by more than a
/// few megabytes.
///
/// Every frame relayouts and repaints: that is the whole tree's worth of layout
/// output and paint ops being produced and dropped, three hundred times. If any
/// of it is retained, 300 × even a few hundred bytes per node runs into tens of
/// megabytes and this test says so.
#[test]
fn frames_do_not_raise_the_memory_high_water_mark() {
    let Some(_) = peak_rss_bytes() else {
        eprintln!("no peak-RSS reading on this platform, so the memory gate does not run here");
        return;
    };

    let mut host = wide_tree(1_000);
    // Warm up: the first frames build the caches every later frame reuses, and
    // counting that as growth would measure startup, not retention.
    for _ in 0..5 {
        host.frame();
    }
    let before = peak_rss_bytes().expect("the reading worked before warm-up");

    for frame in 0..300 {
        // A size change forces layout and a full repaint, so each frame does
        // the whole pipeline rather than reusing the settled path.
        let width = if frame % 2 == 0 { 320.0 } else { 321.0 };
        host.set_size(Size::new(width, 2_000.0));
        host.frame();
    }

    let after = peak_rss_bytes().expect("the reading worked after the frames");
    let growth = after.saturating_sub(before);

    println!(
        "peak RSS: {:.1} MiB before, {:.1} MiB after 300 frames ({:+.2} MiB)",
        before as f64 / (1024.0 * 1024.0),
        after as f64 / (1024.0 * 1024.0),
        growth as f64 / (1024.0 * 1024.0),
    );

    assert!(
        growth < 8 * 1024 * 1024,
        "300 frames raised peak RSS by {:.2} MiB, which is retention, not churn",
        growth as f64 / (1024.0 * 1024.0)
    );
}
