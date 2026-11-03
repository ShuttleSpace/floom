/* Linux native half of the camera plugin.
 *
 * Builds a `FloomNativeProviderV1` vtable that composites a live GStreamer
 * capture pipeline into the host's X11 window, and registers it with the Rust
 * half from a constructor. The Rust side (`src/linux.rs`) then drives this
 * vtable through the provider ABI.
 *
 * Compiled by `build.rs` with no extra flags; the compiler is expected to find
 * GStreamer 1.x and X11 headers at build time and to link the libraries the
 * target machine names them by.
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <gst/gst.h>
#include <gst/video/video.h>

#include <X11/Xlib.h>

#include "floom_plugin_camera.h"

/* One camera for the process. A plugin is a singleton, so a single pipeline
 * and one embedded X11 window are enough; the host's slot accounting is what
 * keeps several participants from colliding. */
static GstPipeline *g_pipeline = NULL;
static GstVideoOverlay *g_overlay = NULL;
static Display *g_display = NULL;
static Window g_parent = None;
static Window g_child = None;
static int g_visible = -1;
static int g_gst_ready = 0;
static int g_live = 0;
static int g_refused = 0;

/* Tears the participant down: pipeline, child window, and the Display opened
 * in attach. Used by both `destroy` and `disable`. */
static void cam_teardown(void) {
    if (g_pipeline != NULL) {
        gst_element_set_state(GST_ELEMENT(g_pipeline), GST_STATE_NULL);
        gst_object_unref(GST_OBJECT(g_pipeline));
        g_pipeline = NULL;
    }
    g_overlay = NULL;
    if (g_display != NULL && g_child != None) {
        XUnmapWindow(g_display, g_child);
        XDestroyWindow(g_display, g_child);
        g_child = None;
    }
    g_visible = -1;
    if (g_display != NULL) {
        XCloseDisplay(g_display);
        g_display = NULL;
        g_parent = None;
    }
    if (g_live > 0) {
        g_live--;
    }
}

static int32_t cam_attach(void *self, const void *host_window) {
    if (host_window == NULL) {
        return 1;
    }
    /* The value crosses as the XID itself, not a pointer to it. */
    Window parent = (Window)(uintptr_t)host_window;
    Display *display = XOpenDisplay(NULL);
    if (display == NULL) {
        return 1;
    }
    /* Replace any window from an earlier attach, so the follow-up teardown in
     * `destroy` cannot see a Display that no longer matches it. */
    cam_teardown();
    g_display = display;
    g_parent = parent;
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
    if (g_pipeline != NULL) {
        /* Already created for this process; the host tracks the slot. */
        return 0;
    }
    if (!g_gst_ready) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "GStreamer was never initialised");
        }
        return 1;
    }
    if (g_display == NULL || g_parent == None) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no host window attached");
        }
        return 1;
    }

    GError *err = NULL;
    GstElement *pipeline =
        gst_parse_launch("autovideosrc ! videoconvert ! xvimagesink", &err);
    if (pipeline == NULL) {
        g_refused++;
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "pipeline build failed: %s",
                     err ? err->message : "unknown error");
        }
        if (err != NULL) {
            g_error_free(err);
        }
        return 1;
    }

    GstElement *sink = NULL;
    if (GST_IS_BIN(pipeline)) {
        sink = gst_bin_get_by_interface(GST_BIN(pipeline), GST_TYPE_VIDEO_OVERLAY);
    } else if (GST_IS_VIDEO_OVERLAY(pipeline)) {
        sink = pipeline;
    }
    if (sink == NULL) {
        g_refused++;
        gst_object_unref(GST_OBJECT(pipeline));
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "no sink that takes a window");
        }
        return 1;
    }

    /* The video window is the participant: a plain subwindow of the host
     * window that X11 clips and stacks for us. */
    Window child = XCreateSimpleWindow(g_display, g_parent, 0, 0, 1, 1, 0, 0, 0);
    if (child == None) {
        g_refused++;
        gst_object_unref(GST_OBJECT(sink));
        gst_object_unref(GST_OBJECT(pipeline));
        if (out_error && out_capacity) {
            snprintf(out_error, out_capacity, "XCreateSimpleWindow failed");
        }
        return 1;
    }
    gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(sink),
                                        (uintptr_t)child);
    XMapWindow(g_display, child);
    XFlush(g_display);
    g_visible = 1;
    gst_object_unref(GST_OBJECT(sink));

    g_pipeline = GST_PIPELINE(pipeline);
    g_overlay = GST_VIDEO_OVERLAY(sink);
    g_child = child;

    gst_element_set_state(GST_ELEMENT(g_pipeline), GST_STATE_PLAYING);
    g_live++;
    return 0;
}

static void cam_destroy(void *self, FloomNativeSlot slot) {
    if (g_pipeline != NULL) {
        cam_teardown();
    }
}

static void cam_apply(void *self, FloomNativeSlot slot,
                      const FloomNativeState *state) {
    if (g_pipeline == NULL || state == NULL || g_display == NULL ||
        g_child == None) {
        return;
    }

    /* Position the subwindow itself; the parent clips whatever falls outside
     * it, so `clip` is honoured here only by showing the clipped part of
     * `rect` in place, through the sink's render rectangle. */
    XMoveResizeWindow(g_display, g_child, state->rect.x, state->rect.y,
                      state->rect.width, state->rect.height);
    if (g_overlay != NULL) {
        if (state->has_clip) {
            gst_video_overlay_set_render_rectangle(
                g_overlay, state->clip.x - state->rect.x,
                state->clip.y - state->rect.y, state->clip.width,
                state->clip.height);
        } else {
            gst_video_overlay_set_render_rectangle(g_overlay, 0, 0,
                                                   state->rect.width,
                                                   state->rect.height);
        }
    }
    if (state->visible != g_visible) {
        g_visible = state->visible;
        if (g_visible) {
            XMapWindow(g_display, g_child);
        } else {
            XUnmapWindow(g_display, g_child);
        }
    }
    if (g_overlay != NULL) {
        gst_video_overlay_expose(g_overlay);
    }
    XFlush(g_display);
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

int floom_camera_enable(void) {
    GError *err = NULL;
    if (!gst_init_check(NULL, NULL, &err)) {
        fprintf(stderr, "floom-plugin-camera: gst_init_check failed: %s\n",
                err ? err->message : "unknown error");
        if (err != NULL) {
            g_error_free(err);
        }
        return 1;
    }
    g_gst_ready = 1;
    return 0;
}

void floom_camera_disable(void) {
    if (g_pipeline != NULL) {
        cam_teardown();
        return;
    }
    /* No participant alive, but the claim on the library and the Display may
     * still exist. */
    if (g_display != NULL) {
        XCloseDisplay(g_display);
        g_display = NULL;
        g_parent = None;
    }
    if (g_gst_ready) {
        g_gst_ready = 0;
    }
}

int floom_camera_refused_count(void) { return g_refused; }
int floom_camera_live_count(void) { return g_live; }

int floom_camera_slot_is_attached(FloomNativeSlot slot) {
    if (g_pipeline == NULL || g_child == None || g_display == NULL) {
        return 0;
    }
    /* Really in the host window: ask X what the child's parent is. */
    Window root = None, parent = None, *children = NULL;
    unsigned int n = 0;
    if (!XQueryTree(g_display, g_child, &root, &parent, &children, &n)) {
        return 0;
    }
    if (children != NULL) {
        XFree(children);
    }
    return (parent == g_parent) ? 1 : 0;
}
