// SPDX-License-Identifier: MIT
//! Task spawning and async Resource state for Floom 2.0.
//!
//! Moved here from the standalone `floom-task` crate during crate
//! consolidation (2026-11-04). Executors are abstracted so desktop, WASM, and
//! tests can plug in without baking a concrete async runtime into framework
//! core.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{RwSignal, Scope, SignalGet, SignalUpdate};

/// Lifecycle of a fetched resource.
///
/// `Initial` is the default state and the only one that carries no value, so
/// `Default` is derived: the derive puts the bound on the enum's own parameters
/// only where a variant needs them, which `Initial` does not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResourceState<T, E> {
    #[default]
    Initial,
    Loading,
    Ready(T),
    Refreshing(T),
    Error(E),
}

/// Abstraction over how work is scheduled.
pub trait TaskExecutor {
    /// Run work on the UI / main cooperative queue.
    fn spawn_ui(&self, task: Box<dyn FnOnce()>);

    /// Run background work. Default forwards to [`Self::spawn_ui`].
    fn spawn(&self, task: Box<dyn FnOnce()>) {
        self.spawn_ui(task);
    }
}

/// Executor that runs tasks immediately on the calling thread.
///
/// Suitable for unit tests and no-runtime / WASM-style single-threaded adapters.
#[derive(Debug, Default, Clone, Copy)]
pub struct SyncExecutor;

impl TaskExecutor for SyncExecutor {
    fn spawn_ui(&self, task: Box<dyn FnOnce()>) {
        task();
    }
}

/// Spawn on the UI queue of `executor`.
pub fn ui_spawn(executor: &dyn TaskExecutor, task: impl FnOnce() + 'static) {
    executor.spawn_ui(Box::new(task));
}

/// Spawn background work on `executor`.
pub fn spawn(executor: &dyn TaskExecutor, task: impl FnOnce() + 'static) {
    executor.spawn(Box::new(task));
}

/// Cancellation token; flipping it asks in-flight loaders to stop applying results.
#[derive(Clone, Debug)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a live resource fetch.
///
/// Dropping the handle cancels the token. Prefer creating the resource under a
/// dedicated [`Scope`] and disposing that scope to cancel early.
pub struct Resource<T: 'static, E: 'static> {
    pub state: RwSignal<ResourceState<T, E>>,
    cancel: CancelToken,
    /// Keeps the owning scope alive for signal lifetime.
    _scope: Scope,
}

impl<T: Clone + 'static, E: Clone + 'static> Resource<T, E> {
    pub fn get(&self) -> ResourceState<T, E> {
        self.state.get()
    }

    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}

impl<T: 'static, E: 'static> Drop for Resource<T, E> {
    fn drop(&mut self) {
        self.cancel.cancel();
        self._scope.dispose();
    }
}

/// Start a resource fetch under a fresh child of `parent` (or a new scope).
///
/// `loader` runs via `executor.spawn`. If cancelled before completion, the
/// result is discarded and state stays at `Loading`.
pub fn resource<T, E, F>(parent: Scope, executor: &dyn TaskExecutor, loader: F) -> Resource<T, E>
where
    T: Clone + 'static,
    E: Clone + 'static,
    F: FnOnce(CancelToken) -> Result<T, E> + 'static,
{
    let scope = parent.create_child();
    let state = scope.create_rw_signal(ResourceState::<T, E>::Loading);
    let cancel = CancelToken::new();
    let cancel_for_task = cancel.clone();
    let state_for_task = state;

    spawn(executor, move || {
        if cancel_for_task.is_cancelled() {
            return;
        }
        let result = loader(cancel_for_task.clone());
        if cancel_for_task.is_cancelled() {
            return;
        }
        match result {
            Ok(value) => state_for_task.set(ResourceState::Ready(value)),
            Err(err) => state_for_task.set(ResourceState::Error(err)),
        }
    });

    Resource {
        state,
        cancel,
        _scope: scope,
    }
}

/// Refresh an existing ready/error resource; preserves last ready value while loading.
pub fn refresh<T, E, F>(resource: &Resource<T, E>, executor: &dyn TaskExecutor, loader: F)
where
    T: Clone + 'static,
    E: Clone + 'static,
    F: FnOnce(CancelToken) -> Result<T, E> + 'static,
{
    let previous = resource.state.get();
    let next = match previous {
        ResourceState::Ready(v) | ResourceState::Refreshing(v) => ResourceState::Refreshing(v),
        _ => ResourceState::Loading,
    };
    resource.state.set(next);

    let cancel = resource.cancel.clone();
    // Reset cancel for a new attempt while keeping the same token identity for Drop.
    // Callers who cancelled permanently should not refresh; clear only if not dropped.
    let cancel_for_task = cancel.clone();
    let state = resource.state;

    spawn(executor, move || {
        if cancel_for_task.is_cancelled() {
            return;
        }
        let result = loader(cancel_for_task.clone());
        if cancel_for_task.is_cancelled() {
            return;
        }
        match result {
            Ok(value) => state.set(ResourceState::Ready(value)),
            Err(err) => state.set(ResourceState::Error(err)),
        }
    });
}
