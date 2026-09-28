// Index — the ArkTS host page for the Floem OHOS backend.
//
// It draws nothing. The page is a single full-window `XComponent` of type
// SURFACE; `libraryname: 'entry'` makes the framework load libentry.so, whose
// `Init` (entry/src/main/cpp/napi_init.cpp) unwraps the
// `OH_NATIVE_XCOMPONENT_OBJ` handed to it and registers the native callbacks:
//
//   OnSurfaceCreated / OnSurfaceChanged  → ohos_winit_runtime_surface_created
//   DispatchTouchEvent                   → ohos_winit_runtime_touch
//   OnFrame                              → ohos_winit_runtime_frame
//
// libentry.so in turn links libohos.so, the Rust cdylib whose
// `export_ohos_winit_app!` entry point owns the real (winit + wgpu) window and
// renders the floem app into this surface. All pixels come from Rust.
//
// The platform bridge (clipboard / file picker / external URL / a11y / IME) is
// registered separately by EntryAbility via `FloemBridge.ets`.

@Entry
@Component
struct Index {
  build() {
    Column() {
      XComponent({
        id: 'floem_surface',
        type: XComponentType.SURFACE,
        libraryname: 'entry',
      })
        .width('100%')
        .height('100%')
    }
    .width('100%')
    .height('100%')
  }
}