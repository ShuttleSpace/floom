<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Gate: Linux

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | device-gated: desktop native-view bridge Phase 16.2 incomplete; no Linux machine in CI; verified headlessly on macOS deputy | CI matrix (`ci-core.yml`) runs Linux builds but not the full example suite |
| 2 | IME Latin + one CJK | device-gated: X11 IME composition bridge incomplete; verified headlessly on macOS deputy (`crates/floom/src/event/dispatch.rs:1197`) | `crates/floom-platform-desktop/src/ime/` |
| 3 | a11y label + Default action | device-gated: no AT-SPI bridge on Linux yet; semantics tree retained in-process | `crates/floom-accessibility/` |
| 4 | Clipboard read/write | device-gated: cross-platform clipboard trait stubbed; X11 `XConvertSelection`/`GTK` adapter not yet wired | `crates/floom-platform-desktop/src/clipboard.rs` |
| 5 | File pick / share-open as applicable | device-gated: no dialog/file-picker trait exists yet — true on every desktop target, not just Linux | v1 `crates/floom/src/platform/*` bridges not carried over |
| 6 | Safe area insets | n/a | desktop |
| 7 | Focus + basic keyboard nav | device-gated: no Linux focus bridge; keyboard dispatch verified headlessly on macOS deputy | `crates/floom/src/event/dispatch.rs` |
| 8 | CI/target compile | device-gated: `ubuntu-latest` build runner covers `cargo build`; `cargo test` on Linux requires native-view completion | `.github/workflows/ci-core.yml` |

## Sign-off

- Spot-check date:
- Environment (device/simulator):
- Reviewer:
