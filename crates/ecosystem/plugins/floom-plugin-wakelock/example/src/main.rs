// SPDX-License-Identifier: MIT
//! Runs the `wakelock` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_wakelock::{is_keep_screen_on, keep_screen_on};

fn main() {
    println!("before: {}", is_keep_screen_on());
    println!("keep_screen_on(true): {:?}", keep_screen_on(true));
    println!("after: {}", is_keep_screen_on());
    println!("keep_screen_on(false): {:?}", keep_screen_on(false));
}
