use std::{
    any::Any, cell::RefCell, collections::HashSet, fmt, marker::PhantomData, rc::Rc, sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    create_effect, create_updater,
    id::Id,
    memo::{Memo, create_memo},
    runtime::{RUNTIME, Runtime},
    signal::{ReadSignal, RwSignal, SignalState, SignalValue, WriteSignal},
    storage::{SyncStorage, UnsyncStorage},
    trigger::{Trigger, create_trigger},
};

/// You can manually control Signal's lifetime by using Scope.
///
/// Every Signal has a Scope created explicitly or implicitly,
/// and when you Dispose the Scope, it will clean up all the Signals
/// that belong to the Scope and all the child Scopes
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Scope(pub(crate) Id, pub(crate) PhantomData<()>);

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Scope");
        s.field("id", &self.0);
        s.finish()
    }
}

impl Scope {
    /// Create a new Scope that isn't a child or parent of any scope
    pub fn new() -> Self {
        Self(Id::next(), PhantomData)
    }

    /// The current Scope in the Runtime. Any Signal/Effect/Memo created with
    /// implicitly Scope will be under this Scope
    pub fn current() -> Scope {
        RUNTIME.with(|runtime| Scope(*runtime.current_scope.borrow(), PhantomData))
    }

    /// Create a child Scope of this Scope
    pub fn create_child(&self) -> Scope {
        let child = Id::next();
        RUNTIME.with(|runtime| {
            runtime
                .children
                .borrow_mut()
                .entry(self.0)
                .or_default()
                .insert(child);
            runtime.parents.borrow_mut().insert(child, self.0);
        });
        Scope(child, PhantomData)
    }

    /// Re-parent this scope to be a child of another scope.
    ///
    /// If this scope already has a parent, it will be removed from that parent first.
    /// This is useful when the scope hierarchy needs to be adjusted after construction
    /// to match the view hierarchy.
    ///
    /// # Example
    /// ```rust
    /// # use floom_legacy_reactive::Scope;
    /// let parent = Scope::new();
    /// let child = Scope::new(); // Initially has no parent
    ///
    /// child.set_parent(parent);
    /// // Now child is a child of parent, and will be disposed when parent is disposed
    /// ```
    pub fn set_parent(&self, new_parent: Scope) {
        RUNTIME.with(|runtime| {
            // Remove from old parent's children set (if any)
            if let Some(old_parent) = runtime.parents.borrow_mut().remove(&self.0)
                && let Some(children) = runtime.children.borrow_mut().get_mut(&old_parent)
            {
                children.remove(&self.0);
            }

            // Add to new parent's children set
            runtime
                .children
                .borrow_mut()
                .entry(new_parent.0)
                .or_default()
                .insert(self.0);

            // Set new parent
            runtime.parents.borrow_mut().insert(self.0, new_parent.0);
        });
    }

    /// Returns the parent scope of this scope, if any.
    pub fn parent(&self) -> Option<Scope> {
        RUNTIME.with(|runtime| {
            runtime
                .parents
                .borrow()
                .get(&self.0)
                .map(|id| Scope(*id, PhantomData))
        })
    }

    /// Create a new Signal under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_signal<T>(self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Any + 'static,
    {
        self.enter(|| RwSignal::new_split(value))
    }

    /// Create a RwSignal under this Scope (local/unsync by default)
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_rw_signal<T>(self, value: T) -> RwSignal<T>
    where
        T: Any + 'static,
    {
        self.enter(|| RwSignal::new(value))
    }

