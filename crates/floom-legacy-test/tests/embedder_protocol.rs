//! Embedder protocol type tests.
//!
//! These exercise the data-oriented side of the embedder API: identifiers,
//! capability reporting, and construction / equality of the event, command,
//! surface, and IME protocol types. Backends translate their native types
//! into these neutral values, so they must be cheap to build and compare.

use floom::embedder::{
    BackendCapabilities, CommandBatch, CommandSink, DragDropEvent, EmbedderCommand, EmbedderEvent,
    EmbedderPointerId, EmbedderWindowId, FocusEvent, GestureEvent, ImeEvent, ImeHint, ImePurpose,
    ImeState, SurfaceDescriptor, SurfaceEvent, SurfaceKind, Theme, VecCommandSink,
    WindowLifecycleEvent,
};
use floom::peniko::kurbo::{Point, Rect, Size};

#[test]
fn window_id_is_u64_alias() {
    // The protocol id must be a plain value a non-winit backend can mint.
    let id: EmbedderWindowId = 42;
    assert_eq!(id, 42);
    let pid: EmbedderPointerId = 7;
    assert_eq!(pid, 7);
}

#[test]
fn capabilities_winit_default_reports_desktop_features() {
    let caps = BackendCapabilities::winit_default();
    // Desktop winit backend advertises the conventional feature set.
    assert!(caps.clipboard);
    assert!(caps.cursor);
    assert!(caps.timers);
    assert!(caps.file_dialog);
    assert!(caps.context_menu);
    assert!(caps.window_title);
    assert!(caps.system_theme);
    assert!(caps.display_info);
    assert!(caps.drag_drop);
    assert!(caps.ime);
    assert!(caps.gpu_surface);
    assert!(caps.software_surface);
    // Accessibility is wired up on the three desktop platforms (the backend
    // publishes a semantics tree there); everywhere else it is still advertised
    // as unsupported. Mirrors `BackendCapabilities::winit_default`.
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    assert!(caps.accessibility);
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    assert!(!caps.accessibility);
    // No safe-area / virtual-keyboard yet on desktop.
    assert!(!caps.safe_area);
    assert!(!caps.virtual_keyboard);
    assert!(!caps.open_external);
    assert!(!caps.headless);
}

#[test]
fn capabilities_headless_only_advertises_headless() {
    let caps = BackendCapabilities::headless();
    assert!(caps.headless);
    assert!(!caps.clipboard);
    assert!(!caps.cursor);
    assert!(!caps.gpu_surface);
    assert!(!caps.ime);
}

#[test]
fn capabilities_default_is_all_false() {
    let caps = BackendCapabilities::default();
    assert!(!caps.headless);
    assert!(!caps.clipboard);
    assert!(!caps.ime);
}

#[test]
fn build_events_and_compare() {
    let id: EmbedderWindowId = 1;

    let close = EmbedderEvent::Window {
        window_id: id,
        event: WindowLifecycleEvent::CloseRequested,
    };
    assert_eq!(
        close,
        EmbedderEvent::Window {
            window_id: id,
            event: WindowLifecycleEvent::CloseRequested
        }
    );

    let focus = EmbedderEvent::Focus {
        window_id: id,
        event: FocusEvent::WindowFocused(true),
    };
    assert_eq!(
        focus,
        EmbedderEvent::Focus {
            window_id: id,
            event: FocusEvent::WindowFocused(true)
        }
    );

    let ime = EmbedderEvent::Ime {
        window_id: id,
        event: ImeEvent::CommitText("hello".to_string()),
    };
    assert_eq!(
        ime,
        EmbedderEvent::Ime {
            window_id: id,
            event: ImeEvent::CommitText("hello".to_string())
        }
    );

    let redraw = EmbedderEvent::RedrawRequested { window_id: id };
    assert_eq!(redraw, EmbedderEvent::RedrawRequested { window_id: id });
}

#[test]
fn theme_enum_is_neutral() {
    // Theme must not depend on winit so the protocol stays backend-free.
    assert_eq!(Theme::Light, Theme::Light);
    assert_ne!(Theme::Light, Theme::Dark);
    assert_ne!(Theme::Dark, Theme::Unknown);
}

#[test]
fn surface_descriptor_and_event_construct() {
    let descriptor = SurfaceDescriptor {
        kind: SurfaceKind::GpuSurface,
        logical_size: Size::new(800.0, 600.0),
        pixel_size: (1600, 1200),
        scale: 2.0,
        transparent: false,
    };
    let created = SurfaceEvent::Created(descriptor.clone());
    let resized = SurfaceEvent::Resized(descriptor.clone());
    assert_ne!(created, resized);
    assert_eq!(created, SurfaceEvent::Created(descriptor));

    // All surface kinds are distinct.
    assert_ne!(SurfaceKind::GpuSurface, SurfaceKind::SoftwareSurface);
    assert_ne!(SurfaceKind::CellSurface, SurfaceKind::ExternalTexture);
    assert_ne!(SurfaceKind::None, SurfaceKind::GpuSurface);
}

