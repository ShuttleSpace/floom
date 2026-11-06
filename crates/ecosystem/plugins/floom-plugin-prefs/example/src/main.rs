// SPDX-License-Identifier: MIT
//! Runs the `prefs` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_prefs::Preferences;

fn main() {
    let prefs = Preferences::open("floom-example").expect("open the example store");
    println!("before: {:?}", prefs.get("greeting"));
    println!("set: {:?}", prefs.set("greeting", "hello"));
    println!("after: {:?}", prefs.get("greeting"));
    println!("remove: {:?}", prefs.remove("greeting"));
}
