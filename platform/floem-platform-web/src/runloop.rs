//! The real run loop: turn a browser tab into a live Floem window.
//!
//! `floem-platform-desktop` owns a winit loop; this is the web analogue. The
//! loop is thin and generic: it owns the DOM machinery (event listeners +
//! `requestAnimationFrame`) but not the app. The app is any [`WebApp`], which
//! drains [`Platform::poll_events`] into its own `AppRunner`. That keeps this
//! crate at L4 — it never names `floem-app`; the beginner-facing entry
//! (`floem2::run_web`) lives in the facade above everything.
//!
//! [`Platform::poll_events`]: floem_platform::Platform::poll_events

use crate::WebPlatform;

/// The application side of a web-hosted Floem app.
///
/// Implementors get called from inside the rAF loop's frame hook, with the
/// platform handy, and are expected to drain [`Platform::poll_events`] — i.e.
/// call `runner.pump(&mut platform)`. The loop owns the DOM; the app owns the
/// runner.
///
/// [`Platform::poll_events`]: floem_platform::Platform::poll_events
pub trait WebApp {
    /// One frame is ready to be serviced. Drain platform events into the runner.
    fn events(&mut self, platform: &mut WebPlatform);

    /// The loop is about to tear down. Release any resources the app holds
    /// (none are required, so this is a no-op by default).
    fn about_to_exit(&mut self, _platform: &mut WebPlatform) {}
}

#[cfg(target_arch = "wasm32")]
pub use wasm::run;

