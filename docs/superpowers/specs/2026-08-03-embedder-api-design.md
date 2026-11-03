# Floom Experimental Embedder API Design

Date: 2026-08-03

## Summary

Floom will add an experimental public `floom::embedder` API so platform backends can integrate Floom without treating `winit` desktop windows as the only runtime model. The first phase prioritizes the public embedder contract, keeps `floom::launch(...)` working through the default winit backend, and adds a headless/embedder proof of concept for testing the contract without a real platform window.

The design uses a hybrid boundary:

- A protocol layer built from data-oriented `EmbedderEvent`, `EmbedderCommand`, capability, surface, and IME types.
- A Rust adapter layer with traits for ergonomic native Rust backends.

This supports later Android, iOS, TUI, and HarmonyOS backends while preserving Floom's current strengths: fine-grained reactivity, high performance, small default builds, and an out-of-the-box desktop experience.

## Goals

- Add `floom::embedder` as an experimental public module.
- Make winit the first official implementation of the embedder contract.
- Add a headless/embedder proof of concept that drives Floom through the same protocol.
- Keep the default launch path simple and available.
- Cover common cross-platform framework pain points in the API shape: lifecycle, surface loss, touch, keyboard, IME, focus, DPI, safe areas, platform services, accessibility hooks, timers, and redraw scheduling.
- Avoid adding mobile, TUI, or HarmonyOS SDK dependencies to default builds.
- Allow small breaking changes to semi-public internals when they make the embedder boundary cleaner and examples remain easy to update.

## Non-Goals

- Do not fully implement Android, iOS, TUI, or HarmonyOS backends in this phase.
- Do not rewrite `floom_renderer` or replace the existing Vello, Vger, Skia, and tiny-skia renderer strategy.
- Do not freeze the embedder API as stable. The module is public so backend authors can test it, but it remains experimental.
- Do not build a full high-level gesture recognition system in this phase.
- Do not make accessibility, safe areas, virtual keyboard, or URL opening complete on every platform in this phase.

## Architecture

The new `floom::embedder` module owns the platform contract. It is split into protocol types and Rust adapter traits.

The protocol layer contains neutral data structures that do not depend on winit:

- `EmbedderEvent`: events sent from a backend into Floom.
- `EmbedderCommand`: commands emitted by Floom for the backend to execute.
- `BackendCapabilities`: capability flags reported by a backend at startup or when a window is created.
- `SurfaceDescriptor`: the drawing target and its scale, size, transparency, and surface kind.
- `ImeState`: text input requirements Floom sends to the platform.
- `WindowDescriptor`: requested window title, size constraints, visibility, transparency, decoration, theme preference, and platform-neutral lifecycle configuration.

The adapter layer makes native Rust backend implementation ergonomic:

- `EmbedderRuntime`: event loop, wakeups, timers, and frame pacing.
- `EmbedderWindow`: window identity, lifecycle, scale, size, title, visibility, and focus integration.
- `SurfaceHandle`: surface creation, resize, restoration, and presentation state.
- `PlatformServices`: clipboard, cursor, menus, file dialogs, drag/drop, theme, safe area, virtual keyboard, URL opening, and accessibility hooks.

The default desktop path remains:

```text
floom::launch(view)
  -> default winit embedder
  -> floom runtime
  -> view/reactive/style/layout/paint/event dispatch
```

Custom backends can either implement the Rust adapter traits or feed protocol events directly and execute returned commands.

## Event And Input Model

Backends send platform input to Floom as `EmbedderEvent`. Floom converts those events into its existing event dispatch, path, focus, style, and reactive update systems. The current `ui-events-winit` crate remains useful as the winit translation layer, but the core runtime no longer treats winit events as the only input source.

Pointer input covers:

- Mouse, touch, and pen pointer types.
- Stable pointer ids.
- Logical position, button state, pressure, tilt, phase, and cancellation.
- Explicit pointer capture for mouse and pen.
- Implicit touch capture following pointer-event expectations.

Keyboard input covers:

- Logical key.
- Physical code.
- Modifiers.
- Key location.
- Repeat.
- Down/up state.
- Synthetic event marker.
- Global shortcut broadcast phase.

Focus is split into three concepts:

- Platform window focus reported by the backend.
- Floom view focus owned by Floom.
- Keyboard traversal focus owned by Floom focus policy.

The backend reports only platform focus changes. Floom continues to own view-level focus, tab traversal, and focus styling.

Raw platform gestures such as pinch, pan, rotate, and double tap may be represented as optional raw gesture events. First-phase Floom does not commit to a full cross-platform high-level gesture recognizer.

## IME And Text Input

IME support is part of the first-phase contract but not a full platform implementation project.

Backends can send:

- `PreeditChanged`.
- `CommitText`.
- `CompositionEnd`.
- `ImeEnabledChanged`.

Floom can emit a `SetImeState` command containing:

