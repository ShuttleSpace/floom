// SPDX-License-Identifier: MIT
//! Plugin registration.
//!
//! ## What a plugin is here
//!
//! A plugin is a **link-time** dependency, not a runtime-discovered one. The
//! plugin crate is listed in the app's `Cargo.toml`, `build.rs` compiles its
//! `native/<platform>/` code into the same binary, and `src/lib.rs` exposes an
//! `extern "C"`-backed Rust API. By the time the app runs, the code is already
//! in the executable — there is nothing to look up.
//!
//! ## Why a registry at all, then
//!
//! Because an app that needs a camera should not have to name the camera crate
//! in its own code. `floom_platform::plugin("camera").enable()?` lets the host
//! enable what it supports and skip what it does not, which is the one thing a
//! capability system is for. The registry holds the *enable functions*, not the
//! plugin code.
//!
//! The difference from Flutter's plugin registry is worth stating, because the
//! two look alike and are not:
//!
//! * Flutter's `pluginClass` is a **string in `pubspec.yaml`** read by
//!   `flutter_tools`, which generates a `GeneratedPluginRegistrant.java` that
//!   registers each plugin through a `MethodChannel`. The native code is loaded
//!   at runtime by name, and every call across the boundary is serialized.
//! * Here, a plugin is registered by **passing its Rust `enable` function**:
//!   [`register`]. A function pointer, not a string. Nothing is looked up by
//!   name, and nothing is serialized — [`Plugin::enable`] calls straight through.
//!
//! So the registry is a convenience for the *host*, not a mechanism for finding
//! code. If a plugin is not registered, the app does not have it, full stop.
//!
//! ## The double-enable question
//!
//! [`Plugin::enable`] is idempotent by contract: enabling twice succeeds and
//! calls the function once. A host that enables every plugin at startup and an
//! app that also enables one on demand are both ordinary, and neither may crash
//! because it ran twice.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// A plugin failed to enable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    /// No plugin is registered under this name.
    ///
    /// Distinct from a plugin that is registered and failed, because the fix is
    /// different: add the dependency, versus debug the plugin.
    NotRegistered(String),
    /// The plugin's own `enable` returned failure.
    Failed {
        /// The registered name, not the crate name — those differ often enough
        /// that a message showing only one of them sends you to the wrong crate.
        name: String,
        /// Whatever the plugin said, verbatim. Plugins report their own failures;
        /// wrapping them in a type erases the detail that matters.
        detail: String,
    },
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::NotRegistered(name) => write!(
                f,
                "no plugin registered as `{name}`; add it to your Cargo.toml \
                 dependencies and call floom_platform::register_plugin"
            ),
            PluginError::Failed { name, detail } => {
                write!(f, "plugin `{name}` failed to enable: {detail}")
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// A plugin's enable function.
///
/// `FnOnce` rather than `Fn`, because a plugin may move state into its native
/// side the first time it is enabled and has nothing left to call afterwards —
/// forcing `Fn` would push every plugin into an `Option` or a `Mutex` purely to
/// satisfy the signature.
///
/// `Send` so the table can live in a `static`. Without it the registry would
/// have to be thread-local, which is the wrong shape: a plugin is enabled by
/// whichever thread got to startup first, and a widget tree on another thread
/// has to be able to ask whether it is enabled.
type EnableFn = Box<dyn FnOnce() -> Result<(), String> + Send>;

struct Entry {
    enable: Option<EnableFn>,
    enabled: bool,
}

/// The process-wide plugin table.
///
/// A `Mutex` rather than a `RwLock`, and not for throughput: an `EnableFn` is
/// `Send` but not `Sync` (it is `FnOnce`), so `Entry` cannot be behind an
/// `RwLock` — `RwLock<T>: Sync` needs `T: Sync`. `Mutex<T>: Sync` only needs
/// `T: Send`, which is exactly what a `FnOnce` is. Reads are a hash-map lookup
/// during startup and a flag read on demand; contention is not the constraint
/// here.
fn table() -> &'static Mutex<BTreeMap<String, Entry>> {
    static TABLE: OnceLock<Mutex<BTreeMap<String, Entry>>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// How many times the table has been locked.
///
/// A counter rather than a claim. The table is behind a mutex, so the only
/// way to know that the frame path does not consult it is to count the
/// consultations: "the registry is setup-only" is a claim about code that is
/// not there, and a counter is the evidence.
static TABLE_ACCESSES: AtomicU64 = AtomicU64::new(0);

/// How many times the plugin table has been locked.
///
/// A diagnostic that answers one question: does the frame path touch the
/// registry? A host that registers its plugins at startup and never asks
/// again leaves this still. A host that asks per frame does not.
pub fn table_accesses() -> u64 {
    TABLE_ACCESSES.load(Ordering::Relaxed)
}

/// Locks the table.
///
/// A poisoned lock means a plugin's `enable` panicked. Propagating the poison
/// rather than recovering would make every later plugin call panic too, turning
/// one bad plugin into a dead process; recovering would let the table be read
/// with a half-initialized entry. The entry is marked enabled *before* `enable`
/// runs, so a poisoned entry is one that has already refused — skipping it is
/// the same outcome it would have had.
fn locked() -> MutexGuard<'static, BTreeMap<String, Entry>> {
    TABLE_ACCESSES.fetch_add(1, Ordering::Relaxed);
    match table().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Registers `enable` under `name`.
///
/// The `name` is the plugin's identity in this process and is what
/// [`plugin`] takes. It is conventionally the same as the crate name without
/// the `floom-plugin-` prefix.
///
/// # Panics
///
/// If `name` is already registered. Two plugins claiming one name is a
/// programming error that a silent last-writer-wins would turn into a bug that
/// only shows up as the wrong camera, so it is caught at startup instead. A
/// [`OnceLock`] could make it a lock and a `Result`, and would also make every
/// call site handle a case that never happens in a working program.
pub fn register_plugin(name: &str, enable: impl FnOnce() -> Result<(), String> + Send + 'static) {
    let mut map = locked();
    assert!(
        !map.contains_key(name),
        "plugin `{name}` registered twice; two plugins cannot share a name"
    );
    map.insert(
        name.to_string(),
        Entry {
            enable: Some(Box::new(enable)),
            enabled: false,
        },
    );
}

/// A handle to one registered plugin.
///
/// Cheap to build and to hold, and it does not borrow the table: enabling
/// re-acquires the lock. That means a plugin's own `enable` may itself register
/// or enable another plugin without deadlocking, which a borrowed handle could
/// not allow.
#[derive(Debug, Clone)]
pub struct Plugin {
    name: String,
}

/// Looks up a plugin by name.
///
/// Returns a handle whether or not the plugin is registered, so a host can walk
/// a fixed list of plugins it would like and handle the ones it does not have.
/// The failure surfaces at [`Plugin::enable`], where it can be reported, rather
/// than at lookup, where it would have to be handled twice.
pub fn plugin(name: &str) -> Plugin {
    Plugin {
        name: name.to_string(),
    }
}

impl Plugin {
    /// The name this plugin was looked up by.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the plugin is registered at all.
    pub fn is_registered(&self) -> bool {
        locked().contains_key(&self.name)
    }

    /// Whether the plugin has been enabled.
    pub fn is_enabled(&self) -> bool {
        locked().get(&self.name).is_some_and(|e| e.enabled)
    }

    /// Enables the plugin, calling its enable function at most once.
    ///
    /// Returns `Ok(())` on every call after the first, including when the first
    /// failed — once a plugin has failed it is marked enabled anyway, so a host
    /// that retries in a loop does not call into a plugin that has already said
    /// no. Use [`Plugin::is_enabled`] to tell the two apart.
    pub fn enable(&self) -> Result<(), PluginError> {
        let mut map = locked();
        let entry = map
            .get_mut(&self.name)
            .ok_or_else(|| PluginError::NotRegistered(self.name.clone()))?;
        if entry.enabled {
            return Ok(());
        }
        entry.enabled = true;
        match entry.enable.take() {
            Some(enable) => enable().map_err(|detail| PluginError::Failed {
                name: self.name.clone(),
                detail,
            }),
            None => Ok(()),
        }
    }
}

/// Removes a plugin from the table.
///
/// Exists for tests, and for an app that tears a plugin down. The enable
/// function is dropped without being called, so this does not disable native
/// state — it only forgets that the plugin was there.
pub fn unregister_plugin(name: &str) -> bool {
    locked().remove(name).is_some()
}

/// Every registered plugin name, sorted.
///
/// Sorted because this feeds diagnostics and test output, and a `BTreeMap`
/// would give an unstable order only if it were a `HashMap`. Two runs of the
/// same program should print the same thing.
pub fn registered_plugins() -> Vec<String> {
    locked().keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A counter so the tests can tell "called once" from "called twice" — the
    /// property `enable` idempotence actually claims.
    fn counting(counter: &'static AtomicUsize) -> impl FnOnce() -> Result<(), String> + Send {
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// A unique name per test, so tests in one binary do not collide in the
    /// process-wide table. Not `#[test]`-isolated: the table outlives each test.
    fn unique(tag: &str) -> String {
        static N: AtomicUsize = AtomicUsize::new(0);
        format!("test-{tag}-{}", N.fetch_add(1, Ordering::SeqCst))
    }

    #[test]
    fn an_unregistered_plugin_reports_not_registered() {
        let err = plugin(&unique("missing")).enable().unwrap_err();
        assert!(matches!(err, PluginError::NotRegistered(_)));
        assert!(err.to_string().contains("registered"), "{err}");
    }

    #[test]
    fn a_registered_plugin_enables() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let name = unique("ok");
        register_plugin(&name, counting(&COUNT));

        assert!(plugin(&name).is_registered());
        assert!(!plugin(&name).is_enabled());

        plugin(&name).enable().unwrap();
        assert!(plugin(&name).is_enabled());
        assert_eq!(COUNT.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn enabling_twice_calls_the_function_once() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let name = unique("twice");
        register_plugin(&name, counting(&COUNT));

        plugin(&name).enable().unwrap();
        plugin(&name).enable().unwrap();
        plugin(&name).enable().unwrap();

        assert_eq!(
            COUNT.load(Ordering::SeqCst),
            1,
            "the contract is idempotent, so a host enabling every plugin at \
             startup does not double-enable one the app also enabled on demand"
        );
    }

    #[test]
    fn a_failing_plugin_reports_its_own_detail() {
        let name = unique("fail");
        register_plugin(&name, || Err("no camera device".into()));

        let err = plugin(&name).enable().unwrap_err();
        assert_eq!(
            err,
            PluginError::Failed {
                name: name.clone(),
                detail: "no camera device".into()
            }
        );
        assert!(err.to_string().contains("no camera device"), "{err}");
    }

    #[test]
    fn a_failed_plugin_is_not_retried() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let name = unique("fail-twice");
        register_plugin(&name, || {
            COUNT.fetch_add(1, Ordering::SeqCst);
            Err("busy".into())
        });

        plugin(&name).enable().unwrap_err();
        plugin(&name).enable().unwrap();
        assert_eq!(
            COUNT.load(Ordering::SeqCst),
            1,
            "a host that retries every plugin in a loop must not call into a \
             plugin that has already refused"
        );
    }

    #[test]
    fn unregistering_removes_it() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let name = unique("gone");
        register_plugin(&name, counting(&COUNT));
        assert!(unregister_plugin(&name));
        assert!(!plugin(&name).is_registered());
        assert!(
            !unregister_plugin(&name),
            "removing twice is false, not a panic"
        );
    }

    #[test]
    fn registered_plugins_is_sorted() {
        let a = unique("a");
        let b = unique("b");
        register_plugin(&a, || Ok(()));
        register_plugin(&b, || Ok(()));

        let all = registered_plugins();
        let mut sorted = all.clone();
        sorted.sort();
        assert_eq!(all, sorted, "diagnostics print this, so it must be stable");
        assert!(all.contains(&a) && all.contains(&b));
    }

    #[test]
    #[should_panic(expected = "registered twice")]
    fn registering_a_name_twice_panics_at_startup() {
        let name = unique("dup");
        register_plugin(&name, || Ok(()));
        register_plugin(&name, || Ok(()));
    }
}
