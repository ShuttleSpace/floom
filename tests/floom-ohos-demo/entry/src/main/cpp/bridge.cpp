// SPDX-License-Identifier: MIT
// The HarmonyOS host half of the Floom demo: the `ohos-rs`/Flutter seam.
//
// An ArkUI `XComponent` (type=surface) owns the pixels. This N-API module grabs
// its `OH_NativeXComponent`, waits for `OnSurfaceCreated`, and hands the
// `OHNativeWindow*` to `libfloom_platform_harmony.so`, which blits an RGBA8
// frame into the window's buffer queue. A `OH_NativeVSync` loop drives frames.
//
// This mirrors Flutter's `ohos_xcomponent_adapter.cpp` exactly; the only Floom
// code is the three `floom_harmony_*` calls.

#include <cstdint>
#include <cstring>
#include <vector>

#include <napi/native_api.h>
#include <ace/xcomponent/native_interface_xcomponent.h>
#include <native_window/external_window.h>
#include <native_vsync/native_vsync.h>
#include <hilog/log.h>
#define LOG_TAG "FloomBridge"
#define LOGI(...) OH_LOG_INFO(LOG_APP, __VA_ARGS__)

// The Floom backend's C ABI (crates/platform/floom-platform-harmony).
extern "C" bool floom_harmony_bind_surface(void* window, int32_t width, int32_t height);
extern "C" bool floom_harmony_present_frame(const uint8_t* rgba, size_t len, int32_t width,
                                            int32_t height);
extern "C" void floom_harmony_release_surface();
// Runs the Floom widget tree on its own thread; it presents through
// `floom_harmony_present_frame` and therefore needs a bound surface first.
extern "C" void floom_demo_start(int32_t width, int32_t height, float density);
extern "C" void floom_harmony_touch(double x, double y, uint8_t phase);

