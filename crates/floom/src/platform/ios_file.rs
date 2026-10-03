#![cfg(target_os = "ios")]

//! iOS file dialogs via `UIDocumentPickerViewController`.
//!
//! iOS has no `rfd` equivalent. Opening or saving a file means presenting a
//! `UIDocumentPickerViewController` (the system document picker) and receiving
//! the chosen `NSURL`s back through its `UIDocumentPickerDelegate`. This mirrors
//! Android's SAF bridge (`android_file.rs`) but uses `objc2`/`objc2-ui-kit`
//! instead of JNI.
//!
//! The delegate is a runtime-declared `NSObject` subclass (via `ClassBuilder`)
//! that implements the two `UIDocumentPickerDelegate` selectors; the picker's
//! `setDelegate:` is invoked through a raw `msg_send!`, exactly like the IME
//! bridge in `ios_ime.rs` (the Rust `UIDocumentPickerDelegate` trait requires
//! the Rust 2024 `#[unsafe(...)]` attributes, which this edition-2021 crate
//! cannot use, so we bridge at the ObjC level instead).
//!
//! The picked URLs are handed back to the caller through the `file_info_action`
//! callback, marshaled onto the reactive scope via `create_ext_action` (like
//! the Android path).

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, ClassBuilder, NSObject, Sel};
use objc2::{msg_send, sel, ClassType, MainThreadMarker};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_ui_kit::{UIApplication, UIDocumentPickerViewController, UIWindow};
use objc2_uniform_type_identifiers::UTType;

use floom_legacy_reactive::Scope;

use crate::ext_event::create_ext_action;
use crate::platform::file::{FileDialogOptions, FileInfo};

/// Pending callback for the in-flight document picker (single dialog at a time,
/// like Android). The Objective-C delegate runs on the main thread, so a
/// `thread_local` (rather than a `Sync` static) is sufficient. The callback is
/// `FnOnce` because the picker resolves exactly once (pick or cancel).
thread_local! {
    static PENDING: RefCell<Option<Box<dyn FnOnce(Option<FileInfo>) + 'static>>> =
        const { RefCell::new(None) };
}

enum PickerKind {
    Open,
    Save,
}

/// Open a file (or directory) using the system document picker.
pub fn open_file(
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + 'static,
) {
    launch(PickerKind::Open, options, file_info_action);
}

/// Open a system file save dialog.
pub fn save_as(
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + 'static,
) {
    launch(PickerKind::Save, options, file_info_action);
}

fn launch(
    kind: PickerKind,
    options: FileDialogOptions,
    file_info_action: impl Fn(Option<FileInfo>) + 'static,
) {
    // Marshal the callback onto the reactive scope so signal updates from the
    // caller's closure run on the event-loop thread (mirrors Android).
    let send = create_ext_action(Scope::new(), move |info: Option<FileInfo>| {
        file_info_action(info);
    });
    let cb: Box<dyn FnOnce(Option<FileInfo>) + 'static> = Box::new(move |info| send(info));

    // The picker must be presented on the main thread.
    let mtm = match MainThreadMarker::new() {
        Some(mtm) => mtm,
        None => {
            cb(None);
            return;
        }
    };

    let picker = match build_picker(kind, &options, mtm) {
        Some(p) => p,
        None => {
            cb(None);
            return;
        }
    };
    picker.setAllowsMultipleSelection(options.multi_selection);

    // Attach the delegate through a raw message send (bypassing the Rust
    // `ProtocolObject` wrapper, which this edition cannot construct for the
    // `UIDocumentPickerDelegate` trait).
    let delegate = make_delegate();
    unsafe {
        let _: () = msg_send![&picker, setDelegate: &*delegate];
    }

    // Present from the key window's root view controller.
    //
    // `keyWindow()` is deprecated for multi-scene apps, but we ship a
    // single-scene app (`UIApplicationSupportsMultipleScenes=false` in
    // Info.plist), so it is correct here.
    #[allow(deprecated)]
    let app = UIApplication::sharedApplication(mtm);
    // `cb` must be consumed in exactly one branch: stored into PENDING when we
    // actually present (the delegate will deliver the result), or delivered
    // `None` immediately when there is no root view controller to present from
    // (otherwise the caller would wait forever on a callback that never fires).
    match app
        .keyWindow()
        .and_then(|window: Retained<UIWindow>| window.rootViewController())
    {
        Some(root) => {
            PENDING.with(|p| *p.borrow_mut() = Some(cb));
            root.presentViewController_animated_completion(&picker, true, None);
        }
        None => {
            cb(None);
        }
    }
}

