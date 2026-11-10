<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Gate: Windows

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | device-gated: desktop native-view bridge Phase 16.2 incomplete; no Windows machine in CI; verified headless on Linux+macOS deputy | CI has no Windows runner for the full test suite (`build` job covers `cargo build` on `windows-latest`) |
| 2 | IME Latin + one CJK | device-gated: winit IME layer + composition bridge incomplete on Win32; verified headlessly on Linux/macOS deputy | `crates/floom-platform-desktop/src/ime/` |
| 3 | a11y label + Default action | device-gated: no `NSAccessibility`/`UIA` bridge on Windows yet; semantics tree retained in-process | `crates/floom-accessibility/` |
| 4 | Clipboard read/write | device-gated: cross-platform clipboard trait stubbed; Windows `OpenClipboard`/`GetClipboardData` adapter not yet wired | `crates/floom-platform-desktop/src/clipboard.rs` |
| 5 | File pick / share-open as applicable | device-gated: no dialog/file-picker trait exists yet — true on every desktop target, not just Windows | v1 `crates/floom/src/platform/*` bridges not carried over |
| 6 | Safe area insets | n/a | desktop |
| 7 | Focus + basic keyboard nav | device-gated: no Windows IME/focus bridge; keyboard dispatch path verified headlessly on macOS/Linux | `crates/floom/src/event/dispatch.rs` |
| 8 | CI/target compile | device-gated: `windows-latest` build runner covers `cargo build`; `cargo test` on Windows requires native-view completion | `.github/workflows/ci-core.yml` |

## Sign-off

- Spot-check date:
- Environment (device/simulator):
- Reviewer:
