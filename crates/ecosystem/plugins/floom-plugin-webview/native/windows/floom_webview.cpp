/*
 * The Windows half of the `webview` plugin.
 *
 * C++17, plain Win32 plus WebView2. A participant is a child `HWND` of the
 * host window, and the web view in it is an `ICoreWebView2Controller` bound
 * to that `HWND`. The contract is the one the macOS half implements: the
 * plugin's own four functions, the provider vtable, the diagnostics.
 *
 * The shared plugin header lives in `../macos/floom_webview.h`; the include
 * path for it and for the provider ABI header comes from the build wiring, as
 * on macOS. Link `WebView2LoaderStatic.lib` (or `WebView2Loader.lib`) beside
 * the usual user32 / gdi32 / ole32.
 *
 * WebView2's environment and controller creation is asynchronous while this
 * ABI is synchronous, so `create` makes the child `HWND` synchronously and
 * starts the initialization; a navigation that arrives before the controller
 * completes is queued on the record and flushed at completion.
 *
 * Every entry point is expected on the thread that owns the host `HWND` and
 * pumps its messages: that is where WebView2's completion callbacks fire.
 */

#ifdef _WIN32

#include <windows.h>
#include <objbase.h>

#include <WebView2.h>
#include <wrl/client.h>

#include <algorithm>
#include <cctype>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <unordered_map>
#include <utility>

#include "floom_webview.h"
#include "floom_native_provider.h"

namespace {

/* --- what the provider remembers ----------------------------------------- */

struct WebViewRecord {
    HWND hwnd = nullptr; /* the child window, owned here */
    Microsoft::WRL::ComPtr<ICoreWebView2Controller> controller;
    Microsoft::WRL::ComPtr<ICoreWebView2> web;
    Microsoft::WRL::ComPtr<ICoreWebView2_2> web2;
    EventRegistrationToken message_token = {};
    bool message_hooked = false;
    bool webview_ok = false; /* the controller completed */
    std::string pending_url; /* flushes once the controller exists */
};

typedef std::unordered_map<std::string, WebViewRecord> ViewTable;
ViewTable* g_views = nullptr; /* keyed by "index:generation" */

HWND g_host_hwnd = nullptr;

void ensure_table() {
    if (g_views == nullptr) {
        g_views = new ViewTable();
    }
}

std::string slot_key(FloomNativeSlot slot) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%u:%u", (unsigned)slot.index, (unsigned)slot.generation);
    return std::string(buf);
}

WebViewRecord* find_record(FloomNativeSlot slot) {
    if (g_views == nullptr) {
        return nullptr;
    }
    auto it = g_views->find(slot_key(slot));
    return it != g_views->end() ? &it->second : nullptr;
}

/* --- diagnostics --------------------------------------------------------- */

int g_created = 0;
int g_destroyed = 0;
int g_applied = 0;
int g_focused = 0;
int g_refused = 0;
double g_last_x = 0.0;
double g_last_y = 0.0;
double g_last_w = 0.0;
double g_last_h = 0.0;

/* The URL the most recent web view was asked to load, for a host to check. */
std::string g_last_url;
/* What the last refusal said, so a host can read the true reason. */
std::string g_last_refusal;

void (*g_message_cb)(FloomNativeSlot, const char*) = nullptr;

/* --- text helpers -------------------------------------------------------- */

/* Never overflow the caller's buffer, and always NUL-terminate: the host
 * treats the message as a C string. */
void set_error(char* out_error, uint32_t capacity, const char* message) {
    if (out_error == nullptr || capacity == 0) {
        return;
    }
    size_t len = strlen(message);
    if (len > capacity - 1) {
        len = capacity - 1;
    }
    memcpy(out_error, message, len);
    out_error[len] = '\0';
}

void set_errorf(char* out_error, uint32_t capacity, const char* fmt, ...) {
    char buf[512];
    va_list args;
    va_start(args, fmt);
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    set_error(out_error, capacity, buf);
}

std::wstring utf8_to_wide(const char* text) {
    if (text == nullptr) {
        return std::wstring();
    }
    int n = MultiByteToWideChar(CP_UTF8, 0, text, -1, nullptr, 0);
    if (n <= 0) {
        return std::wstring();
    }
    std::wstring out(static_cast<size_t>(n) - 1, L'\0');
    if (!out.empty()) {
        MultiByteToWideChar(CP_UTF8, 0, text, -1, &out[0], n);
    }
    return out;
}