/// Build the picker for the requested kind.
fn build_picker(
    kind: PickerKind,
    options: &FileDialogOptions,
    mtm: MainThreadMarker,
) -> Option<Retained<UIDocumentPickerViewController>> {
    match kind {
        PickerKind::Open => {
            // `public.data` selects all files. We deliberately do not map
            // `allowed_types` to specific UTIs yet (mirrors Android's `*/*`
            // punt in `android_file.rs`); a precise extension→UTI mapping can
            // be layered on later without changing this bridge's shape.
            //
            // NOTE: directory selection (`select_directories`) has no direct
            // `UIDocumentPickerViewController` equivalent; it is ignored for
            // now (the picker selects files only).
            let data_type = UTType::exportedTypeWithIdentifier(&NSString::from_str("public.data"));
            let types = NSArray::from_slice(&[&*data_type]);
            // `Allocated<T>` does not implement `Deref`, so the init methods
            // (which take `this: Allocated<Self>`) must be invoked as
            // associated functions rather than via method syntax.
            let picker: Retained<UIDocumentPickerViewController> = unsafe {
                UIDocumentPickerViewController::initForOpeningContentTypes(
                    mtm.alloc::<UIDocumentPickerViewController>(),
                    &types,
                )
            };
            Some(picker)
        }
        PickerKind::Save => {
            // Export/create flow: hand the picker an initial URL built from the
            // default name so the user can choose a destination. Runtime save
            // semantics (exporting a file that does not yet exist) should be
            // validated on-device; verification here is compile-only.
            let name = options
                .default_name
                .clone()
                .unwrap_or_else(|| "Untitled".to_string());
            let ns_name = NSString::from_str(&name);
            let url = NSURL::fileURLWithPath(&ns_name);
            let urls = NSArray::from_slice(&[&*url]);
            let picker: Retained<UIDocumentPickerViewController> = unsafe {
                UIDocumentPickerViewController::initForExportingURLs(
                    mtm.alloc::<UIDocumentPickerViewController>(),
                    &urls,
                )
            };
            Some(picker)
        }
    }
}

/// Create (once) the custom `UIDocumentPickerDelegate` subclass instance.
fn make_delegate() -> Retained<NSObject> {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    let cls = *CLASS.get_or_init(|| {
        let mut builder = ClassBuilder::new(c"FloomDocPickerDelegate", NSObject::class())
            .expect("FloomDocPickerDelegate class already registered");
        unsafe {
            builder.add_method(
                sel!(documentPicker:didPickDocumentsAtURLs:),
                did_pick_documents as unsafe extern "C-unwind" fn(_, _, _, _),
            );
            builder.add_method(
                sel!(documentPickerWasCancelled:),
                did_cancel as unsafe extern "C-unwind" fn(_, _, _),
            );
        }
        builder.register()
    });
    let obj: Option<Retained<NSObject>> =
        unsafe { msg_send![msg_send![cls, alloc], init] };
    obj.expect("failed to allocate FloomDocPickerDelegate")
}

/// Deliver the result (or cancellation) to the pending caller callback.
fn invoke_pending(info: Option<FileInfo>) {
    let cb = PENDING.with(|p| p.borrow_mut().take());
    if let Some(cb) = cb {
        cb(info);
    }
}

unsafe extern "C-unwind" fn did_pick_documents(
    _self: &NSObject,
    _cmd: Sel,
    _controller: &UIDocumentPickerViewController,
    urls: &NSArray<NSURL>,
) {
    let count = urls.count() as usize;
    let mut paths: Vec<PathBuf> = Vec::with_capacity(count);
    for i in 0..count {
        let url: Retained<NSURL> = urls.objectAtIndex(i as _);
        if let Some(s) = url.absoluteString() {
            let s = s.to_string();
            if !s.is_empty() {
                paths.push(PathBuf::from(s));
            }
        }
    }
    let info = if paths.is_empty() {
        None
    } else {
        Some(FileInfo {
            path: paths,
            format: None,
        })
    };
    invoke_pending(info);
}

unsafe extern "C-unwind" fn did_cancel(
    _self: &NSObject,
    _cmd: Sel,
    _controller: &UIDocumentPickerViewController,
) {
    invoke_pending(None);
}
