//! Navigation ↔ widget binding.
//!
//! [`floem-navigation`](floem_navigation) is deliberately widget-free: it
//! models page stacks as data. This module is the glue that renders that data
//! through the composite layer — a [`NavHost`] whose children are the
//! navigator's pages, keyed by stable page key so per-page widget state
//! survives pushes and pops.
//!
//! ```text
//! NavHandle (app owns)          NavHost (in the tree)
//! ┌────────────────────┐        ┌──────────────────────────┐
//! │ Mutex<Navigator>   │◀─read──│ children(): pages keyed  │
//! │ dirty: AtomicBool  │◀─set───│ by page key, absolute-fill│
//! └────────────────────┘        └──────────────────────────┘
//! ```
//!
//! The app mutates navigation through the [`NavHandle`] (which also raises the
//! dirty flag); the widget's `update` sees the flag and marks its children
//! dirty, so the next `frame()` reconciles the page set. The popped page stays
//! mounted while [`Navigator::exiting`] holds it — the exit-transition window
//! — and unmounts when the app calls [`NavHandle::finish_exit`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use floem_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext, SemanticsContext,
    UpdateContext, Widget, WidgetAny,
};
use floem_navigation::{Navigator, Page};

/// Shared navigation state: the [`Navigator`] plus a dirty flag the app-side
/// handle raises after every mutation.
struct NavState {
    navigator: Mutex<Navigator>,
    dirty: AtomicBool,
}

/// The app's handle onto a [`NavHost`]'s page stack.
///
/// Cheap to clone; every navigation method marks the bound widget dirty so the
/// next [`WidgetHost::frame`] picks the change up. Navigation mutations are
/// `&self` — interior mutability keeps callbacks (`on_click` closures and
/// composite builders, which are `Fn`) able to drive navigation.
#[derive(Clone)]
pub struct NavHandle(Arc<NavState>);

impl NavHandle {
    /// A handle whose stack starts with one root page.
    pub fn new(root_path: impl Into<String>) -> Self {
        Self(Arc::new(NavState {
            navigator: Mutex::new(Navigator::new(root_path)),
            dirty: AtomicBool::new(false),
        }))
    }

    /// Wrap an existing [`Navigator`].
    pub fn from_navigator(navigator: Navigator) -> Self {
        Self(Arc::new(NavState {
            navigator: Mutex::new(navigator),
            dirty: AtomicBool::new(false),
        }))
    }

    /// Push a page; returns its stable key.
    pub fn push(&self, path: impl Into<String>) -> u64 {
        let key = self.lock().push(path);
        self.mark_dirty();
        key
    }

    /// Pop the top page. The popped page stays mounted (as the exiting page)
    /// until [`Self::finish_exit`]. Returns `false` at the root.
    pub fn pop(&self) -> bool {
        let popped = self.lock().pop();
        if popped {
            self.mark_dirty();
        }
        popped
    }

    /// Pop everything above the root; returns how many pages left the stack.
    pub fn pop_to_root(&self) -> usize {
        let removed = self.lock().pop_to_root();
        if removed > 0 {
            self.mark_dirty();
        }
        removed
    }

    /// Drop the exiting page, ending its exit transition.
    pub fn finish_exit(&self) {
        let exited = self.lock().exiting().is_some();
        self.lock().finish_exit();
        if exited {
            self.mark_dirty();
        }
    }

    /// The topmost page (a snapshot).
    pub fn top(&self) -> Page {
        self.lock().top().clone()
    }

