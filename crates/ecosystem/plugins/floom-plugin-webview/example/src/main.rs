// SPDX-License-Identifier: MIT
//! Runs the `webview` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that has no native view. A real web view needs
//! a live window and the platform's web engine, which is why `provider` is not
//! called here; `examples/` drives that path.

use floom_plugin_webview::{capability, disable, enable, version};

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
