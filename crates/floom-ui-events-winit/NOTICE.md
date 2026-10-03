# NOTICE — `floem-ui-events-winit` is a fork of `ui-events-winit`

## Upstream

- **Project:** ui-events — <https://github.com/endoli/ui-events>
- **Crate:** `ui-events-winit` 0.1.0 (published 2025-05-08), MSRV 1.73
- **Licence:** Apache-2.0 OR MIT. The upstream `LICENSE-APACHE` and `LICENSE-MIT`
  files are kept verbatim in this directory.

This crate is the `winit` → `ui-events` input bridge: it turns `winit`'s native
mouse/touch/keyboard events into the `ui-events` model (see `README.md`, which
upstream generates from `lib.rs` via cargo-rdme).

## Why a fork

floem pins `winit` to the lapce fork revision it is built against (see
`vendor/floem-winit/NOTICE.md`). Depending on the *published* `ui-events-winit`
would pull a second, differently-revisioned `winit` into the dependency graph,
so the bridge is carried in-tree against that pin.

## Local delta

- **Renamed** `ui-events-winit` → `floem-ui-events-winit`, so the workspace keeps
  a single `floem-` namespace (see `docs/workspace-conventions.md` §1).
- **Metadata** inherits `[workspace.package]` (`version` / `license` / `edition`
  / `repository`) rather than upstream's own values.
- **No behavioural change** to the bridge logic.

Unlike the two forks in `vendor/`, this one remains a **workspace member** (it
must compile against the workspace-wide `winit` pin, so it is not `exclude`d) —
but the attribution rule is the same, hence this file.

## Exit criterion

Delete the fork and depend on the published `ui-events-winit` once it targets a
`winit` revision floem can use directly — i.e. once `vendor/floem-winit` is gone
too.
