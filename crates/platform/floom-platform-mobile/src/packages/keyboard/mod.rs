// SPDX-License-Identifier: MIT
//! Android keyboard input mapping + soft-keyboard state.
//!
//! * [`android_table`] — NDK key-code → Floom key (pure, always compiled).
//! * [`soft_input`] — soft-keyboard visibility / height with hysteresis.
//!
//! See [`android_table`] for the mapping caveats (missing `F1`–`F12`, dropped
//! `FUNCTION`/`CAPS_LOCK` modifiers, lower-case letter convention).

pub mod android_table;
pub mod soft_input;
