/* OpenHarmony native half of the camera plugin.
 *
 * Builds a `FloomNativeProviderV1` vtable that opens the default camera
 * through the OHOS camera NDK (`libcamera_ndk.z.so`) and pipes its preview
 * into the host's `OHNativeWindow`, and registers it with the Rust half from
 * a static constructor. The Rust side (`src/ohos.rs`) then drives this vtable
 * through the provider ABI.
 *
 * Cross-compiled by DevEco's toolchain with the OHOS sysroot; linked against
 * libcamera_ndk.z.so, libnative_window.so and libace_napi.so.
 */

#include <stdio.h>
#include <string.h>

#include <camera/camera.h>
#include <ace/xcomponent/native_interface_xcomponent.h>
#include <native_window/external_window.h>

#include "floom_plugin_camera.h"

/* One camera for the process. A plugin is a singleton, so a single session
 * and preview output are enough; the host's slot accounting is what keeps
 * several participants from colliding. */
static Camera_Manager *g_manager = NULL;
static Camera_Profile *g_capability = NULL;
static Camera_Input *g_input = NULL;
static Camera_Session *g_session = NULL;
static Camera_PreviewOutput *g_preview = NULL;
static OHNativeWindow *g_window = NULL;
static int g_live = 0;
static int g_refused = 0;

static int32_t cam_attach(void *self, const void *host_window) {
    if (host_window == NULL) {
        return 1;
    }
    /* The host hands its OHNativeWindow* across the ABI; casting away the
     * const is what the ABI exists for. */
    g_window = reinterpret_cast<OHNativeWindow *>(
        const_cast<void *>(host_window));
    return 0;
}

static FloomNativeCapabilities cam_capabilities(void *self) {
    FloomNativeCapabilities c;
    memset(&c, 0, sizeof(c));
    c.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    c.mode_count = 1;
    c.snapshot = 0;
    c.input_passthrough = 1;
    c.input_capture = 0;
    c.focusable = 0;
    c.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return c;
}

/* Open the first camera device and bind its preview to the host window. */
static int32_t cam_open_camera(char *out_error, uint32_t out_capacity) {
    if (g_manager == NULL) {
        Camera_ErrorCode rc = OH_Camera_GetCameraManager(&g_manager);
        if (rc != CAMERA_OK || g_manager == NULL) {
            if (out_error && out_capacity) {
                snprintf(out_error, out_capacity, "no camera manager");
            }
            return 1;
        }
    }

    Camera_Device *devices = NULL;
    uint32_t device_count = 0;
    if (OH_CameraManager_GetSupportedCameras(g_manager, &devices,
                                             &device_count) != CAMERA_OK ||
        device_count == 0 || devices == NULL) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no video capture device");
        }
        return 1;
    }
    /* First camera wins; the list is ordered so the back camera shows up
     * before the front one where both exist. */
    Camera_Device *device = &devices[0];

    Camera_OutputCapability *caps = NULL;
    if (OH_CameraManager_GetSupportedCameraOutputCapability(
            g_manager, device, &caps) != CAMERA_OK ||
        caps == NULL || caps->previewProfiles == NULL ||
        caps->previewProfilesSize == 0) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no preview profile");
        }
        return 1;
    }
    g_capability = &caps->previewProfiles[0];

    if (OH_CameraManager_CreateCameraInput(g_manager, device, &g_input) !=
            CAMERA_OK ||
        g_input == NULL ||
        OH_CameraInput_Open(g_input) != CAMERA_OK) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "failed to open camera input");
        }
        return 1;
    }

    /* The session carries the preview output built from the host window's
     * surface id, which the platform generates for any OHNativeWindow. */
    if (OH_CameraManager_CreateSession(g_manager, SCENE_MODE_NORMAL,
                                       &g_session) != CAMERA_OK ||
        g_session == NULL) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "failed to create session");
        }
        return 1;
    }

    char surface_id[128] = {0};
    if (OH_NativeWindow_GetSurfaceId(g_window, surface_id, sizeof(surface_id)) !=
        0) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no surface id on host window");
        }
        return 1;
    }
    if (OH_CameraManager_CreatePreviewOutput(g_manager, g_capability, surface_id,
                                             &g_preview) != CAMERA_OK ||
        g_preview == NULL) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "failed to create preview");
        }
        return 1;
    }

    if (OH_Session_BeginConfig(g_session) != CAMERA_OK ||
        OH_Session_AddInput(g_session, g_input) != CAMERA_OK ||
        OH_Session_AddPreviewOutput(g_session, g_preview) != CAMERA_OK ||
        OH_Session_CommitConfig(g_session) != CAMERA_OK ||
        OH_Session_Start(g_session) != CAMERA_OK) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "failed to start session");
        }
        return 1;
    }
    return 0;
}

