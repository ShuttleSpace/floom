//! Discovering how big the character grid is.
//!
//! A terminal is a grid, and the framework lays out a window into that grid, so
//! the backend has to know its dimensions before it can create a window.
//! `detect_size` tries, in order: the kernel's idea of the connected terminal
//! (`TIOCGWINSZ`), the `COLUMNS`/`LINES` environment variables, and finally a
//! conventional 80×24. Every path yields a usable size, which is what lets the
//! backend run headless in tests and CI where there is no tty at all.

/// A terminal's character-grid dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: usize,
    pub rows: usize,
}

impl TerminalSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

const DEFAULT_COLS: usize = 80;
const DEFAULT_ROWS: usize = 24;

/// Detect the terminal size, falling back to 80×24 if nothing else applies.
pub fn detect_size() -> TerminalSize {
    detect_size_impl().unwrap_or(TerminalSize::new(DEFAULT_COLS, DEFAULT_ROWS))
}

#[cfg(unix)]
fn detect_size_impl() -> Option<TerminalSize> {
    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // Try each standard stream; when stdout is piped (tests, CI) the kernel has
    // no window size for it and returns ENOTTY, so we fall through to the next.
    for fd in [1i32, 0i32, 2i32] {
        let rc = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws as *mut Winsize) };
        if rc == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
            return Some(TerminalSize::new(ws.ws_col as usize, ws.ws_row as usize));
        }
    }
    env_size()
}

#[cfg(not(unix))]
fn detect_size_impl() -> Option<TerminalSize> {
    env_size()
}

fn env_size() -> Option<TerminalSize> {
    let cols = std::env::var("COLUMNS").ok().and_then(|s| s.parse::<usize>().ok());
    let rows = std::env::var("LINES").ok().and_then(|s| s.parse::<usize>().ok());
    match (cols, rows) {
        (Some(c), Some(r)) if c > 0 && r > 0 => Some(TerminalSize::new(c, r)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_size_is_always_positive() {
        let size = detect_size();
        assert!(size.cols >= 1, "a terminal has at least one column");
        assert!(size.rows >= 1, "a terminal has at least one row");
    }

    #[test]
    fn default_size_is_conventional() {
        // With no tty and no env (the common test/CI situation) we get 80×24.
        if std::env::var("COLUMNS").is_err() && std::env::var("LINES").is_err() {
            assert_eq!(detect_size(), TerminalSize::new(80, 24));
        }
    }
}
