# vendor

Forked / vendored third-party trees used by Floom (lore-style `crates/vendor/`
directory). Each tree notes its upstream source and local delta in a `NOTICE.md`.
Not a public Rust API.

| Tree | Upstream | Local delta | Exit criteria |
|------|----------|-------------|---------------|
| `floem-winit/` | lapce/winit @ 133268de | iOS `set_on_open_url` / share cold-start | Upstream merged; iOS share gate `pass`; then delete tree and `[patch]` |
| `floom-winit-ohos/` | tgui-winit-ohos 0.4.11 | `floem_winit_core` retarget + `!Send` on RuntimeApplication; tree and package renamed off the `tgui` namespace | Upstream supports floem-winit-core; OHOS example gate `pass` |
| `harmony-app-rs/` | taoyuan11/harmony-app-rs | macOS/Linux hvigor wrappers | Upstream accepts patch; keep only docs link |

Rules:
- No Floom widgets or reactive code here.
- Pin `dpi` / winit-core identity as documented in each tree README.
- CHANGELOG: label commits under **vendor**.

## Do not rename the `floem-winit*` packages

`floem-winit/` is a fork of `lapce/winit`, whose crates are *actually published*
under the `floem-` prefix (`floem-winit`, `floem-winit-core`, `floem-winit-common`,
…). Those names are load-bearing:

- the git dependency declares `package = "floem-winit"`, and
- `[patch."https://github.com/lapce/winit"]` is keyed on `floem-winit` /
  `floem-winit-core`.

A `[patch]` key must match the name of the package it replaces, so renaming the
vendored packages would not fail loudly — the patch would simply stop applying
and the git versions would be fetched instead, reintroducing the duplicate
`floem-winit-core` / `dpi` instances documented above. Only the OHOS fork
(`floem-winit-ohos`) was ours alone to rename; it is now `floom-winit-ohos`.
