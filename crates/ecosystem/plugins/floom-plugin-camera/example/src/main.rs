// SPDX-License-Identifier: MIT
//! Runs the `camera` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that has no camera half. A real preview needs
//! a live window and a camera, which is why `provider` is not called here.

use floom_plugin_camera::{capability, disable, enable, version};

fn main() {
    println!("version: {}", version());
    println!("capability: {}", capability());
    match enable() {
        Ok(()) => println!("enable: ok"),
        Err(e) => println!("enable: {e}"),
    }
    disable();
    println!("disable: ok");
}
