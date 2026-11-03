# floom-cli

Command line tools for Floom. Ships the `floom` binary.

```sh
floom create myapp
```

`floom create` scaffolds a Flutter-style project: a Rust core under `src/` plus
one directory per target platform (`android`, `ios`, `ohos`, `web`, `windows`,
`linux`, `macos`, `tui`).

```
myapp/
├── src/          Rust core — all cross-platform application logic
├── Cargo.toml
├── tests/
├── android/  ios/  ohos/  web/  windows/  linux/  macos/  tui/
└── README.md
```

The split is deliberate: logic lives in `src/` so it stays portable, and each
platform directory only holds the native host project that embeds the compiled
binary. Plugin capabilities (camera, sensors, ...) reach the OS through Kotlin /
Java / Swift host libraries called from Rust over a narrow `extern "C"` ABI — see
[`docs/platform-and-plugins.md`](../../docs/platform-and-plugins.md).

## Options

| Option | Meaning |
|---|---|
| `--platforms <list>` | Comma-separated subset of the platforms above (default: all) |
| `--org <reverse-dns>` | Organisation for the native projects (default `com.example`) |
| `--floom-crate <name>` | Crate to depend on (default `floom`) |
| `--floom-path <path>` | Depend on a local checkout instead of a published version |
| `--force` | Overwrite a non-empty target directory |

## Status: stage 1 of 2

This release emits the Rust core, the platform directory skeleton, and the files
that can be correct without a native SDK present (`web/index.html`). The native
project files — Gradle, Xcode, DevEco, CMake — are deliberately **not** emitted
yet. A hand-generated `.pbxproj` or `build.gradle.kts` that does not open is
worse than none, so each platform directory currently carries a README naming the
tool that will open it and what stage 2 will place there.

The crate is intentionally dependency-free: the CLI is installed on machines
that are only about to run `floom create`, so it must build from `std` alone.