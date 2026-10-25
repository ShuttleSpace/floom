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
//! the assertion we hold on macOS) and tracks it where it does not
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
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows"
    )))]
    {
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
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows"
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
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
mod tests {
    use super::*;

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

    /// On a platform with no API, the refusal names the platform and the
    /// nearest way out — and `false` is the honest reading, not a guess.
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "windows"
    )))]
    #[test]
    fn a_platform_without_the_api_says_so_and_reads_false() {
        let err = keep_screen_on(true).unwrap_err();
        assert!(err.contains("no keep-display-awake API"), "{err}");
        assert!(!is_keep_screen_on());
    }
}
