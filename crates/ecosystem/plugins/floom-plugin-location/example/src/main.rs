// SPDX-License-Identifier: MIT
//! Runs the `location` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_location::{
    authorization_status, current_location, is_location_enabled, request_authorization,
};

fn main() {
    println!("authorization_status: {:?}", authorization_status());
    println!("is_location_enabled: {}", is_location_enabled());
    if std::env::args().any(|arg| arg == "--read") {
        println!("request_authorization: {:?}", request_authorization());
        match current_location() {
            Ok(fix) => println!("{fix:?}"),
            Err(e) => println!("current_location: {e}"),
        }
    } else {
        println!("pass --read to request authorisation and read one fix");
    }
}
