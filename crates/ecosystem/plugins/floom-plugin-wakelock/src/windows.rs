//! Windows screen-awake via `SetThreadExecutionState`.
//!
//! The classic keep-awake: `ES_CONTINUOUS | ES_DISPLAY_REQUIRED` asks that
//! the display not sleep while the state stands; `ES_CONTINUOUS` alone
//! releases the ask.
//!
//! # The API is thread-scoped, and the arm says so
//!
//! `SetThreadExecutionState` sets the state of *the calling thread*, and
//! the documentation is explicit that one thread cannot clear a state
//! another set. A boolean API over that has to be honest about it: if the
//! release arrives from a different thread than the acquire, the arm
//! returns an `Err` naming exactly that, rather than a silent no-op that
//! would leave the display held. The acquiring thread is remembered for
//! the comparison — which is also why `is_keep_screen_on` here reports what
//! this crate asked: querying the OS's state is not possible without
//! setting it, and a read that changes the world is not a read.

use std::sync::Mutex;

use windows_sys::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, SetThreadExecutionState,
};

/// The state plus the thread that set it, because the API is thread-scoped.
///
/// Guarded rather than atomic: the acquire needs "is it already held by
/// *this* thread" answered as one step, and two racing acquires from the
/// same thread must not double-set.
fn state() -> &'static Mutex<Option<std::thread::ThreadId>> {
    static STATE: std::sync::OnceLock<Mutex<Option<std::thread::ThreadId>>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    let mut state = state()
        .lock()
        .map_err(|_| "keep_screen_on: the state is poisoned".to_string())?;
    match (on, *state) {
        (true, Some(thread)) if thread == std::thread::current().id() => Ok(()),
        (true, Some(_)) => Err(
            "keep_screen_on: the display ask on Windows is thread-scoped, and it is \
             already held by another thread — release it from that thread"
                .into(),
        ),
        (false, None) => Ok(()),
        (false, Some(thread)) => {
            if thread != std::thread::current().id() {
                // Clearing from the wrong thread is the documented silent
                // no-op; an honest Err beats a `Ok` that changed nothing.
                return Err(
                    "keep_screen_on: the display ask on Windows is thread-scoped, and \
                     was set by another thread — release it from that thread"
                        .into(),
                );
            }
            let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
            *state = None;
            if previous == 0 {
                return Err("keep_screen_on: SetThreadExecutionState failed".into());
            }
            Ok(())
        }
        (true, None) => {
            let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED) };
            if previous == 0 {
                return Err("keep_screen_on: SetThreadExecutionState failed".into());
            }
            *state = Some(std::thread::current().id());
            Ok(())
        }
    }
}

pub fn is_keep_screen_on() -> bool {
    state().lock().map(|s| s.is_some()).unwrap_or(false)
}
