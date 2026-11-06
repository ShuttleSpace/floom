// SPDX-License-Identifier: MIT
//! Runs the `connectivity` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_connectivity::{connectivity_status, is_connected, is_metered};

fn main() {
    let status = connectivity_status();
    println!(
        "type={:?} connected={} metered={}",
        status.connection_type, status.is_connected, status.is_metered
    );
    println!(
        "is_connected={} is_metered={}",
        is_connected(),
        is_metered()
    );
}
