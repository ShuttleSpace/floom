#![cfg(target_os = "ios")]

//! iOS soft-keyboard / IME bridge.
//!
//! The lapce winit fork's iOS backend has no software-keyboard or IME support:
//! it never emits `WindowEvent::Ime`, so Floem must drive the iOS keyboard
//! itself. The approach is the standard iOS pattern — a hidden `UITextField`
//! is added to the key window and made the first responder. Its
//! `UITextFieldDelegate` captures edits and re-injects them into Floem as
//! `Event::Ime` (`Commit` / `Delete` / `Preedit`) through the same
//! `UserEvent` event-loop proxy the rest of the app uses.
//!
//! Everything here runs on the main thread (UIKit and winit's
//! `UIApplicationMain` both live there), which is why the Objective-C objects
//! are kept in a `thread_local` rather than a `Send` static.

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, Bool, ClassBuilder, NSObject, Sel};
use objc2::{sel, msg_send, ClassType};
// `NSRect` is a type alias for `CGRect` (and `NSPoint`/`NSSize` for
// `CGPoint`/`CGSize`); `objc2_foundation` re-exports the aliases but not the
// CoreFoundation names, so we use `NSRect::ZERO` to size the hidden field.
use objc2_foundation::{NSRect, NSString, NSRange, MainThreadMarker};
use objc2_ui_kit::{UIApplication, UITextField};

use crate::app::{Application, UserEvent};
use crate::event::ImeEvent;

/// Neutral IME edit produced by the iOS keyboard delegate, destined for Floem.
///
/// This is the payload carried by [`UserEvent::IosIme`]; it maps 1:1 onto the
/// Floem-internal [`ImeEvent`] that `text_input` consumes.
#[derive(Clone, Debug)]
pub(crate) enum IosImeEvent {
    /// Text inserted at the cursor (typed character(s), paste, autofill…).
    Commit(String),
    /// Surrounding text removed (backspace / selection delete).
    Delete { before: usize, after: usize },
    // `Preedit` / `Enabled` are reserved for CJK marked-text composition, which
    // is not yet wired from `UITextField.markedTextRange` (see docs/ios.md §Step
    // 5). They are matched in `to_floem` but never constructed yet.
    #[allow(dead_code)]
    Preedit { text: String, cursor: Option<(usize, usize)> },
    #[allow(dead_code)]
    /// IME enabled/disabled state changed.
    Enabled(bool),
}

impl IosImeEvent {
    /// Convert into the Floem-internal [`ImeEvent`] consumed by `text_input`.
    pub(crate) fn to_floem(self) -> ImeEvent {
        match self {
            IosImeEvent::Commit(text) => ImeEvent::Commit(text),
            IosImeEvent::Delete { before, after } => ImeEvent::DeleteSurrounding {
                before_bytes: before,
                after_bytes: after,
            },
            IosImeEvent::Preedit { text, cursor } => ImeEvent::Preedit { text, cursor },
            IosImeEvent::Enabled(true) => ImeEvent::Enabled,
            IosImeEvent::Enabled(false) => ImeEvent::Disabled,
        }
    }
}

/// The hidden text field plus its (non-retained-by-UIKit) delegate.
///
/// `UITextField::setDelegate:` does **not** retain the delegate, so we must
/// keep the `NSObject` alive for as long as the field exists.
struct ImeField {
    #[allow(dead_code)]
    field: Retained<UITextField>,
    #[allow(dead_code)]
    delegate: Retained<NSObject>,
}

thread_local! {
    /// The hidden first-responder text field. Objective-C objects are `!Send`,
    /// so they live on the main thread via this `thread_local`.
    static IME_FIELD: RefCell<Option<ImeField>> = const { RefCell::new(None) };
}

/// The winit window id whose text input currently drives the keyboard.
///
/// Plain data (`Copy` / `Send`), so a `Mutex` static is fine even though the
/// UIKit objects live in the `thread_local` above.
static ACTIVE_IME_WINDOW: Mutex<Option<winit::window::WindowId>> = Mutex::new(None);

/// Show or hide the iOS soft keyboard for `window_id`.
///
/// Called from `WindowHandle::process_update_message` when a `text_input`
/// gains/loses focus (via `set_ime_allowed`). Making the hidden field the
/// first responder pops the keyboard; resigning hides it.
pub(crate) fn set_keyboard_visible(visible: bool, window_id: winit::window::WindowId) {
    let mtm = match MainThreadMarker::new() {
        Some(mtm) => mtm,
        // IME must run on the main thread; bail safely if somehow off-thread.
        None => return,
    };

    if visible {
        ensure_field(mtm);
        *ACTIVE_IME_WINDOW.lock().unwrap() = Some(window_id);
        IME_FIELD.with(|f| {
            if let Some(field) = f.borrow().as_ref() {
                let _ = field.field.becomeFirstResponder();
            }
        });
    } else {
        IME_FIELD.with(|f| {
            if let Some(field) = f.borrow().as_ref() {
                let _ = field.field.resignFirstResponder();
            }
        });
        *ACTIVE_IME_WINDOW.lock().unwrap() = None;
    }
}

