// SPDX-License-Identifier: MIT
//! Runs the `vibration` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_vibration::{HapticFeedback, can_vibrate, haptic_feedback, vibrate};

fn main() {
    println!("can_vibrate: {}", can_vibrate());
    println!("vibrate(100): {:?}", vibrate(100));
    println!(
        "haptic(Light): {:?}",
        haptic_feedback(HapticFeedback::Light)
    );
}
