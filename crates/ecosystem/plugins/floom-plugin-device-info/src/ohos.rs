//! OpenHarmony device info is not wired to this crate yet.

use super::DeviceInfo;

pub fn device_info() -> DeviceInfo {
    DeviceInfo::default()
}

pub fn device_name() -> String {
    "unknown".to_string()
}
