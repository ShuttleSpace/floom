/*
 * The Linux half of the `webview` plugin.
 *
 * C11 + GTK3 with WebKitGTK 2.x, because that is what the host window the
 * host hands over lives inside and a plugin that contributes a native view
 * has to reach the toolkit that draws it. The contract it implements is the
 * C ABI shared across platforms (`floom_webview.h`): the plugin's own
 * four functions, `load_url`, the JS bridge, diagnostics, and the provider
 * vtable that says how the view is created, placed and torn down.
 *
 * No GTK main loop: the host owns the event loop; the widget repaints into
 * the host's own surface whenever GTK's event dispatch runs.
 *
 * # Kinds
 *
 * As on macOS, `create` refuses every kind it cannot build with a message
 * naming the kind — a plugin that silently substituted a blank view for a
 * camera would hand back a rectangle no one can tell apart from a view that
 * was never asked for.
 *
 * # Host handles, and why attach has two paths
 *
 * `attach` is given the host's window once, as an opaque pointer. On Linux
 * that is either a `GtkWidget *` — a `GtkFixed` or `GtkLayout` the host set
 * aside as the area native participants land in — or a raw X11 `Window`
 * (an XEmbed socket) widened to pointer width. A `Window` is a 32-bit XID,
 * so the two are told apart by a size check, not by dereferencing an
 * arbitrary integer: see `looks_like_widget`.
 *
 * Widget path: the web view is `gtk_fixed_put` / `gtk_layout_put` into the
 * host widget at `apply` time. Window path: `attach` wraps the XID in a
 * `GtkPlug` and the web view is a plain child of the plug.
 *
 * # A per-slot table
 *
 * Same reason as macOS: a scene can hold more than one participant at a
 * time, and one global view let the second `create` steal the first one's
 * handle. The slot identity includes a generation, so a recycled slot is a
 * different key by construction.
 */

#include <gtk/gtk.h>
/* GtkPlug/GtkSocket: `gtkx.h` is the only header GTK allows them to come
 * from. Without X11 the type does not exist, and the plug path is compiled
 * out below -- which leaves the widget path, the one a Wayland host uses. */
#include <gtk/gtkx.h>
/* WebKit2GTK 4.1, the GTK3 build: `webkit/webkit.h` is WebKit 6.0's GTK4
 * header, and everything below howls for GTK3 (`gtk_fixed_put`,
 * `gtk_window_resize`), so the two cannot be mixed. */
#include <webkit2/webkit2.h>

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "floom_webview.h"
#include "floom_native_provider.h"

/* --- what the provider remembers ---------------------------------------- */

/*
 * The host's window, one of:
 *   - `g_host_widget`: a GtkWidget (GtkFixed / GtkLayout) — child-of-host path
 *   - `g_host_plug`:   a GtkPlug built from an X11 Window XID — plug path
 * The host owns the handle; the plugin keeps the cast pointer and nothing
 * else.
 */
static int g_host_is_widget = 0;
static GtkWidget *g_host_widget = NULL;
static GtkWidget *g_host_plug = NULL;

static void webview_handle_js_message(FloomNativeSlot slot, const char *json);

/* One living participant. */
typedef struct FloomView {
    FloomNativeSlot slot;
    GtkWidget *web; /* the WebKitWebView; the participant itself */
    struct FloomView *next;
} FloomView;

static FloomView *g_views = NULL;

/*
 * Live participants, keyed by slot, newest first. The generation makes a
 * recycled slot a different key by construction.
 */
