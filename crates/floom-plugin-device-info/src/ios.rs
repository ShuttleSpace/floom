//! iOS device facts via `UIDevice`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::DeviceInfo;

pub fn device_info() -> DeviceInfo {
    // TODO: read `UIDevice.currentDevice` (model, systemName, systemVersion)
    // and map the `model` string to a marketing name if needed.
    DeviceInfo::default()
}

pub fn device_name() -> String {
    // TODO: read `UIDevice.currentDevice.name`.
    "unknown".to_string()
}
