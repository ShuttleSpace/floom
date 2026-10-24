//! Scoped, typed service registry (DI) for Floom 2.0.
//!
//! Widget-free (framework sits below widgets). The model:
//!
//! * [`Services`] — a registry mapping `TypeId` to shared instances. Typed
//!   first: `provide`/`require`/`get` take the type parameter, so there are no
//!   string keys and no downcasting in app code.
//! * [`Scope`] — a child registry with a parent. Resolution walks outward
//!   (child first, then ancestors), which is how a window or a route gets its
//!   own session-scoped services while still seeing app-wide ones.
//! * Disposal — services may register a [`Disposal`] callback that runs when
//!   the scope they live in drops. Child scopes dispose before their parent,
//!   children of one parent dispose in creation order (deterministic, easy to
//!   reason about in tests).
//!
//! Everything is single-threaded by design: a scope tree belongs to one UI
//! context, mirroring how widgets are used.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// A callback run when the scope owning the service is disposed.
pub type Disposal = Box<dyn Fn()>;

/// One registered service: the instance plus its optional disposal hook.
struct Entry {
    instance: Rc<dyn Any>,
    disposal: Option<Disposal>,
}

/// A typed service registry. Cheap to clone (all handles are `Rc`).
#[derive(Clone, Default)]
pub struct Services {
    inner: Rc<RefCell<Registry>>,
}

/// The registry's empty state is also its default one: no entries, not
/// disposing.
#[derive(Default)]
struct Registry {
    entries: Vec<(TypeId, Entry)>,
    /// Set while a disposal callback is running, so a service trying to
    /// resolve from a dying scope fails loudly instead of resurrecting state.
    disposing: bool,
}

impl Registry {
    fn index_of(&self, id: TypeId) -> Option<usize> {
        self.entries.iter().position(|(tid, _)| *tid == id)
    }
}

impl std::fmt::Debug for Services {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Services")
            .field("services", &inner.entries.len())
            .field("disposing", &inner.disposing)
            .finish()
    }
}

impl Services {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) the instance for `T`.
    ///
    /// Replacing an existing registration runs the old entry's disposal
    /// callback immediately — a scope never holds two live instances of one
    /// service type.
    pub fn provide<T: Any>(&self, instance: T) {
        self.provide_with_disposal(instance, Option::<fn(Rc<T>)>::None);
    }

    /// Register `T` with an optional disposal callback that receives a fresh
    /// handle to the instance when the scope drops (or the value is replaced).
    pub fn provide_with_disposal<T: Any>(
        &self,
        instance: T,
        disposal: Option<impl Fn(Rc<T>) + 'static>,
    ) {
        let instance = Rc::new(instance);
        let entry = Entry {
            instance: Rc::clone(&instance) as Rc<dyn Any>,
            disposal: disposal.map(|disposal| {
                // The typed handle is captured now; the callback is exactly
                // `Fn(Rc<T>)` so no downcast is needed later.
                let typed = Rc::clone(&instance);
                Box::new(move || disposal(Rc::clone(&typed))) as Disposal
            }),
        };
        let mut inner = self.inner.borrow_mut();
        if let Some(index) = inner.index_of(TypeId::of::<T>()) {
            let (_, old) = inner.entries.remove(index);
            if let Some(disposal) = old.disposal {
                disposal();
            }
        }
        inner.entries.push((TypeId::of::<T>(), entry));
    }

    /// A shared handle to the registered `T`, if present.
    pub fn get<T: Any>(&self) -> Option<Rc<T>> {
        let inner = self.inner.borrow();
        let index = inner.index_of(TypeId::of::<T>())?;
        let instance = inner.entries[index].1.instance.clone();
        drop(inner);
        instance.downcast::<T>().ok()
    }

    /// Like [`get`](Self::get) but panics with the type name when missing —
    /// for services the app *knows* its composition root provided.
    pub fn require<T: Any>(&self) -> Rc<T> {
        self.get::<T>().unwrap_or_else(|| {
            panic!(
                "service {} is not registered in this scope",
                std::any::type_name::<T>()
            )
        })
    }

    /// Whether `T` is registered here (ignores ancestors — see [`Scope::get`]).
    pub fn contains<T: Any>(&self) -> bool {
        self.inner.borrow().index_of(TypeId::of::<T>()).is_some()
    }

    /// Remove `T`, running its disposal callback. Returns whether it existed.
    pub fn remove<T: Any>(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        match inner.index_of(TypeId::of::<T>()) {
            Some(index) => {
                let (_, entry) = inner.entries.remove(index);
                drop(inner);
                if let Some(disposal) = entry.disposal {
                    disposal();
                }
                true
            }
            None => false,
        }
    }

    /// Run every disposal callback this registry holds, newest first.
    fn dispose(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.disposing = true;
        let entries = std::mem::take(&mut inner.entries);
        drop(inner);
        for (_, entry) in entries.into_iter().rev() {
            if let Some(disposal) = entry.disposal {
                disposal();
            }
        }
    }
}

/// A child registry tied to a lifetime (a window, a route, a dialog).
///
/// The scope *owns* its [`Services`]; when it drops, those services dispose.
/// Resolution falls back to a shared parent, so route-scoped services can see
/// app-scoped ones without re-registering.
pub struct Scope {
    services: Services,
    parent: Option<Services>,
}

impl Scope {
    /// A root scope: no parent, disposal on drop.
    pub fn root() -> Self {
        Self {
            services: Services::new(),
            parent: None,
        }
    }

