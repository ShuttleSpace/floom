//! Android device facts via `Build` / `SystemProperty`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::DeviceInfo;

pub fn device_info() -> DeviceInfo {
    // TODO: read `android.os.Build` (MODEL, MANUFACTURER, DEVICE, BRAND) and
    // `Build.VERSION` (RELEASE, SDK_INT) via the JNI bridge in `floom_platform_mobile::jni`.
    DeviceInfo::default()
}

pub fn device_name() -> String {
    // TODO: resolve the user-visible device name. On Android this is not a
    // single public field; fall back to `Build.MODEL` for now.
    "unknown".to_string()
}
