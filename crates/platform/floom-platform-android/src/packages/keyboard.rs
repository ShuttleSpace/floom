// SPDX-License-Identifier: MIT
//! Android keyboard input: the pure key-code table and soft-keyboard state.
//!
//! Both the table and the shared state types live in `floom_platform::mobile`
//! so the two platform crates and their callers see one definition.

pub use floom_platform::mobile::android_table;
pub mod soft_input;
