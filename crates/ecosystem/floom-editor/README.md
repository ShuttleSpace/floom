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
use floom_editor::{editor::text::SimpleStyling, text_editor};
```