static FloomView *find_view(FloomNativeSlot slot) {
    for (FloomView *view = g_views; view != NULL; view = view->next) {
        if (view->slot.index == slot.index && view->slot.generation == slot.generation) {
            return view;
        }
    }
    return NULL;
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
static char *g_last_url = NULL;
/* What the last refusal said, so a host can read the true reason. */
static char *g_last_refusal = NULL;

/* --- shared helpers ------------------------------------------------------ */

/* Write a NUL-terminated message into the caller's error buffer, if it gave
 * one. Never overflow the caller's buffer, and always NUL-terminate: the
 * host treats the message as a C string. */
static void set_error(char *out_error, uint32_t out_capacity, const char *fmt, ...) {
    if (out_error == NULL || out_capacity == 0) {
        return;
    }
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(out_error, out_capacity, fmt, ap);
    va_end(ap);
}

/* The reason of the most recent refusal, replacing any previous one. */
static void remember_refusal(const char *msg) {
    free(g_last_refusal);
    g_last_refusal = strdup(msg != NULL ? msg : "");
}

static void remember_url(const char *url) {
    free(g_last_url);
    g_last_url = strdup(url != NULL ? url : "");
}

/* Initialize GTK at most once. The host owns the main loop; this only opens
 * the toolkit (input, display, fontconfig) so widgets can exist. */
static int gtk_ready(void) {
    static int ready = -1;
    if (ready < 0) {
        ready = gtk_init_check(NULL, NULL) ? 1 : 0;
    }
    return ready;
}

/*
 * Whether the host's opaque handle is a GtkWidget rather than a raw X11
 * Window.
 *
 * `GTK_IS_WIDGET` dereferences the pointer, so an integer XID passed as a
 * pointer would crash inside it. The check is size, not type: an X11 Window
 * is a 32-bit XID, so the widened value keeps its high bits zero, while a
 * live heap `GtkWidget *` on 64-bit Linux is a mmap address well above the
 * 4 GiB line. `GTK_IS_WIDGET` runs only when that held. On 32-bit Linux the
 * size test cannot fire, and the handle is assumed to be a widget — the XID
 * path there needs a build that knows the difference.
 */
static int looks_like_widget(const void *p) {
    if (p == NULL) {
        return 0;
    }
    if (sizeof(void *) > 4 && (uintptr_t)p <= 0xffffffffull) {
        return 0;
    }
    return GTK_IS_WIDGET(p) ? 1 : 0;
}

/* The scale factor the host places at, for the device-pixel division. */
static double host_scale(void) {
    GtkWidget *w = g_host_is_widget ? g_host_widget : g_host_plug;
    if (w != NULL) {
        int s = gtk_widget_get_scale_factor(w);
        if (s > 0) {
            return (double)s;
        }
    }
    return 1.0;
}

/* Whether the host's widget accepts placed children (GtkFixed / GtkLayout).
 * Any other container gets the web view added as a plain child and manages
 * the geometry itself. */
static int host_is_placeable(void) {
    if (!g_host_is_widget || g_host_widget == NULL) {
        return 0;
    }
    return GTK_IS_FIXED(g_host_widget) || GTK_IS_LAYOUT(g_host_widget);
}

/* --- the plugin's own four functions ------------------------------------- */

const char *floom_webview_version(void) {
    static const char version[] = "webview 0.2.0";
    return version;
}

int floom_webview_enable(void) {
    if (!gtk_ready()) {
        return 1;
    }
    return 0;
}

void floom_webview_disable(void) {
}

int floom_webview_capability(void) {
    return 0;
}

/* --- the provider -------------------------------------------------------- */

/*
 * `script-message-received::floomNative` fires with a `JSCValue` from 2.40
 * and a `WebKitJavascriptResult` before it; the same message arrives both
 * ways, only the wrapper changes.
 */
typedef struct {
    FloomNativeSlot slot;
} SlotData;

#if WEBKIT_CHECK_VERSION(2, 40, 0)
static void on_script_message(WebKitUserContentManager *manager, JSCValue *value,
                              gpointer user_data) {
    (void)manager;
    SlotData *data = (SlotData *)user_data;
    if (value == NULL) {
        return;
    }
    /* A string is the message itself; anything else (objects, arrays) goes
     * out as JSON, which is what the Rust callback expects. */
    char *json = jsc_value_is_string(value) ? jsc_value_to_string(value)
                                            : jsc_value_to_json(value, 0);
    webview_handle_js_message(data->slot, json != NULL ? json : "");
    g_free(json);
}
#else
static void on_script_message(WebKitUserContentManager *manager, WebKitJavascriptResult *result,
                              gpointer user_data) {
    (void)manager;
    SlotData *data = (SlotData *)user_data;
    if (result == NULL) {
        return;
    }
    JSCValue *value = webkit_javascript_result_get_js_value(result);
    char *json = jsc_value_is_string(value) ? g_strdup(jsc_value_to_string(value))
                                            : jsc_value_to_json(value, 0);
    webview_handle_js_message(data->slot, json != NULL ? json : "");
    g_free(json);
}
#endif

static int32_t webview_create(void *self, FloomNativeSlot slot, uint32_t kind,
                                 const char *kind_name, char *out_error,
                                 uint32_t out_capacity) {
    (void)self;

    if (kind != FLOOM_NATIVE_KIND_WEB) {
        /*
         * Every other kind is refused. The message names what was asked for,
         * because "the plugin said no" without the kind is the kind of error
         * that sends someone reading the C ABI to find out which argument was
         * wrong.
         */
        char msg[256];
        if (kind_name != NULL) {
            snprintf(msg, sizeof(msg), "this plugin builds web views only, not kind %u (%s)",
                     kind, kind_name);
        } else {
            snprintf(msg, sizeof(msg), "this plugin builds web views only, not kind %u", kind);
        }
        g_refused += 1;
        remember_refusal(msg);
        set_error(out_error, out_capacity, "%s", msg);
        return 1;
    }

    if (!gtk_ready()) {
        set_error(out_error, out_capacity, "GTK could not initialize");
        return 1;
    }

    FloomView *view = calloc(1, sizeof(FloomView));
    if (view == NULL) {
        set_error(out_error, out_capacity, "out of memory");
        return 1;
    }
    view->slot = slot;

    /*
     * `WebKitWebView` rather than the deprecated GtkHTML family. Built with
     * its own user content manager: the manager carries the JS bridge for
     * this participant and dies with the view, so two participants never
     * hear each other's messages.
     */
    WebKitUserContentManager *ucm = webkit_user_content_manager_new();
    /* JS -> Rust bridge: inject a `window.floomNative` shim. */
    const char *shim =
        "window.floomNative = window.floomNative || { postMessage: function(m){"
        " window.webkit.messageHandlers.floomNative.postMessage(m); } };";
    WebKitUserScript *script = webkit_user_script_new(
        shim, WEBKIT_USER_CONTENT_INJECT_ALL_FRAMES, WEBKIT_USER_SCRIPT_INJECT_AT_DOCUMENT_START,
        NULL, NULL);
    webkit_user_content_manager_add_script(ucm, script);
    webkit_user_script_unref(script);
    webkit_user_content_manager_register_script_message_handler(ucm, "floomNative");
    SlotData *data = g_new(SlotData, 1);
    data->slot = slot;
    g_signal_connect(ucm, "script-message-received::floomNative", G_CALLBACK(on_script_message),
                     data);

    view->web = webkit_web_view_new_with_user_content_manager(ucm);
    /* The view holds its own reference to the manager; ours goes here. */
    g_object_unref(ucm);

    /*
     * Spoof a desktop Chrome UA. A bare WebKitGTK UA makes YouTube (and
     * similar properties) serve a stripped page — as on macOS, because they
     * fingerprint the UA and opt out of the full experience. Chrome's UA is
     * the one they target.
     */
    WebKitSettings *settings = webkit_web_view_get_settings(WEBKIT_WEB_VIEW(view->web));
    webkit_settings_set_user_agent(
        settings,
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
        "Chrome/120.0.0.0 Safari/537.36");
    /* Developer extras: the inspector, reachable from the view's context
     * menu — WebKitGTK's "Inspect Element" asks for exactly this. */
    webkit_settings_set_enable_developer_extras(settings, TRUE);

    gtk_widget_show(view->web);

    /*
     * The widget path parents nothing yet: the web view is `gtk_fixed_put`
     * (or `gtk_layout_put`) into the host widget at `apply` time, once the
     * host has told us where. The plug path needs a parent to exist now,
     * because the plug *is* the participant's window.
     */
    if (!g_host_is_widget && g_host_plug != NULL) {
        gtk_container_add(GTK_CONTAINER(g_host_plug), view->web);
        gtk_widget_show(g_host_plug);
    }

    view->next = g_views;
    g_views = view;
    g_created += 1;
    return 0;
}

static void webview_destroy(void *self, FloomNativeSlot slot) {
    (void)self;
    FloomView **link = &g_views;
    while (*link != NULL) {
        FloomView *view = *link;
        if (view->slot.index == slot.index && view->slot.generation == slot.generation) {
            *link = view->next;
            /* Destroys the web view with it. The content manager and its
             * signal die here too, before the SlotData it hangs off does. */
            gtk_widget_destroy(view->web);
            g_free(view);
            g_destroyed += 1;
            return;
        }
        link = &view->next;
    }
    /* Must tolerate a slot it never created. */
    g_destroyed += 1;
}

/*
 * Place the view.
 *
 * The rect arrives in whole device pixels; GTK's coordinates are logical,
 * so it is divided by the host's scale factor. That division is the
 * platform's own business — the framework snapped to whole device pixels
 * exactly once, and this is the other side of that boundary.
 *
 * Applying is absolute and idempotent: `move` plus `size_request` overwrite
 * the previous state entirely, so a missed frame self-heals instead of
 * accumulating error.
 */
static void webview_apply(void *self, FloomNativeSlot slot,
                             const FloomNativeState *state) {
    (void)self;
    if (state == NULL) {
        return;
    }
    FloomView *view = find_view(slot);
    if (view == NULL) {
        return;
    }
    g_applied += 1;

    const double scale = host_scale();
    g_last_x = (double)state->rect.x / scale;
    g_last_y = (double)state->rect.y / scale;
    g_last_w = (double)state->rect.width / scale;
    g_last_h = (double)state->rect.height / scale;

    if (g_host_is_widget) {
        GtkWidget *parent = gtk_widget_get_parent(view->web);
        /* First apply parents the view: `put` once, `move` from then on. */
        if (parent == NULL && host_is_placeable()) {
            if (GTK_IS_FIXED(g_host_widget)) {
                gtk_fixed_put(GTK_FIXED(g_host_widget), view->web, (gint)g_last_x,
                              (gint)g_last_y);
            } else {
                gtk_layout_put(GTK_LAYOUT(g_host_widget), view->web, (gint)g_last_x,
                               (gint)g_last_y);
            }
            parent = g_host_widget;
        } else if (parent == NULL && GTK_IS_CONTAINER(g_host_widget)) {
            /* The host's widget is not placeable; it owns the geometry and
             * this call can only put the view somewhere in it. */
            gtk_container_add(GTK_CONTAINER(g_host_widget), view->web);
            parent = g_host_widget;
        }
        if (parent == NULL) {
            return;
        }
        if (GTK_IS_FIXED(parent)) {
            gtk_fixed_move(GTK_FIXED(parent), view->web, (gint)g_last_x, (gint)g_last_y);
        } else if (GTK_IS_LAYOUT(parent)) {
            gtk_layout_move(GTK_LAYOUT(parent), view->web, (gint)g_last_x, (gint)g_last_y);
        }
        /* GtkFixed and GtkLayout allocate children at their requisition, so
         * the size request *is* the size allocation here. */
        gtk_widget_set_size_request(view->web, (gint)g_last_w, (gint)g_last_h);
        if (state->visible) {
            gtk_widget_show(view->web);
        } else {
            gtk_widget_hide(view->web);
        }
    } else if (g_host_plug != NULL) {
        /* The plug is the embedded top level; it is the geometry the host's
         * X window sees. The web view fills it. */
        gtk_window_resize(GTK_WINDOW(g_host_plug), (gint)g_last_w, (gint)g_last_h);
        gtk_widget_set_size_request(view->web, (gint)g_last_w, (gint)g_last_h);
        if (state->visible) {
            gtk_widget_show(view->web);
            gtk_widget_show(g_host_plug);
        } else {
            gtk_widget_hide(view->web);
        }
    }
}

static void webview_focus(void *self, FloomNativeSlot slot) {
    (void)self;
    FloomView *view = find_view(slot);
    if (view != NULL && view->web != NULL) {
        gtk_widget_grab_focus(view->web);
    }
    g_focused += 1;
}

static void webview_blur(void *self, FloomNativeSlot slot) {
    (void)self;
    FloomView *view = find_view(slot);
    if (view == NULL || view->web == NULL) {
        return;
    }
    /* Hand focus back to the parent: the GTK toplevel clears its focus
     * widget, and the host's own surface receives keyboard events again. */
    GtkWidget *top = gtk_widget_get_toplevel(view->web);
    if (GTK_IS_WINDOW(top)) {
        gtk_window_set_focus(GTK_WINDOW(top), NULL);
        return;
    }
    GtkWidget *parent = gtk_widget_get_parent(view->web);
    if (GTK_IS_WIDGET(parent)) {
        gtk_widget_grab_focus(parent);
    }
}

/*
 * The host's window, given once.
 *
 * Two shapes reach here: a container the plugin can parent a child into
 * (`GtkFixed`, `GtkLayout`, any `GtkContainer`) and a `GtkPlug` built from an
 * X11 `Window` id. A `GtkPlug` is itself a widget, so the plug is asked about
 * first — otherwise every plug would be treated as a plain container and the
 * view would be parented into it as if it were a box.
 */
static int32_t webview_attach(void *self, const void *host_window) {
    (void)self;
    if (host_window == NULL) {
        g_host_is_widget = 0;
        g_host_widget = NULL;
        g_host_plug = NULL;
        return 0;
    }
    GtkWidget *widget = (GtkWidget *)host_window;
#ifdef GDK_WINDOWING_X11
    if (GTK_IS_PLUG(widget)) {
        g_host_plug = widget;
        g_host_widget = NULL;
        g_host_is_widget = 0;
        return 0;
    }
#endif
    {
        g_host_widget = widget;
        g_host_plug = NULL;
        g_host_is_widget = 1;
    }
    return 0;
}

static FloomNativeCapabilities webview_capabilities(void *self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay only: the web view is a GTK child of the host's widget (or of a
     * plug), so it composites above Floom's own surface and never inside it. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.mode_count = 1;
    caps.snapshot = 0;
    /* The pointer belongs to the page: links, selection and scrolling all need
     * clicks to arrive. A passthrough web view is a picture. */
    caps.input_passthrough = 0;
    caps.input_capture = 1;
    caps.focusable = 1;
    /* GTK crops a child's allocation when the parent clips it, so a rect that
     * is smaller than the view is honoured rather than rescaled. */
    caps.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return caps;
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

/*
 * The Rust half needs the vtable, and the only party that knows when the
 * native code is ready is the native code itself. A constructor runs before
 * `main`, which is early enough: the Rust half reads the pointer when the
 * app asks for the provider, not before.
 */
__attribute__((constructor)) static void webview_register(void) {
    floom_webview_register_provider(&g_provider);
}

/* --- the web view -------------------------------------------------------- */

/*
 * Load a URL into a live web participant.
 *
 * Separate from `create` because the framework's provider ABI has no
 * "navigate" verb: a participant is created, placed and destroyed, and what
 * a web view *shows* is the web's own business, not the compositor's. The
 * host calls this through the plugin's own ABI after binding a `Web`
 * participant.
 *
 * Returns 0 when the URL was handed to WebKit, non-zero when the slot holds
 * no web view (a participant of another kind, or one already destroyed) or
 * the URL could not be parsed. `Ok` claims the load was *started*, not that
 * the page arrived — the page's own loading is asynchronous and belongs to
 * WebKit.
 *
 * Whether `url` is a URL a web view can be pointed at: WebKit's own loader
 * resolves a scheme-less string against a `file://` base and produces a
 * surprising page, so — as on macOS — a non-empty scheme is required. That
 * is the check that makes an error here mean something.
 */
int floom_webview_load_url(FloomNativeSlot slot, const char *url,
                              char *out_error, uint32_t out_capacity) {
    FloomView *view = find_view(slot);
    if (view == NULL || view->web == NULL) {
        set_error(out_error, out_capacity, "no web view is bound to that slot");
        return 1;
    }
    if (url == NULL) {
        set_error(out_error, out_capacity, "no URL was given");
        return 1;
    }
    const char *scheme_end = strstr(url, "://");
    if (scheme_end == NULL || scheme_end == url) {
        set_error(out_error, out_capacity, "not a URL with a scheme: %s", url);
        return 1;
    }
    remember_url(url);
    webkit_web_view_load_uri(WEBKIT_WEB_VIEW(view->web), url);
    return 0;
}

/* --- JS bridge ------------------------------------------------------ */

static void (*g_message_cb)(FloomNativeSlot, const char *) = NULL;

void floom_webview_set_message_callback(void (*cb)(FloomNativeSlot, const char *)) {
    g_message_cb = cb;
}

int floom_webview_post_message(FloomNativeSlot slot, const char *json,
                                  char *out_error, uint32_t out_capacity) {
    FloomView *view = find_view(slot);
    if (view == NULL || view->web == NULL) {
        set_error(out_error, out_capacity, "no web view is bound to that slot");
        return 1;
    }
    const char *text = json != NULL ? json : "{}";
    const char *prefix =
        "if (window.floomNative && window.floomNative.onMessage)"
        " { window.floomNative.onMessage(";
    const char *suffix = "); }";
    size_t need = strlen(prefix) + strlen(text) + strlen(suffix) + 1;
    char *js = malloc(need);
    if (js == NULL) {
        set_error(out_error, out_capacity, "out of memory");
        return 1;
    }
    snprintf(js, need, "%s%s%s", prefix, text, suffix);
    webkit_web_view_evaluate_javascript(WEBKIT_WEB_VIEW(view->web), js, (gssize)-1, NULL,
                                        "floom-webview", NULL, NULL, NULL);
    free(js);
    return 0;
}

/* Called from the `script-message-received::floomNative` handler above. */
static void webview_handle_js_message(FloomNativeSlot slot, const char *json) {
    if (g_message_cb == NULL) {
        return;
    }
    g_message_cb(slot, json != NULL ? json : "");
}

/* The URL most recently handed to a web view, or NULL. */
const char *floom_webview_last_url(void) {
    return g_last_url;
}

/* The message the most recent refusal carried, or NULL when nothing was
 * refused. */
const char *floom_webview_last_refusal(void) {
    return g_last_refusal;
}

/* --- diagnostics --------------------------------------------------------- */

/*
 * What the provider did, for a host that wants to check.
 *
 * Plain C, so a test — or an app's own diagnostics — can read it without
 * knowing anything about GTK or WebKit.
 */

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
    int n = 0;
    for (FloomView *view = g_views; view != NULL; view = view->next) {
        n += 1;
    }
    return n;
}

/* Whether the participant in `slot` is really in the host's widget (or plug).
 * Widget path checks the parent link; plug path checks the plug is it. */
int floom_webview_slot_is_attached(FloomNativeSlot slot) {
    FloomView *view = find_view(slot);
    if (view == NULL || view->web == NULL) {
        return 0;
    }
    if (g_host_is_widget) {
        return g_host_widget != NULL && gtk_widget_get_parent(view->web) == g_host_widget ? 1 : 0;
    }
    if (g_host_plug != NULL) {
        return gtk_widget_get_parent(view->web) == GTK_WIDGET(g_host_plug) ? 1 : 0;
    }
    return 0;
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
 * Whether the participant in `slot` holds a real `WebKitWebView`.
 *
 * The check the capability claim rests on: "this plugin draws web views" is
 * only true if the object it built is a web view and not a container with
 * nothing in it.
 */
int floom_webview_slot_is_web(FloomNativeSlot slot) {
    FloomView *view = find_view(slot);
    return view != NULL && view->web != NULL ? 1 : 0;
}
