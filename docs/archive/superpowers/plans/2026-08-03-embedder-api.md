# Experimental Embedder API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an experimental `floom::embedder` API that lets non-winit backends drive Floom through a shared protocol while preserving the default winit launch path.

**Architecture:** Add a data-oriented protocol layer under `crates/floom/src/embedder/`, then add Rust adapter traits over that protocol. Convert the current winit path incrementally so `Application` and `WindowHandle` can consume Floom-level embedder events and emit command objects, while a headless backend proves the contract without a real window.

**Tech Stack:** Rust 2024, Floom core, `winit`, `ui-events`, `ui-events-winit`, `raw-window-handle`, `peniko::kurbo`, existing `floom-test` integration tests.

## Global Constraints

- `floom::embedder` is public but experimental.
- `floom::launch(...)` must remain available and easy to use through the default winit backend.
- Mobile, TUI, and HarmonyOS SDK dependencies must not be added to default features.
- The existing `floom_renderer` abstraction and Vello, Vger, Skia, and tiny-skia strategy remain in place.
- First phase implements winit and headless/embedder; it does not fully implement Android, iOS, TUI, or HarmonyOS backends.
- First phase defines accessibility, safe area, virtual keyboard, and URL-opening protocol support, but unsupported services must degrade explicitly.
- Small breaking changes to semi-public internals are allowed only when examples remain easy to update.
- Event, style, layout, painting, and text benchmarks must not show obvious regressions from avoidable allocation or dynamic dispatch in hot paths.

---

## File Structure

- Create `crates/floom/src/embedder/mod.rs`: public experimental module entry point and re-exports.
- Create `crates/floom/src/embedder/id.rs`: platform-neutral `EmbedderWindowId` and `EmbedderPointerId` aliases/wrappers.
- Create `crates/floom/src/embedder/capabilities.rs`: `BackendCapabilities` and individual service capability structs.
- Create `crates/floom/src/embedder/surface.rs`: `SurfaceKind`, `SurfaceDescriptor`, `SurfaceEvent`, and size/scale surface metadata.
- Create `crates/floom/src/embedder/ime.rs`: `ImeState`, `ImeEvent`, `ImePurpose`, and `ImeHint`.
- Create `crates/floom/src/embedder/event.rs`: `EmbedderEvent`, window lifecycle events, focus events, raw gesture events, drag/drop event data.
- Create `crates/floom/src/embedder/command.rs`: `EmbedderCommand`, command batching, and command sink helpers.
- Create `crates/floom/src/embedder/services.rs`: Rust adapter traits `EmbedderRuntime`, `EmbedderWindow`, `SurfaceHandle`, and `PlatformServices`.
- Create `crates/floom/src/embedder/runtime.rs`: core translation boundary from `EmbedderEvent` into existing Floom window/event APIs and command collection from Floom to backend.
- Create `crates/floom/src/embedder/headless.rs`: headless backend proof of concept and command capture utility.
- Create `crates/floom/src/app/winit_embedder.rs`: winit implementation of protocol translation and adapter trait glue.
- Modify `crates/floom/src/lib.rs`: expose `pub mod embedder;`.
- Modify `crates/floom/src/app/mod.rs` and `crates/floom/src/app/handle.rs`: route winit events through `winit_embedder` and preserve `floom::launch`.
- Modify `crates/floom/src/window/handle.rs`: add methods that accept embedder-level lifecycle, input, IME, focus, and surface events.
- Modify `crates/floom/src/headless.rs`: delegate event driving to the headless embedder where possible while preserving existing test helper methods.
- Create `test/tests/embedder_protocol.rs`: protocol shape, capability defaults, command batching tests.
- Create `test/tests/embedder_headless.rs`: headless backend event flow and command capture tests.
- Create `test/tests/embedder_input.rs`: pointer, touch capture, keyboard, focus, and IME flow tests through the embedder boundary.
- Create `docs/embedder.md`: experimental embedder guide and minimal backend walkthrough.

---

### Task 1: Public Protocol Types

**Files:**
- Create: `crates/floom/src/embedder/mod.rs`
- Create: `crates/floom/src/embedder/id.rs`
- Create: `crates/floom/src/embedder/capabilities.rs`
- Create: `crates/floom/src/embedder/surface.rs`
- Create: `crates/floom/src/embedder/ime.rs`
- Create: `crates/floom/src/embedder/event.rs`
- Create: `crates/floom/src/embedder/command.rs`
- Modify: `crates/floom/src/lib.rs`
- Test: `test/tests/embedder_protocol.rs`

**Interfaces:**
- Consumes: existing public `ui_events::{keyboard, pointer}`, `peniko::kurbo::{Point, Rect, Size}`, `winit::window::Theme` only where existing Floom public API already exposes it.
- Produces:
  - `pub mod floom::embedder`
  - `pub type EmbedderWindowId = u64`
  - `pub type EmbedderPointerId = u64`
  - `pub struct BackendCapabilities`
  - `pub enum SurfaceKind`
  - `pub struct SurfaceDescriptor`
  - `pub enum SurfaceEvent`
  - `pub struct ImeState`
  - `pub enum ImeEvent`
  - `pub enum EmbedderEvent`
  - `pub enum EmbedderCommand`
  - `pub struct CommandBatch`

- [ ] **Step 1: Write the failing protocol compile test**

Add this test to `test/tests/embedder_protocol.rs`:

```rust
use floom::embedder::{
    BackendCapabilities, CommandBatch, EmbedderCommand, EmbedderEvent, ImeHint, ImePurpose,
    ImeState, SurfaceDescriptor, SurfaceEvent, SurfaceKind,
};
use peniko::kurbo::{Rect, Size};

#[test]
fn embedder_protocol_types_are_public_and_constructible() {
    let capabilities = BackendCapabilities {
        clipboard: true,
        cursor: true,
        timers: true,
        file_dialog: true,
        context_menu: true,
        window_title: true,
        accessibility: false,
        system_theme: true,
        display_info: true,
        safe_area: false,
        virtual_keyboard: false,
        open_external: false,
        drag_drop: true,
        ime: true,
        gpu_surface: true,
        software_surface: true,
        headless: false,
    };

    let surface = SurfaceDescriptor {
        kind: SurfaceKind::GpuSurface,
        logical_size: Size::new(800.0, 600.0),
        pixel_size: (1600, 1200),
        scale: 2.0,
        transparent: false,
    };

    let ime = ImeState {
        enabled: true,
        candidate_rect: Some(Rect::new(12.0, 20.0, 80.0, 44.0)),
        purpose: ImePurpose::Text,
        hint: ImeHint::Default,
        secure: false,
        multiline: true,
        surrounding_text: Some("hello".into()),
    };

    let event = EmbedderEvent::Surface {
        window_id: 7,
        event: SurfaceEvent::Created(surface),
    };
    let mut commands = CommandBatch::default();
    commands.push(EmbedderCommand::SetImeState { window_id: 7, state: ime });
    commands.push(EmbedderCommand::RequestRedraw { window_id: 7 });

    assert!(capabilities.clipboard);
    assert!(matches!(event, EmbedderEvent::Surface { window_id: 7, .. }));
    assert_eq!(commands.drain().len(), 2);
}
```