    /// Create a sync Signal under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_sync_signal<T>(
        self,
        value: T,
    ) -> (ReadSignal<T, SyncStorage>, WriteSignal<T, SyncStorage>)
    where
        T: Any + Send + Sync + 'static,
    {
        self.enter(|| RwSignal::<T, SyncStorage>::new_sync_split(value))
    }

    /// Create a sync RwSignal under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_sync_rw_signal<T>(self, value: T) -> RwSignal<T, SyncStorage>
    where
        T: Any + Send + Sync + 'static,
    {
        self.enter(|| RwSignal::<T, SyncStorage>::new_sync(value))
    }

    /// Create a local (unsync) Signal under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_local_signal<T>(
        self,
        value: T,
    ) -> (ReadSignal<T, UnsyncStorage>, WriteSignal<T, UnsyncStorage>)
    where
        T: Any + 'static,
    {
        self.enter(|| RwSignal::new_split(value))
    }

    /// Create a local (unsync) RwSignal under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_local_rw_signal<T>(self, value: T) -> RwSignal<T, UnsyncStorage>
    where
        T: Any + 'static,
    {
        self.enter(|| RwSignal::new(value))
    }

    /// Create a Memo under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_memo<T>(self, f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
    where
        T: PartialEq + 'static,
    {
        self.enter(|| create_memo(f))
    }

    /// Create a Trigger under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_trigger(self) -> Trigger {
        self.enter(create_trigger)
    }

    /// Create effect under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_effect<T>(self, f: impl Fn(Option<T>) -> T + 'static)
    where
        T: Any + 'static,
    {
        self.enter(|| create_effect(f))
    }

    /// Create updater under this Scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_updater<R>(
        self,
        compute: impl Fn() -> R + 'static,
        on_change: impl Fn(R) + 'static,
    ) -> R
    where
        R: 'static,
    {
        self.enter(|| create_updater(compute, on_change))
    }

    /// Store a context value in this scope.
    ///
    /// The stored context value can be retrieved by this scope and any of its
    /// descendants using [`Scope::get_context`] or [`Context::get`](crate::Context::get).
    /// Child scopes can provide their own values of the same type, which will
    /// shadow the parent's value for that subtree.
    ///
    /// Context values are automatically cleaned up when the scope is disposed.
    ///
    /// # Example
    /// ```rust
    /// # use floom_legacy_reactive::Scope;
    /// let scope = Scope::new();
    /// scope.provide_context(42i32);
    /// scope.enter(|| {
    ///     assert_eq!(scope.get_context::<i32>(), Some(42));
    /// });
    /// ```
    pub fn provide_context<T>(&self, value: T)
    where
        T: Clone + 'static,
    {
        self.enter(|| crate::context::Context::provide(value))
    }

    /// Try to retrieve a stored context value from this scope or its ancestors.
    ///
    /// Context lookup walks up the scope tree from this scope to find the
    /// nearest ancestor that provides a value of the requested type.
    ///
    /// Note: This method must be called while the scope is entered (i.e., inside
    /// [`Scope::enter`]) for the lookup to work correctly.
    ///
    /// # Example
    /// ```rust
    /// # use floom_legacy_reactive::Scope;
    /// let parent = Scope::new();
    /// parent.provide_context(42i32);
    ///
    /// let child = parent.create_child();
    /// child.enter(|| {
    ///     // Child sees parent's context
    ///     assert_eq!(child.get_context::<i32>(), Some(42));
    /// });
    /// ```
    pub fn get_context<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.enter(crate::context::Context::get::<T>)
    }

    /// Runs the given closure within this scope.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn enter<T>(&self, f: impl FnOnce() -> T) -> T
    where
        T: 'static,
    {
        Runtime::assert_ui_thread();
        let prev_scope = RUNTIME.with(|runtime| {
            let mut current_scope = runtime.current_scope.borrow_mut();
            let prev_scope = *current_scope;
            *current_scope = self.0;
            prev_scope
        });

        let result = f();

        RUNTIME.with(|runtime| {
            *runtime.current_scope.borrow_mut() = prev_scope;
        });

        result
    }

    /// Wraps a closure so it runs under a new child scope of this scope.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn enter_child<T, U>(
        self,
        f: impl Fn(T) -> U + 'static,
    ) -> impl Fn(T) -> (U, Scope) + 'static
    where
        T: 'static,
    {
        Runtime::assert_ui_thread();
        let parent = self;
        move |t| {
            let scope = parent.create_child();
            let prev_scope = RUNTIME.with(|runtime| {
                let mut current_scope = runtime.current_scope.borrow_mut();
                let prev_scope = *current_scope;
                *current_scope = scope.0;
                prev_scope
            });

            let result = f(t);

            RUNTIME.with(|runtime| {
                *runtime.current_scope.borrow_mut() = prev_scope;
            });

            (result, scope)
        }
    }

    /// This is normally used in create_effect, and it will bind the effect's lifetime
    /// to this scope
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn track(&self) {
        Runtime::assert_ui_thread();
        let signal = if let Some(signal) = self.0.signal() {
            signal
        } else {
            let signal = SignalState {
                id: self.0,
                subscribers: Arc::new(Mutex::new(HashSet::new())),
                value: SignalValue::Local(Rc::new(RefCell::new(()))),
            };
            self.0.add_signal(signal.clone());
            signal
        };
        signal.subscribe();
    }

    /// Dispose this Scope, and it will cleanup all the Signals and child Scope
    /// of this Scope.
    pub fn dispose(&self) {
        self.0.dispose();
    }
}

