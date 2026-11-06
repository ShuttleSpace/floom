//! A **web view** in a real window, with a JS ↔ Rust bridge, staying up until
//! you close it.
//!
//! Where `native_view` proves the ABI one call at a time and then exits, this
//! is the thing you actually look at: a window that stays open, a page you can
//! click, and messages crossing between the page's JavaScript and Rust in both
//! directions.
//!
//! # What it shows
//!
//! By default it loads a small page built into this file. The page has a
//! button; pressing it sends a JSON message to Rust, Rust answers, and the
//! answer appears in the page. Rust's message arrives through
//! `window.floomNative.onMessage`, and the page's message leaves through
//! `window.floomNative.postMessage` — both of which are the WebKit message
//! handler the plugin installed, not a new transport.
//!
//! To load a real site instead:
//!
//! ```text
//! FLOOM_DEMO_URL=https://www.bing.com \
//!   cargo run -p floom-plugin-webview --example webview_demo
//! ```
//!
//! The bridge still works on a real site if the site opts in by calling
//! `window.floomNative.postMessage(...)`; it is injected at document start,
//! so it is there before the page's own scripts run.
//!
//! # The address bar, and the inspector
//!
//! A native text field sits above the web view. Type a URL and press Return (or
//! click **Go**) and it is handed to the plugin's `load_url` — a second caller
//! of the same ABI the framework uses, so navigation is exercised rather than
//! assumed. It is native on purpose: page controls vanish the moment the web
//! view leaves the demo page.
//!
//! Right-clicking the page offers **Inspect Element**, which opens Web Inspector
//! (the Console tab is one click away). The plugin sets `inspectable` on the
//! `WKWebView`, so this is WebKit's own inspector, not a reimplementation.
//!
//! # Why a run loop rather than a sleep
//!
//! WebKit and AppKit both deliver their work through the main run loop. A
//! `thread::sleep` would keep the process alive while the window painted
//! nothing and the page never finished loading — the window would look
//! frozen, and messages would never arrive. Running the loop is what lets the
//! web view be a web view.
//!
//! # Capability boundary
//!
//! Everything this example asks of the plugin goes through the unified API in
//! `floom_plugin_webview` — `provider`, `load_url`, `post_message`,
//! `set_message_callback` and `diagnostics`, plus the framework's
//! `NativeViewController` to drive the returned provider. No plugin FFI entry
//! point (`floom_webview_*`) is called here. The AppKit that follows builds
//! only the host window, the demo address bar and the close observer, and
//! reaches back into the plugin through the same unified calls.

use std::ffi::{CStr, c_char, c_void};

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{AllocAnyThread, DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSAutoresizingMaskOptions, NSBackingStoreType,
    NSButton, NSControl, NSTextField, NSWindow, NSWindowStyleMask, NSWindowWillCloseNotification,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSPoint, NSRect, NSSize, NSString};

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeSlot, NativeState, NativeViewController, NativeViewId,
    NativeViewKind, Rect,
};

use floom_plugin_webview::{diagnostics, load_url, post_message, provider, set_message_callback};

/// The page loaded when no URL is given on the command line.
///
/// Kept in the example rather than shipped as a separate file so the demo is
/// one command with nothing to find: the whole bridge, page included, is in
/// the file you are reading.
const DEMO_PAGE: &str = r#"data:text/html;charset=utf-8,
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  :root { color-scheme: dark; }
  body {
    margin: 0; height: 100vh;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 18px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background: rgb(23,24,28); color: rgb(232,232,234);
  }
  h1 { margin: 0; font-size: 22px; font-weight: 600; letter-spacing: .2px; }
  p  { margin: 0; opacity: .6; font-size: 13px; }
  .send {
    font: inherit; font-size: 14px; padding: 9px 18px; border-radius: 8px;
    border: 1px solid rgb(58,61,70); background: rgb(38,41,50);
    color: rgb(232,232,234); cursor: pointer;
  }
  .send:hover { background: rgb(47,51,62); }
  .log {
    min-width: 320px; min-height: 74px; box-sizing: border-box;
    padding: 10px 12px; border-radius: 8px;
    background: rgb(16,17,20); border: 1px solid rgb(38,41,50);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px; line-height: 1.6; white-space: pre-wrap;
    color: rgb(159,211,160);
  }
</style>
</head>
<body>
  <h1>Floom native view</h1>
  <p>a WKWebView owned by an Objective-C plugin, bridged to Rust</p>
  <button class="send" id="ping">Send a message to Rust</button>
  <div class="log" id="log">waiting for Rust…</div>
<script>
  var log = document.getElementById('log');
  var count = 0;

  // Rust to JS. The plugin's message handler delivers into this function.
  window.floomNative = window.floomNative || {};
  window.floomNative.onMessage = function (payload) {
    log.textContent = 'rust to js   ' + payload;
  };

  // JS to Rust. postMessage is the shim the plugin injected at document start.
  document.getElementById('ping').addEventListener('click', function () {
    count += 1;
    log.textContent = 'js to rust   ping ' + count;
    window.floomNative.postMessage(JSON.stringify({ type: 'ping', n: count }));
  });

  // Tell Rust the page is up, so the first message is not a guess about timing.
  window.floomNative.postMessage(JSON.stringify({ type: 'ready' }));