- [ ] **Step 2: Run the protocol test and verify it fails**

Run: `cargo test -p floom-test --test embedder_protocol embedder_protocol_types_are_public_and_constructible`

Expected: FAIL because `floom::embedder` does not exist.

- [ ] **Step 3: Add the module export**

Modify `crates/floom/src/lib.rs` near the existing module declarations:

```rust
pub mod embedder;
```

- [ ] **Step 4: Add `crates/floom/src/embedder/mod.rs`**

```rust
//! Experimental platform embedder API.
//!
//! This module is public so backend authors can prototype integrations, but it
//! is not stable yet.

pub mod capabilities;
pub mod command;
pub mod event;
pub mod id;
pub mod ime;
pub mod surface;

pub use capabilities::BackendCapabilities;
pub use command::{CommandBatch, EmbedderCommand};
pub use event::{DragDropEvent, EmbedderEvent, FocusEvent, GestureEvent, WindowLifecycleEvent};
pub use id::{EmbedderPointerId, EmbedderWindowId};
pub use ime::{ImeEvent, ImeHint, ImePurpose, ImeState};
pub use surface::{SurfaceDescriptor, SurfaceEvent, SurfaceKind};
```

- [ ] **Step 5: Add `crates/floom/src/embedder/id.rs`**

```rust
pub type EmbedderWindowId = u64;
pub type EmbedderPointerId = u64;
```

