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
//!   cargo run -p floom-plugin-nativeview --example webview_demo
//! ```
//!
//! The bridge still works on a real site if the site opts in by calling
//! `window.floomNative.postMessage(...)`; it is injected at document start,
//! so it is there before the page's own scripts run.
//!
//! # Why a run loop rather than a sleep
//!
//! WebKit and AppKit both deliver their work through the main run loop. A
//! `thread::sleep` would keep the process alive while the window painted
//! nothing and the page never finished loading — the window would look
//! frozen, and messages would never arrive. Running the loop is what lets the
//! web view be a web view.

use std::ffi::{CStr, c_char, c_void};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSDate, NSPoint, NSRect, NSRunLoop, NSSize, NSString};

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeSlot, NativeState, NativeViewController, NativeViewId,
    NativeViewKind, Rect,
};

use floom_plugin_nativeview::{
    diagnostics, load_url, post_message, provider, set_message_callback,
};

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

fn main() {
    // AppKit wants the main thread, and `main` is it.
    let mtm = MainThreadMarker::new().expect("an example runs on the main thread");

    // --- the window ------------------------------------------------------

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);

    let rect = NSRect::new(NSPoint::new(220.0, 220.0), NSSize::new(560.0, 420.0));
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
    let title = NSString::from_str("floom nativeview — JS ↔ Rust bridge");
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
        .bind(pslot, NativeViewKind::Web, &[NativeCompositionMode::Overlay])
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
            x1: 560.0 * scale,
            y1: 420.0 * scale,
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

    // --- run until the window closes -------------------------------------
    //
    // A short deadline per turn, so the loop can notice the window closing.
    // This is the only reason the process is alive: it is what delivers
    // WebKit's loading, the page's paint, and every bridge message.
    while window.isVisible() {
        let turn = NSDate::dateWithTimeIntervalSinceNow(0.05);
        NSRunLoop::mainRunLoop().runUntilDate(&turn);
    }

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
        unsafe { CStr::from_ptr(body) }.to_string_lossy().into_owned()
    };
    println!("js -> rust  [slot {}:{}] {text}", slot.index, slot.generation);

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