</script>
</body>
</html>"#;

/// Where a message came from, for the printout.
const SLOT: NativeViewId = NativeViewId::new(1, 0);

/// Height of the address-bar strip, in points, at the top of the window.
///
/// The web view is placed below it; the two are siblings in the content view,
/// so the split is a plain subtraction rather than a layout system.
const BAR_H: f64 = 34.0;

/// The window's content size, in points.
const W: f64 = 560.0;
const H: f64 = 420.0;

fn main() {
    // AppKit wants the main thread, and `main` is it.
    let mtm = MainThreadMarker::new().expect("an example runs on the main thread");

    // --- the window ------------------------------------------------------

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    // Establish the connection to the window server. Without this, `makeKeyAndOrderFront`
    // only marks the window to be shown; it is never pushed to the WindowServer and no
    // window appears — even though `isVisible()` already returns true and a WebView loaded
    // into it runs fine (its web process is independent of the host window).
    app.finishLaunching();

    let rect = NSRect::new(NSPoint::new(220.0, 220.0), NSSize::new(W, H));
    let window: Retained<NSWindow> = unsafe {
        let this = mtm.alloc::<NSWindow>();
        msg_send![this,
            initWithContentRect: rect,
            styleMask: NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable,
            backing: NSBackingStoreType::Buffered,
            defer: false]
    };
    let title = NSString::from_str("floom webview — JS ↔ Rust bridge");
    window.setTitle(&title);
    window.makeKeyAndOrderFront(None);
    app.activate();

    // --- the provider ----------------------------------------------------

    let host = Retained::as_ptr(&window) as *const c_void;
    // SAFETY: `window` is alive until `main` returns, and the provider is
    // driven only inside this function while it is still alive.
    let provider = unsafe { provider(host) }.expect("the plugin registered a provider");
    let mut controller = NativeViewController::boxed(provider);

    let pslot = NativeSlot {
        id: SLOT,
        generation: 0,
    };
    controller
        .bind(
            pslot,
            NativeViewKind::Web,
            &[NativeCompositionMode::Overlay],
        )
        .expect("a web view");

    // The window's content view is 560×420 points; the rect crosses as
    // **device** pixels, so it is multiplied by the backing scale here and
    // divided back by the native half. That round trip is the boundary the
    // plugin documents, exercised rather than assumed.
    let scale: f64 = unsafe { msg_send![&*window, backingScaleFactor] };
    let state = NativeState {
        rect: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: W * scale,
            y1: (H - BAR_H) * scale,
        },
        clip: None,
        scale,
        visible: true,
        z: 0,
        input: floom_native_ffi::floom_native::InputMode::Capture,
        mode: NativeCompositionMode::Overlay,
    };
    controller.apply(pslot, state).expect("placed");
    println!("web view placed, backing scale {scale}");

    // --- the address bar -------------------------------------------------
    //
    // A native text field above the web view, not page HTML: page controls
    // disappear the moment the web view navigates to a real site, while a
    // native bar survives every navigation. Return (or the Go button) calls
    // `load_url` through the same plugin the framework uses — the address bar
    // is a second caller of the ABI, not a new path.
    let content = window.contentView().expect("a window has a content view");
    let bar_y = H - BAR_H + (BAR_H - 24.0) / 2.0;
    let field_w = W - 10.0 - 10.0 - 64.0 - 6.0;
    let field: Retained<NSTextField> =
        NSTextField::textFieldWithString(&NSString::from_str(""), mtm);
    field.setFrame(NSRect::new(
        NSPoint::new(10.0, bar_y),
        NSSize::new(field_w, 24.0),
    ));
    field.setPlaceholderString(Some(&NSString::from_str("Enter a URL, then press Return")));
    field.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewMinYMargin,
    );

    let go: Retained<NSButton> = NSButton::new(mtm);
    go.setTitle(&NSString::from_str("Go"));
    go.setFrame(NSRect::new(
        NSPoint::new(W - 10.0 - 64.0, bar_y),
        NSSize::new(64.0, 24.0),
    ));
    go.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewMinYMargin,
    );

    let address = AddressBar::new(field.clone(), pslot);
    unsafe {
        field.setTarget(Some(&*address));
        field.setAction(Some(objc2::sel!(go:)));
        go.setTarget(Some(&*address));
        go.setAction(Some(objc2::sel!(go:)));
    }
    content.addSubview(&field);
    content.addSubview(&go);

    // --- the bridge ------------------------------------------------------
    //
    // One callback, installed once. It runs on the main thread, delivered by
    // WebKit as the page posts; the reply is sent straight back through the
    // same plugin, so what is demonstrated is a full round trip and not two
    // independent writes.
    set_message_callback(Some(on_js_message));

    let target = std::env::var("FLOOM_DEMO_URL")
        .ok()
        .filter(|url| !url.trim().is_empty());
    match &target {
        Some(url) => {
            load_url(pslot, url).unwrap_or_else(|e| panic!("refused {url}: {e}"));
            println!("loaded {url}");
        }
        None => {
            load_url(pslot, DEMO_PAGE).expect("the built-in page is a valid data: url");
            println!("loaded the built-in demo page");
        }
    }

    println!("\nbridge ready — press the button in the window, or close it to quit.");

    // --- run the real Cocoa run loop until the window closes -------------
    //
    // `NSApp.run()` is what makes the window a key app and pumps AppKit's own
    // event sources, which a manual `runUntilDate` loop does not. Without it the
    // window's controls stay dead and a `WKWebView` in it paints black and loads
    // nothing visible — the bridge still works because its messages arrive through
    // the web view's own run-loop source, which is the only thing the manual loop
    // was actually servicing.
    let observer = AppObserver::new(mtm);
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &observer,
            objc2::sel!(windowWillClose:),
            Some(NSWindowWillCloseNotification),
            Some(&*window),
        );
    }
    app.run();

    controller.destroy(pslot).expect("torn down");
    set_message_callback(None);
    println!(
        "\nwindow closed — created {}, destroyed {}, refused {}",
        diagnostics::created(),
        diagnostics::destroyed(),
        diagnostics::refused()
    );
}