#[deprecated(
    since = "0.2.0",
    note = "Use Scope::enter instead; this will be removed in a future release"
)]
/// Runs the given code with the given Scope
pub fn with_scope<T>(scope: Scope, f: impl FnOnce() -> T) -> T
where
    T: 'static,
{
    scope.enter(f)
}

/// Wrap the closure so that whenever the closure runs, it will be under a child Scope
/// of the current Scope
#[deprecated(
    since = "0.2.0",
    note = "Use Scope::current().enter_child instead; this will be removed in a future release"
)]
pub fn as_child_of_current_scope<T, U>(f: impl Fn(T) -> U + 'static) -> impl Fn(T) -> (U, Scope)
where
    T: 'static,
{
    let parent = Scope::current();
    move |t| {
        let scope = parent.create_child();
        let prev_scope = RUNTIME.with(|runtime| {
            let mut current_scope = runtime.current_scope.borrow_mut();
            let prev_scope = *current_scope;
            *current_scope = scope.0;
            prev_scope
        });

        let result = f(t);

        RUNTIME.with(|runtime| {
            *runtime.current_scope.borrow_mut() = prev_scope;
        });

        (result, scope)
    }
}

/// Returns the stable root [`Scope`] of the current runtime.
///
/// This is a convenience accessor that forwards to [`Runtime::root_scope`]. The
/// root scope is created when the runtime is constructed and is the scope that
/// signals/effects created outside of any explicitly entered scope are attached
/// to. Use it to create global, application-lifetime reactive state via the
/// `create_global_*` helpers, or to enter a scope that lives for the whole app.
///
/// # Example
/// ```rust
/// # use floom_legacy_reactive::{root_scope, create_global_rw_signal, SignalGet};
/// let counter = create_global_rw_signal(0);
/// root_scope().enter(|| {
///     assert_eq!(counter.get(), 0);
/// });
/// ```
pub fn root_scope() -> Scope {
    Runtime::root_scope()
}

/// Create a global [`RwSignal`] attached to the runtime's root scope.
///
/// The signal lives for the lifetime of the application (it is not disposed when
/// a view subtree is torn down) and can be accessed from anywhere without first
/// obtaining a scope. Its lifetime can still be explicitly managed by disposing
/// [`root_scope`].
///
/// # Example
/// ```rust
/// # use floom_legacy_reactive::create_global_rw_signal;
/// let theme = create_global_rw_signal(String::from("dark"));
/// ```
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_rw_signal<T>(value: T) -> RwSignal<T>
where
    T: Any + 'static,
{
    root_scope().create_rw_signal(value)
}

/// Create a global paired `(ReadSignal, WriteSignal)` attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_signal<T>(value: T) -> (ReadSignal<T>, WriteSignal<T>)
where
    T: Any + 'static,
{
    root_scope().create_signal(value)
}

/// Create a global sync [`RwSignal`] (for cross-thread state) attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_sync_rw_signal<T>(value: T) -> RwSignal<T, SyncStorage>
where
    T: Any + Send + Sync + 'static,
{
    root_scope().create_sync_rw_signal(value)
}

/// Create a global paired sync signal attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_sync_signal<T>(
    value: T,
) -> (ReadSignal<T, SyncStorage>, WriteSignal<T, SyncStorage>)
where
    T: Any + Send + Sync + 'static,
{
    root_scope().create_sync_signal(value)
}