namespace {

OH_NativeXComponent* g_component = nullptr;
// ArkUI keeps a POINTER to this struct (Flutter stores it as an XComponentBase
// member, ohos_xcomponent_adapter.h:96). It must outlive Init — a stack-local
// one leaves ArkUI calling a dangling/junk function pointer on the first
// OnSurfaceCreated, which crashes at pc=0. Module-level static keeps it alive.
OH_NativeXComponent_Callback g_callback{};
OH_NativeVSync* g_vsync = nullptr;
int32_t g_width = 0;
int32_t g_height = 0;
uint64_t g_frame = 0;
std::vector<uint8_t> g_pixels;

// A frame the screenshot can prove came from us: a diagonal gradient that
// drifts each frame, so a frozen image is visibly wrong.
void fill_frame() {
  const int32_t w = g_width;
  const int32_t h = g_height;
  g_pixels.resize(static_cast<size_t>(w) * h * 4);
  const uint8_t drift = static_cast<uint8_t>(g_frame * 4);
  for (int32_t y = 0; y < h; ++y) {
    for (int32_t x = 0; x < w; ++x) {
      size_t i = (static_cast<size_t>(y) * w + x) * 4;
      g_pixels[i + 0] = static_cast<uint8_t>(x + drift); // R ramps across x
      g_pixels[i + 1] = static_cast<uint8_t>(y + drift); // G ramps down y
      g_pixels[i + 2] = static_cast<uint8_t>(128 + drift); // B constant-ish
      g_pixels[i + 3] = 0xFF; // opaque
    }
  }
}

void on_vsync(long long /*timestamp*/, void* /*data*/) {
  if (g_width > 0 && g_height > 0) {
    fill_frame();
    bool ok = floom_harmony_present_frame(g_pixels.data(), g_pixels.size(), g_width, g_height);
    if (g_frame < 3 || g_frame % 60 == 0) {
      LOGI("vsync frame=%{public}llu present=%{public}d w=%{public}d h=%{public}d len=%{public}zu",
           (unsigned long long)g_frame, ok ? 1 : 0, g_width, g_height, g_pixels.size());
    }
    ++g_frame;
  }
  if (g_vsync != nullptr) {
    OH_NativeVSync_RequestFrame(g_vsync, on_vsync, nullptr);
  }
}

// Touches arrive here, in the component's own pixel space: the Floom side divides
// by the display scale because layout is in points.
void on_touch(OH_NativeXComponent* component, void* window) {
  OH_NativeXComponent_TouchEvent event;
  if (OH_NativeXComponent_GetTouchEvent(component, window, &event) != 0) {
    return;
  }
  // Only the down edge: a move stream would flood hilog.
  if (event.type == OH_NATIVEXCOMPONENT_DOWN) {
    LOGI("touch points=%{public}u x=%{public}f y=%{public}f", event.numPoints, event.x,
         event.y);
  }
  for (uint32_t i = 0; i < event.numPoints; i++) {
    const auto& point = event.touchPoints[i];
    uint8_t phase = 1;  // move
    switch (point.type) {
      case OH_NATIVEXCOMPONENT_DOWN:
        phase = 0;
        break;
      case OH_NATIVEXCOMPONENT_UP:
        phase = 2;
        break;
      case OH_NATIVEXCOMPONENT_CANCEL:
        phase = 3;
        break;
      default:
        break;
    }
    floom_harmony_touch(point.x, point.y, phase);
  }
}

void on_surface_created(OH_NativeXComponent* component, void* window) {
  uint64_t w = 0;
  uint64_t h = 0;
  int32_t sz = OH_NativeXComponent_GetXComponentSize(component, window, &w, &h);
  g_width = static_cast<int32_t>(w);
  g_height = static_cast<int32_t>(h);
  bool bound = floom_harmony_bind_surface(window, g_width, g_height);
  LOGI("OnSurfaceCreated sz=%{public}d w=%{public}llu h=%{public}llu win=%{public}p bound=%{public}d",
       sz, (unsigned long long)w, (unsigned long long)h, window, bound ? 1 : 0);
  // The tree owns the pixels from here on: no gradient, no vsync loop of our own.
  if (bound) {
    floom_demo_start(g_width, g_height, 3.125f);
    LOGI("floom_demo_start returned");
  }
}

void on_surface_changed(OH_NativeXComponent* component, void* window) {
  uint64_t w = 0;
  uint64_t h = 0;
  OH_NativeXComponent_GetXComponentSize(component, window, &w, &h);
  g_width = static_cast<int32_t>(w);
  g_height = static_cast<int32_t>(h);
  floom_harmony_bind_surface(window, g_width, g_height);
}

void on_surface_destroyed(OH_NativeXComponent* /*component*/, void* /*window*/) {
  floom_harmony_release_surface();
  g_width = 0;
  g_height = 0;
}

// The ArkUI XComponent runtime loads libfloom_entry.so because the ArkTS
// component sets `libraryname: 'floom_entry'`, and injects
// OH_NATIVE_XCOMPONENT_OBJ into this module's `exports` before calling Init.
// This is the Flutter `library_loader.cpp` / `XComponentAdapter::Export` seam:
// the handle comes from `exports`, NOT from an onLoad callback argument.
napi_value Init(napi_env env, napi_value exports) {
  napi_value xcObj = nullptr;
  napi_get_named_property(env, exports, OH_NATIVE_XCOMPONENT_OBJ, &xcObj);
  if (xcObj != nullptr) {
    napi_unwrap(env, xcObj, reinterpret_cast<void**>(&g_component));
  }
  int32_t reg = -1;
  if (g_component != nullptr) {
    g_callback.OnSurfaceCreated = on_surface_created;
    g_callback.OnSurfaceChanged = on_surface_changed;
    g_callback.OnSurfaceDestroyed = on_surface_destroyed;
    g_callback.DispatchTouchEvent = on_touch;
    reg = OH_NativeXComponent_RegisterCallback(g_component, &g_callback);
  }
  LOGI("Init component=%{public}p RegisterCallback=%{public}d", g_component, reg);
  return exports;
}

} // namespace

NAPI_MODULE(floom_entry, Init)
