# floom-platform-embedded

The embedded platform backend: a window onto a framebuffer, and an event
queue.

An embedded target has no window manager and no GPU. It has a rectangle of
memory that the hardware scans out, and a way to receive input. This crate is
the platform half of that.

## What it does

- **A window** with a size, a title, a focus state and a surface.
- **A surface** that is memory, not a swap-chain: `Surface::target()` is
  `None`, which is how a GPU presenter knows to refuse it and a software
  presenter knows it may draw into it.
- **An event queue** the board pushes into — pointer motion, buttons, keys,
  resizes — and the app drains in order.
- **An exit**, so a board that has nothing left to show can stop the loop.

## How a frame gets on the screen

There is no swap-chain here. The software rasteriser owns a CPU buffer, and
copying that buffer into the framebuffer is the present:

```rust,ignore
let mut platform = EmbeddedPlatform::new();
platform.create_window(WindowConfig::default());

// ... the app draws a frame into the software presenter ...

let frame = runner.presenter().pixels();
if let Some(surface) = platform.framebuffer_mut() {
    surface.present(frame);
}
```

`floom::platform::embedded::run_frames` does exactly that, and the app
facade's `embedded` feature wires it into the backend cascade.

## Why it is a sibling crate

`floom-platform` is the OS-neutral ABI crate, and the architecture gate
holds it to "no concrete OS dependency". An embedded backend that owns a
framebuffer is not OS-neutral — it is a concrete backend, and it belongs
beside the desktop, terminal and web ones.

## Testing

```
cargo test -p floom-platform-embedded
```

The end-to-end test renders a scene through the software rasteriser and
asserts the pixels arrive in the framebuffer, at the right place, with the
right colour.
