// SPDX-License-Identifier: MIT
//! Runs the `mdns` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use std::time::Duration;

use floom_plugin_mdns::browse;

fn main() {
    let service_type = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "_http._tcp.local.".to_string());
    match browse(&service_type, Duration::from_secs(2)) {
        Ok(found) => {
            println!("{} service(s) for {service_type}", found.len());
            for service in found {
                println!("{service:#?}");
            }
        }
        Err(e) => println!("browse: {e}"),
    }
}
