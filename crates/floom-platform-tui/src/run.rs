//! The real run loop: turn a terminal into a live Floom window.
//!
//! `floom-platform-desktop` exposes `run` over winit; this is the terminal
//! analogue. The loop is deliberately thin and generic — it owns the terminal
//! machinery (raw mode, alternate screen, a stdin reader, SIGWINCH) but not the
//! app: the app is any [`TuiApp`], which drains [`Platform::poll_events`] into
//! its own `AppRunner`. That keeps this crate at L4 (it never names
//! `floom-app`); the beginner-facing one-call entry lives in the `floom2`
//! facade, which sits above everything.
//!
//! [`Platform::poll_events`]: floom_platform::Platform::poll_events

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use floom_platform::{Platform, WindowConfig};

use crate::TuiPlatform;

/// A resize request, set by the SIGWINCH handler and read by the loop. It is a
/// plain `AtomicBool` (not gated to unix) so the loop's check is uniform; it is
/// only ever raised where a SIGWINCH handler is installed.
static RESIZE_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn on_winch(_sig: i32) {
    RESIZE_REQUESTED.store(true, Ordering::SeqCst);
}

/// The application side of a terminal-hosted Floom app.
///
/// Implementors get called from inside the run loop's frame hook, with the
/// backend handy, and are expected to drain [`Platform::poll_events`] — i.e.
/// call `runner.pump(&mut platform)`. The loop owns the terminal; the app owns
/// the runner.
///
/// [`Platform::poll_events`]: floom_platform::Platform::poll_events
pub trait TuiApp {
    /// One frame is ready to be serviced. Drain platform events into the runner.
    fn events(&mut self, platform: &mut TuiPlatform);

    /// The loop is about to tear the terminal down. Release any OS resources
    /// the app holds (none are required for a terminal app, so this is a no-op
    /// by default).
    fn about_to_exit(&mut self, _platform: &mut TuiPlatform) {}
}

/// Run a Floom app on a terminal. Blocks until the loop exits (stdin EOF, a
/// Ctrl-C/Ctrl-D, or [`TuiPlatform::request_exit`]).
///
/// On a real terminal it switches to the alternate screen and raw mode so the
/// app owns the whole viewport and the user's scrollback is untouched; both are
/// restored on return (including on panic). Piped output (CI, tests) is detected
/// and left alone, so the loop still runs without a tty.
pub fn run<A: TuiApp + 'static>(mut app: A) -> Result<(), String> {
    let _guard = TerminalGuard::enter()?;

    // A thread reads raw stdin and forwards chunks; the loop consumes them with
    // a short timeout so resize signals (SIGWINCH) and the exit flag are still
    // observed promptly without spinning the CPU.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut platform = TuiPlatform::new();
    platform.create_window(WindowConfig::default());

    let mut pending = Vec::new();
    let quit = AtomicBool::new(false);
    loop {
        // Always pump first: this is what turns the window creation the loop
        // scheduled into a frame, and what turns a `RedrawRequested` into
        // pixels. On a closed stdin we pump once more after the loop below
        // breaks, so the first frame is always drawn before exit.
        app.events(&mut platform);

        // Collect any input that arrived (block up to 16ms so we also wake for
        // resize/timeouts).
        match rx.recv_timeout(Duration::from_millis(16)) {
            Ok(chunk) => pending.extend_from_slice(&chunk),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // stdin closed: service one last frame, then leave.
                app.events(&mut platform);
                break;
            }
        }
        while let Ok(chunk) = rx.try_recv() {
            pending.extend_from_slice(&chunk);
        }

        if RESIZE_REQUESTED.swap(false, Ordering::SeqCst) {
            platform.resize(crate::terminal::detect_size());
        }

        if !pending.is_empty() {
            let (kept, saw_quit) = strip_quit_bytes(&pending);
            pending = kept;
            if saw_quit {
                quit.store(true, Ordering::SeqCst);
            }
            if !pending.is_empty() {
                platform.inject_input(&pending);
                pending.clear();
            }
        }

        if quit.load(Ordering::SeqCst) || platform.exit_requested() {
            break;
        }
    }

    app.about_to_exit(&mut platform);
    Ok(())
}

/// Ctrl-C (0x03) and Ctrl-D (0x04) are consumed as quit signals and never reach
/// the widget tree (they are not text). Returns the surviving bytes and whether
/// a quit was seen. Split out so the policy is unit-testable without a terminal.
pub(crate) fn strip_quit_bytes(input: &[u8]) -> (Vec<u8>, bool) {
    let mut out = Vec::with_capacity(input.len());
    let mut quit = false;
    for &b in input {
        if b == 0x03 || b == 0x04 {
            quit = true;
        } else {
            out.push(b);
        }
    }
    (out, quit)
}

