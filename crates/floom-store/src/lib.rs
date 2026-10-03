//! Central store with fine-grained selectors over Floom reactive signals.

use floom_legacy_reactive::{Memo, RwSignal, SignalGet, SignalUpdate, SignalWith};

/// A reactive store holding a single root value.
#[derive(Debug)]
pub struct Store<T: 'static> {
    inner: RwSignal<T>,
}

impl<T: 'static> Copy for Store<T> {}

impl<T: 'static> Clone for Store<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Create a new [`Store`] with the given initial value.
pub fn store<T: 'static>(value: T) -> Store<T> {
    Store {
        inner: RwSignal::new(value),
    }
}

impl<T: 'static> Store<T> {
    /// Read the full store value (tracks the root signal).
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.get()
    }

    /// Mutate the store value in place.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.inner.update(f);
    }

    /// Replace the store value.
    pub fn set(&self, value: T) {
        self.inner.set(value);
    }

    /// Derive a memoized selector. Dependents of the memo are notified only when
    /// the selected `U` changes (`PartialEq`), even if other store fields update.
    pub fn select<U>(&self, selector: impl Fn(&T) -> U + 'static) -> Memo<U>
    where
        U: PartialEq + 'static,
    {
        let inner = self.inner;
        Memo::new(move |_| inner.with(|value| selector(value)))
    }

    /// Access the underlying `RwSignal` for advanced composition.
    pub fn signal(&self) -> RwSignal<T> {
        self.inner
    }
}
