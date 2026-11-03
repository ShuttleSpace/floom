/* Windows native half of the camera plugin.
 *
 * Builds a `FloomNativeProviderV1` vtable that opens the default Media
 * Foundation capture device into the host's `HWND` child window, and registers
 * it with the Rust half from a static initializer. The Rust side
 * (`src/windows.rs`) then drives this vtable through the provider ABI.
 *
 * Compiled by `build.rs` with `/std:c++17`; Media Foundation comes in through
 * the `#pragma comment` lib list below.
 */

#include <stdio.h>
#include <string.h>

#include <windows.h>

#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>

#include "floom_plugin_camera.h"

#pragma comment(lib, "user32.lib")
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "mfplat.lib")
#pragma comment(lib, "mf.lib")
#pragma comment(lib, "mfreadwrite.lib")

/* One camera for the process. A plugin is a singleton, so a single source,
 * reader and surface are enough; the host's slot accounting is what keeps
 * several participants from colliding. */
static IMFMediaSource *g_source = NULL;
static IMFSourceReader *g_reader = NULL;
static HWND g_hwnd = NULL;
static HWND g_host = NULL;
static int g_live = 0;
static int g_refused = 0;
static int g_mf_refs = 0;
static int g_com_owned = 0;

static void cam_write_error(char *out_error, uint32_t out_capacity,
                            const char *message) {
    if (out_error != NULL && out_capacity != 0) {
        snprintf(out_error, out_capacity, "%s", message);
    }
}

static LRESULT CALLBACK cam_wndproc(HWND hwnd, UINT msg, WPARAM wparam,
                                    LPARAM lparam) {
    return DefWindowProcW(hwnd, msg, wparam, lparam);
}

static int cam_register_class(void) {
    static BOOL registered = FALSE;
    if (registered) {
        return 0;
    }
    WNDCLASSEXW wc;
    memset(&wc, 0, sizeof(wc));
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = cam_wndproc;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.hCursor = NULL;
    wc.hbrBackground = (HBRUSH)GetStockObject(BLACK_BRUSH);
    wc.lpszClassName = L"FloomCameraWindow";
    if (RegisterClassExW(&wc) == 0) {
        return 1;
    }
    registered = TRUE;
    return 0;
}

static int32_t cam_attach(void *self, const void *host_window) {
    if (host_window == NULL) {
        return 1;
    }
    g_host = (HWND)(void *)host_window;
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
    if (g_reader != NULL) {
        /* Already created for this process; the host tracks the slot. */
        return 0;
    }
    if (g_mf_refs == 0) {
        g_refused++;
        cam_write_error(out_error, out_capacity, "Media Foundation not started");
        return 1;
    }
    if (cam_register_class() != 0) {
        g_refused++;
        cam_write_error(out_error, out_capacity, "failed to register window class");
        return 1;
    }

    /* `MFEnumDeviceSources` puts the default device first. */
    IMFAttributes *attrs = NULL;
    IMFActivate **devices = NULL;
    UINT32 device_count = 0;
    HRESULT hr = MFCreateAttributes(&attrs, 1);
    if (SUCCEEDED(hr)) {
        hr = attrs->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                            MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID);
    }
    if (SUCCEEDED(hr)) {
        hr = MFEnumDeviceSources(attrs, &devices, &device_count);
    }
    if (SUCCEEDED(hr) && device_count == 0) {
        g_refused++;
        cam_write_error(out_error, out_capacity, "no video capture device");
        if (attrs != NULL) {
            attrs->Release();
        }
        return 1;
    }
    if (FAILED(hr)) {
        g_refused++;
        cam_write_error(out_error, out_capacity, "failed to enumerate capture devices");
        if (attrs != NULL) {
            attrs->Release();
        }
        return 1;
    }

    hr = devices[0]->ActivateObject(IID_IMFMediaSource, (void **)&g_source);
    CoTaskMemFree(devices);
    devices = NULL;
    attrs->Release();
    attrs = NULL;
    if (FAILED(hr)) {
        g_refused++;
        cam_write_error(out_error, out_capacity, "failed to open camera source");
        return 1;
    }

    hr = MFCreateSourceReaderFromMediaSource(g_source, NULL, &g_reader);
    if (FAILED(hr)) {
        g_reader = NULL;
        g_source->Shutdown();
        g_source->Release();
        g_source = NULL;
        g_refused++;
        cam_write_error(out_error, out_capacity, "failed to create source reader");
        return 1;
    }
    g_reader->SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM, TRUE);
    /* One read kicks the source into the running state. */
    IMFMediaBuffer *sample_buffer = NULL;
    if (FAILED(g_reader->ReadSample(MF_SOURCE_READER_FIRST_VIDEO_STREAM, 0, NULL,
                                    NULL, NULL, &sample_buffer))) {
        /* Tolerated: presentation is the host's business, not this plugin's. */
    }
    if (sample_buffer != NULL) {
        sample_buffer->Release();
    }

    DWORD style;
    DWORD ex_style = WS_EX_TOOLWINDOW;
    HWND parent;
    if (g_host != NULL) {
        style = WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN;
        parent = g_host;
    } else {
        style = WS_POPUP;
        parent = NULL;
    }
    g_hwnd = CreateWindowExW(ex_style, L"FloomCameraWindow", L"floom-camera",
                             style, 0, 0, 320, 240, parent, NULL,
                             GetModuleHandleW(NULL), NULL);
    if (g_hwnd == NULL) {
        g_reader->Release();
        g_reader = NULL;
        g_source->Shutdown();
        g_source->Release();
        g_source = NULL;
        g_refused++;
        cam_write_error(out_error, out_capacity, "failed to create camera window");
        return 1;
    }
    ShowWindow(g_hwnd, SW_HIDE);
    g_live++;
    return 0;
}

