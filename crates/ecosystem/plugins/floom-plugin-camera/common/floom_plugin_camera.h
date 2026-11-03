/* The native/Rust contract for the `camera` plugin.
 *
 * The native half (one implementation per platform directory: `macos/`,
 * `ios/`, `windows/`, `linux/`, `ohos/`) implements the functions below and
 * registers a `FloomNativeProviderV1` vtable through
 * `floom_camera_register_provider`. The Rust half (`src/<platform>.rs`) calls
 * the lifecycle and diagnostics functions through `extern "C"` and wraps the
 * vtable in a `NativeViewProvider`.
 *
 * There is no serialization boundary and no runtime lookup: the symbols resolve
 * when the crate is linked, so a call from the app is a machine call.
 *
 * `android/` keeps its own Java half driven through JNI, and `web/` (pure
 * Rust, via `web-sys`) and `tui/` (unsupported) need none of this — they do
 * not compile a C native half.
 */

#ifndef FLOOM_PLUGIN_CAMERA_H
#define FLOOM_PLUGIN_CAMERA_H

#include "floom_native_provider.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Claims the plugin's platform state. Returns 0 on success. */
int floom_camera_enable(void);

/* Releases whatever `floom_camera_enable` claimed. */
void floom_camera_disable(void);

/* Hands the provider vtable to the Rust half.
 *
 * Exported by the Rust half, called by the native half's constructor: the
 * native code is the only party that knows when it is ready, and a constructor
 * runs before `main`.
 */
void floom_camera_register_provider(const void *provider);

/* How many `create` calls were refused for a kind this plugin cannot build. */
int floom_camera_refused_count(void);

/* How many participants are alive right now. */
int floom_camera_live_count(void);

/* Whether the participant in `slot` is really in the host window. */
int floom_camera_slot_is_attached(FloomNativeSlot slot);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* FLOOM_PLUGIN_CAMERA_H */
