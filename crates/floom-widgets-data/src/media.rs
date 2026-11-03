//! Media transport controls.
//!
//! The transport is a state machine ([`MediaState`]) plus a description of
//! itself. Two rules shape it, and both come from failure modes that are easy
//! to ship by accident:
//!
//! - An **unknown duration is not a zero duration.** A live stream reports
//!   `duration == 0`, and every seek must be refused rather than computed
//!   against a zero denominator. The seek slider is reported `DISABLED` for the
//!   same reason.
//! - **Muting is not zero volume.** Muting is a separate flag so unmuting can
//!   restore the level the user chose, instead of whatever the slider was left
//!   at.
//!
//! The control cluster publishes its parts ([`MediaControls::parts`]) because
//! one element can only publish one accessible node: a host that wants play,
//! time, seek and volume to be individually reachable needs them named.

use std::any::Any;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, FontWeight, Key, KeyPhase, LayoutRequest,
    NamedKey, PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::{PointerButton, PointerPhase};
use floom_widgets::{SemanticsHint, to_core_role, to_core_state};

const PLAY_WIDTH: f64 = 32.0;
const TIME_WIDTH: f64 = 72.0;
const VOLUME_WIDTH: f64 = 80.0;
const GAP: f64 = 8.0;
/// Seconds one arrow press moves the position.
const SEEK_STEP: f64 = 5.0;
/// Volume one arrow press changes.
const VOLUME_STEP: f64 = 0.05;
/// Height of the seek/volume track's filled bar.
const TRACK_HEIGHT: f64 = 4.0;

const LABEL_FONT_SIZE: f32 = 13.0;
const LINE_BOX: f64 = LABEL_FONT_SIZE as f64 * 1.2;
const BAR_BG: Color = Color::rgb(0xF4, 0xF6, 0xF9);
const FOREGROUND: Color = Color::rgb(0x22, 0x28, 0x30);
const TRACK_BG: Color = Color::rgb(0xD8, 0xDE, 0xE5);
const ACCENT: Color = Color::rgb(0x2E, 0x6C, 0xC4);

fn text_y(top: f64, height: f64) -> f64 {
    top + (height - LINE_BOX) / 2.0
}

/// Formats a position the way a transport display does: `m:ss`, and `h:mm:ss`
/// past an hour.
///
/// Nonsense input (`NaN`, negative, infinite) formats as `0:00` rather than
/// producing a display the user cannot read.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".to_string();
    }
    let total = seconds.floor() as u64;
    let seconds = total % 60;
    let minutes = (total / 60) % 60;
    let hours = total / 3600;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MediaKind {
    #[default]
    Audio,
    Video,
}

impl MediaKind {
    pub const fn noun(self) -> &'static str {
        match self {
            MediaKind::Audio => "Audio player",
            MediaKind::Video => "Video player",
        }
    }
}

/// Everything a transport needs to draw itself and answer a keypress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediaState {
    pub kind: MediaKind,
    pub playing: bool,
    /// Seconds elapsed.
    pub position: f64,
    /// Seconds total; `0.0` means "unknown", as a live stream reports.
    pub duration: f64,
    /// Seconds buffered ahead.
    pub buffered: f64,
    /// 0..1, the *chosen* level — unaffected by [`MediaState::muted`].
    pub volume: f64,
    pub muted: bool,
    pub rate: f64,
    pub looping: bool,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            kind: MediaKind::Audio,
            playing: false,
            position: 0.0,
            duration: 0.0,
            buffered: 0.0,
            volume: 1.0,
            muted: false,
            rate: 1.0,
            looping: false,
        }
    }
}

impl MediaState {
    /// Whether a position can be named at all.
    pub fn is_seekable(&self) -> bool {
        self.duration.is_finite() && self.duration > 0.0
    }

    /// 0..1 through the track, or `0.0` when there is no track to be through.
    pub fn progress(&self) -> f64 {
        if !self.is_seekable() {
            return 0.0;
        }
        (self.position / self.duration).clamp(0.0, 1.0)
    }

