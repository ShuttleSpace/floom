# floom-editor

Official **satellite** library for Floom code and text editor views (`floom-editor-core` + `floom-editor`).

It lives in the floom monorepo under [`crates/ecosystem/`](..), as a **nested workspace**:
it is listed in the root `Cargo.toml`'s `exclude`, so `cargo test --workspace` at the
repo root does not build it, and it is built by `cd crates/ecosystem/floom-editor && cargo …`.

This project is **not** part of the floom 1.0 core promise. It depends on **public** floom APIs only. See the compatibility matrix in [floom `docs/ecosystem.md`](../../docs/ecosystem.md).

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
of the v1 editor it does *not* reproduce — are in
[`docs/porting-floem-editor-to-2.0.md`](../../../docs/porting-floem-editor-to-2.0.md)
and in the crate's own module note.

The *model* — the rope buffer, the cursor, the selection, the word motions —
is `floom-editor-core`, which now lives in the root workspace at
`crates/floom-editor-core` and is framework-free.
