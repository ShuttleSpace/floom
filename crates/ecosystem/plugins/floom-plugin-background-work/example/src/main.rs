// SPDX-License-Identifier: MIT
//! Runs the `background-work` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use std::time::{Duration, SystemTime};

fn main() {
    let at = SystemTime::now() + Duration::from_secs(60);
    match floom_plugin_background_work::schedule_app_launch(at) {
        Ok(()) => println!("scheduled an app launch at {at:?}"),
        Err(e) => println!("schedule_app_launch: {e}"),
    }
    match floom_plugin_background_work::cancel_scheduled_app_launch() {
        Ok(()) => println!("cancelled the pending launch"),
        Err(e) => println!("cancel_scheduled_app_launch: {e}"),
    }
}
