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
| [`crates/ecosystem/floom-editor`](../ecosystem/floom-editor) | Code/text editor views + `floom-editor-core` | `floom` public API (path) |
| [`crates/ecosystem/flameo`](../ecosystem/flameo) | Demo application exercising the floom-ui kit | `floom` + the floom-ui crates |

`floom-ui` was spelled **`flome-ui`** (a typo) until it moved in here.

> **Nested-workspace caveat.** A `[patch]` only applies to the workspace that
> declares it, so `floom-editor` and `flameo` re-declare the lapce/winit and
> lapce/floom patches that the root `Cargo.toml` carries. Without that, a path
> dependency on `floom` would resolve `floem-winit-core` / `dpi` from git while
> the rest of the graph uses the vendored copies — two instances of one type.
>
> Two packages are named `floom-editor`: the 2.0 crate at `crates/widgets/floom-editor`
> and the satellite at `crates/ecosystem/floom-editor/floom-editor`. They never share a
> resolve graph as *members* (the satellite is excluded), but
> `examples/{custom_keymap,widget-gallery}` pull the satellite in by path, so
> the root graph does contain both. `scripts/check-architecture-deps.sh`
> resolves nodes among `workspace_members` first for exactly this reason.

## Compatibility matrix

| floom | floom-ui | floom-editor | Notes |
|-------|----------|--------------|-------|
| 0.2.x (pre-extract) | path/git | in-tree `editor` feature (removed) | historical |
| 0.2.x (industrial-1.0) | path/git | separate `floom-editor` repo | editor extracted |
| 1.0.0 | (fill on release) | (fill on release) | |
| monorepo (2026-09-18) | [`crates/ecosystem/floom-ui`](../ecosystem/floom-ui) | [`crates/ecosystem/floom-editor`](../ecosystem/floom-editor) | both imported into this repo |

Update this table when tagging floom or publishing satellites.