- Enabled or disabled state.
- Candidate rectangle in window coordinates.
- Input purpose and hint.
- Secure text input requirement.
- Single-line or multiline mode.
- Optional surrounding text summary.

winit maps its existing IME capabilities into this contract. The headless backend tests event flow and command capture. Mobile and HarmonyOS backends can later map soft keyboard, candidate UI, secure input, and CJK composition behavior into the same protocol.

## Rendering, Surface, And Frame Scheduling

The existing renderer abstraction remains in place. The embedder API focuses on surface lifecycle and frame scheduling.

`SurfaceDescriptor` describes:

- Surface kind, such as GPU surface, software surface, cell surface, external texture, or no surface.
- Raw display/window handles when available.
- Pixel size.
- Logical size.
- Scale factor.
- Transparency.
- Color space and present-mode capabilities where available.

Surface events cover:

- Created.
- Resized.
- Lost.
- Restored.
- Occluded.
- Destroyed.

Floom requests rendering by emitting `EmbedderCommand::RequestRedraw { window_id }`. The backend decides when that request becomes `EmbedderEvent::RedrawRequested`.

Frame scheduling distinguishes:

- Normal update work.
- Animation frame work.
- Live resize frame work.

Floom keeps the existing bounded update idea, including the current 4 ms update budget, so reactive, style, and layout work cannot monopolize a frame indefinitely.

The headless backend has `SurfaceKind::None`; it drives events, updates, layout, focus, IME flow, and command capture without painting.

## Platform Services And Capabilities

Backends report `BackendCapabilities`. Floom uses those capabilities to decide whether a feature is available, downgraded, or a no-op.

The first-phase API covers:

- Clipboard.
- Cursor.
- Timers and wakeups.
- File open/save dialogs.
- Context menu and menu hooks.
- Window title and visibility.
- Accessibility tree updates and accessibility actions.
- System theme.
- DPI and display information.
- Safe area and insets.
- Virtual keyboard.
- URL or external-open requests.
- Drag/drop.
- IME state and candidate rect.
- Secure text input.
- Keyboard type and purpose.
- GPU surface.
- Software surface.
- Surface loss and restoration.
- Headless event injection and command capture.

First-phase implementation depth:

- winit maps existing window, cursor, clipboard, menu/context menu, file dialog, drag/drop, theme, scale, surface lifecycle, keyboard, pointer, and IME behavior.
- headless implements no-op services plus command capture.
- accessibility, safe area, virtual keyboard, and URL opening are defined in protocol and capabilities. Unsupported functionality must degrade explicitly instead of failing implicitly.
- mobile, TUI, and HarmonyOS dependencies stay out of default features.

## Migration

Implementation should proceed in small reviewable steps:

1. Add `floom::embedder` protocol types and mark the module experimental.
2. Add Rust adapter traits over the protocol.
3. Introduce a runtime boundary that accepts embedder events and emits embedder commands.
4. Move winit window-event translation into the winit embedder implementation.
5. Update `Application` and `WindowHandle` so they depend on Floom-level embedder events and commands where practical.
6. Keep `floom::launch(...)` wired to the default winit embedder.
7. Add the headless/embedder proof of concept.
8. Update examples only where semi-public API movement requires it.
9. Add documentation for the experimental module and minimal backend shape.

This phase may move or rename small semi-public internals, but it should not force broad changes on ordinary Floom users.

## Testing

Required coverage:

- Existing examples compile, especially `counter`, `widget-gallery`, and `keyboard_handler`.
- Pointer capture behavior is covered.
- Touch implicit capture is covered.
- Keyboard modifiers, repeat, logical key, and physical code are covered.
- Window focus and view focus separation is covered.
- IME preedit and commit event flow is covered.
- `SetImeState` command capture is covered in headless.
- Headless can drive update, layout, event dispatch, focus, and command capture without a winit window.
- Surface lost and restored behavior remains covered for the winit backend.
- Existing benches still run, especially event dispatch, style, layout, painting, and text.

Performance checks should watch for avoidable allocation and dynamic dispatch in hot paths. The protocol boundary should be cheap enough that event dispatch, style, and layout benchmarks do not show obvious regressions.

## Acceptance Criteria

- `floom::embedder` exists as an experimental public module.
- The default winit launch path remains available and easy to use.
- winit is represented as an embedder backend rather than being the only runtime model in core code.
- A headless/embedder proof of concept drives Floom through the same contract.
- The API shape covers future mobile, TUI, and HarmonyOS needs without adding their dependencies.
- Existing major examples compile after any small required updates.
- Keyboard, touch, focus, and IME edge cases have focused tests.
- Surface loss and restoration behavior does not regress.
- Default features do not grow mobile, TUI, or HarmonyOS dependencies.
- Documentation explains the experimental status, capability model, protocol layer, adapter layer, and minimal backend flow.
