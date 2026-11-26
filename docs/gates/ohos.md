<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Gate: OpenHarmony

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/ohos` (arm64-v8a HAP via `cargo ohos-app package`, DevEco SDK 26.0.0) on the DevEco emulator instance `Mate X7` (HarmonyOS 7.0.0): hilog `ffi: surface_created 2210x2416 density=3.125` then `first RedrawRequested dispatched` — no panic; the screenshot shows the complete counter UI (fonts included — `HarmonyOS_Sans*` registered from `/system/fonts` because OHOS has no fontconfig). Interaction, both directions, on the emulator tap: `+` → `Value: 0` becomes `Value: 1` (render #2–#6 dispatched), `-` → back to `Value: 0`. Took three root-cause fixes to get here: the XComponent touch callback's second parameter is a `NativeWindow` handle, not the event (must go through `OH_NativeXComponent_GetTouchEvent`); the ui-events reducer built touch `PointerState` with `scale_factor: 1.0` so hit-testing divided physical pixels by 1 instead of 3.125; and nothing ever calls `ohos_winit_runtime_frame`, so `frame_ready` gated all repainting after the first frame. **Re-verified 2026-09-27** on the same image, after the N-API single-module + cross-thread marshalling work (see Notes): `surface_created 2210x2416 density=3.125` → `dispatching can_create_surfaces` → `create_window` → `first RedrawRequested dispatched`, with `0` occurrences of `ecma_vm cannot run in multi-thread` / `Failed to register platform bridge` / `SIGABRT` and a stable pid. The tap leg (`+` / `-`) was not re-driven in that run. **Re-verified again 2026-09-27** with the HAP rebuilt after the native-lib packaging fix (Notes): same startup chain plus `[floom] a11y tree updated: {…}` ×3 logged from the JS thread (tid 7491 = pid), and the tap leg **was** re-driven this time — tapping `+` renders `Value: 1`, tapping `-` renders `Value: 0` (emulator screenshots), pid 7491 stable throughout, `0` crash markers.  **2026-11-27 — the 2.x path (`floom-platform-harmony` + `examples/floom-ohos`).** HAP `com.honorbox.reminder` (1.7 MB) installed on the DevEco emulator at `127.0.0.1:5557`: hilog shows `buffer w=2210 h=2208 stride_raw=8840 size=19518720` (stride in bytes, one frame exactly), `ink=40461 bbox=0x0-540x74` (a 173x24-point button at density 3.125), `presented 2210x2208 stride=8840 flush_rc=0`, and `host size=Size { width: 707.2, height: 706.56 } scale=3.125 nodes=3`. The screenshot shows the label and the button at the right size. Interaction: `uitest uiInput click 200 158` -> `touch points=1 x=200.000000 y=37.000000` -> `FloomHarmony: tapped`, i.e. hit-test and click handler both ran. This took one root cause, not three: **fontique has no OpenHarmony backend**, so `GenericFamily::SansSerif` resolved to nothing, text measured 0x0 and the button collapsed to its padding — registering `HarmonyOS_Sans` *and* calling `set_generic_families` is what fixed it. Touch reaches Rust through `DispatchTouchEvent` -> `floom_harmony_touch(x, y, phase)` (component pixels) -> queue -> drained before each pump as `PointerMoved`/`PointerButton` with the coordinates divided by the display scale. |
| 2 | IME Latin + one CJK | device-gated: no IME bridge on the 2.x path | Touch and focus routing work (item 1), but nothing forwards text composition: the ArkTS shell owns the keyboard and there is no `floom_harmony_ime_*` entry point yet. |
| 3 | a11y label + Default action | device-gated: not exercised on emulator | accessibility tree retained in-process; OHOS native bridge not yet wired |
| 4 | Clipboard read/write | device-gated: not exercised on emulator | clipboard adapter pending |
| 5 | File pick / share-open as applicable | device-gated: not exercised on emulator | file picker UI/bridge pending |
| 6 | Safe area insets | device-gated: not exercised on emulator | `WindowHandle::safe_area()` public accessor pending |
| 7 | Focus + basic keyboard nav | device-gated: not exercised on emulator | focus indicator/ring not yet painted in root `floom` example |
| 8 | CI/target compile | pass | The `ohos-check` job also builds `--package=floom-ohos`, so the example link (harmony + widgets + text + `native_window`) is covered, not only the backend crate. | Both OHOS targets build: `cargo build -p ohos --target aarch64-unknown-linux-ohos` → `libohos.so` (ARM64 ELF), and `--target x86_64-unknown-linux-ohos` → x86-64 ELF; `cargo check` for `floom-platform-harmony` clean on both. The arm64-v8a HAP from item 1 was packaged by `cargo ohos-app package` (hvigor chain) with the same source tree. |

## Sign-off

- Spot-check date: 2026-09-24
- Re-verified: 2026-09-27 (item 1; same image), and again 2026-09-27 (item 1 incl. the tap leg, HAP rebuilt after the native-lib packaging fix)
- Re-verified: 2026-11-27 (item 1 on the 2.x path: render at the right size + tap reaching a click handler; see the dated paragraph in item 1)
- Environment (device/simulator): DevEco Studio emulator instance `Mate X7` (HarmonyOS 7.0.0 / API 26 image), on macOS / Apple M4 Pro. HAP: `com.example.ohos`, arm64-v8a, packaged with `cargo ohos-app package --target arm64-v8a --sdk-version 26.0.0`; deployed over `hdc` to `127.0.0.1:5557`. (The 2026-09-27 re-verification rebuilt the HAP with `hvigorw assembleHap` directly and deployed to `127.0.0.1:5555`; the emulator is started with `Emulator -start "Mate X7" -instancePath ~/.Huawei/Emulator/deployed -imageRoot ~/Library/Huawei/EmulatorImages -bootMode coldboot`.)
- Reviewer: automated spot-check — every line above is verbatim hilog / screenshot from the run, not a reading of the source.

## Notes

- Items 1 and 8 are recorded with evidence rather than left blank: "it worked last
  session" is not a gate result.
- The tap coordinates were found by pixel-scanning the screenshot (sips → PNG →
  zlib), not by guessing from the layout: the `+` button's physical bbox is
  x 929–996 / y ~1055–1153, center (962, 1104). Same lesson as the Android run:
  log-derived coordinates are not click coordinates.
- Rendering itself needed its own fix chain earlier in the day: DGLES segfaults on
  EGL debug registration (drop `InstanceFlags::VALIDATION|DEBUG` on OHOS),
  desktop-limit device requests fail on the GLES adapter (`required_limits:
  adapter.limits()`), vger needs `VERTEX_STORAGE` that GLES 3.0 lacks and
  softbuffer has no OHOS backend — so tiny-skia presents through a wgpu GL
  surface blit.
- **2026-09-27 — N-API module wiring.** napi-rs 2.x never calls
  `napi_module_register` on OHOS, so the Rust cdylib was not an N-API module:
  `import … from 'libohos.so'` resolved to the wrong module and every floom
  export came back `undefined` (`register_platform_bridge is not callable`).
  A HAP links one N-API module, so `entry/src/main/cpp/napi_init.cpp`'s `Init`
  now ends with `napi_register_module_v1(env, exports)` — libohos.so fills in
  the very `exports` object `libentry.so` registers — and ArkTS imports floom
  from `'libentry.so'`. napi-rs also renames Rust `snake_case` exports to
  `camelCase`, so `FloomBridge.ets` calls `registerPlatformBridge` /
  `onFilePicked` / `onSharedFiles` / `onAccessibilityAction` / `onIme*`.
- **2026-09-27 — the HAP must contain `libohos.so`, and a green build does not
  prove it.** `libentry.so` carries `DT_NEEDED libohos.so` (CMake links the Rust
  runtime as a shared library), and hvigor only packs CMake outputs plus
  prebuilt `.so` under `entry/libs/<abi>/`. Removing the stale duplicate there
  (`ProcessLibs … Duplicated files found`) also removed the *only* packaged copy,
  so `clean` + `assembleHap` succeeded and produced a **1.4 MB** HAP with no
  native runtime — it would install and then fail at load, with no rendering.
  The correct state is a ≈ 43.9 MB HAP containing
  `libs/arm64-v8a/libohos.so` (42.6 MB stripped) **and** no duplicate from a
  second, differently-built copy. Check with
  `unzip -l entry/build/default/outputs/default/entry-default-unsigned.hap`.
- **2026-09-27 — cross-thread marshalling.** Registering the bridge exposed the
  next failure: once a bridge call actually ran, the process aborted with
  `Fatal: ecma_vm cannot run in multi-thread! thread:11644 currentThread:12000`
  (the a11y push, called from the event-loop worker). N-API values belong to the
  JS thread, and floom's OHOS runtime pumps the event loop on a worker. The fix
  is in `crates/floom/src/platform/ohos_bridge.rs`: a thread-safe function is created at
  registration, and `call_bridge` runs inline when already on the JS thread or
  otherwise posts the closure and waits on a `Condvar`, degrading to `None` after
  5 s. Positive evidence that the round trip really crosses threads — not a
  silent timeout — is the ArkTS-side log emitted on the JS thread:
  `A03D00/JSAPP: [floom] a11y tree updated: {"root":…,"nodes":[{"id":…,"role":"button","label":"+"…}]}`
  (3×, tid 29653 = pid).