/// Create the hidden text field + delegate once and attach it to the key
/// window (winit's `UIWindow`).
// `keyWindow()` is deprecated in favor of the `connectedScenes` API, but we
// ship a single-scene app (`UIApplicationSupportsMultipleScenes=false` in
// Info.plist), so the key window is unambiguous and this call is correct.
#[allow(deprecated)]
fn ensure_field(mtm: MainThreadMarker) {
    let exists = IME_FIELD.with(|f| f.borrow().is_some());
    if exists {
        return;
    }

    let field = UITextField::new(mtm);
    // Zero-size and fully transparent: it never covers a touch point (so it
    // cannot intercept Floem's pointer input) yet can still become the first
    // responder and summon the system keyboard.
    field.setFrame(NSRect::ZERO);
    field.setAlpha(0.0);

    let app = UIApplication::sharedApplication(mtm);
    if let Some(window) = app.keyWindow() {
        window.addSubview(&field);
    }

    // Build and install the delegate that translates edits into Floem IME.
    //
    // We cannot use the typed `UITextField::setDelegate` (which wants a
    // `&ProtocolObject<dyn UITextFieldDelegate>`): our delegate is a
    // runtime-declared `NSObject` subclass that only *responds* to the delegate
    // selectors, it does not carry the generated protocol conformance. The ABI
    // of `setDelegate:` is just `id`, so a raw message send with the object
    // pointer is correct and UIKit will dispatch the selector to our class.
    let delegate = create_delegate();
    unsafe {
        let _: () = msg_send![&field, setDelegate: &*delegate];
    }

    IME_FIELD.with(|f| {
        *f.borrow_mut() = Some(ImeField { field, delegate });
    });
}

/// Queue an IME edit for the active window through the event-loop proxy.
fn dispatch_ime(event: IosImeEvent) {
    let window_id = match *ACTIVE_IME_WINDOW.lock().unwrap() {
        Some(id) => id,
        None => return,
    };
    Application::send_proxy_event(UserEvent::IosIme { window_id, event });
}

// --- Objective-C delegate class ----------------------------------------

/// Register (once) and return the custom `UITextFieldDelegate` subclass.
fn delegate_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    *CLASS.get_or_init(|| {
        let mut builder =
            ClassBuilder::new(c"FloemIosImeDelegate", NSObject::class())
                .expect("FloemIosImeDelegate class already registered");
        unsafe {
            builder.add_method(
                sel!(textField:shouldChangeCharactersInRange:replacementString:),
                should_change as unsafe extern "C-unwind" fn(_, _, _, _, _) -> _,
            );
            builder.add_method(
                sel!(textFieldShouldReturn:),
                should_return as unsafe extern "C-unwind" fn(_, _, _) -> _,
            );
        }
        builder.register()
    })
}

/// Instantiate the delegate as an `NSObject` subclass instance.
fn create_delegate() -> Retained<NSObject> {
    let cls = delegate_class();
    let obj: Option<Retained<NSObject>> =
        unsafe { objc2::msg_send![objc2::msg_send![cls, alloc], init] };
    obj.expect("failed to allocate FloemIosImeDelegate")
}

/// `UITextFieldDelegate` edit hook: translate the proposed text change into
/// Floem `Commit` / `Delete` IME events.
///
/// The first parameter is the delegate (`self`); `text_field` is the field the
/// user is editing (our hidden host). We allow the change (returning `true`)
/// and diff the edit against the field's pre-change text.
unsafe extern "C-unwind" fn should_change(
    _self: &NSObject,
    _cmd: Sel,
    text_field: &UITextField,
    range: NSRange,
    replacement: &NSString,
) -> Bool {
    let current = text_field.text();
    let current_str = current.as_ref().map(|s| s.to_string()).unwrap_or_default();
    let inserted = replacement.to_string();

    // `range` is expressed in UTF-16 code units; convert the deleted slice to
    // UTF-8 to obtain the exact byte count Floem's `DeleteSurrounding` expects.
    let current_utf16: Vec<u16> = current_str.encode_utf16().collect();
    let loc = range.location as usize;
    let len = range.length as usize;
    let deleted_bytes = if loc + len <= current_utf16.len() {
        let deleted = String::from_utf16_lossy(&current_utf16[loc..loc + len]);
        deleted.len()
    } else {
        0
    };

    if deleted_bytes > 0 {
        dispatch_ime(IosImeEvent::Delete {
            before: deleted_bytes,
            after: 0,
        });
    }
    if !inserted.is_empty() {
        dispatch_ime(IosImeEvent::Commit(inserted));
    }
    Bool::YES
}

/// `Return` key: inject a newline (matches desktop Enter in multi-line
/// fields). Apps that want Return to dismiss/submit must handle this
/// themselves; see `docs/ios.md` §Step 5.
unsafe extern "C-unwind" fn should_return(
    _self: &NSObject,
    _cmd: Sel,
    _text_field: &UITextField,
) -> Bool {
    dispatch_ime(IosImeEvent::Commit("\n".to_string()));
    Bool::YES
}
