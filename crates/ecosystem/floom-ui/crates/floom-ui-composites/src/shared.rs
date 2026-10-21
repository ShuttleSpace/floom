//! `SharedNumber` -- a shared number.
//!
//! The engine's shared family has a flag, a buffer and a map, and no number.
//! So it has one now: a number behind a mutex, in the same shape as the
//! other three, so that a value which changes after mount can cross a slot
//! boundary.
//!
//! The numbers the floem-ui kits needed were not the same number: a
//! parallax's scroll offset and a scroller's content height are in layout
//! units (`f32`, like the engine's own `LayoutRequest`), while a scroller's
//! pixel offset is in the engine's scroll units (`f64`, like `ScrollOffset`).
//! The `f32` one is this.

use std::sync::{Arc, Mutex};

/// A shared number.
pub type SharedNumber = Arc<Mutex<f32>>;

/// A fresh shared number.
pub fn shared_number(value: f32) -> SharedNumber {
    Arc::new(Mutex::new(value))
}

/// Read the shared number, treating a poisoned mutex as zero.
///
/// The same posture as the engine's `read_flag`: a caller that poisoned the
/// mutex is as stuck as the widget is, and panicking here would take down a
/// paint pass over a number nobody was looking at.
pub fn read_number(number: &SharedNumber) -> f32 {
    number.lock().map(|guard| *guard).unwrap_or(0.0)
}

/// Write the shared number. Reports whether it changed, so a caller can skip a
/// frame.
pub fn write_number(number: &SharedNumber, value: f32) -> bool {
    match number.lock() {
        Ok(mut guard) if *guard != value => {
            *guard = value;
            true
        }
        _ => false,
    }
}