#[test]
fn ime_state_constructs_with_all_fields() {
    let state = ImeState {
        enabled: true,
        candidate_rect: Some(Rect::new(10.0, 20.0, 110.0, 40.0)),
        purpose: ImePurpose::Number,
        hint: ImeHint::Lowercase,
        secure: true,
        multiline: false,
        surrounding_text: Some("12".to_string()),
    };
    // Equality covers all fields.
    assert_eq!(
        state,
        ImeState {
            enabled: true,
            candidate_rect: Some(Rect::new(10.0, 20.0, 110.0, 40.0)),
            purpose: ImePurpose::Number,
            hint: ImeHint::Lowercase,
            secure: true,
            multiline: false,
            surrounding_text: Some("12".to_string()),
        }
    );
    assert_ne!(ImePurpose::Text, ImePurpose::Email);
    assert_ne!(ImeHint::Default, ImeHint::Uppercase);
}

#[test]
fn ime_events_construct() {
    let preedit = ImeEvent::PreeditChanged {
        text: "ni".to_string(),
        cursor_byte_range: Some((0, 2)),
    };
    assert_eq!(
        preedit,
        ImeEvent::PreeditChanged {
            text: "ni".to_string(),
            cursor_byte_range: Some((0, 2)),
        }
    );
    assert_eq!(
        ImeEvent::EnabledChanged(true),
        ImeEvent::EnabledChanged(true)
    );
    assert_ne!(
        ImeEvent::EnabledChanged(true),
        ImeEvent::EnabledChanged(false)
    );
}

#[test]
fn gesture_and_dragdrop_events_construct() {
    let pinch = EmbedderEvent::Gesture {
        window_id: 1,
        event: GestureEvent::Pinch {
            delta: 1.5,
            position: Point::new(10.0, 10.0),
        },
    };
    assert_eq!(
        pinch,
        EmbedderEvent::Gesture {
            window_id: 1,
            event: GestureEvent::Pinch {
                delta: 1.5,
                position: Point::new(10.0, 10.0),
            },
        }
    );

    let drag = EmbedderEvent::DragDrop {
        window_id: 1,
        event: DragDropEvent::Left,
    };
    assert_eq!(
        drag,
        EmbedderEvent::DragDrop {
            window_id: 1,
            event: DragDropEvent::Left,
        }
    );
}

#[test]
fn commands_construct_and_compare() {
    let id: EmbedderWindowId = 9;
    assert_eq!(
        EmbedderCommand::RequestRedraw { window_id: id },
        EmbedderCommand::RequestRedraw { window_id: id }
    );
    assert_ne!(
        EmbedderCommand::RequestRedraw { window_id: id },
        EmbedderCommand::RequestRedraw { window_id: id + 1 }
    );

    let ime_cmd = EmbedderCommand::SetImeState {
        window_id: id,
        state: ImeState {
            enabled: true,
            candidate_rect: None,
            purpose: ImePurpose::Text,
            hint: ImeHint::Default,
            secure: false,
            multiline: false,
            surrounding_text: None,
        },
    };
    assert_eq!(
        ime_cmd,
        EmbedderCommand::SetImeState {
            window_id: id,
            state: ImeState {
                enabled: true,
                candidate_rect: None,
                purpose: ImePurpose::Text,
                hint: ImeHint::Default,
                secure: false,
                multiline: false,
                surrounding_text: None,
            },
        }
    );
}

#[test]
fn vec_command_sink_collects_and_drains() {
    let mut sink = VecCommandSink::default();
    sink.push_command(EmbedderCommand::RequestRedraw { window_id: 1 });
    sink.push_command(EmbedderCommand::RequestRedraw { window_id: 2 });
    let collected = sink.take_commands();
    assert_eq!(collected.len(), 2);
    assert!(sink.take_commands().is_empty());
}

#[test]
fn command_batch_push_drain_and_is_empty() {
    let mut batch = CommandBatch::default();
    assert!(batch.is_empty());
    batch.push(EmbedderCommand::ReadClipboard);
    assert!(!batch.is_empty());
    let drained = batch.drain();
    assert_eq!(drained, vec![EmbedderCommand::ReadClipboard]);
    assert!(batch.is_empty());
}

#[test]
fn debug_format_available_for_core_types() {
    // Ensure the protocol types implement Debug (used by backends / logging).
    let event = EmbedderEvent::Window {
        window_id: 1,
        event: WindowLifecycleEvent::Resized(Size::new(1.0, 1.0)),
    };
    let debug = format!("{event:?}");
    assert!(debug.contains("Window"));
    let caps = format!("{:?}", BackendCapabilities::winit_default());
    assert!(caps.contains("BackendCapabilities"));
}
