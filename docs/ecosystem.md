<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Floom ecosystem

Official satellites (not part of the floom 1.0 core promise).

They live **inside this monorepo** under `crates/ecosystem/`, each as a nested
workspace. The directory is listed in the root `Cargo.toml`'s `exclude`, so the
core `cargo test --workspace` stays focused on the 2.0 crates; build a satellite
by running cargo from its own directory.

| Satellite | Role | Depends on |
|-----------|------|------------|
| [`crates/ecosystem/floom-ui`](../ecosystem/floom-ui) | Material / Cupertino / Fluent kit (`floom-elements`), ~350 crates | `floom` public API (path, `[patch]`ed to this checkout) |
| [`crates/ecosystem/floom-editor`](../ecosystem/floom-editor) | Code/text editor views; `floom_editor::core` model + `TextEditor` API | `floom` public API (path) |
| [`crates/ecosystem/flameo`](../ecosystem/flameo) | Demo application exercising the floom-ui kit | `floom` + the floom-ui crates |

`floom-ui` was spelled **`flome-ui`** (a typo) until it moved in here.

> **Nested-workspace caveat.** A `[patch]` only applies to the workspace that
> declares it, so `floom-editor` and `flameo` re-declare the lapce/winit and
> lapce/floom patches that the root `Cargo.toml` carries. Without that, a path
> dependency on `floom` would resolve `floem-winit-core` / `dpi` from git while
> the rest of the graph uses the vendored copies — two instances of one type.
>
> There is now **one** `floom-editor` package: the satellite at
> `crates/ecosystem/floom-editor/floom-editor`. It owns the editor model as
> `floom_editor::core` and exposes the `text_editor()` / `TextEditor` view API,
> so no second same-named package shares the root resolve graph. The showcase
> pulls this one crate in by path.

## Compatibility matrix

| floom | floom-ui | floom-editor | Notes |
|-------|----------|--------------|-------|
| 0.2.x (pre-extract) | path/git | in-tree `editor` feature (removed) | historical |
| 0.2.x (industrial-1.0) | path/git | separate `floom-editor` repo | editor extracted |
| 1.0.0 | (fill on release) | (fill on release) | |
| monorepo (2026-09-18) | [`crates/ecosystem/floom-ui`](../ecosystem/floom-ui) | [`crates/ecosystem/floom-editor`](../ecosystem/floom-editor) | both imported into this repo |

Update this table when tagging floom or publishing satellites.