/// Restores the terminal to its original state when dropped.
struct TerminalGuard {
    /// Whether raw mode + alternate screen were actually engaged (only on a tty).
    engaged: bool,
    #[cfg(unix)]
    original: libc::termios,
}

impl TerminalGuard {
    fn enter() -> Result<Self, String> {
        // Only take over the terminal when we are attached to one; piping the
        // app's output (CI, tests) must not fail or emit control sequences.
        #[cfg(unix)]
        {
            if unsafe { libc::isatty(0) } == 0 {
                return Ok(Self {
                    engaged: false,
                    original: unsafe { std::mem::zeroed() },
                });
            }
            let mut original: libc::termios = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(0, &mut original) } != 0 {
                return Err("failed to read terminal attributes".to_string());
            }
            let mut raw = original;
            // No canonical mode, no echo, no signal chars (so Ctrl-C is a byte
            // we can intercept rather than SIGINT).
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG);
            raw.c_iflag &= !(libc::IXON | libc::IXOFF | libc::ICRNL);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;
            if unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) } != 0 {
                return Err("failed to set raw terminal mode".to_string());
            }
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(b"\x1b[?1049h\x1b[?25l");
            let _ = out.flush();
            unsafe {
                // macOS' libc spells the handler argument as `usize`; the rest
                // of unix uses `sighandler_t` (an `Option<extern "C" fn(i32)>`).
                #[cfg(target_os = "macos")]
                let _ = libc::signal(libc::SIGWINCH, on_winch as *const () as usize);
                #[cfg(not(target_os = "macos"))]
                let _ = libc::signal(libc::SIGWINCH, Some(on_winch));
            }
            Ok(Self {
                engaged: true,
                original,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = 0; // no terminal takeover off unix; the loop still runs
            Ok(Self { engaged: false })
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.engaged {
            return;
        }
        #[cfg(unix)]
        {
            // Restore attributes, leave the alternate screen, show the cursor.
            unsafe {
                libc::tcsetattr(0, libc::TCSANOW, &self.original);
            }
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(b"\x1b[?25h\x1b[?1049l");
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TerminalSize;
    use floom_platform::{Platform, PlatformEvent};

    #[test]
    fn resize_pushes_window_resized() {
        let mut p = TuiPlatform::with_override(Some(TerminalSize::new(80, 24)));
        let window = p.create_window(WindowConfig::default());
        p.resize(TerminalSize::new(120, 40));
        let mut events = Vec::new();
        p.poll_events(&mut events);
        let resized = events.iter().any(|e| {
            matches!(e, PlatformEvent::WindowResized { window: w, size }
                if *w == window && size.width as usize == 120 && size.height as usize == 40)
        });
        assert!(resized, "a resize must produce a WindowResized event: {events:?}");
    }

    #[test]
    fn resize_to_the_same_size_is_a_no_op() {
        let mut p = TuiPlatform::with_override(Some(TerminalSize::new(80, 24)));
        p.create_window(WindowConfig::default());
        p.resize(TerminalSize::new(80, 24));
        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, PlatformEvent::WindowResized { .. })),
            "resizing to the same size should not emit an event: {events:?}"
        );
    }

    #[test]
    fn ctrl_c_is_a_quit_signal() {
        let (kept, quit) = strip_quit_bytes(&[b'a', 0x03, b'b']);
        assert!(quit, "Ctrl-C marks a quit");
        assert_eq!(kept, vec![b'a', b'b'], "Ctrl-C does not reach the tree");
    }

    #[test]
    fn ctrl_d_is_a_quit_signal() {
        let (kept, quit) = strip_quit_bytes(&[0x04, b'x']);
        assert!(quit);
        assert_eq!(kept, vec![b'x']);
    }

    #[test]
    fn normal_bytes_are_not_a_quit() {
        let (kept, quit) = strip_quit_bytes(b"hello");
        assert!(!quit);
        assert_eq!(kept, b"hello".to_vec());
    }

    #[test]
    fn utf8_survives_quit_stripping() {
        let input: Vec<u8> = "你hi".as_bytes().to_vec();
        let (kept, quit) = strip_quit_bytes(&input);
        assert!(!quit);
        assert_eq!(kept, input);
    }
}
