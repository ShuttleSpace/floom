// SPDX-License-Identifier: MIT
//! Runs the `device-info` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_device_info::{device_info, device_name};

fn main() {
    println!("{:#?}", device_info());
    println!("device_name: {}", device_name());
}
