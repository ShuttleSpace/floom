# floom-editor

Official **satellite** library for Floom code and text editor views. One crate, `floom-editor`, provides the `text_editor()` / `TextEditor` API and its framework-free model under `floom_editor::core`.

It lives in the floom monorepo under [`crates/ecosystem/`](..), as a **nested workspace**:
it is listed in the root `Cargo.toml`'s `exclude`, so `cargo test --workspace` at the
repo root does not build it, and it is built by `cd crates/ecosystem/floom-editor && cargo …`.

This project is **not** part of the floom 1.0 core promise. It depends on **public** floom APIs only. See the compatibility matrix in [floom `docs/roadmap.md`](../../docs/roadmap.md).

```toml
# From anywhere inside the monorepo (path is relative to your own manifest):
floom-editor = { path = "crates/ecosystem/floom-editor/floom-editor" }
```

```rust
use floom_editor::{EditorStyle, SimpleStyling, text_editor};
use std::sync::Arc;

let editor = text_editor("hello\nworld").style(EditorStyle::dark());
```

The editor is a Floom 2.0 `Widget`: it is mounted into a 2.0 host like any
other, and it paints the document, the caret, the selection and the gutter
itself. It soft-wraps long lines, it takes more than one caret, and it can
draw the whitespace and the indent guides. The port's scope — and the parts
of the v1 editor it does *not* reproduce — are described in the crate's own
module note.

The *model* — the rope buffer, the cursor, the selection, the word motions —
lives in this same crate as `floom_editor::core` and is framework-free. There
is no separate core package to depend on.

## The examples

Three real windows, all run from this workspace's own directory (the satellite
is excluded from the root workspace, so `cargo run -p ...` has to be started
here):

```sh
cd crates/ecosystem/floom-editor

cargo run -p editor           # the editor filling a window, plus a button
                              # that clears the document from outside it
cargo run -p syntax-editor    # the same editor with a hand-written Rust
                              # scanner behind the `Styling` trait
cargo run -p ide              # an IDE: explorer, tabs, status bar
```

`ide` is the one that shows the editor the way an application uses it. A tab is
not a widget — the editor is mounted once and a click swaps the *document*
behind it, so every file keeps its own undo history, caret, selection and scroll
position. The chrome around it (explorer, tabs, status bar) is three leaf
widgets that read the shared model while they paint, which is what lets the
status bar follow the caret in the same frame.

`cargo run -p ide -- --render out.ppm` writes one frame as a PPM through the
software renderer instead of opening a window — the same scene, rasterised with
no display, which is how the demo's pixels are checked on a headless machine.
