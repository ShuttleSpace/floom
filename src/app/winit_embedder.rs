//! Translation from winit window events into the backend-neutral
//! [`EmbedderEvent`] protocol.
//!
//! This is the first official implementation of the embedder contract for the
//! default desktop path: `floem::launch` routes its winit events through here,
//! so every backend (winit, Android, iOS, HarmonyOS) shares one neutral event
//! model. Lifecycle events that need extra platform bookkeeping (live-resize,
//! drag-and-drop, window close) are still handled by the caller's existing
//! `match`; this module supplies the neutral equivalent and the caller decides
//! what to route through [`crate::embedder::EmbedderRuntimeState`].

use peniko::kurbo::{Point, Size};
use smallvec::SmallVec;
use ui_events_winit::WindowEventTranslation;
use winit::{dpi::PhysicalPosition, event::WindowEvent};

use crate::embedder::{
    DragDropEvent, EmbedderEvent, EmbedderWindowId, FocusEvent, ImeEvent, SurfaceDescriptor,
    SurfaceEvent, SurfaceKind, WindowLifecycleEvent,
};

/// Convert a winit window id into the plain, backend-neutral protocol id.
///
/// The lapce winit fork's `WindowId::into_raw()` returns `usize`; we widen to
/// `u64` so the protocol id is stable across platforms and backends.
pub(crate) fn winit_window_id(window_id: winit::window::WindowId) -> EmbedderWindowId {
    window_id.into_raw() as EmbedderWindowId
}

/// Translate a single winit `WindowEvent` into zero or more neutral
/// [`EmbedderEvent`]s.
///
/// Keyboard and pointer input are produced by `ui_events_winit`'s reducer
/// (the canonical winit → ui-events bridge). Lifecycle, IME, surface, focus,
/// theme, and drag-and-drop events are mapped directly.
pub(crate) fn translate_window_event(
    embedder_window_id: EmbedderWindowId,
    scale: f64,
    reducer: &mut ui_events_winit::WindowEventReducer,
    event: &WindowEvent,
) -> SmallVec<[EmbedderEvent; 4]> {
    let mut events = SmallVec::new();

    match reducer.reduce(scale, event) {
        Some(WindowEventTranslation::Keyboard(keyboard)) => {
            let synthetic = matches!(
                event,
                WindowEvent::KeyboardInput {
                    is_synthetic: true,
                    ..
                }
            );
            events.push(EmbedderEvent::Keyboard {
                window_id: embedder_window_id,
                event: keyboard,
                synthetic,
            });
        }
        Some(WindowEventTranslation::Pointer(pointer)) => {
            events.push(EmbedderEvent::Pointer {
                window_id: embedder_window_id,
                event: pointer,
            });
        }
        None => {}
    }

    match event {
        WindowEvent::SurfaceResized(size) => {
            let logical = size.to_logical(scale);
            events.push(EmbedderEvent::Surface {
                window_id: embedder_window_id,
                event: SurfaceEvent::Resized(SurfaceDescriptor {
                    kind: SurfaceKind::GpuSurface,
                    logical_size: Size::new(logical.width, logical.height),
                    pixel_size: (size.width, size.height),
                    scale,
                    transparent: false,
                }),
            });
        }
        WindowEvent::Moved(position) => {
            let logical = position.to_logical(scale);
            events.push(EmbedderEvent::Window {
                window_id: embedder_window_id,
                event: WindowLifecycleEvent::Moved(Point::new(logical.x, logical.y)),
            });
        }
        WindowEvent::CloseRequested => events.push(EmbedderEvent::Window {
            window_id: embedder_window_id,
            event: WindowLifecycleEvent::CloseRequested,
        }),
        WindowEvent::Destroyed => events.push(EmbedderEvent::Window {
            window_id: embedder_window_id,
            event: WindowLifecycleEvent::Destroyed,
        }),
        WindowEvent::Focused(focused) => events.push(EmbedderEvent::Focus {
            window_id: embedder_window_id,
            event: FocusEvent::WindowFocused(*focused),
        }),
        WindowEvent::Ime(winit::event::Ime::Preedit(text, cursor)) => {
            events.push(EmbedderEvent::Ime {
                window_id: embedder_window_id,
                event: ImeEvent::PreeditChanged {
                    text: text.clone(),
                    cursor_byte_range: cursor.map(|(a, b)| (a, b)),
                },
            });
        }
        WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
            events.push(EmbedderEvent::Ime {
                window_id: embedder_window_id,
                event: ImeEvent::CommitText(text.clone()),
            });
        }
        WindowEvent::Ime(winit::event::Ime::Enabled) => events.push(EmbedderEvent::Ime {
            window_id: embedder_window_id,
            event: ImeEvent::EnabledChanged(true),
        }),
        WindowEvent::Ime(winit::event::Ime::Disabled) => events.push(EmbedderEvent::Ime {
            window_id: embedder_window_id,
            event: ImeEvent::EnabledChanged(false),
        }),
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            events.push(EmbedderEvent::Window {
                window_id: embedder_window_id,
                event: WindowLifecycleEvent::ScaleChanged(*scale_factor),
            });
        }
        WindowEvent::ThemeChanged(theme) => {
            // winit::window::Theme is Light/Dark only; map to the neutral enum.
            let neutral = match theme {
                winit::window::Theme::Light => crate::embedder::Theme::Light,
                winit::window::Theme::Dark => crate::embedder::Theme::Dark,
            };
            events.push(EmbedderEvent::Window {
                window_id: embedder_window_id,
                event: WindowLifecycleEvent::ThemeChanged(neutral),
            });
        }
        WindowEvent::Occluded(occluded) => events.push(EmbedderEvent::Surface {
            window_id: embedder_window_id,
            event: SurfaceEvent::Occluded(*occluded),
        }),
        WindowEvent::RedrawRequested => events.push(EmbedderEvent::RedrawRequested {
            window_id: embedder_window_id,
        }),
        WindowEvent::DragDropped { paths, position } => {
            let logical = PhysicalPosition::new(position.x, position.y).to_logical(scale);
            events.push(EmbedderEvent::DragDrop {
                window_id: embedder_window_id,
                event: DragDropEvent::Dropped {
                    paths: paths.clone(),
                    position: Point::new(logical.x, logical.y),
                },
            });
        }
        WindowEvent::DragEntered { paths, position } => {
            let logical = PhysicalPosition::new(position.x, position.y).to_logical(scale);
            events.push(EmbedderEvent::DragDrop {
                window_id: embedder_window_id,
                event: DragDropEvent::Entered {
                    paths: paths.clone(),
                    position: Point::new(logical.x, logical.y),
                },
            });
        }
        WindowEvent::DragMoved { position } => {
            let logical = PhysicalPosition::new(position.x, position.y).to_logical(scale);
            events.push(EmbedderEvent::DragDrop {
                window_id: embedder_window_id,
                event: DragDropEvent::Moved {
                    position: Point::new(logical.x, logical.y),
                },
            });
        }
        WindowEvent::DragLeft { .. } => events.push(EmbedderEvent::DragDrop {
            window_id: embedder_window_id,
            event: DragDropEvent::Left,
        }),
        _ => {}
    }

    events
}
