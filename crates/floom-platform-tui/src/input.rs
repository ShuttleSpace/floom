//! Turning terminal input bytes into canonical platform events.
//!
//! The interesting design choice here is the *text* path. A terminal sends
//! keystrokes as raw bytes — ASCII for Latin, UTF-8 for everything else — and
//! the canonical [`KeyCode`] space only models printable ASCII (`0x20..=0x7E`).
//! So a Cyrillic or CJK character (the whole reason this backend exists, given
//! how badly pixel-downsampling mangles non-Latin text) would have no `KeyCode`
//! and would be dropped before it reached a text field.
//!
//! Floom already has a path that bypasses `KeyCode` entirely: committed input
//! text is delivered as [`PlatformEvent::ImeCommit`], one `Key::Char` per
//! scalar, and a text field consumes it through the same machinery as ordinary
//! typing. So *every* character this parser sees — ASCII or UTF-8 — is emitted
//! as `ImeCommit`. Named keys (arrows, Enter, …) and the control/alt chords
//! that mean *commands* rather than *text* still go through [`KeyCode`], where
//! the app's shortcut layer can see them.

use floom_platform::{KeyCode, PlatformEvent};
use floom_types::{Modifiers, WindowId};

/// A parsed input, classified into "this is text" or "this is a key command".
enum Outcome {
    /// A character to commit as input text (ASCII or UTF-8).
    Text(char),
    /// A named / modified key the shortcut layer should see.
    Key(KeyCode, Modifiers),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Reading ordinary bytes.
    Ground,
    /// Just saw `ESC`; the next byte decides Alt+char vs a CSI sequence.
    Esc,
    /// Inside a `ESC [` control sequence; accumulating its body.
    Csi,
}

/// Stateful terminal-input parser.
///
/// Feed it raw bytes (from stdin, or injected by a test) and it appends the
/// resulting [`PlatformEvent`]s. It is stateful because escape sequences and
/// multi-byte UTF-8 characters span several bytes that must be stitched
/// together across `feed` calls.
#[derive(Debug)]
pub struct InputParser {
    state: State,
    /// Bytes of the current CSI body (after `ESC [`).
    csi: Vec<u8>,
    /// Partial UTF-8 character being accumulated.
    utf8: Vec<u8>,
}

impl Default for InputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl InputParser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            csi: Vec::new(),
            utf8: Vec::new(),
        }
    }

    /// Feed raw bytes, appending generated events to `out`.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<PlatformEvent>, window: WindowId) {
        for &b in bytes {
            self.step(b, out, window);
        }
        // A trailing `ESC` with no following byte is the Escape key itself.
        if self.state == State::Esc {
            out.push(key(window, KeyCode::ESCAPE, Modifiers::NONE));
            self.state = State::Ground;
        }
        // A dangling partial UTF-8 sequence is discarded; we cannot invent a
        // character from it.
        self.utf8.clear();
    }

    fn step(&mut self, b: u8, out: &mut Vec<PlatformEvent>, window: WindowId) {
        match self.state {
            State::Ground => {
                if b == 0x1b {
                    self.utf8.clear();
                    self.state = State::Esc;
                    return;
                }
                if b < 0x80 {
                    if let Some(o) = Self::map_ascii(b, Modifiers::NONE) {
                        emit(out, window, o);
                    }
                } else {
                    self.utf8.push(b);
                    if let Some(ch) = decode_utf8(&mut self.utf8) {
                        out.push(PlatformEvent::ImeCommit {
                            window,
                            text: ch.to_string(),
                        });
                    }
                }
            }
            State::Esc => {
                if b == b'[' {
                    self.state = State::Csi;
                    self.csi.clear();
                } else if b == b']' {
                    // OSC opener (e.g. a clipboard reply). We never request
                    // reads, so none should arrive; ignore the run.
                    self.state = State::Ground;
                } else if let Some(o) = Self::map_ascii(b, Modifiers::ALT) {
                    // Alt+key.
                    emit(out, window, o);
                    self.state = State::Ground;
                } else {
                    // An Alt combination we do not model; drop it.
                    self.state = State::Ground;
                }
            }
            State::Csi => {
                self.csi.push(b);
                if (0x40..=0x7e).contains(&b) {
                    if let Some((code, mods)) = parse_csi(&self.csi) {
                        out.push(key(window, code, mods));
                    }
                    self.state = State::Ground;
                    self.csi.clear();
                } else if self.csi.len() > 32 {
                    // Runaway sequence; bail rather than keep buffering.
                    self.state = State::Ground;
                    self.csi.clear();
                }
            }
        }
    }

    /// Classify an ASCII byte (with accumulated modifier) into an outcome.
    fn map_ascii(b: u8, mods: Modifiers) -> Option<Outcome> {
        match b {
            0x0d | 0x0a => Some(Outcome::Key(KeyCode::ENTER, mods)),
            0x09 => Some(Outcome::Key(KeyCode::TAB, mods)),
            0x7f => Some(Outcome::Key(KeyCode::BACKSPACE, mods)),
            0x20..=0x7e => {
                let c = b as char;
                if mods.is_empty() {
                    Some(Outcome::Text(c))
                } else {
                    Some(Outcome::Key(KeyCode::from_char(c), mods))
                }
            }
            0x01..=0x1a => Some(Outcome::Key(KeyCode::letter(b - 1), Modifiers::CONTROL.union(mods))),
            _ => None,
        }
    }
}