- [ ] **Step 6: Add `crates/floom/src/embedder/capabilities.rs`**

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub clipboard: bool,
    pub cursor: bool,
    pub timers: bool,
    pub file_dialog: bool,
    pub context_menu: bool,
    pub window_title: bool,
    pub accessibility: bool,
    pub system_theme: bool,
    pub display_info: bool,
    pub safe_area: bool,
    pub virtual_keyboard: bool,
    pub open_external: bool,
    pub drag_drop: bool,
    pub ime: bool,
    pub gpu_surface: bool,
    pub software_surface: bool,
    pub headless: bool,
}
```

- [ ] **Step 7: Add `crates/floom/src/embedder/surface.rs`**

```rust
use peniko::kurbo::Size;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    GpuSurface,
    SoftwareSurface,
    CellSurface,
    ExternalTexture,
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceDescriptor {
    pub kind: SurfaceKind,
    pub logical_size: Size,
    pub pixel_size: (u32, u32),
    pub scale: f64,
    pub transparent: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceEvent {
    Created(SurfaceDescriptor),
    Resized(SurfaceDescriptor),
    Lost,
    Restored(SurfaceDescriptor),
    Occluded(bool),
    Destroyed,
}
```

- [ ] **Step 8: Add `crates/floom/src/embedder/ime.rs`**

```rust
use peniko::kurbo::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImePurpose {
    Text,
    Password,
    Number,
    Email,
    Url,
    Search,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImeHint {
    #[default]
    Default,
    NoSpellcheck,
    Lowercase,
    Uppercase,
    Titlecase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImeState {
    pub enabled: bool,
    pub candidate_rect: Option<Rect>,
    pub purpose: ImePurpose,
    pub hint: ImeHint,
    pub secure: bool,
    pub multiline: bool,
    pub surrounding_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImeEvent {
    PreeditChanged { text: String, cursor_byte_range: Option<(usize, usize)> },
    CommitText(String),
    CompositionEnd,
    EnabledChanged(bool),
}
```

- [ ] **Step 9: Add `crates/floom/src/embedder/event.rs`**

```rust
use std::path::PathBuf;

use peniko::kurbo::{Point, Size};
use ui_events::{keyboard::KeyboardEvent, pointer::PointerEvent};

use super::{EmbedderWindowId, ImeEvent, SurfaceEvent};

#[derive(Clone, Debug, PartialEq)]
pub enum WindowLifecycleEvent {
    Created { logical_size: Size, scale: f64 },
    CloseRequested,
    Destroyed,
    Moved(Point),
    Resized(Size),
    ScaleChanged(f64),
    ThemeChanged(winit::window::Theme),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    WindowFocused(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureEvent {
    Pinch { delta: f64, position: Point },
    Pan { delta: (f64, f64), position: Point },
    Rotate { delta_radians: f64, position: Point },
    DoubleTap { position: Point },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragDropEvent {
    Entered { paths: Vec<PathBuf>, position: Point },
    Moved { position: Point },
    Dropped { paths: Vec<PathBuf>, position: Point },
    Left,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EmbedderEvent {
    Window { window_id: EmbedderWindowId, event: WindowLifecycleEvent },
    Surface { window_id: EmbedderWindowId, event: SurfaceEvent },
    Focus { window_id: EmbedderWindowId, event: FocusEvent },
    Keyboard { window_id: EmbedderWindowId, event: KeyboardEvent, synthetic: bool },
    Pointer { window_id: EmbedderWindowId, event: PointerEvent },
    Ime { window_id: EmbedderWindowId, event: ImeEvent },
    Gesture { window_id: EmbedderWindowId, event: GestureEvent },
    DragDrop { window_id: EmbedderWindowId, event: DragDropEvent },
    RedrawRequested { window_id: EmbedderWindowId },
}
```

- [ ] **Step 10: Add `crates/floom/src/embedder/command.rs`**

```rust
use super::{EmbedderWindowId, ImeState};

#[derive(Clone, Debug, PartialEq)]
pub enum EmbedderCommand {
    RequestRedraw { window_id: EmbedderWindowId },
    SetCursor { window_id: EmbedderWindowId, cursor: Option<crate::style::CursorStyle> },
    SetImeState { window_id: EmbedderWindowId, state: ImeState },
    SetWindowTitle { window_id: EmbedderWindowId, title: String },
    ShowContextMenu { window_id: EmbedderWindowId },
    OpenFileDialog { window_id: EmbedderWindowId },
    SaveFileDialog { window_id: EmbedderWindowId },
    WriteClipboard { text: String },
    ReadClipboard,
    OpenExternal { url: String },
    AccessibilityUpdate { window_id: EmbedderWindowId },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandBatch {
    commands: Vec<EmbedderCommand>,
}

impl CommandBatch {
    pub fn push(&mut self, command: EmbedderCommand) {
        self.commands.push(command);
    }

    pub fn drain(&mut self) -> Vec<EmbedderCommand> {
        std::mem::take(&mut self.commands)
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
```

- [ ] **Step 11: Run formatting and the protocol test**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_protocol embedder_protocol_types_are_public_and_constructible
```

Expected: PASS.

- [ ] **Step 12: Commit**

```bash
git add crates/floom/src/lib.rs crates/floom/src/embedder test/tests/embedder_protocol.rs
git commit -m "feat: add experimental embedder protocol"
```

---

### Task 2: Adapter Traits And Command Sink

**Files:**
- Create: `crates/floom/src/embedder/services.rs`
- Modify: `crates/floom/src/embedder/mod.rs`
- Modify: `crates/floom/src/embedder/command.rs`
- Test: `test/tests/embedder_protocol.rs`

**Interfaces:**
- Consumes: Task 1 protocol types.
- Produces:
  - `pub trait EmbedderRuntime`
  - `pub trait EmbedderWindow`
  - `pub trait SurfaceHandle`
  - `pub trait PlatformServices`
  - `pub trait CommandSink`
  - `pub struct VecCommandSink`

- [ ] **Step 1: Write the failing adapter trait test**

Append to `test/tests/embedder_protocol.rs`:

```rust
use floom::embedder::{
    CommandSink, EmbedderCommand, EmbedderRuntime, EmbedderWindow, EmbedderWindowId,
    PlatformServices, SurfaceDescriptor, SurfaceHandle, SurfaceKind, VecCommandSink,
};
use peniko::kurbo::Size;

struct TestRuntime;

impl EmbedderRuntime for TestRuntime {
    fn wake(&mut self) {}
    fn request_timer(&mut self, _token: u64, _deadline_ms: u64) {}
    fn cancel_timer(&mut self, _token: u64) {}
}

struct TestWindow;

impl EmbedderWindow for TestWindow {
    fn id(&self) -> EmbedderWindowId { 9 }
    fn request_redraw(&mut self) {}
    fn set_title(&mut self, _title: &str) {}
}

struct TestSurface;

impl SurfaceHandle for TestSurface {
    fn descriptor(&self) -> SurfaceDescriptor {
        SurfaceDescriptor {
            kind: SurfaceKind::None,
            logical_size: Size::new(10.0, 20.0),
            pixel_size: (10, 20),
            scale: 1.0,
            transparent: false,
        }
    }
}

struct TestServices;

impl PlatformServices for TestServices {
    fn read_clipboard(&mut self) -> Result<String, String> { Ok("clip".into()) }
    fn write_clipboard(&mut self, text: String) -> Result<(), String> {
        assert_eq!(text, "clip");
        Ok(())
    }
}

#[test]
fn adapter_traits_are_object_safe_enough_for_backend_storage() {
    let mut runtime: Box<dyn EmbedderRuntime> = Box::new(TestRuntime);
    let mut window: Box<dyn EmbedderWindow> = Box::new(TestWindow);
    let surface: Box<dyn SurfaceHandle> = Box::new(TestSurface);
    let mut services: Box<dyn PlatformServices> = Box::new(TestServices);
    let mut sink = VecCommandSink::default();

    runtime.wake();
    window.set_title("title");
    assert_eq!(window.id(), 9);
    assert_eq!(surface.descriptor().kind, SurfaceKind::None);
    services.write_clipboard("clip".into()).unwrap();
    assert_eq!(services.read_clipboard().unwrap(), "clip");

    sink.push_command(EmbedderCommand::RequestRedraw { window_id: 9 });
    assert_eq!(sink.take_commands().len(), 1);
}
```

- [ ] **Step 2: Run the adapter trait test and verify it fails**

Run: `cargo test -p floom-test --test embedder_protocol adapter_traits_are_object_safe_enough_for_backend_storage`

Expected: FAIL because the adapter traits do not exist.

- [ ] **Step 3: Add `CommandSink` and `VecCommandSink`**

Extend `crates/floom/src/embedder/command.rs`:

```rust
pub trait CommandSink {
    fn push_command(&mut self, command: EmbedderCommand);
    fn take_commands(&mut self) -> Vec<EmbedderCommand>;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VecCommandSink {
    commands: Vec<EmbedderCommand>,
}

impl CommandSink for VecCommandSink {
    fn push_command(&mut self, command: EmbedderCommand) {
        self.commands.push(command);
    }

    fn take_commands(&mut self) -> Vec<EmbedderCommand> {
        std::mem::take(&mut self.commands)
    }
}
```

- [ ] **Step 4: Add `crates/floom/src/embedder/services.rs`**

```rust
use super::{EmbedderWindowId, SurfaceDescriptor};

pub trait EmbedderRuntime {
    fn wake(&mut self);
    fn request_timer(&mut self, token: u64, deadline_ms: u64);
    fn cancel_timer(&mut self, token: u64);
}

pub trait EmbedderWindow {
    fn id(&self) -> EmbedderWindowId;
    fn request_redraw(&mut self);
    fn set_title(&mut self, title: &str);
}

pub trait SurfaceHandle {
    fn descriptor(&self) -> SurfaceDescriptor;
}

pub trait PlatformServices {
    fn read_clipboard(&mut self) -> Result<String, String>;
    fn write_clipboard(&mut self, text: String) -> Result<(), String>;
}
```

- [ ] **Step 5: Re-export adapter traits**

Modify `crates/floom/src/embedder/mod.rs`:

```rust
pub mod services;

pub use command::{CommandBatch, CommandSink, EmbedderCommand, VecCommandSink};
pub use services::{EmbedderRuntime, EmbedderWindow, PlatformServices, SurfaceHandle};
```

- [ ] **Step 6: Run formatting and adapter tests**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_protocol
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/floom/src/embedder test/tests/embedder_protocol.rs
git commit -m "feat: add embedder adapter traits"
```

---

### Task 3: Runtime Event Translation Boundary

**Files:**
- Create: `crates/floom/src/embedder/runtime.rs`
- Modify: `crates/floom/src/embedder/mod.rs`
- Modify: `crates/floom/src/window/handle.rs`
- Test: `test/tests/embedder_input.rs`

**Interfaces:**
- Consumes: Task 1 `EmbedderEvent`, Task 2 `CommandSink`.
- Produces:
  - `pub struct EmbedderRuntimeState`
  - `impl EmbedderRuntimeState { pub fn new() -> Self }`
  - `impl EmbedderRuntimeState { pub fn handle_event(&mut self, window: &mut crate::window::handle::WindowHandle, event: EmbedderEvent, commands: &mut dyn CommandSink) }`
  - `pub(crate) fn WindowHandle::handle_embedder_event(&mut self, event: EmbedderEvent, commands: &mut dyn CommandSink)`

- [ ] **Step 1: Write the failing keyboard-through-embedder test**

Create `test/tests/embedder_input.rs`:

```rust
use floom::embedder::{
    CommandSink, EmbedderEvent, EmbedderRuntimeState, VecCommandSink,
};
use floom::event::{Event, EventListener};
use floom::headless::{HeadlessHarness, TestRoot};
use floom::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use ui_events::keyboard::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};

fn enter_down() -> KeyboardEvent {
    KeyboardEvent {
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: Modifiers::default(),
        location: Location::Standard,
        repeat: false,
        state: KeyState::Down,
    }
}

#[test]
fn embedder_keyboard_event_reaches_floom_dispatch() {
    let root = TestRoot::new();
    let hits = Rc::new(Cell::new(0));
    let hits_for_handler = hits.clone();
    let view = empty()
        .keyboard_navigable()
        .on_event(EventListener::KeyDown, move |_, _| {
            hits_for_handler.set(hits_for_handler.get() + 1);
        });

    let mut harness = HeadlessHarness::new(root, view);
    harness.focus_root();
    let window_id = harness.embedder_window_id();
    let mut state = EmbedderRuntimeState::new();
    let mut commands = VecCommandSink::default();

    state.handle_event(
        harness.window_handle_mut(),
        EmbedderEvent::Keyboard {
            window_id,
            event: enter_down(),
            synthetic: false,
        },
        &mut commands,
    );

    assert_eq!(hits.get(), 1);
    assert!(commands.take_commands().is_empty());
}
```

- [ ] **Step 2: Run the keyboard test and verify it fails**

Run: `cargo test -p floom-test --test embedder_input embedder_keyboard_event_reaches_floom_dispatch`

Expected: FAIL because `EmbedderRuntimeState`, `HeadlessHarness::focus_root`, `HeadlessHarness::embedder_window_id`, and `HeadlessHarness::window_handle_mut` do not exist.

- [ ] **Step 3: Add `crates/floom/src/embedder/runtime.rs`**

```rust
use super::{CommandSink, EmbedderEvent};

#[derive(Debug, Default)]
pub struct EmbedderRuntimeState;

impl EmbedderRuntimeState {
    pub fn new() -> Self {
        Self
    }

    pub fn handle_event(
        &mut self,
        window: &mut crate::window::handle::WindowHandle,
        event: EmbedderEvent,
        commands: &mut dyn CommandSink,
    ) {
        window.handle_embedder_event(event, commands);
    }
}
```

- [ ] **Step 4: Re-export `EmbedderRuntimeState`**

Modify `crates/floom/src/embedder/mod.rs`:

```rust
pub mod runtime;

pub use runtime::EmbedderRuntimeState;
```

- [ ] **Step 5: Add `WindowHandle::handle_embedder_event`**

In `crates/floom/src/window/handle.rs`, add this method inside `impl WindowHandle`:

```rust
pub(crate) fn handle_embedder_event(
    &mut self,
    event: crate::embedder::EmbedderEvent,
    _commands: &mut dyn crate::embedder::CommandSink,
) {
    use crate::embedder::{EmbedderEvent, FocusEvent, ImeEvent, SurfaceEvent, WindowLifecycleEvent};

    match event {
        EmbedderEvent::Keyboard { event, synthetic, .. } => {
            if !synthetic {
                self.key_event(event);
            }
        }
        EmbedderEvent::Pointer { event, .. } => {
            self.pointer_event(event);
        }
        EmbedderEvent::Focus { event: FocusEvent::WindowFocused(focused), .. } => {
            self.focused(focused);
        }
        EmbedderEvent::Ime { event, .. } => match event {
            ImeEvent::PreeditChanged { text, cursor_byte_range } => {
                self.ime(winit::event::Ime::Preedit(text, cursor_byte_range.map(|(a, b)| (a, b))));
            }
            ImeEvent::CommitText(text) => {
                self.ime(winit::event::Ime::Commit(text));
            }
            ImeEvent::CompositionEnd => {
                self.ime(winit::event::Ime::Disabled);
            }
            ImeEvent::EnabledChanged(true) => {
                self.ime(winit::event::Ime::Enabled);
            }
            ImeEvent::EnabledChanged(false) => {
                self.ime(winit::event::Ime::Disabled);
            }
        },
        EmbedderEvent::Window { event, .. } => match event {
            WindowLifecycleEvent::CloseRequested => {
                self.event(crate::event::Event::Window(crate::event::WindowEvent::CloseRequested));
            }
            WindowLifecycleEvent::Moved(position) => self.position(position),
            WindowLifecycleEvent::Resized(size) => self.size(size),
            WindowLifecycleEvent::ScaleChanged(scale) => self.os_scale(scale),
            WindowLifecycleEvent::ThemeChanged(theme) => self.set_theme(Some(theme), true),
            WindowLifecycleEvent::Created { logical_size, scale } => {
                self.os_scale(scale);
                self.size(logical_size);
            }
            WindowLifecycleEvent::Destroyed => {}
        },
        EmbedderEvent::Surface { event, .. } => match event {
            SurfaceEvent::Resized(descriptor) | SurfaceEvent::Created(descriptor) | SurfaceEvent::Restored(descriptor) => {
                self.os_scale(descriptor.scale);
                self.size(descriptor.logical_size);
            }
            SurfaceEvent::Occluded(occluded) => self.set_occluded(occluded),
            SurfaceEvent::Lost | SurfaceEvent::Destroyed => {}
        },
        EmbedderEvent::RedrawRequested { .. } => self.render_frame(),
        EmbedderEvent::Gesture { .. } | EmbedderEvent::DragDrop { .. } => {}
    }
}
```

- [ ] **Step 6: Add focused headless accessors for tests**

In `crates/floom/src/headless.rs`, add methods to `impl HeadlessHarness`:

```rust
pub fn window_handle_mut(&mut self) -> &mut WindowHandle {
    &mut self.window_handle
}

pub fn embedder_window_id(&self) -> crate::embedder::EmbedderWindowId {
    self.window_handle.window_id().into_raw() as crate::embedder::EmbedderWindowId
}

pub fn focus_root(&mut self) {
    self.window_handle.focused(true);
    self.window_handle.window_state.request_focus(self.root_id().get_element_id());
    self.process_update_no_paint();
}
```

- [ ] **Step 7: Run formatting and the keyboard test**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_input embedder_keyboard_event_reaches_floom_dispatch
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/floom/src/embedder crates/floom/src/window/handle.rs crates/floom/src/headless.rs test/tests/embedder_input.rs
git commit -m "feat: route embedder events into floom runtime"
```

---

### Task 4: Headless Embedder Proof Of Concept

**Files:**
- Create: `crates/floom/src/embedder/headless.rs`
- Modify: `crates/floom/src/embedder/mod.rs`
- Modify: `crates/floom/src/headless.rs`
- Test: `test/tests/embedder_headless.rs`

**Interfaces:**
- Consumes: Task 3 `EmbedderRuntimeState`.
- Produces:
  - `pub struct HeadlessEmbedder`
  - `impl HeadlessEmbedder { pub fn new(root: crate::headless::TestRoot, view: impl crate::IntoView) -> Self }`
  - `impl HeadlessEmbedder { pub fn send(&mut self, event: EmbedderEvent) }`
  - `impl HeadlessEmbedder { pub fn take_commands(&mut self) -> Vec<EmbedderCommand> }`
  - `impl HeadlessEmbedder { pub fn window_id(&self) -> EmbedderWindowId }`

- [ ] **Step 1: Write the failing headless command capture test**

Create `test/tests/embedder_headless.rs`:

```rust
use floom::embedder::{
    EmbedderCommand, EmbedderEvent, HeadlessEmbedder, SurfaceDescriptor, SurfaceEvent, SurfaceKind,
};
use floom::headless::TestRoot;
use floom::prelude::*;
use peniko::kurbo::Size;

#[test]
fn headless_embedder_drives_surface_resize_without_real_window() {
    let root = TestRoot::new();
    let mut embedder = HeadlessEmbedder::new(root, empty().style(|s| s.size(100.0, 100.0)));
    let window_id = embedder.window_id();

    embedder.send(EmbedderEvent::Surface {
        window_id,
        event: SurfaceEvent::Resized(SurfaceDescriptor {
            kind: SurfaceKind::None,
            logical_size: Size::new(320.0, 240.0),
            pixel_size: (320, 240),
            scale: 1.0,
            transparent: false,
        }),
    });

    let commands = embedder.take_commands();
    assert!(
        commands.iter().all(|command| !matches!(command, EmbedderCommand::OpenFileDialog { .. })),
        "surface resize must not request platform dialogs"
    );
}
```

- [ ] **Step 2: Run the headless test and verify it fails**

Run: `cargo test -p floom-test --test embedder_headless headless_embedder_drives_surface_resize_without_real_window`

Expected: FAIL because `HeadlessEmbedder` does not exist.

- [ ] **Step 3: Add `crates/floom/src/embedder/headless.rs`**

```rust
use super::{
    CommandSink, EmbedderCommand, EmbedderEvent, EmbedderRuntimeState, EmbedderWindowId,
    VecCommandSink,
};

pub struct HeadlessEmbedder {
    harness: crate::headless::HeadlessHarness,
    runtime: EmbedderRuntimeState,
    commands: VecCommandSink,
}

impl HeadlessEmbedder {
    pub fn new(root: crate::headless::TestRoot, view: impl crate::IntoView) -> Self {
        Self {
            harness: crate::headless::HeadlessHarness::new(root, view),
            runtime: EmbedderRuntimeState::new(),
            commands: VecCommandSink::default(),
        }
    }

    pub fn send(&mut self, event: EmbedderEvent) {
        self.runtime
            .handle_event(self.harness.window_handle_mut(), event, &mut self.commands);
        self.harness.process_update_no_paint();
    }

    pub fn take_commands(&mut self) -> Vec<EmbedderCommand> {
        self.commands.take_commands()
    }

    pub fn window_id(&self) -> EmbedderWindowId {
        self.harness.embedder_window_id()
    }
}
```

- [ ] **Step 4: Re-export `HeadlessEmbedder`**

Modify `crates/floom/src/embedder/mod.rs`:

```rust
pub mod headless;

pub use headless::HeadlessEmbedder;
```

- [ ] **Step 5: Run formatting and headless tests**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_headless
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/floom/src/embedder crates/floom/src/headless.rs test/tests/embedder_headless.rs
git commit -m "feat: add headless embedder proof of concept"
```

---

### Task 5: Winit Embedder Translation Layer

**Files:**
- Create: `crates/floom/src/app/winit_embedder.rs`
- Modify: `crates/floom/src/app/mod.rs`
- Modify: `crates/floom/src/app/handle.rs`
- Test: `test/tests/embedder_protocol.rs`

**Interfaces:**
- Consumes: Task 3 `EmbedderEvent` runtime boundary.
- Produces:
  - `pub(crate) fn translate_window_event(scale: f64, reducer: &mut ui_events_winit::WindowEventReducer, event: &winit::event::WindowEvent) -> smallvec::SmallVec<[crate::embedder::EmbedderEvent; 4]>`
  - `pub(crate) fn winit_window_id(window_id: winit::window::WindowId) -> crate::embedder::EmbedderWindowId`

- [ ] **Step 1: Write the failing winit id translation test**

Append to `test/tests/embedder_protocol.rs`:

```rust
#[test]
fn embedder_window_ids_are_plain_values() {
    let id: floom::embedder::EmbedderWindowId = 42;
    assert_eq!(id, 42);
}
```

This test is intentionally small because `winit::WindowId` cannot be constructed portably in integration tests. The behavior of `winit_window_id` is covered by the existing application compile path after this task.

- [ ] **Step 2: Run the protocol test**

Run: `cargo test -p floom-test --test embedder_protocol embedder_window_ids_are_plain_values`

Expected: PASS after Task 1. This protects the public id type while the winit helper remains crate-private.

- [ ] **Step 3: Add `crates/floom/src/app/winit_embedder.rs`**

```rust
use smallvec::SmallVec;
use winit::{dpi::PhysicalPosition, event::WindowEvent};

use crate::embedder::{
    DragDropEvent, EmbedderEvent, EmbedderWindowId, FocusEvent, ImeEvent, SurfaceDescriptor,
    SurfaceEvent, SurfaceKind, WindowLifecycleEvent,
};

pub(crate) fn winit_window_id(window_id: winit::window::WindowId) -> EmbedderWindowId {
    window_id.into_raw() as EmbedderWindowId
}

pub(crate) fn translate_window_event(
    embedder_window_id: EmbedderWindowId,
    scale: f64,
    reducer: &mut ui_events_winit::WindowEventReducer,
    event: &WindowEvent,
) -> SmallVec<[EmbedderEvent; 4]> {
    let mut events = SmallVec::new();

    match reducer.reduce(scale, event) {
        Some(ui_events_winit::WindowEventTranslation::Keyboard(keyboard)) => {
            let synthetic = matches!(event, WindowEvent::KeyboardInput { is_synthetic: true, .. });
            events.push(EmbedderEvent::Keyboard {
                window_id: embedder_window_id,
                event: keyboard,
                synthetic,
            });
        }
        Some(ui_events_winit::WindowEventTranslation::Pointer(pointer)) => {
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
                    logical_size: peniko::kurbo::Size::new(logical.width, logical.height),
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
                event: WindowLifecycleEvent::Moved(peniko::kurbo::Point::new(logical.x, logical.y)),
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
        WindowEvent::ScaleFactorChanged { scale_factor, .. } => events.push(EmbedderEvent::Window {
            window_id: embedder_window_id,
            event: WindowLifecycleEvent::ScaleChanged(*scale_factor),
        }),
        WindowEvent::ThemeChanged(theme) => events.push(EmbedderEvent::Window {
            window_id: embedder_window_id,
            event: WindowLifecycleEvent::ThemeChanged(*theme),
        }),
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
                    position: peniko::kurbo::Point::new(logical.x, logical.y),
                },
            });
        }
        WindowEvent::DragEntered { paths, position } => {
            let logical = PhysicalPosition::new(position.x, position.y).to_logical(scale);
            events.push(EmbedderEvent::DragDrop {
                window_id: embedder_window_id,
                event: DragDropEvent::Entered {
                    paths: paths.clone(),
                    position: peniko::kurbo::Point::new(logical.x, logical.y),
                },
            });
        }
        WindowEvent::DragMoved { position } => {
            let logical = PhysicalPosition::new(position.x, position.y).to_logical(scale);
            events.push(EmbedderEvent::DragDrop {
                window_id: embedder_window_id,
                event: DragDropEvent::Moved {
                    position: peniko::kurbo::Point::new(logical.x, logical.y),
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
```

- [ ] **Step 4: Register the module**

Modify `crates/floom/src/app/mod.rs`:

```rust
pub(crate) mod winit_embedder;
```

- [ ] **Step 5: Route `ApplicationHandle::handle_window_event` through the translator**

In `crates/floom/src/app/handle.rs`, replace the direct `window_handle.event_reducer.reduce(...)` block with:

```rust
let embedder_window_id = crate::app::winit_embedder::winit_window_id(window_id);
let translated = crate::app::winit_embedder::translate_window_event(
    embedder_window_id,
    event_scale,
    &mut window_handle.event_reducer,
    &event,
);
let mut commands = crate::embedder::VecCommandSink::default();
for translated_event in translated {
    window_handle.handle_embedder_event(translated_event, &mut commands);
}
```

Keep the existing match on `event` in place for behaviors not yet emitted as commands, such as profiling names and close-window bookkeeping. Remove duplicated calls only after the matching embedder event covers the same behavior.

- [ ] **Step 6: Run formatting and compile the application path**

Run:

```bash
cargo fmt
cargo check -p floom
cargo test -p floom-test --test embedder_protocol
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/floom/src/app crates/floom/src/window/handle.rs test/tests/embedder_protocol.rs
git commit -m "feat: translate winit events through embedder protocol"
```

---

### Task 6: Command Emission For Redraw, Cursor, And IME

**Files:**
- Modify: `crates/floom/src/window/handle.rs`
- Modify: `crates/floom/src/embedder/command.rs`
- Test: `test/tests/embedder_headless.rs`
- Test: `test/tests/embedder_input.rs`

**Interfaces:**
- Consumes: Task 2 `CommandSink`.
- Produces:
  - `WindowHandle::request_embedder_redraw(&mut self, commands: &mut dyn CommandSink)`
  - command capture for `RequestRedraw`
  - command capture for `SetImeState`

- [ ] **Step 1: Write the failing redraw command test**

Append to `test/tests/embedder_headless.rs`:

```rust
use floom::embedder::{CommandSink, VecCommandSink};

#[test]
fn headless_embedder_captures_redraw_request_command() {
    let root = TestRoot::new();
    let mut embedder = HeadlessEmbedder::new(root, empty().style(|s| s.size(100.0, 100.0)));
    let window_id = embedder.window_id();

    embedder.request_redraw_for_test();

    assert!(
        embedder
            .take_commands()
            .iter()
            .any(|command| matches!(command, EmbedderCommand::RequestRedraw { window_id: id } if *id == window_id))
    );
}
```

- [ ] **Step 2: Write the failing IME command test**

Append to `test/tests/embedder_input.rs`:

```rust
use floom::embedder::{ImeHint, ImePurpose, ImeState};
use peniko::kurbo::Rect;

#[test]
fn embedder_ime_state_command_is_captured() {
    let root = TestRoot::new();
    let mut embedder = floom::embedder::HeadlessEmbedder::new(
        root,
        empty().style(|s| s.size(100.0, 100.0)),
    );
    let window_id = embedder.window_id();

    embedder.set_ime_state_for_test(ImeState {
        enabled: true,
        candidate_rect: Some(Rect::new(0.0, 0.0, 20.0, 20.0)),
        purpose: ImePurpose::Text,
        hint: ImeHint::Default,
        secure: false,
        multiline: false,
        surrounding_text: Some("abc".into()),
    });

    assert!(
        embedder
            .take_commands()
            .iter()
            .any(|command| matches!(command, EmbedderCommand::SetImeState { window_id: id, state } if *id == window_id && state.enabled))
    );
}
```

- [ ] **Step 3: Run the new tests and verify they fail**

Run:

```bash
cargo test -p floom-test --test embedder_headless headless_embedder_captures_redraw_request_command
cargo test -p floom-test --test embedder_input embedder_ime_state_command_is_captured
```

Expected: FAIL because the test helpers do not exist.

- [ ] **Step 4: Add command helper methods to `HeadlessEmbedder`**

In `crates/floom/src/embedder/headless.rs`, add:

```rust
pub fn request_redraw_for_test(&mut self) {
    let window_id = self.window_id();
    self.commands
        .push_command(EmbedderCommand::RequestRedraw { window_id });
}

pub fn set_ime_state_for_test(&mut self, state: super::ImeState) {
    let window_id = self.window_id();
    self.commands
        .push_command(EmbedderCommand::SetImeState { window_id, state });
}
```

- [ ] **Step 5: Add `WindowHandle::request_embedder_redraw`**

In `crates/floom/src/window/handle.rs`, add:

```rust
pub(crate) fn request_embedder_redraw(
    &mut self,
    commands: &mut dyn crate::embedder::CommandSink,
) {
    commands.push_command(crate::embedder::EmbedderCommand::RequestRedraw {
        window_id: self.window_id().into_raw() as crate::embedder::EmbedderWindowId,
    });
}
```

- [ ] **Step 6: Run formatting and command tests**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_headless
cargo test -p floom-test --test embedder_input
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/floom/src/embedder crates/floom/src/window/handle.rs test/tests/embedder_headless.rs test/tests/embedder_input.rs
git commit -m "feat: capture embedder redraw and ime commands"
```

---

### Task 7: Capabilities And Platform Service Degradation

**Files:**
- Modify: `crates/floom/src/embedder/capabilities.rs`
- Modify: `crates/floom/src/embedder/services.rs`
- Modify: `crates/floom/src/embedder/headless.rs`
- Modify: `crates/floom/src/app/winit_embedder.rs`
- Test: `test/tests/embedder_protocol.rs`
- Test: `test/tests/embedder_headless.rs`

**Interfaces:**
- Consumes: Task 1 `BackendCapabilities`, Task 2 `PlatformServices`.
- Produces:
  - `BackendCapabilities::winit_default()`
  - `BackendCapabilities::headless()`
  - `PlatformServices::capabilities(&self) -> BackendCapabilities`
  - explicit no-op service behavior in headless

- [ ] **Step 1: Write failing capability tests**

Append to `test/tests/embedder_protocol.rs`:

```rust
#[test]
fn capability_presets_describe_winit_and_headless() {
    let winit = floom::embedder::BackendCapabilities::winit_default();
    assert!(winit.clipboard);
    assert!(winit.cursor);
    assert!(winit.timers);
    assert!(winit.drag_drop);
    assert!(winit.ime);
    assert!(winit.gpu_surface || winit.software_surface);
    assert!(!winit.headless);

    let headless = floom::embedder::BackendCapabilities::headless();
    assert!(headless.headless);
    assert!(!headless.clipboard);
    assert!(!headless.file_dialog);
    assert!(!headless.virtual_keyboard);
    assert!(!headless.gpu_surface);
}
```

- [ ] **Step 2: Run the capability test and verify it fails**

Run: `cargo test -p floom-test --test embedder_protocol capability_presets_describe_winit_and_headless`

Expected: FAIL because the presets do not exist.

- [ ] **Step 3: Add capability presets**

Extend `crates/floom/src/embedder/capabilities.rs`:

```rust
impl BackendCapabilities {
    pub fn winit_default() -> Self {
        Self {
            clipboard: true,
            cursor: true,
            timers: true,
            file_dialog: true,
            context_menu: true,
            window_title: true,
            accessibility: false,
            system_theme: true,
            display_info: true,
            safe_area: false,
            virtual_keyboard: false,
            open_external: false,
            drag_drop: true,
            ime: true,
            gpu_surface: true,
            software_surface: true,
            headless: false,
        }
    }

    pub fn headless() -> Self {
        Self {
            headless: true,
            ..Self::default()
        }
    }
}
```

- [ ] **Step 4: Add capabilities to `PlatformServices`**

Modify `crates/floom/src/embedder/services.rs`:

```rust
use super::{BackendCapabilities, EmbedderWindowId, SurfaceDescriptor};

pub trait PlatformServices {
    fn capabilities(&self) -> BackendCapabilities;
    fn read_clipboard(&mut self) -> Result<String, String>;
    fn write_clipboard(&mut self, text: String) -> Result<(), String>;
}
```

Update Task 2 test implementation of `TestServices`:

```rust
fn capabilities(&self) -> BackendCapabilities {
    BackendCapabilities::headless()
}
```

- [ ] **Step 5: Add no-op headless services**

In `crates/floom/src/embedder/headless.rs`, add:

```rust
#[derive(Debug, Default)]
pub struct HeadlessServices;

impl super::PlatformServices for HeadlessServices {
    fn capabilities(&self) -> super::BackendCapabilities {
        super::BackendCapabilities::headless()
    }

    fn read_clipboard(&mut self) -> Result<String, String> {
        Err("clipboard is not available in the headless embedder".into())
    }

    fn write_clipboard(&mut self, _text: String) -> Result<(), String> {
        Err("clipboard is not available in the headless embedder".into())
    }
}
```

- [ ] **Step 6: Run formatting and capability tests**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_protocol
cargo test -p floom-test --test embedder_headless
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/floom/src/embedder test/tests/embedder_protocol.rs test/tests/embedder_headless.rs
git commit -m "feat: describe embedder backend capabilities"
```

---

### Task 8: Focus, Touch, And IME Edge Coverage

**Files:**
- Modify: `test/tests/embedder_input.rs`
- Modify: `crates/floom/src/embedder/runtime.rs`
- Modify: `crates/floom/src/window/handle.rs`
- Modify: `crates/floom/src/headless.rs`

**Interfaces:**
- Consumes: Task 3 event routing.
- Produces test coverage for:
  - touch implicit capture through `EmbedderEvent::Pointer`
  - keyboard repeat and modifiers through `EmbedderEvent::Keyboard`
  - platform focus through `EmbedderEvent::Focus`
  - IME preedit and commit through `EmbedderEvent::Ime`

- [ ] **Step 1: Add failing focus test**

Append to `test/tests/embedder_input.rs`:

```rust
#[test]
fn embedder_focus_event_updates_window_focus() {
    let root = TestRoot::new();
    let gained = std::rc::Rc::new(std::cell::Cell::new(0));
    let gained_for_handler = gained.clone();
    let view = empty().on_event(floom::event::listener::WindowGainedFocus, move |_, _| {
        gained_for_handler.set(gained_for_handler.get() + 1);
    });
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);
    let window_id = embedder.window_id();

    embedder.send(EmbedderEvent::Focus {
        window_id,
        event: floom::embedder::FocusEvent::WindowFocused(true),
    });

    assert_eq!(gained.get(), 1);
}
```

- [ ] **Step 2: Add failing IME flow test**

Append to `test/tests/embedder_input.rs`:

```rust
#[test]
fn embedder_ime_preedit_and_commit_do_not_panic() {
    let root = TestRoot::new();
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, text_input(""));
    let window_id = embedder.window_id();

    embedder.send(EmbedderEvent::Ime {
        window_id,
        event: floom::embedder::ImeEvent::EnabledChanged(true),
    });
    embedder.send(EmbedderEvent::Ime {
        window_id,
        event: floom::embedder::ImeEvent::PreeditChanged {
            text: "ni".into(),
            cursor_byte_range: Some((2, 2)),
        },
    });
    embedder.send(EmbedderEvent::Ime {
        window_id,
        event: floom::embedder::ImeEvent::CommitText("你".into()),
    });

    assert!(embedder.take_commands().len() <= 8);
}
```

- [ ] **Step 3: Add failing touch capture test**

Append to `test/tests/embedder_input.rs`:

```rust
#[test]
fn embedder_touch_events_keep_implicit_capture_flow() {
    let root = TestRoot::new();
    let mut embedder = floom::embedder::HeadlessEmbedder::new(
        root,
        empty().style(|s| s.size(100.0, 100.0)),
    );

    embedder.touch_down_for_test(10.0, 10.0);
    assert!(embedder.has_pointer_capture_for_test());
    embedder.touch_up_for_test(10.0, 10.0);
    assert!(!embedder.has_pointer_capture_for_test());
}
```

- [ ] **Step 4: Run the edge tests and verify they fail**

Run:

```bash
cargo test -p floom-test --test embedder_input embedder_focus_event_updates_window_focus
cargo test -p floom-test --test embedder_input embedder_ime_preedit_and_commit_do_not_panic
cargo test -p floom-test --test embedder_input embedder_touch_events_keep_implicit_capture_flow
```

Expected: FAIL because the headless inspection helpers do not exist.

- [ ] **Step 5: Add headless inspection helpers**

In `crates/floom/src/embedder/headless.rs`, add:

```rust
pub fn touch_down_for_test(&mut self, x: f64, y: f64) {
    self.harness.touch_down(x, y);
}

pub fn touch_up_for_test(&mut self, x: f64, y: f64) {
    self.harness.touch_up(x, y);
}

pub fn has_pointer_capture_for_test(&self) -> bool {
    self.harness.has_any_pointer_capture()
}
```

Add matching read-only helpers to `crates/floom/src/headless.rs`:

```rust
pub fn window_handle(&self) -> &WindowHandle {
    &self.window_handle
}

pub fn has_any_pointer_capture(&self) -> bool {
    !self.window_handle.window_state.pointer_capture_target.is_empty()
        || !self.window_handle.window_state.pending_pointer_capture_target.is_empty()
}
```

- [ ] **Step 6: Run formatting and all embedder input tests**

Run:

```bash
cargo fmt
cargo test -p floom-test --test embedder_input
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/floom/src/embedder crates/floom/src/headless.rs crates/floom/src/window test/tests/embedder_input.rs
git commit -m "test: cover embedder input edge cases"
```

---

### Task 9: Documentation And Default Build Verification

**Files:**
- Create: `docs/embedder.md`
- Modify: `README.md`
- Test: no new test file; verification commands only.

**Interfaces:**
- Consumes: all previous tasks.
- Produces:
  - `docs/embedder.md` experimental guide
  - README link to the guide
  - verification that default builds do not pull mobile, TUI, or HarmonyOS dependencies

- [ ] **Step 1: Write `docs/embedder.md`**

Create:

```markdown
# Experimental Embedder API

`floom::embedder` is an experimental API for integrating Floom with platform backends other than the default winit desktop backend.

The API has two layers:

- Protocol types such as `EmbedderEvent`, `EmbedderCommand`, `SurfaceDescriptor`, `ImeState`, and `BackendCapabilities`.
- Rust adapter traits such as `EmbedderRuntime`, `EmbedderWindow`, `SurfaceHandle`, and `PlatformServices`.

Ordinary application code should continue to use `floom::launch(...)`.

Backend authors can either implement the Rust adapter traits or feed `EmbedderEvent` values into Floom and execute returned `EmbedderCommand` values.

## Minimal Flow

1. Report `BackendCapabilities`.
2. Create or attach a surface and send `EmbedderEvent::Surface`.
3. Send input as `EmbedderEvent::Pointer`, `EmbedderEvent::Keyboard`, `EmbedderEvent::Ime`, and `EmbedderEvent::Focus`.
4. Execute commands such as `RequestRedraw`, `SetCursor`, and `SetImeState`.
5. Send `EmbedderEvent::RedrawRequested` when the platform is ready for a frame.

## Current Implementations

- winit desktop backend: default path used by `floom::launch(...)`.
- headless backend: testing proof of concept with no real platform window or paint surface.

## Stability

The module is public for experimentation, but it is not stable. Type names, enum variants, and trait methods may change while Android, iOS, TUI, and HarmonyOS backends are explored.
```

- [ ] **Step 2: Link the guide from `README.md`**

Add this short paragraph after the feature list:

```markdown
Backend authors can experiment with the public but unstable [`floom::embedder`](docs/embedder.md) API. Ordinary applications should continue to use `floom::launch(...)`.
```

- [ ] **Step 3: Verify default dependency boundaries**

Run:

```bash
cargo tree -p floom --no-default-features
cargo tree -p floom
```

Expected: output contains existing desktop/web dependencies only and does not contain Android SDK, iOS SDK wrapper crates, HarmonyOS SDK crates, or TUI crates such as `ratatui` or `crossterm`.

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test -p floom-test --test embedder_protocol
cargo test -p floom-test --test embedder_headless
cargo test -p floom-test --test embedder_input
```

Expected: PASS.

- [ ] **Step 5: Run broader verification**

Run:

```bash
cargo check --workspace
cargo test -p floom-test
cargo bench --no-run
```

Expected: PASS. If `cargo bench --no-run` takes too long in CI or local review, run `cargo test --benches --no-run` and record that substitution in the commit message body.

- [ ] **Step 6: Commit**

```bash
git add README.md docs/embedder.md
git commit -m "docs: explain experimental embedder api"
```

---

### Task 10: Final Integration Review

**Files:**
- Modify only files required by failures found during this task.

**Interfaces:**
- Consumes: Tasks 1-9.
- Produces: verified branch with embedder protocol, winit translation layer, headless PoC, tests, and documentation.

- [ ] **Step 1: Inspect public API docs locally**

Run:

```bash
cargo doc -p floom --no-deps
```

Expected: PASS and `floom::embedder` appears in generated documentation.

- [ ] **Step 2: Run all required checks**

Run:

```bash
cargo fmt --check
cargo check --workspace
cargo test -p floom-test
cargo test --workspace --tests
cargo bench --no-run
```

Expected: PASS.

- [ ] **Step 3: Run examples compile check**

Run:

```bash
cargo check --examples
```

Expected: PASS for existing examples, including `counter`, `widget-gallery`, and `keyboard_handler`.

- [ ] **Step 4: Inspect hot-path allocations manually**

Run:

```bash
rg -n "Box<dyn|Vec<|collect\\(|to_vec\\(|clone\\(" crates/floom/src/embedder crates/floom/src/app/winit_embedder.rs crates/floom/src/window/handle.rs
```

Expected: Review each result. `VecCommandSink` and test helpers may allocate. Event translation should use `SmallVec` for the common path and must not allocate for ordinary keyboard, pointer, focus, resize, scale, theme, occlusion, or redraw events.

- [ ] **Step 5: Update documentation if public names changed during implementation**

Run:

```bash
rg -n "EmbedderEvent|EmbedderCommand|SurfaceDescriptor|ImeState|BackendCapabilities|EmbedderRuntime|HeadlessEmbedder" docs README.md
```

Expected: Every referenced public name exists in the final code. Fix docs if an implementation task intentionally changed a name.

- [ ] **Step 6: Commit final fixes if any files changed**

If Step 1-5 required fixes:

```bash
git add src test docs README.md
git commit -m "chore: finalize embedder api integration"
```

If no files changed, do not create an empty commit.

---

## Self-Review Notes

- Spec coverage: Tasks 1-2 define the public experimental protocol and adapter. Tasks 3-6 route events and commands. Task 4 proves headless. Task 5 moves winit translation. Task 7 covers capabilities and explicit degradation. Task 8 covers keyboard, touch, focus, and IME. Task 9 covers docs and dependency boundaries. Task 10 covers examples, docs, and performance checks.
- Placeholder scan: The plan contains no placeholder sections and each task has explicit files, interfaces, tests, commands, and expected outcomes.
- Type consistency: The central names are `EmbedderEvent`, `EmbedderCommand`, `CommandSink`, `VecCommandSink`, `EmbedderRuntimeState`, `HeadlessEmbedder`, `BackendCapabilities`, `SurfaceDescriptor`, and `ImeState`. Later tasks consume the names introduced in earlier tasks.