    pub fn remaining(&self) -> f64 {
        if !self.is_seekable() {
            return 0.0;
        }
        (self.duration - self.position).max(0.0)
    }

    /// The level that actually reaches the output.
    pub fn effective_volume(&self) -> f64 {
        if self.muted { 0.0 } else { self.volume }
    }

    pub fn toggle_play(&mut self) -> bool {
        if self.playing {
            self.playing = false;
            return true;
        }
        // Pressing play on a finished track restarts it, rather than leaving
        // the user staring at an end state that will not advance.
        if self.is_seekable() && self.position >= self.duration {
            self.position = 0.0;
        }
        self.playing = true;
        true
    }

    /// Seeks to a fraction of the track; refused when there is no track.
    pub fn seek(&mut self, fraction: f64) -> bool {
        if !self.is_seekable() {
            return false;
        }
        let target = fraction.clamp(0.0, 1.0) * self.duration;
        self.set_position(target)
    }

    pub fn seek_by(&mut self, seconds: f64) -> bool {
        if !self.is_seekable() {
            return false;
        }
        self.set_position(self.position + seconds)
    }

    fn set_position(&mut self, value: f64) -> bool {
        let clamped = value.clamp(0.0, self.duration);
        if (clamped - self.position).abs() < f64::EPSILON {
            return false;
        }
        self.position = clamped;
        true
    }

    /// Advances playback by `dt` seconds. A no-op while paused.
    pub fn tick(&mut self, dt: f64) -> bool {
        if !self.playing || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        let next = self.position + dt * self.rate;
        if self.is_seekable() && next >= self.duration {
            if self.looping {
                // Keep the overshoot rather than snapping to zero, so a long
                // frame does not lose time at the wrap.
                self.position = next % self.duration;
            } else {
                self.position = self.duration;
                self.playing = false;
            }
            return true;
        }
        self.position = next.max(0.0);
        true
    }

    pub fn set_volume(&mut self, volume: f64) -> bool {
        let clamped = volume.clamp(0.0, 1.0);
        if (clamped - self.volume).abs() < f64::EPSILON {
            return false;
        }
        self.volume = clamped;
        true
    }

    pub fn set_muted(&mut self, muted: bool) -> bool {
        if self.muted == muted {
            return false;
        }
        self.muted = muted;
        true
    }
}

/// One reachable piece of the transport cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPart {
    pub role: SemanticsRole,
    pub label: String,
    pub state: SemanticsState,
}

/// Where each control lives inside the cluster's box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MediaLayout {
    pub play: Rect,
    pub time: Rect,
    pub seek: Rect,
    pub volume: Rect,
}

/// The transport control cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaControls {
    state: MediaState,
    last_published: MediaState,
    published: bool,
}

impl MediaControls {
    pub fn new(state: MediaState) -> Self {
        Self {
            last_published: state,
            state,
            published: false,
        }
    }

    pub const fn state(&self) -> &MediaState {
        &self.state
    }

    /// Mutable access, for a host driving the transport from its own timer.
    ///
    /// Invalidation is not this accessor's job: [`Widget::update`] compares the
    /// state against the last published one, so a mutation made here — or by
    /// any other path — still repaints exactly once.
    pub const fn state_mut(&mut self) -> &mut MediaState {
        &mut self.state
    }

    pub fn set_state(&mut self, state: MediaState) -> bool {
        if self.state == state {
            return false;
        }
        self.state = state;
        true
    }

    /// The cluster's own accessible name.
    pub fn label(&self) -> String {
        format!(
            "{}, {}, {}",
            self.state.kind.noun(),
            if self.state.playing {
                "playing"
            } else {
                "paused"
            },
            self.time_label()
        )
    }

    /// Elapsed time, with the total only when there is one.
    pub fn time_label(&self) -> String {
        if self.state.is_seekable() {
            format!(
                "{} of {}",
                format_time(self.state.position),
                format_time(self.state.duration)
            )
        } else {
            format_time(self.state.position)
        }
    }