#[cfg(not(target_arch = "wasm32"))]
pub fn run<A: WebApp>(_app: A, _canvas_id: &str) -> Result<(), String> {
    Err(
        "floem-platform-web `run` requires the wasm32 target (build with --target wasm32-unknown-unknown)"
            .into(),
    )
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    use std::cell::RefCell;
    use std::rc::Rc;

    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{
        EventTarget, KeyboardEvent, PointerEvent as DomPointerEvent, WheelEvent,
    };

    use floem_platform::{KeyCode, Platform, PlatformEvent, PointerButton, WindowConfig};
    use floem_types::{Modifiers, Point, Size};

    /// Pixels per notch of a legacy line-based wheel delta. Mirrors the desktop
    /// backend's `PIXELS_PER_SCROLL_LINE` so a scroll view feels the same.
    const PIXELS_PER_SCROLL_LINE: f64 = 20.0;

    /// The owned app + platform, shared between the rAF loop and the DOM
    /// listeners through an `Rc<RefCell<_>>`.
    struct LoopState<A: WebApp> {
        app: Option<A>,
        platform: WebPlatform,
    }

    thread_local! {
        // One persistent rAF closure lives for the page's lifetime; `schedule`
        // re-arms it via `request_animation_frame`, so we do not leak a closure
        // per frame.
        static FRAME: RefCell<Option<Closure<dyn FnMut()>>> = RefCell::new(None);
    }

    /// Run a Floem app in a browser tab. Blocks the caller (it returns
    /// immediately; the loop is driven by `requestAnimationFrame`).
    pub fn run<A: WebApp + 'static>(app: A, canvas_id: &str) -> Result<(), String> {
        let mut platform = WebPlatform::new(canvas_id);
        let scale = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .filter(|s| s.is_finite() && *s > 0.0)
            .unwrap_or(1.0);
        platform.set_scale(scale);
        let size = initial_window_size();
        let window_id = platform.create_window(WindowConfig {
            title: "floem".into(),
            size,
            resizable: true,
        });
        platform.set_main_window(window_id);

        let state = Rc::new(RefCell::new(LoopState {
            app: Some(app),
            platform,
        }));

        // One persistent rAF closure; `schedule` re-arms it.
        let rc = state.clone();
        let closure = Closure::wrap(
            Box::new(move || {
                rc.borrow_mut().platform.set_raf_pending(false);
                tick(rc.clone());
            }) as Box<dyn FnMut()>,
        );
        FRAME.with(|slot| *slot.borrow_mut() = Some(closure));

        install_listeners(&state, canvas_id)?;
        schedule(&state);
        Ok(())
    }

    /// Schedule a single animation frame if one is not already pending.
    fn schedule<A: WebApp>(state: &Rc<RefCell<LoopState<A>>>) {
        let should = {
            let mut l = state.borrow_mut();
            if l.platform.raf_pending() {
                false
            } else {
                l.platform.set_raf_pending(true);
                true
            }
        };
        if !should {
            return;
        }
        let func = FRAME.with(|slot| {
            slot.borrow()
                .as_ref()
                .expect("rAF closure never initialised")
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .clone()
        });
        if let Some(window) = web_sys::window() {
            let _ = window.request_animation_frame(&func);
        }
    }

    /// One animation frame: pump the platform, then keep animating while needed.
    fn tick<A: WebApp>(state: Rc<RefCell<LoopState<A>>>) {
        // Bind a direct `&mut LoopState` (not a `RefMut`) so the borrow checker
        // can split the disjoint `app` / `platform` field borrows: `app.events`
        // needs `&mut platform` while `app` itself is a `&mut` borrow.
        let pending = {
            let mut l = state.borrow_mut();
            let ls: &mut LoopState<A> = &mut *l;
            if let Some(app) = ls.app.as_mut() {
                app.events(&mut ls.platform);
            }
            ls.platform.take_pending_frame()
        };
        if pending {
            schedule(&state);
        }
    }

    /// Initial logical window size: the browser viewport (a full-window canvas).
    fn initial_window_size() -> Size {
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .map(|w| {
                let h = web_sys::window()
                    .and_then(|w| w.inner_height().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(600.0);
                Size::new(w, h)
            })
            .unwrap_or(Size::new(800.0, 600.0))
    }

    /// Attach DOM listeners for pointer / wheel / keyboard / composition /
    /// resize. Each listener pushes a `PlatformEvent` and wakes the loop.
    fn install_listeners<A: WebApp + 'static>(
        state: &Rc<RefCell<LoopState<A>>>,
        canvas_id: &str,
    ) -> Result<(), String> {
        let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
        let doc = window
            .document()
            .ok_or_else(|| "no document".to_string())?;

        let canvas: EventTarget = doc
            .get_element_by_id(canvas_id)
            .or_else(|| doc.query_selector("canvas").ok().flatten())
            .ok_or_else(|| {
                format!("no <canvas id=\"{canvas_id}\"> and no <canvas> element present")
            })?
            .into();

        add_listener(&canvas, "pointerdown", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: DomPointerEvent = e.unchecked_into();
                let mut l = state.borrow_mut();
                if let Some(w) = l.platform.main_window() {
                    let pos = Point::new(e.offset_x() as f64, e.offset_y() as f64);
                    let mods = modifiers_of(&e);
                    let button = pointer_button(e.button());
                    l.platform.push_event(PlatformEvent::PointerButton {
                        window: w,
                        position: pos,
                        button,
                        pressed: true,
                        modifiers: mods,
                    });
                }
                drop(l);
                schedule(&state);
            }
        })?;

        add_listener(&canvas, "pointerup", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: DomPointerEvent = e.unchecked_into();
                let mut l = state.borrow_mut();
                if let Some(w) = l.platform.main_window() {
                    let pos = Point::new(e.offset_x() as f64, e.offset_y() as f64);
                    let mods = modifiers_of(&e);
                    let button = pointer_button(e.button());
                    l.platform.push_event(PlatformEvent::PointerButton {
                        window: w,
                        position: pos,
                        button,
                        pressed: false,
                        modifiers: mods,
                    });
                }
                drop(l);
                schedule(&state);
            }
        })?;

        add_listener(&canvas, "pointermove", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: DomPointerEvent = e.unchecked_into();
                let mut l = state.borrow_mut();
                if let Some(w) = l.platform.main_window() {
                    let pos = Point::new(e.offset_x() as f64, e.offset_y() as f64);
                    let mods = modifiers_of(&e);
                    l.platform.push_event(PlatformEvent::PointerMoved {
                        window: w,
                        position: pos,
                        modifiers: mods,
                    });
                }
                drop(l);
                schedule(&state);
            }
        })?;

        add_listener(&canvas, "wheel", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: WheelEvent = e.unchecked_into();
                e.prevent_default();
                let mut l = state.borrow_mut();
                if let Some(w) = l.platform.main_window() {
                    let pos = Point::new(e.offset_x() as f64, e.offset_y() as f64);
                    let mods = modifiers_of(&e);
                    let (dx, dy) = wheel_delta(&e);
                    l.platform.push_event(PlatformEvent::Scroll {
                        window: w,
                        position: pos,
                        delta_x: dx,
                        delta_y: dy,
                        modifiers: mods,
                    });
                }
                drop(l);
                schedule(&state);
            }
        })?;

        // Suppress the native context menu over the canvas.
        add_listener(&canvas, "contextmenu", |e: web_sys::Event| {
            e.prevent_default();
        })?;

        let win_target: EventTarget = window.into();

        add_listener(&win_target, "keydown", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: KeyboardEvent = e.unchecked_into();
                handle_key(&state, &e, true);
            }
        })?;
        add_listener(&win_target, "keyup", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let e: KeyboardEvent = e.unchecked_into();
                handle_key(&state, &e, false);
            }
        })?;

        // IME composition → commit the composed string.
        add_listener(&win_target, "compositionend", {
            let state = state.clone();
            move |e: web_sys::Event| {
                let data = e
                    .unchecked_into::<web_sys::CompositionEvent>()
                    .data()
                    .unwrap_or_default();
                let mut l = state.borrow_mut();
                if let Some(w) = l.platform.main_window() {
                    if !data.is_empty() {
                        l.platform.push_event(PlatformEvent::ImeCommit {
                            window: w,
                            text: data,
                        });
                    }
                }
                drop(l);
                schedule(&state);
            }
        })?;

        add_listener(&win_target, "resize", {
            let state = state.clone();
            move |_e: web_sys::Event| {
                let mut l = state.borrow_mut();
                l.platform.resize_to(initial_window_size());
                drop(l);
                schedule(&state);
            }
        })?;

        Ok(())
    }

    /// Register one event listener; the closure is leaked (it lives for the page
    /// lifetime, which is exactly what a persistent listener wants).
    ///
    /// Listeners always receive a `web_sys::Event` (the canonical type
    /// `wasm-bindgen` knows how to marshal across the FFI boundary); handlers
    /// that need a richer view `unchecked_into` it. This keeps the generic
    /// `IntoWasmClosure` impl satisfied for every event kind at once.
    fn add_listener<F>(target: &EventTarget, type_: &str, handler: F) -> Result<(), String>
    where
        F: FnMut(web_sys::Event) + 'static,
    {
        let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(web_sys::Event)>);
        target
            .add_event_listener_with_callback(type_, closure.as_ref().unchecked_ref())
            .map_err(|_| format!("failed to register `{type_}` listener"))?;
        closure.forget();
        Ok(())
    }

    /// Map a DOM key event to a `PlatformEvent::Key` (physical-key identity) and,
    /// for non-ASCII input the `KeyCode` space cannot carry, an `ImeCommit`.
    fn handle_key<A: WebApp>(state: &Rc<RefCell<LoopState<A>>>, e: &KeyboardEvent, pressed: bool) {
        let mut l = state.borrow_mut();
        let Some(w) = l.platform.main_window() else {
            return;
        };
        let kc = code_to_keycode(&e.code());
        let mods = modifiers_of(e);
        let repeat = e.repeat();
        if kc != KeyCode::NONE {
            l.platform.push_event(PlatformEvent::Key {
                window: w,
                code: kc,
                pressed,
                repeat,
                modifiers: mods,
            });
        }
        // Non-ASCII text cannot ride the `KeyCode` path; deliver it through IME.
        let key = e.key();
        let mut chars = key.chars();
        let is_nonascii_printable = matches!(
            chars.next(),
            Some(c) if !c.is_ascii() && !c.is_control()
        ) && chars.next().is_none();
        if is_nonascii_printable {
            l.platform.push_event(PlatformEvent::ImeCommit {
                window: w,
                text: key,
            });
        }
        drop(l);
        schedule(state);
    }

    /// The subset of DOM events we read modifier state from. web-sys 0.3.103
    /// exposes `shift_key`/`ctrl_key`/`alt_key`/`meta_key` as inherent methods
    /// (there is no `ModifierState` trait), so we wrap them in a tiny private
    /// trait to keep `modifiers_of` generic over the three event types.
    trait HasModifiers {
        fn has_shift(&self) -> bool;
        fn has_ctrl(&self) -> bool;
        fn has_alt(&self) -> bool;
        fn has_meta(&self) -> bool;
    }

    impl HasModifiers for KeyboardEvent {
        fn has_shift(&self) -> bool { self.shift_key() }
        fn has_ctrl(&self) -> bool { self.ctrl_key() }
        fn has_alt(&self) -> bool { self.alt_key() }
        fn has_meta(&self) -> bool { self.meta_key() }
    }

    impl HasModifiers for DomPointerEvent {
        fn has_shift(&self) -> bool { self.shift_key() }
        fn has_ctrl(&self) -> bool { self.ctrl_key() }
        fn has_alt(&self) -> bool { self.alt_key() }
        fn has_meta(&self) -> bool { self.meta_key() }
    }

    impl HasModifiers for WheelEvent {
        fn has_shift(&self) -> bool { self.shift_key() }
        fn has_ctrl(&self) -> bool { self.ctrl_key() }
        fn has_alt(&self) -> bool { self.alt_key() }
        fn has_meta(&self) -> bool { self.meta_key() }
    }

    /// Build a [`Modifiers`] bit-set from a DOM event's modifier flags.
    fn modifiers_of<E: HasModifiers>(e: &E) -> Modifiers {
        let mut m = Modifiers::NONE;
        if e.has_shift() {
            m = m.union(Modifiers::SHIFT);
        }
        if e.has_ctrl() {
            m = m.union(Modifiers::CONTROL);
        }
        if e.has_alt() {
            m = m.union(Modifiers::ALT);
        }
        if e.has_meta() {
            m = m.union(Modifiers::META);
        }
        m
    }

    /// Map a DOM mouse button number onto the widget contract's three buttons.
    fn pointer_button(button: i16) -> PointerButton {
        match button {
            0 => PointerButton::Primary,
            1 => PointerButton::Auxiliary,
            2 => PointerButton::Secondary,
            other => PointerButton::Other(other as u8),
        }
    }

    /// Wheel / trackpad delta as `(dx, dy)` in logical pixels. A classic wheel
    /// reports line deltas; normalise them to match pixel-based scrolling.
    fn wheel_delta(e: &WheelEvent) -> (f64, f64) {
        let (mut dx, mut dy) = (e.delta_x(), e.delta_y());
        if e.delta_mode() == WheelEvent::DOM_DELTA_LINE {
            dx *= PIXELS_PER_SCROLL_LINE;
            dy *= PIXELS_PER_SCROLL_LINE;
        }
        (dx, dy)
    }

    /// Canonicalise a DOM `KeyboardEvent.code()` into a [`KeyCode`].
    fn code_to_keycode(code: &str) -> KeyCode {
        match code {
            "Enter" => KeyCode::ENTER,
            "Escape" => KeyCode::ESCAPE,
            "Tab" => KeyCode::TAB,
            "Backspace" => KeyCode::BACKSPACE,
            "Delete" => KeyCode::DELETE,
            "ArrowUp" => KeyCode::ARROW_UP,
            "ArrowDown" => KeyCode::ARROW_DOWN,
            "ArrowLeft" => KeyCode::ARROW_LEFT,
            "ArrowRight" => KeyCode::ARROW_RIGHT,
            "Home" => KeyCode::HOME,
            "End" => KeyCode::END,
            "PageUp" => KeyCode::PAGE_UP,
            "PageDown" => KeyCode::PAGE_DOWN,
            "ShiftLeft" | "ShiftRight" => KeyCode::SHIFT,
            "ControlLeft" | "ControlRight" => KeyCode::CONTROL,
            "AltLeft" | "AltRight" => KeyCode::ALT,
            "MetaLeft" | "MetaRight" => KeyCode::META,
            "CapsLock" => KeyCode::CAPS_LOCK,
            "Insert" => KeyCode::INSERT,
            "ContextMenu" => KeyCode::CONTEXT_MENU,
            "PrintScreen" => KeyCode::PRINT_SCREEN,
            "ScrollLock" => KeyCode::SCROLL_LOCK,
            "Pause" => KeyCode::PAUSE,
            "NumLock" => KeyCode::NUM_LOCK,
            "Space" => KeyCode::SPACE,
            "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11"
            | "F12" => KeyCode::function(code[1..].parse().unwrap_or(1)),
            _ => {
                if let Some(n) = parse_fkey(code) {
                    return n;
                }
                if let Some(rest) = code.strip_prefix("Key") {
                    return rest
                        .chars()
                        .next()
                        .map(KeyCode::from_char)
                        .unwrap_or(KeyCode::NONE);
                }
                if let Some(rest) = code.strip_prefix("Digit") {
                    return rest
                        .parse::<u8>()
                        .ok()
                        .map(KeyCode::digit)
                        .unwrap_or(KeyCode::NONE);
                }
                if let Some(rest) = code.strip_prefix("Numpad") {
                    return rest
                        .parse::<u8>()
                        .ok()
                        .filter(|&d| d <= 9)
                        .map(KeyCode::digit)
                        .unwrap_or(KeyCode::NONE);
                }
                punctuation(code)
            }
        }
    }

    fn parse_fkey(code: &str) -> Option<KeyCode> {
        let rest = code.strip_prefix('F')?;
        let n: u8 = rest.parse().ok()?;
        (1..=24).contains(&n).then(|| KeyCode::function(n))
    }

    /// US-layout unshifted punctuation that `from_char` can represent directly.
    fn punctuation(code: &str) -> KeyCode {
        match code {
            "Minus" => KeyCode::from_char('-'),
            "Equal" => KeyCode::from_char('='),
            "Comma" => KeyCode::from_char(','),
            "Period" => KeyCode::from_char('.'),
            "Slash" => KeyCode::from_char('/'),
            "Semicolon" => KeyCode::from_char(';'),
            "Quote" => KeyCode::from_char('\''),
            "BracketLeft" => KeyCode::from_char('['),
            "BracketRight" => KeyCode::from_char(']'),
            "Backslash" => KeyCode::from_char('\\'),
            "Backquote" => KeyCode::from_char('`'),
            _ => KeyCode::NONE,
        }
    }
}
