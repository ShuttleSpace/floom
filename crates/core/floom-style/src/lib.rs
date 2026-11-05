// SPDX-License-Identifier: MIT
//! Style storage with interaction-state selectors and limited inheritance.

mod props;
mod resolve;

pub use props::{StyleProps, StyleState};
pub use resolve::{StyleMap, StyleSet};
