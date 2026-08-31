//! Platform abstractions for OS-level functionality.
//!
//! This module provides cross-platform abstractions for native operating system
//! features like clipboard access, native menus, and file dialogs.

pub(crate) mod clipboard;
#[cfg(any(target_os = "linux", target_os = "freebsd", target_arch = "wasm32", target_os = "android", target_os = "ios"))]
pub(crate) mod context_menu;
pub mod file;
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
)))]
pub mod file_action;
#[cfg(target_os = "android")]
pub(crate) mod android_jni;
#[cfg(target_os = "android")]
pub mod android_file;
#[cfg(target_os = "android")]
pub(crate) mod android_external;
#[cfg(target_os = "ios")]
pub(crate) mod ios_external;
#[cfg(target_os = "android")]
pub(crate) mod android_intent;
#[cfg(target_os = "ios")]
pub(crate) mod ios_ime;
#[cfg(target_os = "ios")]
pub(crate) mod ios_file;
#[cfg(target_os = "ios")]
pub(crate) mod ios_receive;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_bridge;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_external;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_file;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_receive;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_ime;
#[cfg(target_env = "ohos")]
pub(crate) mod ohos_font;
pub mod menu;
pub(crate) mod menu_types;
pub(crate) mod time;
#[cfg(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
))]
pub mod wasm_stubs;

pub use clipboard::{Clipboard, ClipboardError};
pub use file::{FileDialogOptions, FileInfo, FileSpec};
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
)))]
pub use file_action::{open_file, save_as};
#[cfg(target_os = "android")]
pub use android_file::{dispatch_activity_result, open_file, save_as};
#[cfg(target_os = "ios")]
pub use ios_file::{open_file, save_as};
#[cfg(target_env = "ohos")]
pub use ohos_file::{open_file, save_as};
#[cfg(target_os = "android")]
pub use android_intent::dispatch_android_intent;

/// A file shared into the app from another app (Android `ACTION_SEND` /
/// `ACTION_VIEW` intents; iOS `UIApplicationDelegate application:openURL:`).
///
/// On Android the `uri` is a `content://` URI, not a filesystem path — open it
/// through `ContentResolver` rather than as a plain file. On iOS the `uri` is
/// either a decoded filesystem path (for `file://` URLs from "Open In" / share
/// of a real file) or the absolute URL string (for `http(s)://` / custom
/// schemes / universal links). `mime_type` is the declared type when known
/// (Android intent type; iOS does not surface one here, so it is `None`).
/// `display_name` is the file name for file URLs, else `None`.
#[derive(Clone, Debug)]
pub struct SharedFile {
    /// The shared file URI / path (e.g. `content://...` on Android, a filesystem
    /// path or absolute URL on iOS).
    pub uri: String,
    /// MIME type if the sharing mechanism declared one.
    pub mime_type: Option<String>,
    /// Best-effort display name (file name for file URLs, Uri last path segment
    /// on Android).
    pub display_name: Option<String>,
}

/// Register the handler invoked when files are shared into the app (Android /
/// iOS).
///
/// On platforms without a share/intent mechanism this is a no-op. See the
/// `receive_files` capability. On iOS the hook that feeds this handler is
/// installed by `platform::ios_receive::install_openurl_hook` (called from
/// `Application::new_ios`); it covers warm-start shares but misses cold-start
/// shares — see `docs/ios.md` Step 8.
pub fn set_shared_files_handler(handler: Box<dyn Fn(Vec<SharedFile>) + 'static>) {
    #[cfg(target_os = "android")]
    crate::platform::android_intent::set_shared_files_handler(handler);
    #[cfg(target_os = "ios")]
    crate::platform::ios_receive::set_shared_files_handler(handler);
    #[cfg(target_env = "ohos")]
    crate::platform::ohos_receive::set_shared_files_handler(handler);
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    let _ = handler;
}
pub use menu::{Menu, SubMenu};
pub(crate) use time::{Duration, Instant};

/// Open an external URL or app (e.g. a browser for `http(s)://`).
///
/// On Android this launches an `ACTION_VIEW` intent through the JNI bridge
/// (the `open_external` capability is reported as `true`). On other platforms
/// Floem's core does not open URLs itself — desktop apps should use the `open`
/// crate directly (`open::that(url)`), so this returns an error there.
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        crate::platform::android_external::open_url(url)
    }
    #[cfg(target_os = "ios")]
    {
        crate::platform::ios_external::open_url(url)
    }
    #[cfg(target_env = "ohos")]
    {
        crate::platform::ohos_external::open_url(url)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    {
        let _ = url;
        Err(
            "floem::platform::open_url is only implemented on Android, iOS and OpenHarmony; use the `open` crate on desktop"
                .to_string(),
        )
    }
}
