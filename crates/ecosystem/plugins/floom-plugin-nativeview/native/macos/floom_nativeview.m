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
 *
 * # Kinds, and why this file has a table
 *
 * The provider is asked for a view of a given *kind*, and a plugin that
 * answers every kind with the same blank `NSView` claims a capability it does
 * not have. That was the state before this file grew a table: `create`
 * ignored its `kind` argument. Now the kind decides what is built —
 * `FLOOM_NATIVE_KIND_WEB` builds a `WKWebView`, and every other kind is
 * *refused* with a message naming it.
 *
 * A refusal is the right answer, not a limitation to work around. The host
 * routes by kind and refuses an unclaimed one at bind time; a plugin that
 * silently substituted a blank view for a camera would hand back a rectangle
 * no one can tell apart from a view that was never asked for.
 *
 * # Why a per-slot table rather than one `g_view`
 *
 * A scene can hold more than one participant at a time, and they are not
 * necessarily the same kind. One global view meant the second `create` stole
 * the first one's handle — invisible while every view was a blank `NSView`
 * and only the last one on screen mattered, but a leaked web view the moment
 * two are alive.
 */

#import <Cocoa/Cocoa.h>
#import <WebKit/WebKit.h>

#include "floom_nativeview.h"
#include "floom_native_provider.h"

static void nativeview_handle_js_message(FloomNativeSlot slot, NSString *body);

/* --- what the provider remembers ---------------------------------------- */

static NSWindow *g_window = nil;

/*
 * Live participants, keyed by slot.
 *
 * The slot identity comes from the framework and includes a generation, so a
 * recycled slot is a different key by construction — a view from a previous
 * row can never be mistaken for the current one's.
 */

@interface FloomScriptHandler : NSObject <WKScriptMessageHandler>
@property(nonatomic, assign) FloomNativeSlot slot;
- (instancetype)initWithSlot:(FloomNativeSlot)slot;
@end

@implementation FloomScriptHandler
- (instancetype)initWithSlot:(FloomNativeSlot)slot {
    self = [super init];
    if (self) { _slot = slot; }
    return self;
}
- (void)userContentController:(WKUserContentController *)ucc
      didReceiveScriptMessage:(WKScriptMessage *)message {
    NSString *body = [message.body isKindOfClass:[NSString class]] ? message.body : [message.body description];
    nativeview_handle_js_message(self.slot, body);
}
@end

@interface FloomNativeView : NSObject
@property(nonatomic, strong) NSView *container;
@property(nonatomic, strong) NSView *content;
@property(nonatomic, strong) WKWebView *web;
@property(nonatomic, assign) uint32_t kind;
@end

@implementation FloomNativeView
@end

static NSMutableDictionary<NSString *, FloomNativeView *> *g_views = nil;

static NSString *slot_key(FloomNativeSlot slot) {
    return [NSString stringWithFormat:@"%u:%u", (unsigned)slot.index,
                                      (unsigned)slot.generation];
}

static int g_created = 0;
static int g_destroyed = 0;
static int g_applied = 0;
static int g_focused = 0;
static int g_refused = 0;
static double g_last_x = 0.0;
static double g_last_y = 0.0;
static double g_last_w = 0.0;
static double g_last_h = 0.0;

/* The URL the most recent web view was asked to load, for a host to check. */
static NSString *g_last_url = nil;
/* What the last refusal said, so a host can read the true reason. */
static NSString *g_last_refusal = nil;

/* --- the plugin's own four functions ------------------------------------- */

const char *floom_nativeview_version(void) {
    static const char version[] = "nativeview 0.2.0";
    return version;
}

