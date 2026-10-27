//! macOS: the system share sheet, anchored to the key window.
//!
//! `NSSharingServicePicker` is view-anchored — AppKit has no window-less
//! version — so the anchor is the key window's content view, the same choice
//! Flutter's macOS implementation makes. Call it on the main thread, from a
//! click handler: AppKit opens a popover.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSSharingServicePicker};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSRectEdge, NSSize, NSString, NSURL};

thread_local! {
    /// The picker has to outlive the call that opened it — the user dismisses
    /// it, not us. Holding the last one keeps it alive without leaking one per
    /// share; opening a second closes the first.
    static PICKER: RefCell<Option<Retained<NSSharingServicePicker>>> =
        const { RefCell::new(None) };
}

pub fn share_text(text: &str) -> Result<(), String> {
    let text = NSString::from_str(text);
    let item: &AnyObject = &text;
    present(&[item])
}

pub fn share_files(paths: &[&str]) -> Result<(), String> {
    let urls: Vec<Retained<NSURL>> = paths
        .iter()
        .map(|path| NSURL::fileURLWithPath(&NSString::from_str(path)))
        .collect();
    let items: Vec<&AnyObject> = urls
        .iter()
        .map(|url| {
            let any: &AnyObject = url;
            any
        })
        .collect();
    present(&items)
}

fn present(items: &[&AnyObject]) -> Result<(), String> {
    if items.is_empty() {
        return Err("share: there is nothing to share".into());
    }
    let mtm = MainThreadMarker::new()
        .ok_or("share: the share sheet opens on the main thread — call it from a click handler")?;
    let app = NSApplication::sharedApplication(mtm);
    let window = app
        .keyWindow()
        .ok_or("share: there is no key window to anchor the share sheet to")?;
    let view = window
        .contentView()
        .ok_or("share: the key window has no content view")?;

    let items = NSArray::from_slice(items);
    let picker =
        unsafe { NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items) };

    // AppKit wants the rect of the control that opened the sheet. The plugin
    // does not see that control, so it anchors at the centre of the window —
    // the popover points at the window rather than at a button, which is the
    // honest thing to do without a caller-supplied anchor.
    let bounds = view.bounds();
    let rect = NSRect::new(
        NSPoint::new(bounds.size.width / 2.0, bounds.size.height / 2.0),
        NSSize::new(1.0, 1.0),
    );
    picker.showRelativeToRect_ofView_preferredEdge(rect, &view, NSRectEdge::MinY);

    PICKER.with(|slot| *slot.borrow_mut() = Some(picker));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// libtest runs tests on a worker thread, so the picker's main-thread
    /// requirement is the first thing every call meets — a real assertion
    /// about the arm rather than a skipped test.
    #[test]
    fn the_share_sheet_refuses_off_the_main_thread() {
        let err = share_text("hello").unwrap_err();
        assert!(err.contains("main thread"), "{err}");
    }

    #[test]
    fn an_empty_file_list_is_refused_before_appkit() {
        let err = share_files(&[]).unwrap_err();
        assert!(err.contains("nothing to share"), "{err}");
    }
}
