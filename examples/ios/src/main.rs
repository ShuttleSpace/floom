//! Minimal Floem app for iOS.
//!
//! Unlike Android (where `android_main` is the entry), iOS uses a plain Rust
//! `fn main`: [`floem::Application::new_ios`] builds a winit `EventLoop`, and
//! winit internally calls `UIApplicationMain` with its own
//! `UIApplicationDelegate` when the loop runs. No Objective-C `main` file is
//! required.
//!
//! This example also exercises the **Step 5 soft keyboard / IME** bridge:
//! tapping the text field focuses it, which makes the hidden `UITextField`
//! first responder and summons the iOS keyboard; typed text is re-injected as
//! Floem `Event::Ime`.
//!
//! ```bash
//! # Build an Xcode project (see README.md) and run in the simulator:
//! cargo xcode build --target aarch64-apple-ios-sim
//! ```

use floem::prelude::*;

#[cfg(target_os = "ios")]
fn main() {
    floem::Application::new_ios(floem::AppConfig::default())
        .window(
            |_id| app_view(),
            Some(floem::window::WindowConfig::default().apply_safe_area(true)),
        )
        .run();
}

#[cfg(target_os = "ios")]
fn app_view() -> impl floem::view::IntoView {
    let counter = RwSignal::new(0);
    // Preload a non-ASCII string: this is the *rendering* half of the IME/CJK
    // question. Whether a person can type characters in is a property of the
    // input bridge (driven from outside the app); whether the glyphs are drawn
    // rather than left as tofu boxes is a property of the text stack, and it is
    // visible in a screenshot without any input at all.
    let text = RwSignal::new(String::from("你好世界"));

    Stack::vertical((
        // A small counter to prove pointer input + redraw work.
        Stack::horizontal((
            Button::new("+").action(move || counter.update(|v| *v += 1)),
            Label::derived(move || format!("Value: {}", counter.get())),
            Button::new("-").action(move || counter.update(|v| *v -= 1)),
        ))
        .style(|s| s.gap(10.0)),
        // A text field — tapping it pops the iOS soft keyboard (Step 5 IME).
        TextInput::new(text),
        Label::derived(move || format!("You typed: {}", text.get())),
    ))
    .style(|s| {
        s.size_full()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(12.0)
    })
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("This example targets iOS only; build it for iOS with cargo-xcode / Xcode (see README.md).");
}