    pub fn volume_label(&self) -> String {
        if self.state.muted {
            "Volume (muted)".to_string()
        } else {
            format!("Volume {}%", (self.state.volume * 100.0).round() as i64)
        }
    }

    /// The four reachable parts, in left-to-right order.
    pub fn parts(&self) -> Vec<MediaPart> {
        let interactive = SemanticsState::ENABLED | SemanticsState::FOCUSABLE;
        vec![
            MediaPart {
                role: SemanticsRole::Button,
                label: if self.state.playing { "Pause" } else { "Play" }.to_string(),
                state: interactive,
            },
            MediaPart {
                role: SemanticsRole::StaticText,
                label: self.time_label(),
                state: SemanticsState::ENABLED,
            },
            MediaPart {
                role: SemanticsRole::Slider,
                label: "Seek".to_string(),
                state: if self.state.is_seekable() {
                    interactive
                } else {
                    SemanticsState::DISABLED
                },
            },
            MediaPart {
                role: SemanticsRole::Slider,
                label: self.volume_label(),
                state: interactive,
            },
        ]
    }

    /// Transport, time, seek, volume — left to right, non-overlapping, with the
    /// seek bar absorbing whatever width is left.
    pub fn layout(&self, bounds: Rect) -> MediaLayout {
        let top = bounds.y0;
        let bottom = bounds.y1;
        let play = Rect {
            x0: bounds.x0,
            y0: top,
            x1: bounds.x0 + PLAY_WIDTH,
            y1: bottom,
        };
        let time = Rect {
            x0: play.x1 + GAP,
            y0: top,
            x1: play.x1 + GAP + TIME_WIDTH,
            y1: bottom,
        };
        let volume = Rect {
            x0: (bounds.x1 - VOLUME_WIDTH).max(time.x1 + GAP),
            y0: top,
            x1: bounds.x1,
            y1: bottom,
        };
        let seek = Rect {
            x0: time.x1 + GAP,
            y0: top,
            x1: (volume.x0 - GAP).max(time.x1 + GAP),
            y1: bottom,
        };
        MediaLayout {
            play,
            time,
            seek,
            volume,
        }
    }

    fn pointer(
        &mut self,
        pointer: &floom_types::PointerEvent,
        bounds: Rect,
        cx: &mut dyn EventContext,
    ) -> bool {
        if !matches!(pointer.phase, PointerPhase::Down | PointerPhase::Up) {
            return false;
        }
        cx.request_focus(cx.target());
        if pointer.phase != PointerPhase::Down || pointer.button != Some(PointerButton::Primary) {
            return true;
        }
        let layout = self.layout(bounds);
        let x = pointer.position.x;

        if x >= layout.play.x0 && x < layout.play.x1 {
            return self.state.toggle_play();
        }
        if x >= layout.volume.x0 && x < layout.volume.x1 {
            let fraction = (x - layout.volume.x0) / layout.volume.width().max(1.0);
            return self.state.set_volume(fraction);
        }
        if x >= layout.seek.x0 && x < layout.seek.x1 {
            let fraction = (x - layout.seek.x0) / layout.seek.width().max(1.0);
            return self.state.seek(fraction);
        }
        true
    }

    fn key_event(&mut self, key: &floom_types::KeyEvent, cx: &mut dyn EventContext) -> bool {
        if key.phase == KeyPhase::Up {
            return false;
        }
        cx.request_focus(cx.target());
        match key.key {
            Key::Char(' ') => self.state.toggle_play(),
            Key::Char('m') => self.state.set_muted(!self.state.muted),
            Key::Named(NamedKey::ArrowRight) => self.state.seek_by(SEEK_STEP),
            Key::Named(NamedKey::ArrowLeft) => self.state.seek_by(-SEEK_STEP),
            Key::Named(NamedKey::Home) => self.state.seek(0.0),
            Key::Named(NamedKey::End) => self.state.seek(1.0),
            Key::Named(NamedKey::ArrowUp) => {
                let next = self.state.volume + VOLUME_STEP;
                self.state.set_volume(next)
            }
            Key::Named(NamedKey::ArrowDown) => {
                let next = self.state.volume - VOLUME_STEP;
                self.state.set_volume(next)
            }
            _ => return false,
        }
    }

