// SPDX-License-Identifier: MIT
//! Runs the `network-info` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_network_info::{local_ip, wifi_info};

fn main() {
    println!("local_ip: {:?}", local_ip());
    match wifi_info() {
        Ok(info) => println!("{info:#?}"),
        Err(e) => println!("wifi_info: {e}"),
    }
}
