/* macOS native half of the camera plugin.
 *
 * Builds a `FloomNativeProviderV1` vtable that composites a live
 * AVFoundation capture preview into the host's `NSView`, and registers it with
 * the Rust half from a constructor. The Rust side (`src/macos.rs`) then drives
 * this vtable through the provider ABI.
 *
 * Compiled by `build.rs` with `-fobjc-arc` and linked against AppKit,
 * Foundation and AVFoundation.
 */

#include <stdio.h>
#include <string.h>

#import <AppKit/AppKit.h>
#import <AVFoundation/AVFoundation.h>

#include "floom_plugin_camera.h"

/* One camera for the process. A plugin is a singleton, so a single session and
 * preview layer are enough; the host's slot accounting is what keeps several
 * participants from colliding. */
static AVCaptureSession *g_session = nil;
static AVCaptureVideoPreviewLayer *g_preview = nil;
static NSView *g_host = nil;
static int g_live = 0;
static int g_refused = 0;

static int32_t cam_attach(void *self, const void *host_window) {
    if (host_window == NULL) {
        return 1;
    }
    g_host = (__bridge NSView *)host_window;
    if (g_host != nil && g_host.layer == nil) {
        [g_host setWantsLayer:YES];
    }
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
    if (g_session != nil) {
        /* Already created for this process; the host tracks the slot. */
        return 0;
    }

    NSError *err = nil;
    AVCaptureDevice *device =
        [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
    if (device == nil) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no video capture device");
        }
        return 1;
    }

    AVCaptureDeviceInput *input =
        [AVCaptureDeviceInput deviceInputWithDevice:device error:&err];
    if (input == nil) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "failed to open camera input");
        }
        return 1;
    }

    AVCaptureSession *session = [[AVCaptureSession alloc] init];
    if ([session canSetSessionPreset:AVCaptureSessionPresetHigh]) {
        session.sessionPreset = AVCaptureSessionPresetHigh;
    }
    [session addInput:input];

    AVCaptureVideoPreviewLayer *preview =
        [[AVCaptureVideoPreviewLayer alloc] initWithSession:session];
    preview.videoGravity = AVLayerVideoGravityResizeAspectFill;

    g_session = session;
    g_preview = preview;
    if (g_host != nil && g_host.layer != nil) {
        [g_host.layer addSublayer:preview];
    }
    [session startRunning];
    g_live++;
    return 0;
}

static void cam_destroy(void *self, FloomNativeSlot slot) {
    if (g_session != nil) {
        [g_session stopRunning];
        if (g_preview != nil) {
            [g_preview removeFromSuperlayer];
        }
        g_session = nil;
        g_preview = nil;
        if (g_live > 0) {
            g_live--;
        }
    }
}

static void cam_apply(void *self, FloomNativeSlot slot,
                      const FloomNativeState *state) {
    if (g_preview == nil || state == NULL) {
        return;
    }
    CGFloat x = (CGFloat)state->rect.x;
    CGFloat y = (CGFloat)state->rect.y;
    CGFloat w = (CGFloat)state->rect.width;
    CGFloat h = (CGFloat)state->rect.height;
    g_preview.frame = CGRectMake(x, y, w, h);
    g_preview.hidden = (state->visible == 0);
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

__attribute__((constructor))
static void cam_register(void) {
    floom_camera_register_provider(&g_provider);
}

int floom_camera_enable(void) { return 0; }

void floom_camera_disable(void) {
    if (g_session != nil) {
        [g_session stopRunning];
        g_session = nil;
        g_preview = nil;
    }
}

int floom_camera_refused_count(void) { return g_refused; }
int floom_camera_live_count(void) { return g_live; }

int floom_camera_slot_is_attached(FloomNativeSlot slot) {
    if (g_preview == nil || g_host == nil) {
        return 0;
    }
    return (g_preview.superlayer == g_host.layer) ? 1 : 0;
}
