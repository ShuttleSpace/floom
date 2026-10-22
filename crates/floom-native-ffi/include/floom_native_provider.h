/* The native composition provider ABI.
 *
 * A *participant* is a view the framework does not paint: an `NSView`, an
 * Android `SurfaceView`, a `WKWebView`, a camera preview. A **provider**
 * creates them, places them and tears them down. This header is the contract
 * between the framework and whoever can actually create one.
 *
 * ## Who implements what
 *
 * The **native side implements** the vtable; the **Rust side calls** it. A
 * provider is an *instance* rather than a singleton — one per window — so the
 * contract is a struct of function pointers with a `self` pointer, not a set
 * of named symbols. That is the one place this ABI differs in shape from a
 * plugin's own four-function C ABI: a plugin is enabled once for the
 * process, a provider is instantiated per window.
 *
 * ## Everything crosses in device pixels
 *
 * The rect and the clip are whole device pixels, never logical ones and
 * never a float. The logical-to-device conversion happens once, on the Rust
 * side, in `to_device` — and it happens once because a platform cannot place
 * a view at a fractional pixel, and letting each provider invent its own
 * rounding is how a half-pixel seam appears on one platform and not another.
 *
 * ## Absolute state, never a delta
 *
 * `apply` receives the state the participant should *be*, not a change to
 * its previous state. Applying the same state twice must be free, which is
 * what lets a missed frame self-heal instead of accumulating error.
 *
 * ## Versioning
 *
 * `abi_version` is in the vtable rather than only in a macro, because the
 * Rust side has to be able to ask a provider it did not compile what it was
 * built against. A provider whose major version differs is refused, not
 * called: the two sides exchange pointers and integers whose meaning is
 * defined by this number.
 *
 * This file is valid C and valid C++ — headers in plugins tend to be included
 * from both.
 */

#ifndef FLOOM_NATIVE_PROVIDER_H
#define FLOOM_NATIVE_PROVIDER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The ABI version. The Rust side refuses a provider whose major differs. */
#define FLOOM_NATIVE_PROVIDER_ABI_VERSION 1

/*
 * How a participant is composited relative to the framework's own surface.
 *
 * The order of the values is *not* a strength order. Which of these is
 * "best" is a property of the platform: X11 can raise a subwindow above a
 * surface, Wayland has no subwindows at all, Windows child `HWND`s fight D3D
 * airspace, KMS has real hardware planes. A provider publishes what it can do
 * and the host negotiates.
 */
typedef enum {
    /* A sibling window/view stacked above the framework's surface. */
    FLOOM_NATIVE_MODE_OVERLAY = 0,
    /* Contents imported as a texture and drawn as a scene resource. */
    FLOOM_NATIVE_MODE_TEXTURE = 1,
    /* A real compositor surface or hardware plane. */
    FLOOM_NATIVE_MODE_SURFACE = 2,
    /* Rendered offscreen and blitted into the framework's surface. */
    FLOOM_NATIVE_MODE_OFFSCREEN = 3
} FloomNativeCompositionMode;

/*
 * Whether a provider can crop a participant to a rect it does not own.
 *
 * Conservative by default: a provider that has not thought about cropping is
 * assumed not to do it, so the host degrades the composition instead of
 * shipping a wrong frame.
 */
typedef enum {
    /* The provider cannot crop at all. */
    FLOOM_NATIVE_CLIP_UNSUPPORTED = 0,
    /* The platform clips a participant against its parent for free. */
    FLOOM_NATIVE_CLIP_PLATFORM = 1,
    /* The provider must apply the crop itself. */
    FLOOM_NATIVE_CLIP_PROVIDER = 2
} FloomNativeClipSupport;

/* Whether the participant takes pointer input itself. */
typedef enum {
    /* The native view does not want input. */
    FLOOM_NATIVE_INPUT_PASSTHROUGH = 0,
    /* The native view takes OS input itself. */
    FLOOM_NATIVE_INPUT_CAPTURE = 1
} FloomNativeInputMode;

/*
 * What kind of native view a participant is.
 *
 * `FLOOM_NATIVE_KIND_CUSTOM` is the tag for a kind the framework has no name
 * for; the name itself crosses as the `kind_name` argument of `create`.
 */
typedef enum {
    FLOOM_NATIVE_KIND_MAP = 0,
    FLOOM_NATIVE_KIND_VIDEO = 1,
    FLOOM_NATIVE_KIND_WEB = 2,
    /* A native text field; focusable ones are where the framework yields IME. */
    FLOOM_NATIVE_KIND_TEXT_INPUT = 3,
    /* A kind the framework has no name for. */
    FLOOM_NATIVE_KIND_CUSTOM = 4
} FloomNativeViewKind;

/*
 * A participant slot.
 *
 * The generation is what tells a provider whether the slot is fresh or
 * recycled: a recycled slot's handle belongs to the previous generation, so
 * it has to be re-bound before anything is applied to it.
 */