    fn semantics_hint(&self) -> SemanticsHint {
        let mut state = SemanticsState::ENABLED | SemanticsState::FOCUSABLE;
        if self.state.playing {
            state |= SemanticsState::LIVE;
        }
        SemanticsHint {
            role: SemanticsRole::Group,
            label: Some(self.label()),
            state,
        }
    }
}

impl Widget for MediaControls {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if !self.published || self.state != self.last_published {
            self.published = true;
            self.last_published = self.state;
            cx.mark_dirty(id);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill())
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => {
                let bounds = cx.bounds();
                self.pointer(&pointer, bounds, cx)
            }
            Event::Key(key) => self.key_event(&key, cx),
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let layout = self.layout(bounds);
        cx.fill_rect(bounds, BAR_BG);

        let glyph = if self.state.playing { "❚❚" } else { "▶" };
        cx.fill_text(
            Point::new(
                layout.play.x0 + 10.0,
                text_y(layout.play.y0, layout.play.height()),
            ),
            glyph,
            LABEL_FONT_SIZE,
            FontWeight::Normal,
            FOREGROUND,
        );
        cx.fill_text(
            Point::new(layout.time.x0, text_y(layout.time.y0, layout.time.height())),
            &self.time_label(),
            LABEL_FONT_SIZE,
            FontWeight::Normal,
            FOREGROUND,
        );

        let track = |area: Rect| Rect {
            x0: area.x0,
            y0: area.y0 + area.height() / 2.0 - TRACK_HEIGHT / 2.0,
            x1: area.x1,
            y1: area.y0 + area.height() / 2.0 + TRACK_HEIGHT / 2.0,
        };
        let seek_track = track(layout.seek);
        cx.fill_rect(seek_track, TRACK_BG);
        if self.state.is_seekable() {
            cx.fill_rect(
                Rect {
                    x1: seek_track.x0 + seek_track.width() * self.state.progress(),
                    ..seek_track
                },
                ACCENT,
            );
        }
        let volume_track = track(layout.volume);
        cx.fill_rect(volume_track, TRACK_BG);
        cx.fill_rect(
            Rect {
                x1: volume_track.x0 + volume_track.width() * self.state.effective_volume(),
                ..volume_track
            },
            ACCENT,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        if let Some(label) = self.semantics_hint().label.as_deref() {
            cx.set_label(label);
        }
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for MediaControls {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        let hint = self.semantics_hint();
        Some(floom_core::SemanticsHint {
            role: to_core_role(hint.role),
            label: hint.label,
            state: to_core_state(hint.state),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_an_hours_worth_of_seconds_keeps_the_minutes_padded() {
        assert_eq!(format_time(3599.0), "59:59");
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(3661.0), "1:01:01");
    }

    #[test]
    fn a_live_stream_has_no_total_and_no_remaining() {
        let state = MediaState {
            duration: 0.0,
            position: 12.0,
            ..MediaState::default()
        };
        assert!(!state.is_seekable());
        assert_eq!(state.progress(), 0.0);
        assert_eq!(state.remaining(), 0.0);
        assert_eq!(format_time(state.position), "0:12");
    }

    #[test]
    fn a_tick_of_zero_or_less_is_ignored() {
        let mut state = MediaState {
            duration: 10.0,
            playing: true,
            ..MediaState::default()
        };
        assert!(!state.tick(0.0));
        assert!(!state.tick(-1.0));
        assert!(!state.tick(f64::NAN));
        assert_eq!(state.position, 0.0);
    }
}
