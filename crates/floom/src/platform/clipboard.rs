use parking_lot::Mutex;
use raw_window_handle::RawDisplayHandle;

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
)))]
use copypasta::{ClipboardContext, ClipboardProvider};

#[cfg(target_os = "ios")]
mod ios_clipboard;

static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

pub struct Clipboard {
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    clipboard: Box<dyn ClipboardProvider>,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    #[allow(dead_code)]
    selection: Option<Box<dyn ClipboardProvider>>,
}

#[cfg(target_env = "ohos")]
mod ohos_clipboard;

#[cfg(target_os = "android")]
mod android_clipboard {
    //! JNI bridge to the Android system `ClipboardManager`.
    //!
    //! Android has no native clipboard API accessible from Rust; the system
    //! clipboard is exposed through `android.content.ClipboardManager`, reached
    //! via JNI from the app's `Activity`. The JNI handles are captured once at
    //! startup in [`crate::platform::android_jni`] and attached per call.

    use jni::objects::{JObject, JString, JValue};

    use crate::platform::ClipboardError;

    /// Read the primary clip as text via `android.content.ClipboardManager`.
    pub(crate) fn get_contents(
        env: &mut jni::JNIEnv,
        activity: &JObject<'static>,
    ) -> Result<String, ClipboardError> {
        let service = env
            .new_string("clipboard")
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        let cm = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        if cm.is_null() {
            return Err(ClipboardError::ProviderError(
                "ClipboardManager unavailable".to_string(),
            ));
        }

        let clip = env
            .call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[])
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        if clip.is_null() {
            return Ok(String::new());
        }

        let item = env
            .call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[JValue::Int(0)])
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        if item.is_null() {
            return Ok(String::new());
        }

        let text = env
            .call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[])
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        if text.is_null() {
            return Ok(String::new());
        }

        let s = env
            .call_method(&text, "toString", "()Ljava/lang/String;", &[])
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        let s = JString::from(s);
        let chars = env
            .get_string(&s)
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        Ok(chars.into())
    }

    /// Write `contents` as the primary clip via `android.content.ClipboardManager`.
    pub(crate) fn set_contents(
        env: &mut jni::JNIEnv,
        activity: &JObject<'static>,
        contents: &str,
    ) -> Result<(), ClipboardError> {
        let label = env
            .new_string("floom")
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        let text = env
            .new_string(contents)
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&text)],
            )
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;

        let service = env
            .new_string("clipboard")
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        let cm = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?
            .l()
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        if cm.is_null() {
            return Err(ClipboardError::ProviderError(
                "ClipboardManager unavailable".to_string(),
            ));
        }

        env.call_method(
            &cm,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )
        .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum ClipboardError {
    NotAvailable,
    ProviderError(String),
    PathError(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardError::NotAvailable => write!(f, "clipboard not available"),
            ClipboardError::ProviderError(e) => write!(f, "clipboard provider error: {e}"),
            ClipboardError::PathError(e) => write!(f, "clipboard path error: {e}"),
        }
    }
}

impl Clipboard {
    pub fn get_contents() -> Result<String, ClipboardError> {
        #[cfg(target_os = "android")]
        {
            crate::platform::android_jni::AndroidJni::with_env(|env, activity| {
                android_clipboard::get_contents(env, activity)
            })
            .unwrap_or(Err(ClipboardError::NotAvailable))
        }
        #[cfg(target_env = "ohos")]
        {
            ohos_clipboard::get_contents()
        }
        #[cfg(target_os = "ios")]
        {
            ios_clipboard::get_contents()
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_env = "ohos"
        )))]
        {
            CLIPBOARD
                .lock()
                .as_mut()
                .ok_or(ClipboardError::NotAvailable)?
                .clipboard
                .get_contents()
                .map_err(|e| ClipboardError::ProviderError(e.to_string()))
        }
    }

    pub fn set_contents(s: String) -> Result<(), ClipboardError> {
        if s.is_empty() {
            return Err(ClipboardError::ProviderError(
                "content is empty".to_string(),
            ));
        }
        #[cfg(target_os = "android")]
        {
            crate::platform::android_jni::AndroidJni::with_env(|env, activity| {
                android_clipboard::set_contents(env, activity, &s)
            })
            .unwrap_or(Err(ClipboardError::NotAvailable))
        }
        #[cfg(target_env = "ohos")]
        {
            ohos_clipboard::set_contents(&s)
        }
        #[cfg(target_os = "ios")]
        {
            ios_clipboard::set_contents(&s)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_env = "ohos"
        )))]
        {
            CLIPBOARD
                .lock()
                .as_mut()
                .ok_or(ClipboardError::NotAvailable)?
                .clipboard
                .set_contents(s)
                .map_err(|e| ClipboardError::ProviderError(e.to_string()))
        }
    }

    #[cfg(windows)]
    pub fn get_file_list() -> Result<Vec<std::path::PathBuf>, ClipboardError> {
        use std::{path::PathBuf, str::FromStr};

        let mut out: Vec<String> = Vec::new();
        clipboard_win::raw::get_file_list(&mut out)
            .map_err(|e| ClipboardError::ProviderError(e.to_string()))?;
        out.iter()
            .map(|s| PathBuf::from_str(s).map_err(|e| ClipboardError::PathError(e.to_string())))
            .collect()
    }

    pub(crate) unsafe fn init(display: RawDisplayHandle) {
        *CLIPBOARD.lock() = Some(unsafe { Self::new(display) });
    }

    /// Initialize the Android JNI bridge from the app's `JavaVM*` + `Activity`.
    ///
    /// # Safety
    /// `vm` must be a valid `JavaVM*` and `activity` a valid JNI global
    /// reference to the app's `Activity` (both from `AndroidApp`).
    #[cfg(target_os = "android")]
    pub(crate) unsafe fn init_android(vm: *mut std::ffi::c_void, activity: jni::sys::jobject) {
        crate::platform::android_jni::AndroidJni::init(vm, activity);
    }

    /// # Safety
    /// The `display` must be valid as long as the returned Clipboard exists.
    unsafe fn new(
        #[allow(unused_variables)] /* on some platforms */ display: RawDisplayHandle,
    ) -> Self {
        // Android and iOS use platform bridges (JNI `ClipboardManager` /
        // `UIPasteboard`) that need no `ClipboardContext`; they always succeed
        // with an empty struct.
        #[cfg(any(target_os = "android", target_os = "ios", target_env = "ohos"))]
        {
            return Self {};
        }

        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_env = "ohos"
        )))]
        {
            #[cfg(not(any(
                target_os = "macos",
                target_os = "windows",
                target_arch = "wasm32"
            )))]
            {
                if let RawDisplayHandle::Wayland(display) = display {
                    use copypasta::wayland_clipboard;
                    let (selection, clipboard) = unsafe {
                        wayland_clipboard::create_clipboards_from_external(
                            display.display.as_ptr(),
                        )
                    };
                    return Self {
                        clipboard: Box::new(clipboard),
                        selection: Some(Box::new(selection)),
                    };
                }

                use copypasta::x11_clipboard::{Primary, X11ClipboardContext};
                Self {
                    clipboard: Box::new(ClipboardContext::new().unwrap()),
                    selection: Some(Box::new(X11ClipboardContext::<Primary>::new().unwrap())),
                }
            }

            #[cfg(any(
                target_os = "macos",
                target_os = "windows",
                target_arch = "wasm32"
            ))]
            {
                Self {
                    clipboard: Box::new(ClipboardContext::new().unwrap()),
                    selection: None,
                }
            }
        }
    }
}