int floom_nativeview_enable(void) {
    if (g_views == nil) {
        g_views = [NSMutableDictionary dictionary];
    }
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
 * The window is the `NSWindow`; views go into its content view. The host
 * passes the window rather than the content view because the host has the
 * window and the plugin knows what to do with it.
 */
static int32_t nativeview_attach(void *self, const void *host_window) {
    (void)self;
    /* The bridged cast is what ARC requires: the window belongs to the host,
     * so the plugin does not retain it. */
    g_window = (__bridge NSWindow *)host_window;
    if (g_views != nil && g_window != nil) {
        /* Views created before the window was known go into it now. */
        for (FloomNativeView *view in g_views.allValues) {
            [[g_window contentView] addSubview:view.container];
        }
    }
    return 0;
}

static FloomNativeCapabilities nativeview_capabilities(void *self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay only: the views are siblings of the framework's own surface, so
     * they composite above the whole scene and never inside it. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.mode_count = 1;
    caps.snapshot = 0;
    /*
     * A web view takes the pointer — that is the whole point of embedding one:
     * links, text selection and scrolling all need clicks to reach it. A
     * passthrough web view would look interactive and behave like a picture.
     */
    caps.input_passthrough = 0;
    caps.input_capture = 1;
    /* Focusable: a web page has text fields, and a form the user cannot type
     * into is not an embedded browser. */
    caps.focusable = 1;
    caps.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return caps;
}

static int32_t nativeview_create(void *self, FloomNativeSlot slot, uint32_t kind,
                                 const char *kind_name, char *out_error,
                                 uint32_t out_capacity) {
    (void)self;
    NSString *unsupported = nil;

    switch (kind) {
    case FLOOM_NATIVE_KIND_WEB:
        break;
    default:
        /*
         * Every other kind is refused. The message names what was asked for,
         * because "the plugin said no" without the kind is the kind of error
         * that sends someone reading the C ABI to find out which argument was
         * wrong.
         */
        unsupported = [NSString
            stringWithFormat:@"this plugin builds web views only, not kind %u%s%s",
                             kind, kind_name != NULL ? " (" : "",
                             kind_name != NULL ? kind_name : ""];
        if (kind_name != NULL) {
            unsupported = [unsupported stringByAppendingString:@")"];
        }
        break;
    }

    if (unsupported != nil) {
        g_refused += 1;
        g_last_refusal = unsupported;
        if (out_error != NULL && out_capacity > 0) {
            /* Never overflow the caller's buffer, and always NUL-terminate:
             * the host treats the message as a C string. */
            strncpy(out_error, unsupported.UTF8String, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }

    FloomNativeView *view = [FloomNativeView new];
    view.kind = kind;
    view.container = [[NSView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 100.0, 100.0)];

    switch (kind) {
    case FLOOM_NATIVE_KIND_WEB: {
        /*
         * `WKWebView` rather than `WebView`. The old WebView is deprecated and
         * has no modern JS bridge; `WKWebView` is what every current macOS app
         * embeds.
         */
        WKWebViewConfiguration *config = [[WKWebViewConfiguration alloc] init];
        /* JS -> Rust bridge: inject a `window.floomNative` shim. */
        WKUserContentController *ucc = [[WKUserContentController alloc] init];
        [ucc addScriptMessageHandler:[[FloomScriptHandler alloc] initWithSlot:slot]
                              name:@"floomNative"];
        WKUserScript *bootstrap = [[WKUserScript alloc]
            initWithSource:@"window.floomNative = window.floomNative || { postMessage: function(m){ window.webkit.messageHandlers.floomNative.postMessage(m); } };"
             injectionTime:WKUserScriptInjectionTimeAtDocumentStart
          forMainFrameOnly:NO];
        [ucc addUserScript:bootstrap];
        config.userContentController = ucc;
        view.web = [[WKWebView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 100.0, 100.0)
                                      configuration:config];
        view.content = view.web;
        break;
    }
    default:
        /* Unreachable: the switch above refuses everything else. Kept as a
         * total expression rather than `!` so a future kind added to the C
         * header fails to compile here instead of silently building nothing. */
        view.content = [[NSView alloc] initWithFrame:NSMakeRect(0.0, 0.0, 100.0, 100.0)];
        break;
    }

    [view.container addSubview:view.content];
    if (g_window != nil) {
        /* The view is a sibling of the framework's own surface, so it goes
         * into the window's content view and composites above the scene. */
        [[g_window contentView] addSubview:view.container];
    }
    if (g_views == nil) {
        g_views = [NSMutableDictionary dictionary];
    }
    g_views[slot_key(slot)] = view;
    g_created += 1;
    return 0;
}

static void nativeview_destroy(void *self, FloomNativeSlot slot) {
    (void)self;
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view != nil) {
        [view.container removeFromSuperview];
        [g_views removeObjectForKey:slot_key(slot)];
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
    if (state == NULL) {
        return;
    }
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view == nil) {
        return;
    }
    g_applied += 1;

    const double scale = g_window != nil ? g_window.backingScaleFactor : 1.0;
    g_last_x = (double)state->rect.x / scale;
    g_last_y = (double)state->rect.y / scale;
    g_last_w = (double)state->rect.width / scale;
    g_last_h = (double)state->rect.height / scale;

    [view.container setFrame:NSMakeRect(g_last_x, g_last_y, g_last_w, g_last_h)];
    /* The content fills the container: the container is what carries the
     * placement, so a resize has to reach one level down too. */
    [view.content setFrame:NSMakeRect(0.0, 0.0, g_last_w, g_last_h)];
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

/* --- the web view -------------------------------------------------------- */

/*
 * Load a URL into a live web participant.
 *
 * Separate from `create` because the framework's provider ABI has no "navigate"
 * verb: a participant is created, placed and destroyed, and what a web view
 * *shows* is the web's own business, not the compositor's. The host calls this
 * through the plugin's own ABI after binding a `Web` participant.
 *
 * Returns 0 when the URL was handed to WebKit, non-zero when the slot holds no
 * web view (a participant of another kind, or one already destroyed) or the
 * URL could not be parsed. `Ok` claims the load was *started*, not that the
 * page arrived — the page's own loading is asynchronous and belongs to WebKit.
 */
/*
 * Whether `text` is a URL a web view can be pointed at.
 *
 * `NSURL URLWithString:` is far more permissive than it looks: it parses
 * "not a url" and hands back an object with no scheme. Handing that to WebKit
 * produces a request it cannot resolve, and the failure surfaces later as a
 * blank page rather than as a refused call. Measured, not assumed:
 *
 *     "not a url"            -> parsed, scheme (none)
 *     "notaurl"              -> parsed, scheme (none)
 *     "http://example.com"   -> parsed, scheme http
 *     "about:blank"          -> parsed, scheme about
 *     "://x"                 -> parsed, scheme ""
 *
 * So a scheme is required, and it has to be non-empty. That is the check that
 * makes an `Err` from `floom_nativeview_load_url` mean something.
 */
static BOOL is_loadable_url(NSURL *url) {
    if (url == nil) {
        return NO;
    }
    NSString *scheme = url.scheme;
    return scheme != nil && scheme.length > 0;
}

int floom_nativeview_load_url(FloomNativeSlot slot, const char *url,
                              char *out_error, uint32_t out_capacity) {
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view == nil || view.web == nil) {
        const char *reason = "no web view is bound to that slot";
        if (out_error != NULL && out_capacity > 0) {
            strncpy(out_error, reason, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }
    if (url == NULL) {
        const char *reason = "no URL was given";
        if (out_error != NULL && out_capacity > 0) {
            strncpy(out_error, reason, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }
    NSString *text = [NSString stringWithUTF8String:url];
    NSURL *target = text != nil ? [NSURL URLWithString:text] : nil;
    if (!is_loadable_url(target)) {
        NSString *reason = [NSString stringWithFormat:@"not a URL with a scheme: %s", url];
        if (out_error != NULL && out_capacity > 0) {
            strncpy(out_error, reason.UTF8String, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }
    g_last_url = text;
    [view.web loadRequest:[NSURLRequest requestWithURL:target]];
    return 0;
}

/* --- JS bridge ------------------------------------------------------ */

static void (*g_message_cb)(FloomNativeSlot, const char *) = NULL;

void floom_nativeview_set_message_callback(void (*cb)(FloomNativeSlot, const char *)) {
    g_message_cb = cb;
}

int floom_nativeview_post_message(FloomNativeSlot slot, const char *json,
                                  char *out_error, uint32_t out_capacity) {
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view == nil || view.web == nil) {
        const char *reason = "no web view is bound to that slot";
        if (out_error != NULL && out_capacity > 0) {
            strncpy(out_error, reason, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }
    NSString *js = [NSString stringWithFormat:
        @"if (window.floomNative && window.floomNative.onMessage) { window.floomNative.onMessage(%@); }",
        [NSString stringWithUTF8String:json ?: "{}"]];
    [view.web evaluateJavaScript:js completionHandler:nil];
    return 0;
}

/* Called from WKScriptMessageHandler shim below. */
void nativeview_handle_js_message(FloomNativeSlot slot, NSString *body) {
    if (g_message_cb == NULL) return;
    const char *utf8 = body.UTF8String;
    g_message_cb(slot, utf8 != nil ? utf8 : "");
}

/* The URL most recently handed to a web view, or NULL. */
const char *floom_nativeview_last_url(void) {
    return g_last_url != nil ? g_last_url.UTF8String : NULL;
}

/* The message the most recent refusal carried, or NULL when nothing was
 * refused. */
const char *floom_nativeview_last_refusal(void) {
    return g_last_refusal != nil ? g_last_refusal.UTF8String : NULL;
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

/* How many `create` calls were refused for a kind this plugin cannot build. */
int floom_nativeview_refused_count(void) {
    return g_refused;
}

/* How many participants are alive right now. */
int floom_nativeview_live_count(void) {
    return (int)g_views.count;
}

/* Whether the participant in `slot` is really in the window's content view. */
int floom_nativeview_slot_is_attached(FloomNativeSlot slot) {
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view == nil || g_window == nil) {
        return 0;
    }
    return [view.container.superview isEqual:g_window.contentView] ? 1 : 0;
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
 * Whether the participant in `slot` holds a real `WKWebView`.
 *
 * The check the capability claim rests on: "this plugin draws web views" is
 * only true if the object it built is a web view and not a container with
 * nothing in it.
 */
int floom_nativeview_slot_is_web(FloomNativeSlot slot) {
    FloomNativeView *view = g_views[slot_key(slot)];
    return view != nil && view.web != nil ? 1 : 0;
}
