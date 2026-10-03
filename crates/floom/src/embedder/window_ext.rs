//! Inherent `handle_embedder_event` for [`WindowHandle`].
//!
//! Defined in a separate `impl` block (rather than editing the core window
//! module) so the embedder boundary stays surgically isolated. It maps neutral
//! [`EmbedderEvent`] values onto Floom's existing window methods.

use crate::embedder::{
    CommandSink, EmbedderEvent, FocusEvent, ImeEvent, SurfaceEvent, Theme, WindowLifecycleEvent,
};
use crate::window::handle::WindowHandle;
use floom_legacy_reactive::{SignalGet, SignalUpdate};

impl WindowHandle {
    /// Route a single embedder event into this window.
    ///
    /// `commands` is accepted for symmetry with the protocol; command emission
    /// (e.g. `RequestRedraw`, `SetImeState`) is wired in later phases through
    /// the same signature.
    pub(crate) fn handle_embedder_event(
        &mut self,
        event: EmbedderEvent,
        _commands: &mut dyn CommandSink,
    ) {
        match event {
            EmbedderEvent::Keyboard {
                event, synthetic, ..
            } => {
                if !synthetic {
                    self.key_event(event);
                }
            }
            EmbedderEvent::Pointer { event, .. } => {
                self.pointer_event(event);
            }
            EmbedderEvent::Focus {
                event: FocusEvent::WindowFocused(focused),
                ..
            } => {
                self.focused(focused);
            }
            EmbedderEvent::Ime { event, .. } => match event {
                ImeEvent::PreeditChanged {
                    text,
                    cursor_byte_range,
                } => {
                    self.ime(winit::event::Ime::Preedit(
                        text,
                        cursor_byte_range.map(|(a, b)| (a, b)),
                    ));
                }
                ImeEvent::CommitText(text) => {
                    self.ime(winit::event::Ime::Commit(text));
                }
                ImeEvent::CompositionEnd | ImeEvent::EnabledChanged(false) => {
                    self.ime(winit::event::Ime::Disabled);
                }
                ImeEvent::EnabledChanged(true) => {
                    self.ime(winit::event::Ime::Enabled);
                }
            },
            EmbedderEvent::Window { event, .. } => match event {
                WindowLifecycleEvent::CloseRequested => {
                    self.event(crate::event::Event::Window(
                        crate::event::WindowEvent::CloseRequested,
                    ));
                }
                WindowLifecycleEvent::Moved(position) => self.position(position),
                WindowLifecycleEvent::Resized(size) => self.size(size),
                WindowLifecycleEvent::ScaleChanged(scale) => self.os_scale(scale),
                WindowLifecycleEvent::ThemeChanged(theme) => {
                    let theme = match theme {
                        Theme::Light => Some(winit::window::Theme::Light),
                        Theme::Dark => Some(winit::window::Theme::Dark),
                        Theme::Unknown => None,
                    };
                    self.set_theme(theme, true);
                }
                WindowLifecycleEvent::Created {
                    logical_size,
                    scale,
                } => {
                    self.os_scale(scale);
                    self.size(logical_size);
                }
                WindowLifecycleEvent::Destroyed => {}
            },
            EmbedderEvent::Surface { event, .. } => match event {
                SurfaceEvent::Resized(descriptor)
                | SurfaceEvent::Created(descriptor)
                | SurfaceEvent::Restored(descriptor) => {
                    self.os_scale(descriptor.scale);
                    self.size(descriptor.logical_size);
                }
                SurfaceEvent::Occluded(occluded) => self.set_occluded(occluded),
                SurfaceEvent::Lost | SurfaceEvent::Destroyed => {}
            },
            EmbedderEvent::RedrawRequested { .. } => self.render_frame(),
            EmbedderEvent::SafeArea { insets, .. } => {
                if self.window_state.safe_area.get_untracked() != insets {
                    self.window_state.safe_area.set(insets);
                    // Only re-layout when the window opted into insetting its
                    // root; otherwise the signal is still observable via the
                    // public reader for apps that manage their own padding.
                    if self.window_state.apply_safe_area {
                        self.window_state.needs_layout = true;
                        self.render_frame();
                    }
                }
            }
            EmbedderEvent::Gesture { .. } | EmbedderEvent::DragDrop { .. } => {}
        }
    }
}
