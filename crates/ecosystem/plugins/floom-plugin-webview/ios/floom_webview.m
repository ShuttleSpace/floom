/*
 * The iOS half of the `webview` plugin.
 *
 * Objective-C, because that is what UIKit is written in and a plugin that
 * contributes a native view has to reach the framework it contributes to. The
 * contract it implements is the C ABI in `../common/`: the plugin's own four
 * functions, and the provider vtable that says how the view is created,
 * placed and torn down.
 *
 * This is the macOS half ported to UIKit, with the pieces that are macOS-only
 * dropped rather than faked: the "Inspect Element" context menu and the
 * inspector shims have no iOS equivalent, so the view here is a plain web
 * view with the JS bridge and nothing layered on top.
 *
 * The provider is asked for the host's window once, in `attach`, because a
 * provider is an instance per window and the window does not change over its
 * life.
 *
 * # Kinds
 *
 * `FLOOM_NATIVE_KIND_WEB` builds a `WKWebView`; every other kind is *refused*
 * with a message naming it. A refusal is the right answer, not a limitation
 * to work around: a plugin that silently substituted a blank view for a
 * camera would hand back a rectangle no one can tell apart from a view that
 * was never asked for.
 */

#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>

#include "floom_webview.h"
#include "floom_native_provider.h"

#include <string.h>

static void webview_handle_js_message(FloomNativeSlot slot, NSString *body);

/* --- what the provider remembers ---------------------------------------- */

/* The host's root view. Views go into it directly: on iOS the window does
 * nothing by itself, and the host hands its `UIView *` for exactly this. */
static UIView *g_host = nil;

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
    webview_handle_js_message(self.slot, body);
}
@end

@interface FloomNativeView : NSObject
@property(nonatomic, strong) UIView *container;
@property(nonatomic, strong) UIView *content;
@property(nonatomic, strong) WKWebView *web;
@end

@implementation FloomNativeView
@end

static NSMutableDictionary<NSString *, FloomNativeView *> *g_views = nil;

static void (*g_message_cb)(FloomNativeSlot, const char *) = NULL;

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

static NSString *g_last_url = nil;
static NSString *g_last_refusal = nil;

/* --- the plugin's own four functions ------------------------------------- */

const char *floom_webview_version(void) {
    static const char version[] = "webview 0.2.0";
    return version;
}

int floom_webview_enable(void) {
    if (g_views == nil) {
        g_views = [NSMutableDictionary dictionary];
    }
    return 0;
}

void floom_webview_disable(void) {
}

int floom_webview_capability(void) {
    return 0;
}

/* --- the provider -------------------------------------------------------- */

static int32_t webview_attach(void *self, const void *host_window) {
    (void)self;
    /* The bridged cast is what ARC requires: the host view belongs to the
     * host, so the plugin does not retain it. */
    g_host = (__bridge UIView *)host_window;
    if (g_views != nil && g_host != nil) {
        /* Views created before the host was known go into it now. */
        for (FloomNativeView *view in g_views.allValues) {
            [g_host addSubview:view.container];
        }
    }
    return 0;
}

static FloomNativeCapabilities webview_capabilities(void *self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay only: the views are siblings of the framework's own surface. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.mode_count = 1;
    caps.snapshot = 0;
    /* A web view takes the pointer — links, text selection and scrolling all
     * need touches to reach it. */
    caps.input_passthrough = 0;
    caps.input_capture = 1;
    /* Focusable: a web page has text fields, and a form the user cannot type
     * into is not an embedded browser. */
    caps.focusable = 1;
    caps.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return caps;
}

static int32_t webview_create(void *self, FloomNativeSlot slot, uint32_t kind,
                                 const char *kind_name, char *out_error,
                                 uint32_t out_capacity) {
    (void)self;
    if (kind != FLOOM_NATIVE_KIND_WEB) {
        NSString *unsupported = [NSString
            stringWithFormat:@"this plugin builds web views only, not kind %u%s%s",
                             kind, kind_name != NULL ? " (" : "",
                             kind_name != NULL ? kind_name : ""];
        if (kind_name != NULL) {
            unsupported = [unsupported stringByAppendingString:@")"];
        }
        g_refused += 1;
        g_last_refusal = unsupported;
        if (out_error != NULL && out_capacity > 0) {
            strncpy(out_error, unsupported.UTF8String, out_capacity - 1);
            out_error[out_capacity - 1] = '\0';
        }
        return 1;
    }

    FloomNativeView *view = [FloomNativeView new];
    view.container = [[UIView alloc] initWithFrame:CGRectMake(0.0, 0.0, 100.0, 100.0)];

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
    view.web = [[WKWebView alloc] initWithFrame:CGRectMake(0.0, 0.0, 100.0, 100.0)
                                  configuration:config];
    view.content = view.web;
    /* Spoof a mobile Safari-grade UA. Sites fingerprint the UA and serve a
     * stripped page to a bare WKWebView UA; Safari's own string is the one
     * they target on this platform. */
    view.web.customUserAgent = @"Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1";
    if (@available(iOS 16.0, *)) {
        view.web.inspectable = YES;
    }

    [view.container addSubview:view.content];
    /* The content follows the container on every resize. */
    view.container.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    view.content.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    if (g_host != nil) {
        /* The view is a sibling of the framework's own surface. */
        [g_host addSubview:view.container];
    }
    if (g_views == nil) {
        g_views = [NSMutableDictionary dictionary];
    }
    g_views[slot_key(slot)] = view;
    g_created += 1;
    return 0;
}

