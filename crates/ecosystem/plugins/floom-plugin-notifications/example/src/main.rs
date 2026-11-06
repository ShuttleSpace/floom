// SPDX-License-Identifier: MIT
//! Runs the `notifications` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_notifications::{
    Notification, are_notifications_enabled, request_authorization, schedule,
};

fn main() {
    println!("are_notifications_enabled: {}", are_notifications_enabled());
    if std::env::args().any(|arg| arg == "--send") {
        println!("request_authorization: {:?}", request_authorization());
        let notification = Notification {
            title: "Floom".into(),
            body: "plugin example".into(),
            badge: None,
            sound: true,
            delay_seconds: None,
        };
        match schedule(&notification) {
            Ok(()) => println!("scheduled"),
            Err(e) => println!("schedule: {e}"),
        }
    } else {
        println!("pass --send to request authorisation and schedule a notification");
    }
}
