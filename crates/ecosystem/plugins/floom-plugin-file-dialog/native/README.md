<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# Native halves

| Target | Arm | What it does |
|---|---|---|
| macOS | `src/macos.rs` | real: `NSOpenPanel` / `NSSavePanel` |
| Linux | `src/linux.rs` | real: `zenity`/`kdialog`, whichever is installed |
| Windows | `src/windows.rs` | real: `IFileOpenDialog` / `IFileSaveDialog` |
| Android / iOS | `src/lib.rs` | refuses: the answer arrives through an activity/delegate channel the platform layer does not expose |

This plugin keeps its platform code in Rust (`Shape A` in
[`docs/plugin-catalog.md` §8.1](../../../../../docs/plugin-catalog.md#81-新插件走哪套形状教义与代码已分叉)),
so `build.rs` compiles no native source and `native/common/` holds only the C
ABI contract — the shape a native half would implement if one is added. The
Rust arms are listed in the table above; the contract header is checked against
the crate name by `scripts/check-plugin-templates.sh`.

The generic layout is documented in
[`docs/platform-and-plugins.md`](../../../../../docs/platform-and-plugins.md#the-layout).