static void webview_destroy(void *self, FloomNativeSlot slot) {
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
 * The rect arrives in whole device pixels; UIKit frames are in points, so it
 * is divided by the screen scale. That division is the platform's own
 * business — the framework snapped to whole device pixels exactly once, and
 * this is the other side of that boundary.
 */
static void webview_apply(void *self, FloomNativeSlot slot,
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

    const double scale =
        (g_host != nil && g_host.window != nil) ? g_host.window.screen.scale
                                                : [UIScreen mainScreen].scale;
    g_last_x = (double)state->rect.x / scale;
    g_last_y = (double)state->rect.y / scale;
    g_last_w = (double)state->rect.width / scale;
    g_last_h = (double)state->rect.height / scale;

    view.container.frame = CGRectMake(g_last_x, g_last_y, g_last_w, g_last_h);
    view.content.frame = CGRectMake(0.0, 0.0, g_last_w, g_last_h);
}

static void webview_focus(void *self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view != nil && view.web != nil) {
        [view.web becomeFirstResponder];
    }
    g_focused += 1;
}

static void webview_blur(void *self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view != nil && view.web != nil && view.web.isFirstResponder) {
        [view.web resignFirstResponder];
    }
}

static FloomNativeProviderV1 g_provider = {
    FLOOM_NATIVE_PROVIDER_ABI_VERSION,
    NULL,
    webview_attach,
    webview_capabilities,
    webview_create,
    webview_destroy,
    webview_apply,
    webview_focus,
    webview_blur,
};

/* --- handing the provider to the Rust half ------------------------------ */

__attribute__((constructor)) static void webview_register(void) {
    floom_webview_register_provider(&g_provider);
}

/* --- the web view -------------------------------------------------------- */

/*
 * Whether `url` is a URL a web view can be pointed at.
 *
 * `NSURL URLWithString:` parses "not a url" happily and hands back an object
 * with no scheme, which WebKit resolves into a blank page later rather than a
 * refusal now. So a scheme is required, and it has to be non-empty.
 */
static BOOL is_loadable_url(NSURL *url) {
    if (url == nil) {
        return NO;
    }
    NSString *scheme = url.scheme;
    return scheme != nil && scheme.length > 0;
}

int floom_webview_load_url(FloomNativeSlot slot, const char *url,
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

void floom_webview_set_message_callback(void (*cb)(FloomNativeSlot, const char *)) {
    g_message_cb = cb;
}

int floom_webview_post_message(FloomNativeSlot slot, const char *json,
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

void webview_handle_js_message(FloomNativeSlot slot, NSString *body) {
    if (g_message_cb == NULL) return;
    const char *utf8 = body.UTF8String;
    g_message_cb(slot, utf8 != nil ? utf8 : "");
}

/* The URL most recently handed to a web view, or NULL. */
const char *floom_webview_last_url(void) {
    return g_last_url != nil ? g_last_url.UTF8String : NULL;
}

/* The message the most recent refusal carried, or NULL when nothing was
 * refused. */
const char *floom_webview_last_refusal(void) {
    return g_last_refusal != nil ? g_last_refusal.UTF8String : NULL;
}

/* --- diagnostics --------------------------------------------------------- */

int floom_webview_created_count(void) {
    return g_created;
}

int floom_webview_destroyed_count(void) {
    return g_destroyed;
}

int floom_webview_applied_count(void) {
    return g_applied;
}

int floom_webview_focused_count(void) {
    return g_focused;
}

/* How many `create` calls were refused for a kind this plugin cannot build. */
int floom_webview_refused_count(void) {
    return g_refused;
}

/* How many participants are alive right now. */
int floom_webview_live_count(void) {
    return (int)g_views.count;
}

/* Whether the participant in `slot` is really in the host view. */
int floom_webview_slot_is_attached(FloomNativeSlot slot) {
    FloomNativeView *view = g_views[slot_key(slot)];
    if (view == nil || g_host == nil) {
        return 0;
    }
    return [view.container.superview isEqual:g_host] ? 1 : 0;
}

void floom_webview_last_frame(double *x, double *y, double *w, double *h) {
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
int floom_webview_slot_is_web(FloomNativeSlot slot) {
    FloomNativeView *view = g_views[slot_key(slot)];
    return view != nil && view.web != nil ? 1 : 0;
}