/// Called on the main thread when the page posts to the bridge.
///
/// The body is whatever the page passed to `window.floomNative.postMessage`,
/// which the demo makes a JSON string. It is **not** parsed here: the plugin's
/// job is to carry the message, and what a message means belongs to the app —
/// this example only prints it and answers.
extern "C" fn on_js_message(slot: floom_native_ffi::abi::FloomNativeSlot, body: *const c_char) {
    let text = if body.is_null() {
        String::new()
    } else {
        // SAFETY: the plugin documents the pointer as NUL-terminated and valid
        // for the duration of this call; nothing outlives it.
        unsafe { CStr::from_ptr(body) }
            .to_string_lossy()
            .into_owned()
    };
    println!(
        "js -> rust  [slot {}:{}] {text}",
        slot.index, slot.generation
    );

    // Answer through the same slot the message arrived on, so a scene with
    // several participants cannot answer the wrong one.
    let back = NativeSlot {
        id: NativeViewId::new(slot.index, slot.generation),
        generation: slot.generation,
    };
    let reply = format!(r#"{{"type":"pong","echo":{}}}"#, text.trim());
    if let Err(e) = post_message(back, &reply) {
        println!("rust -> js failed: {e}");
        return;
    }
    println!("rust -> js  {reply}");
}

/// A tiny Objective-C object whose only job is to stop the app when the window
/// closes, so the real Cocoa run loop (`NSApp.run()`) can drive the window and
/// the web view.
///
/// The manual `runUntilDate` loop the example used before only pumps the bridge's
/// own run-loop sources: the window never becomes key, so its controls stay dead
/// and a `WKWebView` in it paints black and loads nothing visible.
struct AppObserverIvars {
    _mtm: MainThreadMarker,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "FloomWebviewObserver"]
    #[ivars = AppObserverIvars]
    struct AppObserver;

    impl AppObserver {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _note: Option<&NSNotification>) {
            NSApplication::sharedApplication(MainThreadMarker::new().expect("on the main thread"))
                .stop(None);
        }
    }
);

impl AppObserver {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc().set_ivars(AppObserverIvars { _mtm: mtm });
        unsafe { msg_send![super(this), init] }
    }
}

/// The address bar's action target: reads the text field and asks the plugin to
/// load whatever was typed.
///
/// A target object rather than a closure because that is the only shape
/// AppKit's target/action gives: the control is handed to an Objective-C
/// object's selector and nothing else.
struct AddressIvars {
    field: Retained<NSTextField>,
    slot: NativeSlot,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "FloomAddressBar"]
    #[ivars = AddressIvars]
    struct AddressBar;

    impl AddressBar {
        #[unsafe(method(go:))]
        fn go(&self, _sender: &NSControl) {
            let typed = self.ivars().field.stringValue().to_string();
            let url = normalize_url(typed.trim());
            if url.is_empty() {
                println!("address bar: no URL typed");
                return;
            }
            println!("address bar -> {url}");
            if let Err(e) = load_url(self.ivars().slot, &url) {
                println!("load_url refused {url}: {e}");
            }
        }
    }
);

impl AddressBar {
    fn new(field: Retained<NSTextField>, slot: NativeSlot) -> Retained<Self> {
        let this = Self::alloc().set_ivars(AddressIvars { field, slot });
        unsafe { msg_send![super(this), init] }
    }
}

/// Add a scheme when the user typed a bare host.
///
/// The plugin refuses a URL with no scheme (`is_loadable_url`), so `example.com`
/// has to become `https://example.com` before it is handed over — otherwise the
/// most natural input would look like a broken address bar.
fn normalize_url(input: &str) -> String {
    let has_scheme = input.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    if has_scheme {
        input.to_string()
    } else {
        format!("https://{input}")
    }
}
