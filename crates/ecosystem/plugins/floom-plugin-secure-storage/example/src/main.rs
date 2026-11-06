// SPDX-License-Identifier: MIT
//! Runs the `secure-storage` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_secure_storage::SecureStorage;

fn main() {
    let store = SecureStorage::open("floom-example").expect("open the example store");
    println!("before: {:?}", store.get("token"));
    println!("set: {:?}", store.set("token", "example"));
    println!("after: {:?}", store.get("token"));
    println!("remove: {:?}", store.remove("token"));
}
