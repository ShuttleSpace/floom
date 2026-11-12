//! A read-out of what the interactive samples do.
//!
//! Two of the gallery's samples are deliberately interactive: the button counts
//! its taps, and the checkbox remembers whether it is ticked. Their state
//! lives here rather than inside the sample, because a sample's state is
//! built inside a closure the host rebuilds, and a test has to be able to
//! read the result of a press.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// How many times the button sample has been tapped.
static BUTTON_TAPS: AtomicUsize = AtomicUsize::new(0);

/// Whether the checkbox sample is ticked.
static CHECKBOX_TICKED: AtomicBool = AtomicBool::new(false);

/// Record a tap on the button sample.
pub fn button_tapped() {
    BUTTON_TAPS.fetch_add(1, Ordering::SeqCst);
}

/// How many taps the button sample has counted.
pub fn button_taps() -> usize {
    BUTTON_TAPS.load(Ordering::SeqCst)
}

/// Record the checkbox sample's new ticked state.
pub fn checkbox_changed(ticked: bool) {
    CHECKBOX_TICKED.store(ticked, Ordering::SeqCst);
}

/// Whether the checkbox sample is ticked.
pub fn checkbox_ticked() -> bool {
    CHECKBOX_TICKED.load(Ordering::SeqCst)
}

/// Forget everything the samples have recorded.
///
/// The counters are process-wide, so a test that wants to count from zero
/// starts by forgetting.
pub fn reset() {
    BUTTON_TAPS.store(0, Ordering::SeqCst);
    CHECKBOX_TICKED.store(false, Ordering::SeqCst);
}