/// Try to decode a complete UTF-8 character from `buf`, returning it and
/// clearing the buffer on success. A still-incomplete sequence leaves `buf`
/// untouched; an invalid one is cleared and yields `None`.
fn decode_utf8(buf: &mut Vec<u8>) -> Option<char> {
    match std::str::from_utf8(buf) {
        Ok(s) => {
            let ch = s.chars().next()?;
            buf.clear();
            Some(ch)
        }
        Err(e) => {
            // If the error is "need more bytes" and we are within a plausible
            // length, keep accumulating; otherwise drop the garbage.
            if e.error_len().is_some() || buf.len() >= 4 {
                buf.clear();
            }
            None
        }
    }
}

fn emit(out: &mut Vec<PlatformEvent>, window: WindowId, o: Outcome) {
    match o {
        Outcome::Text(ch) => out.push(PlatformEvent::ImeCommit {
            window,
            text: ch.to_string(),
        }),
        Outcome::Key(code, mods) => out.push(key(window, code, mods)),
    }
}

fn key(window: WindowId, code: KeyCode, mods: Modifiers) -> PlatformEvent {
    // Terminals report key-down only. A single down drives text entry and
    // shortcut handling; the framework does not need an up it cannot reliably
    // pair.
    PlatformEvent::Key {
        window,
        code,
        pressed: true,
        repeat: false,
        modifiers: mods,
    }
}

/// Parse a CSI body (the bytes after `ESC [`, including the final byte) into a
/// key code and any modifier flag carried in the `;mod` parameter.
fn parse_csi(seq: &[u8]) -> Option<(KeyCode, Modifiers)> {
    let mods = csi_modifiers(seq);
    let last = *seq.last()?;
    let code = match last {
        b'A' => KeyCode::ARROW_UP,
        b'B' => KeyCode::ARROW_DOWN,
        b'C' => KeyCode::ARROW_RIGHT,
        b'D' => KeyCode::ARROW_LEFT,
        // `ESC [ H` / `ESC [ F` are the classic Home/End (distinct from the
        // `ESC [ 1 ~` / `ESC [ 4 ~` spelling, which `csi_tilde` covers).
        b'H' => KeyCode::HOME,
        b'F' => KeyCode::END,
        b'~' => csi_tilde(seq)?,
        _ => return None,
    };
    Some((code, mods))
}

/// The numeric selector that precedes `～` names the function key.
fn csi_tilde(seq: &[u8]) -> Option<KeyCode> {
    let first = seq.split(|&b| b == b';').next()?;
    match atoi(first)? {
        1 | 7 => Some(KeyCode::HOME),
        4 | 8 => Some(KeyCode::END),
        3 => Some(KeyCode::DELETE),
        5 => Some(KeyCode::PAGE_UP),
        6 => Some(KeyCode::PAGE_DOWN),
        _ => None,
    }
}

/// The second CSI parameter, if present, encodes a modifier bitfield
/// (2=Shift, 3=Alt, 4=Alt+Shift, 5=Ctrl, …).
fn csi_modifiers(seq: &[u8]) -> Modifiers {
    let parts: Vec<&[u8]> = seq.split(|&b| b == b';').collect();
    let n = parts.get(1).and_then(|p| atoi(p)).unwrap_or(1);
    match n {
        2 => Modifiers::SHIFT,
        3 => Modifiers::ALT,
        4 => Modifiers::ALT.union(Modifiers::SHIFT),
        5 => Modifiers::CONTROL,
        6 => Modifiers::CONTROL.union(Modifiers::SHIFT),
        7 => Modifiers::CONTROL.union(Modifiers::ALT),
        8 => Modifiers::CONTROL.union(Modifiers::ALT).union(Modifiers::SHIFT),
        _ => Modifiers::NONE,
    }
}

