/*
 * The macOS half of the `nativeview` plugin.
 *
 * Objective-C, because that is what AppKit is written in and a plugin that
 * contributes a native view has to reach the framework it contributes to. The
 * contract it implements is the C ABI in `../common/`: the plugin's own four
 * functions, and the provider vtable that says how the view is created,
 * placed and torn down.
 *
 * The provider is asked for the host's window once, in `attach`, because a
 * provider is an instance per window and the window does not change over its
 * life.
 */

#import <Cocoa/Cocoa.h>

#include "floom_nativeview.h"
#include "floom_native_provider.h"

/* --- what the provider remembers ---------------------------------------- */

static NSWindow *g_window = nil;
static NSView *g_view = nil;
static int g_created = 0;
static int g_destroyed = 0;
static int g_applied = 0;
static int g_focused = 0;
static double g_last_x = 0.0;
static double g_last_y = 0.0;
static double g_last_w = 0.0;
static double g_last_h = 0.0;

/* --- the plugin's own four functions ------------------------------------- */

const char *floom_nativeview_version(void) {
    static const char version[] = "nativeview 0.1.0";
    return version;
}

int floom_nativeview_enable(void) {
    return 0;
}

void floom_nativeview_disable(void) {
}

int floom_nativeview_capability(void) {
    return 0;
}

/* --- the provider -------------------------------------------------------- */

/*
 * The host's window.
 *
 * The window is the `NSWindow`; the view goes into its content view. The
 * host passes the window rather than the content view because the host has
 * the window and the plugin knows what to do with it.
 */
static int32_t nativeview_attach(void *self, const void *host_window) {
    (void)self;
    /* The bridged cast is what ARC requires: the window belongs to the host,
     * so the plugin does not retain it. */
    g_window = (__bridge NSWindow *)host_window;
    if (g_view != nil && g_window != nil) {
        /* A view created before the window was known goes into it now. */
        [[g_window contentView] addSubview:g_view];
    }
    return 0;
}

static FloomNativeCapabilities nativeview_capabilities(void *self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay only: the view is a sibling of the framework's own surface, so
     * it composites above the whole scene and never inside it. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.mode_count = 1;
    caps.snapshot = 0;
    caps.input_passthrough = 1;
    caps.input_capture = 0;
    caps.focusable = 0;
    caps.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return caps;
}

static int32_t nativeview_create(void *self, FloomNativeSlot slot, uint32_t kind,
                                 const char *kind_name, char *out_error,
                                 uint32_t out_capacity) {
    (void)self;
    (void)slot;
    (void)kind;
    (void)kind_name;
    (void)out_error;
    (void)out_capacity;

    g_view = [[NSView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 100.0, 100.0)];
    if (g_window != nil) {
        /* The view is a sibling of the framework's own surface, so it goes
         * into the window's content view and composites above the scene. */
        [[g_window contentView] addSubview:g_view];
    }
    g_created += 1;
    return 0;
}

static void nativeview_destroy(void *self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
    if (g_view != nil) {
        [g_view removeFromSuperview];
        g_view = nil;
    }
    g_destroyed += 1;
}

/*
 * Place the view.
 *
 * The rect arrives in whole device pixels; AppKit's frames are in points, so
 * it is divided by the window's backing scale. That division is the
 * platform's own business — the framework snapped to whole device pixels
 * exactly once, and this is the other side of that boundary.
 */
static void nativeview_apply(void *self, FloomNativeSlot slot,
                             const FloomNativeState *state) {
    (void)self;
    (void)slot;
    if (g_view == nil || state == NULL) {
        return;
    }
    g_applied += 1;

    const double scale = g_window != nil ? g_window.backingScaleFactor : 1.0;
    g_last_x = (double)state->rect.x / scale;
    g_last_y = (double)state->rect.y / scale;
    g_last_w = (double)state->rect.width / scale;
    g_last_h = (double)state->rect.height / scale;

    [g_view setFrame:NSMakeRect(g_last_x, g_last_y, g_last_w, g_last_h)];
}

static void nativeview_focus(void *self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
    g_focused += 1;
}

static void nativeview_blur(void *self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
}

static FloomNativeProviderV1 g_provider = {
    FLOOM_NATIVE_PROVIDER_ABI_VERSION,
    NULL,
    nativeview_attach,
    nativeview_capabilities,
    nativeview_create,
    nativeview_destroy,
    nativeview_apply,
    nativeview_focus,
    nativeview_blur,
};

/* --- handing the provider to the Rust half ------------------------------ */

/*
 * The Rust half needs the vtable, and the only party that knows when the
 * native code is ready is the native code itself. A constructor runs before
 * `main`, which is early enough: the Rust half reads the pointer when the
 * app asks for the provider, not before.
 */
__attribute__((constructor)) static void nativeview_register(void) {
    floom_nativeview_register_provider(&g_provider);
}

/* --- diagnostics --------------------------------------------------------- */

/*
 * What the provider did, for a host that wants to check.
 *
 * Plain C, so a test — or an app's own diagnostics — can read it without
 * knowing anything about AppKit.
 */

int floom_nativeview_created_count(void) {
    return g_created;
}

int floom_nativeview_destroyed_count(void) {
    return g_destroyed;
}

int floom_nativeview_applied_count(void) {
    return g_applied;
}

int floom_nativeview_focused_count(void) {
    return g_focused;
}

void floom_nativeview_last_frame(double *x, double *y, double *w, double *h) {
    if (x != NULL) {
        *x = g_last_x;
    }
    if (y != NULL) {
        *y = g_last_y;
    }
    if (w != NULL) {
        *w = g_last_w;
    }
    if (h != NULL) {
        *h = g_last_h;
    }
}

/*
 * Whether the view is really in the window's content view.
 *
 * The check that matters: a provider that created a view and did not put it
 * anywhere has not contributed a native view, and the difference is invisible
 * from the framework's side.
 */
int floom_nativeview_view_is_attached(void) {
    if (g_view == nil || g_window == nil) {
        return 0;
    }
    return [g_view.superview isEqual:g_window.contentView] ? 1 : 0;
}