static void cam_destroy(void *self, FloomNativeSlot slot) {
    if (g_reader != NULL) {
        g_reader->Flush(MF_SOURCE_READER_ALL_STREAMS);
        g_reader->Release();
        g_reader = NULL;
        if (g_source != NULL) {
            g_source->Shutdown();
            g_source->Release();
            g_source = NULL;
        }
        if (g_hwnd != NULL) {
            DestroyWindow(g_hwnd);
            g_hwnd = NULL;
        }
        if (g_live > 0) {
            g_live--;
        }
    }
}

static void cam_apply(void *self, FloomNativeSlot slot,
                      const FloomNativeState *state) {
    if (g_hwnd == NULL || state == NULL) {
        return;
    }
    SetWindowPos(g_hwnd, NULL, state->rect.x, state->rect.y,
                 (int)state->rect.width, (int)state->rect.height,
                 SWP_NOZORDER | SWP_NOACTIVATE);
    ShowWindow(g_hwnd, (state->visible != 0) ? SW_SHOW : SW_HIDE);
}

static void cam_focus(void *self, FloomNativeSlot slot) {}
static void cam_blur(void *self, FloomNativeSlot slot) {}

static FloomNativeProviderV1 g_provider = {
    FLOOM_NATIVE_PROVIDER_ABI_VERSION,
    NULL,
    cam_attach,
    cam_capabilities,
    cam_create,
    cam_destroy,
    cam_apply,
    cam_focus,
    cam_blur,
};

/* `__attribute__((constructor))` is not MSVC-portable; a global object's
 * constructor runs before `main` just the same. */
static struct Registrar {
    Registrar() { floom_camera_register_provider(&g_provider); }
} g_registrar;

int floom_camera_enable(void) {
    /* MFStartup is ref-counted, so extra calls are balanced by disable. */
    if (g_mf_refs == 0) {
        HRESULT hr = CoInitializeEx(NULL, COINIT_APARTMENTTHREADED |
                                             COINIT_DISABLE_OLE1DDE);
        /* A thread already in the MTA is fine; we just must not pair it with
         * a CoUninitialize that belongs to someone else. */
        if (SUCCEEDED(hr) || hr == S_FALSE) {
            g_com_owned = 1;
        } else if (hr != RPC_E_CHANGED_MODE) {
            return 1;
        }
        hr = MFStartup(MF_VERSION, MFSTARTUP_LITE);
        if (FAILED(hr)) {
            if (g_com_owned) {
                CoUninitialize();
                g_com_owned = 0;
            }
            return 1;
        }
        g_mf_refs = 1;
    } else {
        g_mf_refs++;
    }
    return 0;
}

void floom_camera_disable(void) {
    if (g_reader != NULL) {
        g_reader->Release();
        g_reader = NULL;
    }
    if (g_source != NULL) {
        g_source->Shutdown();
        g_source->Release();
        g_source = NULL;
    }
    if (g_hwnd != NULL) {
        DestroyWindow(g_hwnd);
        g_hwnd = NULL;
    }
    if (g_mf_refs > 0) {
        g_mf_refs--;
        if (g_mf_refs == 0) {
            MFShutdown();
            if (g_com_owned) {
                CoUninitialize();
                g_com_owned = 0;
            }
        }
    }
}

int floom_camera_refused_count(void) { return g_refused; }
int floom_camera_live_count(void) { return g_live; }

int floom_camera_slot_is_attached(FloomNativeSlot slot) {
    if (g_hwnd == NULL || g_host == NULL) {
        return 0;
    }
    return IsChild(g_host, g_hwnd) ? 1 : 0;
}
