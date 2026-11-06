// SPDX-License-Identifier: MIT
//! Runs the `package-info` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_package_info::app_info;

fn main() {
    println!("{:#?}", app_info());
}