fn atoi(bytes: &[u8]) -> Option<u8> {
    if bytes.is_empty() {
        return None;
    }
    let mut n: u32 = 0;
    let mut any = false;
    for &b in bytes {
        if !b.is_ascii_digit() {
            // Stop at the first non-digit: a selector like `5C` carries its
            // trailing final byte, and the modifier argument `5` in `1;5C` is
            // followed by the cursor letter.
            break;
        }
        n = n.saturating_mul(10).saturating_add((b - b'0') as u32);
        any = true;
        if n > 255 {
            return None;
        }
    }
    if any {
        Some(n as u8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bytes: &[u8]) -> Vec<PlatformEvent> {
        let mut p = InputParser::new();
        let mut out = Vec::new();
        p.feed(bytes, &mut out, WindowId::new(0, 1));
        out
    }

    fn texts(events: &[PlatformEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|e| match e {
                PlatformEvent::ImeCommit { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn keys(events: &[PlatformEvent]) -> Vec<KeyCode> {
        events
            .iter()
            .filter_map(|e| match e {
                PlatformEvent::Key { code, .. } => Some(*code),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn plain_ascii_is_committed_as_text() {
        let e = parse(b"Hi");
        assert_eq!(texts(&e), vec!["H".to_string(), "i".to_string()]);
        assert!(keys(&e).is_empty());
    }

    #[test]
    fn utf8_multibyte_is_committed_as_one_text_event() {
        // "你" encoded as UTF-8.
        let e = parse("你".as_bytes());
        assert_eq!(texts(&e), vec!["你".to_string()]);
    }

    #[test]
    fn enter_tab_backspace_are_keys() {
        let e = parse(&[b'\r', b'\t', 0x7f]);
        assert_eq!(
            keys(&e),
            vec![KeyCode::ENTER, KeyCode::TAB, KeyCode::BACKSPACE]
        );
    }

    #[test]
    fn arrow_sequences_map_to_named_keys() {
        assert_eq!(keys(&parse(b"\x1b[A")), vec![KeyCode::ARROW_UP]);
        assert_eq!(keys(&parse(b"\x1b[B")), vec![KeyCode::ARROW_DOWN]);
        assert_eq!(keys(&parse(b"\x1b[C")), vec![KeyCode::ARROW_RIGHT]);
        assert_eq!(keys(&parse(b"\x1b[D")), vec![KeyCode::ARROW_LEFT]);
    }

    #[test]
    fn tilde_functions() {
        assert_eq!(keys(&parse(b"\x1b[3~")), vec![KeyCode::DELETE]);
        assert_eq!(keys(&parse(b"\x1b[H")), vec![KeyCode::HOME]);
        assert_eq!(keys(&parse(b"\x1b[F")), vec![KeyCode::END]);
        assert_eq!(keys(&parse(b"\x1b[5~")), vec![KeyCode::PAGE_UP]);
        assert_eq!(keys(&parse(b"\x1b[6~")), vec![KeyCode::PAGE_DOWN]);
    }

    #[test]
    fn lone_escape_is_the_escape_key() {
        assert_eq!(keys(&parse(b"\x1b")), vec![KeyCode::ESCAPE]);
    }

    #[test]
    fn alt_plus_char_is_a_key_with_alt_modifier() {
        let e = parse(b"\x1b\x61");
        let keys = keys(&e);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], KeyCode::from_char('a'));
        match &e[0] {
            PlatformEvent::Key { modifiers, .. } => {
                assert_eq!(*modifiers, Modifiers::ALT);
            }
            _ => panic!("expected a key event"),
        }
    }

    #[test]
    fn control_letter_is_letter_with_control_modifier() {
        // Ctrl+A is byte 0x01 -> 'a' + CONTROL.
        let e = parse(&[0x01]);
        assert_eq!(keys(&e), vec![KeyCode::letter(0)]);
        match &e[0] {
            PlatformEvent::Key { modifiers, .. } => {
                assert_eq!(*modifiers, Modifiers::CONTROL);
            }
            _ => panic!("expected a key event"),
        }
    }

    #[test]
    fn modified_arrow_carries_the_modifier() {
        // Ctrl+Right = ESC [ 1 ; 5 C
        let e = parse(b"\x1b[1;5C");
        assert_eq!(keys(&e), vec![KeyCode::ARROW_RIGHT]);
        match &e[0] {
            PlatformEvent::Key { modifiers, .. } => {
                assert_eq!(*modifiers, Modifiers::CONTROL);
            }
            _ => panic!("expected a key event"),
        }
    }

    #[test]
    fn cjk_stream_stays_distinct_from_ascii() {
        let e = parse("Hi你".as_bytes());
        assert_eq!(
            texts(&e),
            vec!["H".to_string(), "i".to_string(), "你".to_string()]
        );
        assert!(keys(&e).is_empty());
    }
}
