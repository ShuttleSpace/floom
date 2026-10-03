# floem-editor

Official **satellite** library for Floem code and text editor views (`floem-editor-core` + `floem-editor`).

It lives in the floem monorepo under [`ecosystem/`](..), as a **nested workspace**:
it is listed in the root `Cargo.toml`'s `exclude`, so `cargo test --workspace` at the
repo root does not build it, and it is built by `cd ecosystem/floem-editor && cargo …`.

This project is **not** part of the floem 1.0 core promise. It depends on **public** floem APIs only. See the compatibility matrix in [floem `docs/ecosystem.md`](../../docs/ecosystem.md).

```toml
# From anywhere inside the monorepo (path is relative to your own manifest):
floem-editor = { path = "ecosystem/floem-editor/floem-editor" }
```

```rust
use floem_editor::{editor::text::SimpleStyling, text_editor};
```