/// Create a global local (unsync) [`RwSignal`] attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_local_rw_signal<T>(value: T) -> RwSignal<T, UnsyncStorage>
where
    T: Any + 'static,
{
    root_scope().create_local_rw_signal(value)
}

/// Create a global paired local signal attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_local_signal<T>(
    value: T,
) -> (ReadSignal<T, UnsyncStorage>, WriteSignal<T, UnsyncStorage>)
where
    T: Any + 'static,
{
    root_scope().create_local_signal(value)
}

/// Create a global [`Memo`] attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_memo<T>(f: impl Fn(Option<&T>) -> T + 'static) -> Memo<T>
where
    T: PartialEq + 'static,
{
    root_scope().create_memo(f)
}

/// Create a global [`Trigger`] attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_trigger() -> Trigger {
    root_scope().create_trigger()
}

/// Create a global effect attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_effect<T>(f: impl Fn(Option<T>) -> T + 'static)
where
    T: Any + 'static,
{
    root_scope().create_effect(f)
}

/// Create a global updater attached to the root scope.
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_global_updater<R>(
    compute: impl Fn() -> R + 'static,
    on_change: impl Fn(R) + 'static,
) -> R
where
    R: 'static,
{
    root_scope().create_updater(compute, on_change)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::{SignalGet, SignalTrack, SignalUpdate, SignalWrite};

    use super::*;

    #[test]
    fn root_scope_is_stable_and_default_current() {
        let root = root_scope();
        // The root scope is the current scope before any scope is entered.
        assert_eq!(Scope::current(), root);
        // root_scope() returns the same id on repeated calls.
        assert_eq!(root_scope(), root);
    }

    #[test]
    fn global_signals_attach_to_root_scope() {
        let counter = create_global_rw_signal(0);
        let root = root_scope();

        // The global signal lives on the root scope tree.
        root.enter(|| {
            assert_eq!(counter.get(), 0);
        });

        counter.set(5);
        root.enter(|| {
            assert_eq!(counter.get(), 5);
        });

        // A brand-new child scope can still read the root-scoped global.
        let child = root.create_child();
        child.enter(|| {
            assert_eq!(counter.get(), 5);
        });
    }

    #[test]
    fn global_signal_survives_unrelated_scope_disposal() {
        let global = create_global_rw_signal(1);
        let local_scope = Scope::new();
        let (local, _set_local) = local_scope.create_signal(2);
        local_scope.enter(|| assert_eq!(local.get(), 2));

        // Disposing an unrelated scope must not affect the root-scoped global.
        local_scope.dispose();
        Runtime::drain_pending_work();
        assert_eq!(global.get(), 1);

        // The local signal should be gone, the global should still be alive.
        global.set(42);
        assert_eq!(global.get(), 42);
    }

    #[test]
    fn global_memo_and_trigger_work() {
        let input = create_global_rw_signal(2);
        let doubled = create_global_memo(move |_| input.get() * 2);
        let trigger = create_global_trigger();

        assert_eq!(doubled.get(), 4);
        input.set(5);
        Runtime::drain_pending_work();
        assert_eq!(doubled.get(), 10);
        // trigger exists on the root scope
        trigger.track();
        trigger.notify();
    }

    #[test]
    fn global_effect_runs_on_root_scope() {
        let signal = create_global_rw_signal(0);
        let count = Rc::new(Cell::new(0));
        create_global_effect({
            let signal = signal;
            let count = count.clone();
            move |_| {
                signal.track();
                count.set(count.get() + 1);
            }
        });

        assert_eq!(count.get(), 1);
        signal.set(1);
        Runtime::drain_pending_work();
        assert_eq!(count.get(), 2);
    }
}
