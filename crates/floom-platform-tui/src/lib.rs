//! Terminal (character-grid) platform backend for Floom 2.0.
//!
//! A terminal is not a pixel surface, so this backend fills the `Platform`
//! contract with the minimum a windowing framework needs: a window whose size
//! is the grid, an event queue, and a request-to-redraw signal. The actual
//! window/surface bookkeeping is delegated to [`HostPlatform`] — exactly the
//! pattern the embedded backend uses — so the only terminal-specific pieces
//! here are:
//!
//! * [`terminal::detect_size`] — how many columns and rows we have,
//! * [`input::InputParser`] — bytes in, canonical `PlatformEvent`s out (with
//!   UTF-8 text committed through `ImeCommit` so non-Latin input survives),
//! * an OSC 52 clipboard — the one clipboard protocol a terminal can drive.
//!
//! This crate is `no_std`-free and unix-first (size detection uses an ioctl),
//! but every terminal-specific path degrades gracefully so it compiles and
//! runs wherever `std` does.

pub mod input;
pub mod run;
pub mod terminal;

use floom_platform::{
    Capability, Clipboard, HostPlatform, Ime, Platform, PlatformCapabilities, PlatformEvent,
    Window, WindowConfig,
};
use floom_types::{PlatformId, Size, WindowId};

pub use terminal::TerminalSize;

/// The terminal run loop: [`run`] drives a real terminal event loop for any
/// [`TuiApp`], and is the backend analogue of the desktop backend's `run`.
pub use run::{TuiApp, run};

#[derive(Debug)]
pub struct TuiPlatform {
    host: HostPlatform,
    size: TerminalSize,
    parser: input::InputParser,
    /// Raw input bytes queued for the next `poll_events` (tests inject here).
    pending: Vec<u8>,
    /// The window this backend created, if any — input events tag it.
    window: Option<WindowId>,
    /// Set by [`request_exit`](Self::request_exit); the run loop observes it.
    exit_requested: bool,
}

impl TuiPlatform {
    pub fn new() -> Self {
        Self::with_override(None)
    }

    /// Build a platform, optionally forcing a size instead of probing the
    /// terminal (tests).
    pub fn with_override(size: Option<TerminalSize>) -> Self {
        let size = size.unwrap_or_else(terminal::detect_size);
        Self {
            host: HostPlatform::new()
                .with_id(PlatformId::new(40, 1))
                .with_capabilities(PlatformCapabilities::tui_default()),
            size,
            parser: input::InputParser::new(),
            pending: Vec::new(),
            window: None,
            exit_requested: false,
        }
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// The logical extent of the grid, for callers that size a surface from it.
    pub fn framebuffer_size(&self) -> Size {
        Size::new(self.size.cols as f64, self.size.rows as f64)
    }

    /// Inject raw terminal input bytes for the next pump (test hook).
    pub fn inject_input(&mut self, bytes: &[u8]) {
        self.pending.extend_from_slice(bytes);
    }

    /// Report a new terminal size (typically after a SIGWINCH) and translate it
    /// into a `WindowResized` event for the next pump, so the runner re-lays
    /// out and the presenter rebuilds its grid at the new cell count.
    pub fn resize(&mut self, size: TerminalSize) {
        if size == self.size {
            return;
        }
        self.size = size;
        if let Some(window) = self.window {
            self.host.push_event(PlatformEvent::WindowResized {
                window,
                size: Size::new(size.cols as f64, size.rows as f64),
            });
        }
    }

    /// Ask the run loop to exit after the current iteration. The driver checks
    /// this after each frame, so a widget (or the `TuiApp` impl) can wire a
    /// "quit" key without the loop having to know about keys.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Whether [`request_exit`](Self::request_exit) has been called.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }
}

impl Default for TuiPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for TuiPlatform {
    fn id(&self) -> PlatformId {
        self.host.id()
    }

    fn capabilities(&self) -> PlatformCapabilities {
        self.host.capabilities()
    }

    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        // The window *is* the grid: its logical size is the column/row count.
        let mut cfg = config;
        cfg.size = Size::new(self.size.cols as f64, self.size.rows as f64);
        let id = self.host.create_window(cfg);
        self.window = Some(id);
        id
    }

    fn window(&self, id: WindowId) -> Option<&dyn Window> {
        self.host.window(id)
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window> {
        self.host.window_mut(id)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        // Translate any buffered input into events first, so a key a test
        // injected shows up in the same pump as the window that receives it.
        if !self.pending.is_empty() {
            let bytes = std::mem::take(&mut self.pending);
            let window = self.window.unwrap_or_else(|| WindowId::new(0, 1));
            self.parser.feed(&bytes, out, window);
        }
        self.host.poll_events(out);
    }

    fn request_frame(&mut self, window: WindowId) {
        self.host.request_frame(window)
    }

    fn now_secs(&self) -> f64 {
        self.host.now_secs()
    }

    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> {
        // A terminal has no native input-method surface; the caret-area
        // handshake is a no-op, so report `None` and let the runner skip it.
        None
    }
}

impl Clipboard for TuiPlatform {
    fn get_text(&mut self) -> Option<String> {
        // Reading the clipboard back from a terminal needs a synchronous reply
        // protocol we do not implement, so this stays `None`.
        None
    }

    fn set_text(&mut self, text: &str) -> bool {
        if !self.host.capabilities().supports(Capability::CLIPBOARD) {
            return false;
        }
        // OSC 52 is the only clipboard protocol a terminal can initiate.
        let seq = format!("\x1b]52;c;{}\x07", base64_encode(text));
        use std::io::Write;
        let _ = std::io::stdout().write_all(seq.as_bytes());
        let _ = std::io::stdout().flush();
        true
    }
}

/// Standard base64 with `=` padding — what OSC 52 expects.
fn base64_encode(input: &str) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_platform::Platform;

    #[test]
    fn override_size_is_used() {
        let p = TuiPlatform::with_override(Some(TerminalSize::new(40, 10)));
        assert_eq!(p.size(), TerminalSize::new(40, 10));
    }

    #[test]
    fn creating_a_window_sizes_it_to_the_grid() {
        let mut p = TuiPlatform::with_override(Some(TerminalSize::new(30, 12)));
        let w = p.create_window(WindowConfig::default());
        let win = p.window(w).unwrap();
        assert_eq!(win.size(), Size::new(30.0, 12.0));
    }

    #[test]
    fn injected_text_becomes_ime_commits() {
        let mut p = TuiPlatform::with_override(Some(TerminalSize::new(80, 24)));
        let w = p.create_window(WindowConfig::default());
        p.inject_input(b"Hi");
        let mut events = Vec::new();
        p.poll_events(&mut events);
        let commits: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                PlatformEvent::ImeCommit { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // The window-created event and the two commits.
        assert_eq!(commits, vec!["H", "i"]);
        let _ = w;
    }

    #[test]
    fn base64_is_correct() {
        assert_eq!(base64_encode(""), "");
        assert_eq!(base64_encode("f"), "Zg==");
        assert_eq!(base64_encode("fo"), "Zm8=");
        assert_eq!(base64_encode("foo"), "Zm9v");
        assert_eq!(base64_encode("foob"), "Zm9vYg==");
    }
}