    /// A child scope over `parent`. The parent stays alive independently (it
    /// is shared, not owned); only the child's own registrations dispose here.
    pub fn child(parent: &Services) -> Self {
        Self {
            services: Services::new(),
            parent: Some(parent.clone()),
        }
    }

    /// This scope's own registry (for providing).
    pub fn services(&self) -> &Services {
        &self.services
    }

    /// The shared parent registry, if this is a child scope.
    pub fn parent(&self) -> Option<&Services> {
        self.parent.as_ref()
    }

    /// Resolve `T` from this scope, then the parent chain.
    pub fn get<T: Any>(&self) -> Option<Rc<T>> {
        self.services
            .get::<T>()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get::<T>()))
    }

    /// [`Scope::get`] but panics with the type name when unresolved.
    pub fn require<T: Any>(&self) -> Rc<T> {
        self.get::<T>().unwrap_or_else(|| {
            panic!(
                "service {} is not registered in this scope or its ancestors",
                std::any::type_name::<T>()
            )
        })
    }

    /// Whether `T` resolves from this scope or any ancestor.
    pub fn contains<T: Any>(&self) -> bool {
        self.get::<T>().is_some()
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.services.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Theme {
        name: &'static str,
    }

    struct Session {
        user: &'static str,
    }

    /// Records disposal order in a shared vec (single-threaded by design).
    type DisposalLog = Rc<RefCell<Vec<&'static str>>>;

    fn log_disposal(log: &DisposalLog, name: &'static str) -> DisposalLog {
        log.borrow_mut().push(name);
        log.clone()
    }

    #[test]
    fn typed_provide_and_resolve() {
        let services = Services::new();
        assert!(services.get::<Theme>().is_none());

        services.provide(Theme { name: "dark" });
        assert!(services.contains::<Theme>());
        let theme = services.get::<Theme>().unwrap();
        assert_eq!(theme.name, "dark");

        // Handles are shared: mutation through one is visible via another.
        assert!(Rc::ptr_eq(&theme, &services.require::<Theme>()));
    }

    #[test]
    fn require_panics_with_type_name_when_missing() {
        let services = Services::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            services.require::<Session>()
        }));
        let payload = result.err().expect("require must panic when missing");
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        assert!(
            message.contains("floom_services::tests::Session"),
            "panic names the missing type, got: {message}"
        );
    }

    #[test]
    fn replacing_a_service_disposes_the_old_instance() {
        let log: DisposalLog = Rc::new(RefCell::new(Vec::new()));
        let services = Services::new();

        services.provide_with_disposal(
            Theme { name: "dark" },
            Some({
                let log = log.clone();
                move |_| log.borrow_mut().push("old theme")
            }),
        );
        services.provide_with_disposal(
            Theme { name: "light" },
            Some({
                let log = log.clone();
                move |_| log.borrow_mut().push("new theme")
            }),
        );

        assert_eq!(*log.borrow(), vec!["old theme"]);
        assert_eq!(services.require::<Theme>().name, "light");
    }

    #[test]
    fn scope_resolves_through_the_parent_chain() {
        let parent = Services::new();
        parent.provide(Theme { name: "dark" });

        let scope = Scope::child(&parent);
        // Child sees the parent's service without re-registering…
        assert_eq!(scope.get::<Theme>().unwrap().name, "dark");
        // …and its own service stays its own.
        scope.services().provide(Session { user: "ada" });
        assert_eq!(scope.require::<Session>().user, "ada");
        assert!(
            !parent.contains::<Session>(),
            "child registrations never leak upward"
        );
    }

    #[test]
    fn scope_drop_disposes_its_services_and_shadows_correctly() {
        let log: DisposalLog = Rc::new(RefCell::new(Vec::new()));
        let parent = Services::new();
        parent.provide_with_disposal(
            Theme { name: "app" },
            Some({
                let log = log.clone();
                move |_| {
                    let _ = log_disposal(&log, "parent theme");
                }
            }),
        );

        {
            let scope = Scope::child(&parent);
            scope.services().provide_with_disposal(
                Theme { name: "route" },
                Some({
                    let log = log.clone();
                    move |_| {
                        log.borrow_mut().push("route theme");
                    }
                }),
            );
            assert_eq!(scope.get::<Theme>().unwrap().name, "route");
        } // scope drops here → "route theme" disposes, parent untouched

        assert_eq!(*log.borrow(), vec!["route theme"]);
        assert_eq!(parent.get::<Theme>().unwrap().name, "app");
    }

    #[test]
    fn remove_runs_disposal_exactly_once() {
        let log: DisposalLog = Rc::new(RefCell::new(Vec::new()));
        let services = Services::new();
        services.provide_with_disposal(
            Session { user: "ada" },
            Some({
                let log = log.clone();
                move |_| log.borrow_mut().push("session gone")
            }),
        );

        assert!(services.remove::<Session>());
        assert!(!services.remove::<Session>(), "second remove is a no-op");
        assert_eq!(*log.borrow(), vec!["session gone"]);
        assert!(services.get::<Session>().is_none());
    }

    #[test]
    fn disposal_sees_the_instance_handle() {
        let seen = Rc::new(Cell::new(0));
        let services = Services::new();
        services.provide_with_disposal(
            42u32,
            Some({
                let seen = seen.clone();
                move |value: Rc<u32>| seen.set(*value)
            }),
        );
        services.remove::<u32>();
        assert_eq!(seen.get(), 42, "disposal received the live instance");
    }
}