static int32_t cam_create(void *self, FloomNativeSlot slot, uint32_t kind,
                          const char *kind_name, char *out_error,
                          uint32_t out_capacity) {
    if (kind != FLOOM_NATIVE_KIND_CUSTOM) {
        g_refused++;
        return 1;
    }
    if (kind_name == NULL || strcmp(kind_name, "camera") != 0) {
        g_refused++;
        return 1;
    }
    if (g_session != NULL) {
        /* Already created for this process; the host tracks the slot. */
        return 0;
    }

    if (cam_open_camera(out_error, out_capacity) != 0) {
        return 1;
    }
    g_live++;
    return 0;
}

static void cam_destroy(void *self, FloomNativeSlot slot) {
    if (g_session != NULL) {
        OH_Session_Stop(g_session);
        g_session = NULL;
        g_preview = NULL;
        if (g_input != NULL) {
            OH_CameraInput_Release(g_input);
            g_input = NULL;
        }
        if (g_live > 0) {
            g_live--;
        }
    }
}

static void cam_apply(void *self, FloomNativeSlot slot,
                      const FloomNativeState *state) {
    if (g_window == NULL || state == NULL) {
        return;
    }
    /* The preview still fills the buffer at whatever profile resolution the
     * producer picked; SET_BUFFER_GEOMETRY shapes the queue, SET_UI_GEOMETRY
     * shapes where it lands in the host surface. */
    OH_NativeWindow_NativeWindowHandleOpt(
        g_window, SET_BUFFER_GEOMETRY, static_cast<int32_t>(state->rect.width),
        static_cast<int32_t>(state->rect.height));
    OH_NativeWindow_NativeWindowHandleOpt(
        g_window, SET_UI_GEOMETRY, state->rect.x, state->rect.y,
        static_cast<int32_t>(state->rect.width),
        static_cast<int32_t>(state->rect.height));
    /* OHNativeWindow has no hide operation; visibility degrades to leaving
     * the window where it is, matching what a plugin that owns no child
     * surface can do here. */
}

static void cam_focus(void *self, FloomNativeSlot slot) {}
static void cam_blur(void *self, FloomNativeSlot slot) {}

static FloomNativeProviderV1 g_provider = {
    .abi_version = FLOOM_NATIVE_PROVIDER_ABI_VERSION,
    .self = NULL,
    .attach = cam_attach,
    .capabilities = cam_capabilities,
    .create = cam_create,
    .destroy = cam_destroy,
    .apply = cam_apply,
    .focus = cam_focus,
    .blur = cam_blur,
};

static struct Registrar {
    Registrar() { floom_camera_register_provider(&g_provider); }
} g_registrar;

int floom_camera_enable(void) { return 0; }

void floom_camera_disable(void) {
    if (g_session != NULL) {
        OH_Session_Stop(g_session);
        g_session = NULL;
        g_preview = NULL;
    }
    if (g_input != NULL) {
        OH_CameraInput_Release(g_input);
        g_input = NULL;
    }
    g_window = NULL;
}

int floom_camera_refused_count(void) { return g_refused; }
int floom_camera_live_count(void) { return g_live; }

int floom_camera_slot_is_attached(FloomNativeSlot slot) {
    if (g_preview == NULL || g_window == NULL) {
        return 0;
    }
    /* The native window neither exposes a parent nor a containment query, so
     * "really in the host window" is the strongest statement available: a
     * live preview running over the window's surface id. */
    return (g_session != NULL) ? 1 : 0;
}
