//! macOS: an `NSStatusItem` in the menu bar.
//!
//! The status item is the icon; its menu is a plain `NSMenu`, and the rows
//! report back through one target object whose selector is `menuItemClicked:`.
//! The target exists because that is how AppKit menus work — a menu item sends
//! its action to a target, and the target has to be an Objective-C object. The
//! queue it writes to is the same one [`Tray::events`] drains.

use std::sync::{Arc, Mutex};

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{
    AllocAnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
};
use objc2_app_kit::{
    NSApplication, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::NSString;

use crate::{MenuItem, Queue, TrayEvent, drain, push};

struct TargetIvars {
    queue: Queue,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and this class does not
    // implement Drop — the queue it holds is reference counted.
    #[unsafe(super(NSObject))]
    #[name = "FloomTrayTarget"]
    #[ivars = TargetIvars]
    struct Target;

    impl Target {
        /// `NSMenuItem`'s action. The row's `tag` carries the menu id, which is
        /// the only channel an action method gets: it is handed the sender and
        /// nothing else.
        #[unsafe(method(menuItemClicked:))]
        fn menu_item_clicked(&self, sender: &NSMenuItem) {
            push(&self.ivars().queue, TrayEvent::Menu(sender.tag() as u32));
        }
    }
);

impl Target {
    fn new(queue: Queue) -> Retained<Self> {
        let this = Self::alloc().set_ivars(TargetIvars { queue });
        unsafe { msg_send![super(this), init] }
    }
}

pub struct Tray {
    status_item: Retained<NSStatusItem>,
    /// Held so the menu's target stays alive: AppKit does not retain a menu
    /// item's target, so dropping this would leave the rows pointing at a
    /// freed object.
    _target: Option<Retained<Target>>,
    queue: Queue,
}

impl Tray {
    pub fn show(label: &str, items: &[MenuItem]) -> Result<Self, String> {
        let mtm = MainThreadMarker::new().ok_or(
            "tray: the status item is created on the main thread — call \
             Tray::show from the thread that runs the event loop",
        )?;

        // `NSStatusBar` reaches the menu bar through the application object;
        // asking for it here is also what makes the call a defined one in a
        // process that has not started AppKit yet.
        let _app = NSApplication::sharedApplication(mtm);

        let status_item =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        if let Some(button) = status_item.button(mtm) {
            button.setTitle(&NSString::from_str(label));
        }

        let queue: Queue = Arc::new(Mutex::new(Vec::new()));
        let mut target = None;

        if !items.is_empty() {
            let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(label));
            // Otherwise AppKit enables every row whose target responds to the
            // action — which is all of them — and a `disabled` row becomes
            // selectable the moment the menu opens.
            menu.setAutoenablesItems(false);

            let handler = Target::new(queue.clone());
            for item in items {
                match item {
                    MenuItem::Separator => {
                        menu.addItem(&NSMenuItem::separatorItem(mtm));
                    }
                    MenuItem::Item { id, label, enabled } => {
                        // SAFETY: the selector matches `menu_item_clicked`
                        // above: one `id` argument, no return value. The
                        // target is `handler`, which `Tray` holds for as long
                        // as the item is on screen — AppKit keeps only a weak
                        // reference to an item's target.
                        unsafe {
                            let row = NSMenuItem::initWithTitle_action_keyEquivalent(
                                NSMenuItem::alloc(mtm),
                                &NSString::from_str(label),
                                Some(objc2::sel!(menuItemClicked:)),
                                &NSString::from_str(""),
                            );
                            row.setTarget(Some(&handler));
                            row.setTag(*id as isize);
                            row.setEnabled(*enabled);
                            menu.addItem(&row);
                        }
                    }
                }
            }

            status_item.setMenu(Some(&menu));
            target = Some(handler);
        }

        Ok(Self {
            status_item,
            _target: target,
            queue,
        })
    }

    pub fn events(&mut self) -> Vec<TrayEvent> {
        drain(&self.queue)
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // Removing the item from the status bar is the menu-bar half of
        // dropping: without it the icon outlives the object that owns it.
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status_item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// libtest runs on a worker thread, so the main-thread requirement is the
    /// first thing every call here meets — an assertion about the arm, not a
    /// skipped test.
    #[test]
    fn the_status_item_refuses_off_the_main_thread() {
        let err = Tray::show("app", &[MenuItem::new(1, "Quit")])
            .err()
            .expect("refused off the main thread");
        assert!(err.contains("main thread"), "{err}");
    }
}
