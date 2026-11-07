//! Keep the screen awake while the app is in the foreground.
//!
//! This is `wakelock_plus`'s question, and the catalog's A6
//! (`docs/plugin-catalog.md`): a video is playing, a recipe is on screen
//! with hands busy, a turn-by-turn map is showing — the app asks the OS not
//! to sleep the display, and *only* the display; this crate does not touch
//! CPU or network idle policy, which are different questions with different
//! permissions.
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # The state is the OS's, where the OS lets us read it
//!
//! `keep_screen_on(true)` is a **true launch**: `Ok(())` claims the request
//! was accepted, not that the pixels stayed lit — no arm can observe that.
//! [`is_keep_screen_on`] reads the truth back where the platform exposes a
//! readable truth (the window flag on Android, `idleTimerDisabled` on iOS,
//! `IOPMAssertionCopyProperties` on macOS) and tracks it where it does not
//! (Windows — `SetThreadExecutionState` has no query; asking for the
//! current state would change it, which is the one thing a read here must
//! not do). Linux has no API at all and says so.
//!
//! # Why enable/disable rather than acquire/release handles
//!
//! A handle would make the caller hold something that must be balanced on
//! every path — and the OS already balances it: the flag clears with the
//! window, the assertion with the process, the execution state with the
//! thread. A boolean API matches what the platform actually guarantees,
//! which is "while your app is alive and asking".

/// Ask the OS to keep the display awake (`true`) or stop asking (`false`).
///
/// Idempotent: enabling while enabled is `Ok(())` without a second request
/// (on macOS that matters — a second assertion would be a leak, not a
/// stronger ask).
pub fn keep_screen_on(on: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        android::keep_screen_on(on)
    }
    #[cfg(target_os = "ios")]
    {
        ios::keep_screen_on(on)
    }
    #[cfg(target_os = "macos")]
    {
        macos::keep_screen_on(on)
    }
    #[cfg(target_os = "windows")]
    {
        windows::keep_screen_on(on)
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::keep_screen_on(on)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows",
        target_env = "ohos"
    )))]
    {
        // `false` is a no-op that is also *true* here: this crate never asked
        // the platform for anything, so there is nothing to stop asking. Every
        // other arm's idempotence table says the same thing — `(false, None)`
        // is `Ok(())` — and an `Err` for "you are not holding anything" would
        // be a failure reported where nothing failed.
        if !on {
            return Ok(());
        }
        Err(format!(
            "keep_screen_on: {} has no keep-display-awake API — \
             Android, iOS, macOS and Windows do; Linux has no cross-desktop \
             equivalent (a compositor-specific one would be a different crate)",
            std::env::consts::OS
        ))
    }
}

/// Whether the display is currently being kept awake by this app's request.
///
/// Where the platform exposes a readable truth, this reads it; where it does
/// not, it reports what this crate last asked for (see the crate doc — a
/// fabricated "the OS said so" would be worse than an honest "we asked").
pub fn is_keep_screen_on() -> bool {
    #[cfg(target_os = "android")]
    {
        android::is_keep_screen_on()
    }
    #[cfg(target_os = "ios")]
    {
        ios::is_keep_screen_on()
    }
    #[cfg(target_os = "macos")]
    {
        macos::is_keep_screen_on()
    }
    #[cfg(target_os = "windows")]
    {
        windows::is_keep_screen_on()
    }
    #[cfg(target_env = "ohos")]
    {
        ohos::is_keep_screen_on()
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows",
        target_env = "ohos"
    )))]
    {
        false
    }
}

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises the macOS tests: they drive one process-global assertion, and
    /// the test harness runs them in parallel by default.
    #[cfg(target_os = "macos")]
    static MACOS_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The macOS arm is real on a host build, so this acquires and releases
    /// an actual IOPMAssertion — briefly held, released immediately, and the
    /// test fails if either direction lies about what happened.
    ///
    /// The point is the doctrine with teeth: `keep_screen_on` on this machine
    /// goes through `IOPMAssertionCreateWithName`, the same call a video
    /// player makes, and `is_keep_screen_on` reads the assertion back. A
    /// signature mistake fails here, not in a user's app.
    #[cfg(target_os = "macos")]
    #[test]
    fn enable_and_disable_round_trip_through_the_real_api() {
        let _serial = MACOS_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            !is_keep_screen_on(),
            "the test must start with the display not being held awake"
        );
        keep_screen_on(true).expect("acquiring the assertion");
        assert!(is_keep_screen_on());
        // Idempotence is a real guarantee, not a courtesy: the second enable
        // must not mint a second assertion (which the release below would
        // then leak).
        keep_screen_on(true).expect("re-enabling while enabled");
        assert!(is_keep_screen_on());
        keep_screen_on(false).expect("releasing the assertion");
        assert!(!is_keep_screen_on());
        keep_screen_on(false).expect("disabling while disabled");
        assert!(!is_keep_screen_on());
    }

    /// `is_keep_screen_on` asks the power manager, not this crate's memory.
    ///
    /// The assertion is released through IOKit directly, behind the crate's
    /// back, so the mutex still says "held" while the ID is already dead. A
    /// reader that trusted the mutex would answer `true` here; the honest
    /// answer is `false`, because the power manager no longer knows the ID.
    /// This is the only way to tell the two implementations apart, which is
    /// why it is a test rather than a comment.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_read_comes_from_the_power_manager_not_from_our_own_memory() {
        // Both macOS tests drive the same process-global assertion, so they
        // must not interleave: one releasing while the other is mid-roundtrip
        // would fail for a reason that has nothing to do with the code.
        let _serial = MACOS_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        keep_screen_on(false).expect("starting from nothing held");

        keep_screen_on(true).expect("acquiring the assertion");
        let id = macos::held_id().expect("an assertion id while enabled");

        // Release it out from under the crate. The mutex is untouched.
        assert_eq!(
            unsafe { macos::release_out_of_band(id) },
            macos::K_IO_RETURN_SUCCESS
        );
        assert!(
            !is_keep_screen_on(),
            "the power manager no longer knows the id, so the read must be false"
        );

        // Put the crate's own view back so a later test is not affected. The
        // release of an already-dead id is `kIOReturnNotFound`, which is the
        // goal state, not a failure — that is what this clears.
        keep_screen_on(false).expect("clearing the stale id");
        assert!(macos::held_id().is_none(), "the stale id must be gone");
    }

    /// On a platform with no API, the refusal names the platform and the
    /// nearest way out — and `false` is the honest reading, not a guess.
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows",
        target_env = "ohos"
    )))]
    #[test]
    fn a_platform_without_the_api_says_so_and_reads_false() {
        let err = keep_screen_on(true).unwrap_err();
        assert!(err.contains("no keep-display-awake API"), "{err}");
        assert!(!is_keep_screen_on());
        // Asking to stop asking is not a failure: nothing was ever asked, so
        // there is nothing to release. Every other arm's table says this too.
        assert!(keep_screen_on(false).is_ok());
        assert!(!is_keep_screen_on());
    }
}