typedef struct {
    uint32_t index;
    uint32_t generation;
} FloomNativeSlot;

/* A rect in whole device pixels. */
typedef struct {
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
} FloomNativeDeviceRect;

/* What one provider can do. */
typedef struct {
    /*
     * The modes this provider supports, in the provider's own order of
     * preference. `mode_count` of them are meaningful.
     *
     * An ordered list rather than a bitmask, because the order is part of
     * what a provider says: the first mode is the provider's own choice, and
     * the host uses it when the participant has no preference of its own. A
     * bitmask cannot carry an order, and a provider whose order was lost
     * would be negotiated against in an order it did not ask for.
     */
    uint32_t modes[4];
    /* How many entries of `modes` are meaningful. */
    uint8_t mode_count;
    /* Whether the provider can freeze a participant into a texture. */
    uint8_t snapshot;
    /* Whether the provider honours passthrough input. */
    uint8_t input_passthrough;
    /* Whether the provider honours capturing input. */
    uint8_t input_capture;
    /* Whether participants can join the OS focus chain. */
    uint8_t focusable;
    /* A `FloomNativeClipSupport`. */
    uint8_t clip;
    /* Reserved, so a future minor version can add a field. */
    uint8_t reserved[3];
} FloomNativeCapabilities;

/*
 * The state to apply. Absolute, never a delta.
 *
 * `clip` is a crop, not a placement: show the clipped part of `rect` *in
 * place*, at full size. A provider that honours its rect by fitting its
 * content into it will rescale a map or a video instead of cropping it, and
 * that bug is invisible on a map and obvious on a video.
 */
typedef struct {
    /* Where the participant is, in whole device pixels. */
    FloomNativeDeviceRect rect;
    /* The part of `rect` that may be shown, in the same coordinates. */
    FloomNativeDeviceRect clip;
    /* 0 when nothing clips this participant — which is *not* the same
     * statement as "clipped to nothing". */
    uint8_t has_clip;
    /* 0 when the participant is clipped away entirely or hidden. */
    uint8_t visible;
    /* Reserved, so a future minor version can add a field. */
    uint8_t reserved[2];
    /* Stacking order, following tree paint order. */
    uint32_t z;
    /* A `FloomNativeInputMode`. */
    uint32_t input;
    /* A `FloomNativeCompositionMode`. */
    uint32_t mode;
} FloomNativeState;

/*
 * The provider's entry points.
 *
 * Every function takes `self` first. `self` is the provider's own state
 * pointer and is never interpreted by the framework.
 */
typedef struct {
    /* Must be `FLOOM_NATIVE_PROVIDER_ABI_VERSION`. */
    uint32_t abi_version;
    /* The provider's own state; passed back to every call. */
    void *self;
    /*
     * The host's window, given once.
     *
     * A provider is an instance per window, so the window does not change
     * over the provider's life: telling it once is enough, and a per-call
     * parameter would be a lie about that.
     *
     * What it *is* depends on the platform: an `NSView *` on macOS, an
     * `ANativeWindow *` on Android, a `Window` on X11. A provider that
     * composites its own view — a camera preview, a web view — needs it to
     * put the view somewhere.
     *
     * May be NULL, and may be absent from the vtable entirely: a provider
     * that hands the framework pixels rather than a view never needs a
     * window. Returns 0 on success, non-zero to refuse the provider.
     */
    int32_t (*attach)(void *self, const void *host_window);
    /* What this provider can do. Queried at bind time. */
    FloomNativeCapabilities (*capabilities)(void *self);
    /*
     * Create a participant in `slot`.
     *
     * `kind_name` is the name of a `FLOOM_NATIVE_KIND_CUSTOM` kind, or NULL.
     *
     * Returns 0 on success. On failure it returns non-zero and writes a
     * NUL-terminated message into `out_error` (truncated to `out_capacity`,
     * and never written to at all when `out_error` is NULL or the capacity is
     * zero). A message rather than an exception, because an exception thrown
     * through this frame would unwind across Rust.
     */
    int32_t (*create)(void *self, FloomNativeSlot slot, uint32_t kind,
                      const char *kind_name, char *out_error,
                      uint32_t out_capacity);
    /* Tear the participant down. Must tolerate a slot it never created. */
    void (*destroy)(void *self, FloomNativeSlot slot);
    /* Move/resize/re-stack/show-hide. Called with the desired state. */
    void (*apply)(void *self, FloomNativeSlot slot,
                  const FloomNativeState *state);
    /* Hand OS focus to the participant. */
    void (*focus)(void *self, FloomNativeSlot slot);
    /* Take focus back. */
    void (*blur)(void *self, FloomNativeSlot slot);
} FloomNativeProviderV1;

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* FLOOM_NATIVE_PROVIDER_H */
