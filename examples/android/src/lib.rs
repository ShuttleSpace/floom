//! Minimal Floom app for Android — **shared library** target.
//!
//! Android's `native-activity` glue loads `libfloom_android_example.so`
//! (named by the `android.app.lib_name` meta-data in `AndroidManifest.xml`)
//! and calls [`android_main`]. That is why this crate is a `cdylib` and not a
//! plain binary: `cargo ndk` only copies `*.so` artifacts into `jniLibs/`.
//!
//! Build + package an APK:
//!
//! ```bash
//! scripts/build-apk.sh -t arm64-v8a        # release
//! scripts/build-apk.sh -t arm64-v8a --debug
//! ```

#[cfg(target_os = "android")]
use floom::prelude::*;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

/// Android entry point invoked by the `native-activity` glue.
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    // Surface files shared into the app (Android ACTION_SEND / ACTION_VIEW).
    let shared_files: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    floom::platform::set_shared_files_handler(Box::new(move |files| {
        shared_files.set(files.into_iter().map(|f| f.uri).collect());
    }));
    floom::Application::new_android(app, floom::AppConfig::default())
        .window(move |_id| app_view(shared_files), None)
        .run();
}

#[cfg(target_os = "android")]
fn app_view(shared_files: RwSignal<Vec<String>>) -> impl IntoView {
    let mut counter = RwSignal::new(0);
    Stack::vertical((
        Stack::horizontal((
            Button::new("+").action(move || counter += 1).style(|s| {
                s.focus_visible(|s| s.border(3.0).border_color(Color::from_rgb8(0, 120, 255)))
            }),
            Label::derived(move || format!("Value: {counter}")),
            Button::new("-").action(move || counter -= 1).style(|s| {
                s.focus_visible(|s| s.border(3.0).border_color(Color::from_rgb8(0, 120, 255)))
            }),
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
    .style(|s| {
        s.size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(10)
    })
}

// Native bridge for `FloomNativeActivity.onActivityResult`. The Java class
// (see `FloomNativeActivity.java`) forwards `onActivityResult` here so Floom
// can deliver Storage Access Framework file-picker results. This symbol is
// resolved by the JVM via JNI's default dynamic linking when the APK's
// activity is `dev.floom.android.FloomNativeActivity`.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_dev_floom_android_FloomNativeActivity_floomOnActivityResult(
    _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    request_code: jni::sys::jint,
    result_code: jni::sys::jint,
    data: jni::sys::jobject,
) {
    floom::platform::dispatch_activity_result(request_code as u32, result_code as i32, data);
}

// Native bridge for `FloomNativeActivity.onNewIntent`. Forwards a file
// shared into a running app (ACTION_SEND / ACTION_VIEW) to Floom.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_dev_floom_android_FloomNativeActivity_floomOnNewIntent(
    _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    intent: jni::sys::jobject,
) {
    floom::platform::dispatch_android_intent(intent);
}
