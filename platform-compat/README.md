# platform-compat

Vendor windowing/packaging trees used by Floem. Not a public Rust API.

| Tree | Upstream | Local delta | Exit criteria |
|------|----------|-------------|---------------|
| `floem-winit/` | lapce/winit @ 133268de | iOS `set_on_open_url` / share cold-start | Upstream merged; iOS share gate `pass`; then delete tree and `[patch]` |
| `tgui-winit-ohos/` | tgui-winit-ohos 0.4.x | `floem_winit_core` retarget + `!Send` on RuntimeApplication | Upstream supports floem-winit-core; OHOS example gate `pass` |
| `harmony-app-rs/` | taoyuan11/harmony-app-rs | macOS/Linux hvigor wrappers | Upstream accepts patch; keep only docs link |

Rules:
- No Floem widgets or reactive code here.
- Pin `dpi` / winit-core identity as documented in each tree README.
- CHANGELOG: label commits under **platform-compat**.