std::string wide_to_utf8(const wchar_t* text) {
    if (text == nullptr) {
        return std::string();
    }
    int n = WideCharToMultiByte(CP_UTF8, 0, text, -1, nullptr, 0, nullptr, nullptr);
    if (n <= 0) {
        return std::string();
    }
    std::string out(static_cast<size_t>(n) - 1, '\0');
    if (!out.empty()) {
        WideCharToMultiByte(CP_UTF8, 0, text, -1, &out[0], n, nullptr, nullptr);
    }
    return out;
}

/*
 * Whether `url` has the scheme a Navigate call needs.
 *
 * `ICoreWebView2::Navigate` is as permissive as its WinInet roots: it accepts
 * strings no server can resolve, and the failure surfaces later as a blank
 * page rather than as a refused call. A scheme, present and non-empty, is the
 * same check the macOS half applies.
 */
bool has_scheme(const char* url) {
    if (url == nullptr) {
        return false;
    }
    const char* p = url;
    if (!isalpha(static_cast<unsigned char>(*p))) {
        return false;
    }
    for (p++; isalnum(static_cast<unsigned char>(*p)) || *p == '+' || *p == '-' ||
              *p == '.';
         p++) {
    }
    return *p == ':';
}

/* --- the child window ---------------------------------------------------- */

LRESULT CALLBACK webview_wndproc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
    return DefWindowProcW(hwnd, message, wparam, lparam);
}

bool ensure_window_class() {
    static ATOM atom = 0;
    static bool tried = false;
    if (tried) {
        return atom != 0;
    }
    tried = true;
    WNDCLASSEXW wc;
    memset(&wc, 0, sizeof(wc));
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = webview_wndproc;
    wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = L"FloomWebViewWindow";
    atom = RegisterClassExW(&wc);
    return atom != 0;
}

/* A view created before the host window was known is a popup until it has
 * somewhere to be; the first attach gives it a real parent. */
void attach_to_host(HWND hwnd) {
    if (hwnd == nullptr || g_host_hwnd == nullptr || GetParent(hwnd) == g_host_hwnd) {
        return;
    }
    SetWindowLongPtrW(hwnd, GWL_STYLE, WS_CHILD | WS_CLIPSIBLINGS);
    SetParent(hwnd, g_host_hwnd);
}

/* --- WebView2 completion handlers --------------------------------------- */

/*
 * WebView2 initializes in two asynchronous steps and this ABI is synchronous,
 * so the completion work has to re-find its record. The handlers hold only a
 * copy of the slot, never a pointer into the table: a record destroyed while
 * its web view was initializing is simply not found and the callback drops.
 */

class ControllerHandler;
class WebMessageHandler;

class EnvironmentHandler final : public ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler {
  public:
    explicit EnvironmentHandler(FloomNativeSlot slot) : slot_(slot), ref_(1) {}

