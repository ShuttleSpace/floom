//! Minimal Floem app for Android.
//!
//! The real entry point on Android is [`android_main`], which receives the
//! `AndroidApp` handle from the `native-activity` glue and forwards it to
//! [`floem::Application::new_android`]. The host `main` below is a placeholder
//! so this crate also compiles on desktop (where it intentionally does
//! nothing) — build it for Android with `cargo ndk` (see `README.md`).
//!
//! ```bash
//! cargo ndk -t arm64-v8a -o app/src/main/jniLibs build --release
//! ```

#[cfg(target_os = "android")]
mod android_entry {
    use floem::prelude::*;
    use floem::window::WindowId;
    use winit::platform::android::activity::AndroidApp;

    #[no_mangle]
    pub fn android_main(app: AndroidApp) {
        // Surface files shared into the app (Android ACTION_SEND / ACTION_VIEW).
        let shared_files: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
        floem::platform::set_shared_files_handler(Box::new(move |files| {
            shared_files.set(files.into_iter().map(|f| f.uri).collect());
        }));
        floem::Application::new_android(app, floem::AppConfig::default())
            .window(move |_id| app_view(shared_files), None)
            .run();
    }

    fn app_view(shared_files: RwSignal<Vec<String>>) -> impl IntoView {
        let mut counter = RwSignal::new(0);
        Stack::vertical((
            Stack::horizontal((
                Button::new("+").action(move || counter += 1),
                Label::derived(move || format!("Value: {counter}")),
                Button::new("-").action(move || counter -= 1),
            )),
            Label::derived(move || {
                let files = shared_files.get();
                if files.is_empty() {
                    "No shared files yet - share a file to this app".to_string()
                } else {
                    format!("Shared ({}): {}", files.len(), files.join(", "))
                }
            }),
        ))
        .style(|s| s.size_full().flex_col().items_center().justify_center().gap(10))
    }

    // Native bridge for `FloemNativeActivity.onActivityResult`. The Java class
    // (see `FloemNativeActivity.java`) forwards `onActivityResult` here so Floem
    // can deliver Storage Access Framework file-picker results. This symbol is
    // resolved by the JVM via JNI's default dynamic linking when the APK's
    // activity is `dev.floem.android.FloemNativeActivity`.
    #[no_mangle]
    pub unsafe extern "system" fn Java_dev_floem_android_FloemNativeActivity_floemOnActivityResult(
        _env: jni::JNIEnv,
        _this: jni::objects::JObject,
        request_code: jni::sys::jint,
        result_code: jni::sys::jint,
        data: jni::sys::jobject,
    ) {
        floem::platform::dispatch_activity_result(
            request_code as u32,
            result_code as i32,
            data,
        );
    }

    // Native bridge for `FloemNativeActivity.onNewIntent`. Forwards a file
    // shared into a running app (ACTION_SEND / ACTION_VIEW) to Floem.
    #[no_mangle]
    pub unsafe extern "system" fn Java_dev_floem_android_FloemNativeActivity_floemOnNewIntent(
        _env: jni::JNIEnv,
        _this: jni::objects::JObject,
        intent: jni::sys::jobject,
    ) {
        floem::platform::dispatch_android_intent(intent);
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    eprintln!("This example targets Android only; build it with `cargo ndk`.");
}
