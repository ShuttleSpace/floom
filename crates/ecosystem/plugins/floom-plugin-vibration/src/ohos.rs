//! OpenHarmony haptics have no bridge in this crate yet.

use super::HapticFeedback;

pub fn vibrate(_duration_ms: u32) -> Result<(), String> {
    Err("vibration: OpenHarmony haptics have no bridge in this crate yet".into())
}

pub fn haptic_feedback(_feedback: HapticFeedback) -> Result<(), String> {
    Err("vibration: OpenHarmony haptics have no bridge in this crate yet".into())
}

pub fn can_vibrate() -> bool {
    false
}