    STDMETHODIMP QueryInterface(REFIID riid, void** result) noexcept override {
        if (result == nullptr) {
            return E_POINTER;
        }
        *result = nullptr;
        if (riid == __uuidof(IUnknown) ||
            riid == __uuidof(ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler)) {
            *result = static_cast<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() noexcept override { return static_cast<ULONG>(++ref_); }

    STDMETHODIMP_(ULONG) Release() noexcept override {
        ULONG remaining = static_cast<ULONG>(--ref_);
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    STDMETHODIMP Invoke(HRESULT error, ICoreWebView2Environment* environment) noexcept override {
        WebViewRecord* rec = find_record(slot_);
        if (rec == nullptr) {
            return S_OK; /* destroyed while initializing */
        }
        if (FAILED(error) || environment == nullptr) {
            OutputDebugStringW(L"floom: WebView2 environment failed to initialize\r\n");
            return S_OK;
        }
        ControllerHandler* controller_handler = new ControllerHandler(slot_);
        HRESULT hr = environment->CreateCoreWebView2Controller(rec->hwnd, controller_handler);
        controller_handler->Release();
        if (FAILED(hr)) {
            OutputDebugStringW(L"floom: CreateCoreWebView2Controller failed\r\n");
        }
        return S_OK;
    }

  private:
    FloomNativeSlot slot_;
    long ref_;
};

class ControllerHandler final
    : public ICoreWebView2CreateCoreWebView2ControllerCompletedHandler {
  public:
    explicit ControllerHandler(FloomNativeSlot slot) : slot_(slot), ref_(1) {}

    STDMETHODIMP QueryInterface(REFIID riid, void** result) noexcept override {
        if (result == nullptr) {
            return E_POINTER;
        }
        *result = nullptr;
        if (riid == __uuidof(IUnknown) ||
            riid == __uuidof(ICoreWebView2CreateCoreWebView2ControllerCompletedHandler)) {
            *result =
                static_cast<ICoreWebView2CreateCoreWebView2ControllerCompletedHandler*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() noexcept override { return static_cast<ULONG>(++ref_); }

    STDMETHODIMP_(ULONG) Release() noexcept override {
        ULONG remaining = static_cast<ULONG>(--ref_);
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    STDMETHODIMP Invoke(HRESULT error, ICoreWebView2Controller* controller) noexcept override {
        WebViewRecord* rec = find_record(slot_);
        if (rec == nullptr) {
            return S_OK;
        }
        if (FAILED(error) || controller == nullptr) {
            OutputDebugStringW(L"floom: WebView2 controller failed to create\r\n");
            return S_OK;
        }
        rec->controller = controller;
        controller->get_CoreWebView2(&rec->web);

        /* Developer tools: the desktop-Edge UA is already the one pages
         * target, so unlike macOS there is no user-agent spoof to apply. */
        Microsoft::WRL::ComPtr<ICoreWebView2Settings> settings;
        if (rec->web != nullptr && SUCCEEDED(rec->web->get_Settings(&settings))) {
            settings->put_AreDevToolsEnabled(TRUE);
            Microsoft::WRL::ComPtr<ICoreWebView2Settings2> settings2;
            (void)settings.As(&settings2);
        }

        /* JS -> Rust bridge: inject a `window.floomNative` shim that posts
         * through the WebView2 message channel. */
        if (rec->web != nullptr) {
            (void)rec->web->AddScriptToExecuteOnDocumentCreated(
                L"window.floomNative = window.floomNative || { postMessage: "
                L"function(m){ window.chrome.webview.postMessage(m); } };",
                nullptr);
            (void)rec->web.As(&rec->web2);
            WebMessageHandler* message_handler = new WebMessageHandler(slot_);
            HRESULT hr = rec->web2->add_WebMessageReceived(message_handler, &rec->message_token);
            if (FAILED(hr)) {
                rec->message_token = {};
            } else {
                rec->message_hooked = true;
            }
            message_handler->Release();
        }

        /* The child window carries the placement, so the web view fills it. */
        RECT bounds = {0, 0, 0, 0};
        if (rec->hwnd != nullptr) {
            GetClientRect(rec->hwnd, &bounds);
        }
        controller->put_Bounds(bounds);

        rec->webview_ok = true;

        /* Anything asked of the web view before the controller existed. */
        if (!rec->pending_url.empty() && rec->web != nullptr) {
            std::wstring url = utf8_to_wide(rec->pending_url.c_str());
            rec->pending_url.clear();
            (void)rec->web->Navigate(url.c_str());
        }
        return S_OK;
    }

  private:
    FloomNativeSlot slot_;
    long ref_;
};

class WebMessageHandler final : public ICoreWebView2WebMessageReceivedHandler {
  public:
    explicit WebMessageHandler(FloomNativeSlot slot) : slot_(slot), ref_(1) {}

    STDMETHODIMP QueryInterface(REFIID riid, void** result) noexcept override {
        if (result == nullptr) {
            return E_POINTER;
        }
        *result = nullptr;
        if (riid == __uuidof(IUnknown) ||
            riid == __uuidof(ICoreWebView2WebMessageReceivedHandler)) {
            *result = static_cast<ICoreWebView2WebMessageReceivedHandler*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() noexcept override { return static_cast<ULONG>(++ref_); }

    STDMETHODIMP_(ULONG) Release() noexcept override {
        ULONG remaining = static_cast<ULONG>(--ref_);
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    STDMETHODIMP Invoke(HRESULT error, ICoreWebView2* web_view,
                        ICoreWebView2WebMessageReceivedEventArgs* args) noexcept override {
        (void)error;
        (void)web_view;
        WebViewRecord* rec = find_record(slot_);
        if (rec == nullptr) {
            return S_OK;
        }
        std::string payload;
        wchar_t* raw = nullptr;
        if (args != nullptr && SUCCEEDED(args->TryGetWebMessageAsString(&raw)) && raw != nullptr) {
            payload = wide_to_utf8(raw);
        } else if (args != nullptr && SUCCEEDED(args->get_WebMessageAsJson(&raw)) &&
                   raw != nullptr) {
            payload = wide_to_utf8(raw);
        }
        if (raw != nullptr) {
            CoTaskMemFree(raw);
        }
        if (g_message_cb != nullptr) {
            g_message_cb(slot_, payload.c_str());
        }
        return S_OK;
    }

  private:
    FloomNativeSlot slot_;
    long ref_;
};

/* --- the provider -------------------------------------------------------- */

/*
 * The host's window. On Windows it is the top-level `HWND`; the child windows
 * go into its client area.
 */
static int32_t webview_attach(void* self, const void* host_window) {
    (void)self;
    if (host_window != nullptr) {
        g_host_hwnd = static_cast<HWND>(const_cast<void*>(host_window));
    }
    if (g_views != nullptr && g_host_hwnd != nullptr) {
        for (auto& pair : *g_views) {
            attach_to_host(pair.second.hwnd);
        }
    }
    return 0;
}

static FloomNativeCapabilities webview_capabilities(void* self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay only: the child HWNDs are siblings of the framework's own
     * surface. Child HWNDs fight D3D airspace, which is the trade this mode
     * is honest about. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.mode_count = 1;
    caps.snapshot = 0;
    caps.input_passthrough = 0;
    caps.input_capture = 1;
    caps.focusable = 1;
    caps.clip = FLOOM_NATIVE_CLIP_PLATFORM;
    return caps;
}

static int32_t webview_create(void* self, FloomNativeSlot slot, uint32_t kind,
                                 const char* kind_name, char* out_error,
                                 uint32_t out_capacity) {
    (void)self;
    if (kind != FLOOM_NATIVE_KIND_WEB) {
        /* Refused, with the kind named: "the plugin said no" without the kind
         * sends someone reading the C ABI to find out which argument was
         * wrong. */
        char message[512];
        snprintf(message, sizeof(message), "this plugin builds web views only, not kind %u%s%s%s",
                 kind, kind_name != nullptr ? " (" : "", kind_name != nullptr ? kind_name : "",
                 kind_name != nullptr ? ")" : "");
        g_refused += 1;
        g_last_refusal = message;
        set_error(out_error, out_capacity, message);
        return 1;
    }

    if (!ensure_window_class()) {
        set_error(out_error, out_capacity, "could not register the FloomWebViewWindow class");
        return 1;
    }
    ensure_table();

    DWORD style = g_host_hwnd != nullptr ? (WS_CHILD | WS_CLIPSIBLINGS) : WS_POPUP;
    HWND hwnd = CreateWindowExW(0, L"FloomWebViewWindow", L"FloomWebView", style, 0, 0, 100, 100,
                                g_host_hwnd, nullptr, GetModuleHandleW(nullptr), nullptr);
    if (hwnd == nullptr) {
        set_errorf(out_error, out_capacity, "CreateWindowExW failed (error %lu)",
                   (unsigned long)GetLastError());
        return 1;
    }

    WebViewRecord rec;
    rec.hwnd = hwnd;
    std::string key = slot_key(slot);
    g_views->emplace(std::move(key), std::move(rec));
    g_created += 1;

    /* The synchronous part of the creation is the child HWND; the web view
     * itself initializes in two steps whose completion callbacks run on the
     * host's message pump and re-find this record by slot. */
    EnvironmentHandler* handler = new EnvironmentHandler(slot);
    HRESULT hr = CreateCoreWebView2EnvironmentWithOptions(nullptr, nullptr, nullptr, handler);
    handler->Release();
    if (FAILED(hr)) {
        OutputDebugStringW(L"floom: CreateCoreWebView2EnvironmentWithOptions failed\r\n");
    }
    return 0;
}

static void webview_destroy(void* self, FloomNativeSlot slot) {
    (void)self;
    if (g_views != nullptr) {
        auto it = g_views->find(slot_key(slot));
        if (it != g_views->end()) {
            WebViewRecord& rec = it->second;
            if (rec.message_hooked && rec.web2 != nullptr) {
                rec.web2->remove_WebMessageReceived(rec.message_token);
            }
            if (rec.controller != nullptr) {
                rec.controller->Close(); /* releases its hold on the child HWND */
                rec.controller.Reset();
            }
            if (rec.hwnd != nullptr) {
                DestroyWindow(rec.hwnd);
            }
            g_views->erase(it);
        }
    }
    g_destroyed += 1;
}

/*
 * Place the view.
 *
 * The rect crosses in device pixels, and Windows child-window geometry is
 * device pixels too, so unlike macOS there is no scale division here: the
 * placement is used as given.
 */
static void webview_apply(void* self, FloomNativeSlot slot,
                             const FloomNativeState* state) {
    (void)self;
    if (state == nullptr) {
        return;
    }
    WebViewRecord* rec = find_record(slot);
    if (rec == nullptr) {
        return;
    }
    g_applied += 1;

    int32_t x = (int32_t)state->rect.x;
    int32_t y = (int32_t)state->rect.y;
    int32_t w = (int32_t)state->rect.width;
    int32_t h = (int32_t)state->rect.height;
    g_last_x = (double)x;
    g_last_y = (double)y;
    g_last_w = (double)w;
    g_last_h = (double)h;

    if (rec->hwnd != nullptr) {
        SetWindowPos(rec->hwnd, nullptr, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER);
        ShowWindow(rec->hwnd, state->visible != 0 ? SW_SHOWNA : SW_HIDE);
        /* `clip` is a crop shown in place, not a resize: child HWNDs are
         * clipped against their parent for free, and anything further out is
         * a window region, which the RHS owns afterwards. */
        if (state->has_clip != 0) {
            int32_t left = (int32_t)state->clip.x;
            int32_t top = (int32_t)state->clip.y;
            int32_t right = (int32_t)state->clip.x + (int32_t)state->clip.width;
            int32_t bottom = (int32_t)state->clip.y + (int32_t)state->clip.height;
            left = std::max(left, x);
            top = std::max(top, y);
            right = std::min(right, x + w);
            bottom = std::min(bottom, y + h);
            HRGN region = (right <= left || bottom <= top)
                              ? CreateRectRgn(0, 0, 0, 0)
                              : CreateRectRgn(left - x, top - y, right - x, bottom - y);
            SetWindowRgn(rec->hwnd, region, TRUE);
        } else {
            SetWindowRgn(rec->hwnd, nullptr, TRUE);
        }
    }

    if (rec->controller != nullptr) {
        RECT bounds;
        bounds.left = 0;
        bounds.top = 0;
        bounds.right = w;
        bounds.bottom = h;
        rec->controller->put_Bounds(bounds);
    }
}

static void webview_focus(void* self, FloomNativeSlot slot) {
    (void)self;
    g_focused += 1;
    WebViewRecord* rec = find_record(slot);
    if (rec == nullptr) {
        return;
    }
    /* A web page has text fields, and a form the user cannot type into is not
     * an embedded browser. */
    if (rec->hwnd != nullptr) {
        SetFocus(rec->hwnd);
    }
    if (rec->controller != nullptr) {
        rec->controller->MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
    }
}

static void webview_blur(void* self, FloomNativeSlot slot) {
    (void)self;
    (void)slot;
    /* Hand focus back to the host: MoveFocus only moves within the web view,
     * so the parent is where the focus goes. */
    if (g_host_hwnd != nullptr) {
        SetFocus(g_host_hwnd);
    }
}

static FloomNativeProviderV1 g_provider = {
    FLOOM_NATIVE_PROVIDER_ABI_VERSION,
    nullptr,
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
 * native code is ready is the native code itself. An MSVC global object
 * constructor runs before `main`, which is early enough: the Rust half reads
 * the pointer when the app asks for the provider, not before.
 *
 * This must stay defined *after* `g_provider`, whose constant initializer is
 * applied before any dynamic initialization in this transcription unit.
 */
struct ProviderRegistrar {
    ProviderRegistrar() { floom_webview_register_provider(&g_provider); }
};
const ProviderRegistrar g_provider_registrar;

} /* namespace */

/* --- the plugin's own four functions ------------------------------------- */

extern "C" {

const char* floom_webview_version(void) {
    static const char version[] = "webview 0.2.0";
    return version;
}

int floom_webview_enable(void) {
    /* Nothing global to claim beyond the window class; a failure is reported
     * at `create`, which has an error channel to report it through. */
    (void)ensure_window_class();
    return 0;
}

void floom_webview_disable(void) {}

int floom_webview_capability(void) { return 0; }

/* --- the web view -------------------------------------------------------- */

/*
 * Load a URL into a live web participant.
 *
 * Separate from `create` because the provider ABI has no "navigate" verb.
 * Returns 0 when the URL was handed to the browser, non-zero when the slot
 * holds no web view or the URL could not be parsed. If the controller has not
 * finished creating (a possible state on this platform and not on macOS), the
 * URL is queued and flushed at completion rather than refused.
 */
int floom_webview_load_url(FloomNativeSlot slot, const char* url, char* out_error,
                              uint32_t out_capacity) {
    WebViewRecord* rec = find_record(slot);
    if (rec == nullptr) {
        set_error(out_error, out_capacity, "no web view is bound to that slot");
        return 1;
    }
    if (url == nullptr) {
        set_error(out_error, out_capacity, "no URL was given");
        return 1;
    }
    if (!has_scheme(url)) {
        set_errorf(out_error, out_capacity, "not a URL with a scheme: %s", url);
        return 1;
    }
    g_last_url = url;
    if (rec->webview_ok && rec->web != nullptr) {
        std::wstring target = utf8_to_wide(url);
        rec->web->Navigate(target.c_str());
    } else {
        rec->pending_url = url;
    }
    return 0;
}

int floom_webview_post_message(FloomNativeSlot slot, const char* json, char* out_error,
                                  uint32_t out_capacity) {
    WebViewRecord* rec = find_record(slot);
    if (rec == nullptr || rec->hwnd == nullptr) {
        set_error(out_error, out_capacity, "no web view is bound to that slot");
        return 1;
    }
    if (!rec->webview_ok || rec->web == nullptr) {
        set_error(out_error, out_capacity, "the web view in that slot is still initializing");
        return 1;
    }
    std::string js = std::string("if (window.floomNative && window.floomNative.onMessage) "
                                 "{ window.floomNative.onMessage(") +
                     (json != nullptr ? json : "{}") + "); }";
    std::wstring script = utf8_to_wide(js.c_str());
    rec->web->ExecuteScript(script.c_str(), nullptr);
    return 0;
}

void floom_webview_set_message_callback(void (*cb)(FloomNativeSlot, const char*)) {
    g_message_cb = cb;
}

/* The URL most recently handed to a web view, or NULL. */
const char* floom_webview_last_url(void) {
    return g_last_url.empty() ? nullptr : g_last_url.c_str();
}

/* The message the most recent refusal carried, or NULL when nothing was
 * refused. */
const char* floom_webview_last_refusal(void) {
    return g_last_refusal.empty() ? nullptr : g_last_refusal.c_str();
}

/* --- diagnostics --------------------------------------------------------- */

/*
 * What the provider did, for a host that wants to check. Plain C, so a test
 * can read it without knowing anything about Win32.
 */

int floom_webview_created_count(void) { return g_created; }

int floom_webview_destroyed_count(void) { return g_destroyed; }

int floom_webview_applied_count(void) { return g_applied; }

int floom_webview_focused_count(void) { return g_focused; }

/* How many `create` calls were refused for a kind this plugin cannot build. */
int floom_webview_refused_count(void) { return g_refused; }

/* How many participants are alive right now. */
int floom_webview_live_count(void) { return g_views != nullptr ? (int)g_views->size() : 0; }

/*
 * Whether the participant in `slot` holds a real web view.
 *
 * On this platform `create` returns before the controller exists, so the
 * honest mirror of the macOS check is "the controller completed", not "a
 * create call was accepted" — the capability claim rests on the browser
 * actually being there.
 */
int floom_webview_slot_is_web(FloomNativeSlot slot) {
    WebViewRecord* rec = find_record(slot);
    return rec != nullptr && rec->webview_ok && rec->web != nullptr ? 1 : 0;
}

/* Whether the participant in `slot` is really in the host window's client
 * area. */
int floom_webview_slot_is_attached(FloomNativeSlot slot) {
    WebViewRecord* rec = find_record(slot);
    if (rec == nullptr || rec->hwnd == nullptr || g_host_hwnd == nullptr) {
        return 0;
    }
    return IsChild(g_host_hwnd, rec->hwnd) ? 1 : 0;
}

void floom_webview_last_frame(double* x, double* y, double* w, double* h) {
    if (x != nullptr) {
        *x = g_last_x;
    }
    if (y != nullptr) {
        *y = g_last_y;
    }
    if (w != nullptr) {
        *w = g_last_w;
    }
    if (h != nullptr) {
        *h = g_last_h;
    }
}

} /* extern "C" */

#endif /* _WIN32 */
