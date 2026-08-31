# Experimental Embedder API

`floem::embedder` is an experimental API for integrating Floem with platform
backends other than the default winit desktop backend.

The API has two layers:

- Protocol types such as `EmbedderEvent`, `EmbedderCommand`, `SurfaceDescriptor`,
  `ImeState`, and `BackendCapabilities`.
- Rust adapter traits such as `EmbedderRuntime`, `EmbedderWindow`,
  `SurfaceHandle`, and `PlatformServices`.

Ordinary application code should continue to use `floem::launch(...)`.

Backend authors can either implement the Rust adapter traits or feed
`EmbedderEvent` values into Floem and execute returned `EmbedderCommand`
values.

## Why a neutral protocol

The default desktop path (`floem::launch`) routes its winit window events
through `floem::app::winit_embedder::translate_window_event`, which produces
`EmbedderEvent` values. Those same neutral events are what every other backend
(Android, iOS, HarmonyOS, a headless test backend, a future TUI backend) must
produce. Keeping one event model means input handling, focus navigation, IME,
and layout behave identically across backends, and the core only needs to
understand the protocol — never a specific windowing toolkit.

## Minimal Flow

1. Report `BackendCapabilities`.
2. Create or attach a surface and send `EmbedderEvent::Surface`.
3. Send input as `EmbedderEvent::Pointer`, `EmbedderEvent::Keyboard`,
   `EmbedderEvent::Ime`, and `EmbedderEvent::Focus`.
4. Execute commands such as `RequestRedraw`, `SetCursor`, and `SetImeState`.
5. Send `EmbedderEvent::RedrawRequested` when the platform is ready for a frame.

## Current Implementations

- **winit desktop backend**: the default path used by `floem::launch(...)`.
  Translation lives in `src/app/winit_embedder.rs`.
- **headless backend**: a testing proof of concept (`HeadlessEmbedder`) with no
  real platform window or paint surface. It drives the real Floem runtime from
  neutral events so input, focus, IME, and layout can be unit tested without a
  display.

## Capabilities and graceful degradation

`BackendCapabilities` lets a backend declare what it supports. Unsupported
functionality must degrade explicitly (return an error or become a no-op)
rather than failing implicitly. For example, the headless backend advertises
only `headless = true` and returns errors from clipboard services, while the
winit backend advertises the full desktop feature set. Query capabilities at
startup and when a window is created to decide whether a feature is available,
downgraded, or a no-op.

## Stability

The module is public for experimentation, but it is **not stable**. Type names,
enum variants, and trait methods may change while Android, iOS, TUI, and
HarmonyOS backends are explored. Pin a specific Floem version if you build
against it.