    /// Back-stack depth excluding the root.
    pub fn depth(&self) -> usize {
        self.lock().pages().len() - 1
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Navigator> {
        self.0
            .navigator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn mark_dirty(&self) {
        self.0.dirty.store(true, Ordering::Release);
    }

    fn take_dirty(&self) -> bool {
        self.0.dirty.swap(false, Ordering::AcqRel)
    }
}

/// Renders a [`NavHandle`]'s page stack as its children.
///
/// Every page slot is `absolute_fill`, so pages overlay in stack order (top
/// page painted last, on top). Children are keyed by the page's stable key: a
/// push mounts exactly one new widget, a pop keeps the popped widget mounted
/// until [`NavHandle::finish_exit`], and everything else is reused as-is.
#[derive(Clone)]
pub struct NavHost {
    handle: NavHandle,
    build_page: Arc<dyn Fn(&Page) -> Box<dyn WidgetAny> + Send + Sync>,
}

impl NavHost {
    /// A host that builds each page through `build_page`.
    pub fn new(
        handle: NavHandle,
        build_page: impl Fn(&Page) -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            handle,
            build_page: Arc::new(build_page),
        }
    }

    /// A stable `ChildKey` for a page. Live pages and the exiting page share
    /// one key space (page keys are unique and monotonically assigned), which
    /// is what makes "same page, same widget" hold across any transition.
    fn key_for(page: &Page) -> ChildKey {
        ChildKey::keyed(page.key)
    }
}

impl Widget for NavHost {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The navigator lives behind a shared handle, so the only way `update`
        // learns about navigation is the dirty flag the handle raises.
        if self.handle.take_dirty() {
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let navigator = self.handle.lock();
        let mut slots = Vec::with_capacity(navigator.pages().len() + 1);
        // The exiting page renders beneath the live stack (it is leaving).
        if let Some(page) = navigator.exiting() {
            slots.push(Child::new(Self::key_for(page), page_layout(), {
                let page = page.clone();
                let build = Arc::clone(&self.build_page);
                move || build(&page)
            }));
        }
        for page in navigator.pages() {
            slots.push(Child::new(Self::key_for(page), page_layout(), {
                let page = page.clone();
                let build = Arc::clone(&self.build_page);
                move || build(&page)
            }));
        }
        slots
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

/// Every page covers the whole host: pages overlay, they do not flow.
fn page_layout() -> LayoutRequest {
    LayoutRequest::new().absolute_fill()
}

impl NavHost {
    /// Accessibility description: a plain container (the pages speak for
    /// themselves).
    pub fn semantics_hint(&self) -> crate::primitives::SemanticsHint {
        crate::primitives::SemanticsHint::decorative()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// A bare page widget carrying its page path, for counting children.
    struct Probe(String);

    impl Widget for Probe {
        fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
            cx.alloc_child()
        }

        fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
    }

    impl WidgetAny for Probe {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn semantics_hint(&self) -> Option<floem_core::SemanticsHint> {
            None
        }
    }

    fn build_probe() -> Box<dyn WidgetAny> {
        Box::new(Probe(String::new()))
    }

    #[test]
    fn handle_mutations_raise_the_dirty_flag() {
        let nav = NavHandle::new("root");
        assert!(!nav.take_dirty(), "fresh handle is clean");
        nav.push("detail");
        assert!(nav.take_dirty());
        assert!(!nav.take_dirty(), "flag is consumed by take");
        nav.pop();
        assert!(nav.take_dirty());
        // Popping at the root is a no-op and stays clean.
        assert!(!nav.pop());
        assert!(!nav.take_dirty());
    }

    #[test]
    fn children_mirror_pages_with_stable_keys() {
        let nav = NavHandle::new("root");
        let detail = nav.push("detail");
        let host = NavHost::new(nav.clone(), |_| Box::new(Probe(String::new())) as _);

        let children = host.children();
        assert_eq!(children.len(), 2, "root + detail");
        assert_eq!(
            children[1].key,
            ChildKey::keyed(detail),
            "the pushed page keeps the key the handle returned"
        );

        // Pop moves the page to `exiting`: still mounted, same key.
        assert!(nav.pop());
        let children = host.children();
        assert_eq!(children.len(), 2, "exiting page stays mounted");
        assert_eq!(children[0].key, ChildKey::keyed(detail));

        // Finishing the exit unmounts it.
        nav.finish_exit();
        assert_eq!(host.children().len(), 1, "only the root remains");
    }

    #[test]
    fn builders_receive_the_page_they_built_for() {
        let nav = NavHandle::new("home");
        nav.push("settings");
        let host = NavHost::new(nav, |page| {
            Box::new(Probe(page.path.clone())) as Box<dyn WidgetAny>
        });

        let children = host.children();
        // Rebuild both slots and check each builder saw its own page.
        let paths: Vec<String> = children
            .iter()
            .map(|child| {
                let probe = child.build();
                let probe = probe
                    .as_any()
                    .downcast_ref::<Probe>()
                    .expect("builder produced a Probe");
                probe.0.clone()
            })
            .collect();
        assert_eq!(paths, vec!["home".to_string(), "settings".to_string()]);
    }

    #[test]
    fn pop_to_root_and_finish_exit_unmount_everything_above() {
        let nav = NavHandle::new("root");
        nav.push("a");
        nav.push("b");
        let host = NavHost::new(nav.clone(), |_| build_probe());

        assert_eq!(nav.pop_to_root(), 2);
        // Only the *last* popped page gets an exit window (Navigator's
        // contract); the rest are gone immediately.
        assert_eq!(host.children().len(), 2, "root + one exiting page");
        nav.finish_exit();
        assert_eq!(host.children().len(), 1);
        assert_eq!(nav.depth(), 0);
    }

    #[test]
    fn probes_are_send_sync_compatible_builders() {
        // The builder closure must be `Fn + Send + Sync` — a regression here
        // would poison every composite a NavHost sits inside.
        fn assert_build<F: Fn() -> Box<dyn WidgetAny> + Send + Sync>(_f: F) {}
        let nav = NavHandle::new("root");
        let count = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::clone(&count);
        assert_build(move || {
            count2.fetch_add(1, Ordering::SeqCst);
            build_probe()
        });
        let _ = NavHost::new(nav, |_| build_probe());
    }
}
