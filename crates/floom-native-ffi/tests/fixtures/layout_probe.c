/*
 * The C half of the ABI proof.
 *
 * Two jobs:
 *
 * 1. **Report the layout.** Every struct's size and every field's offset is
 *    printed, and the Rust test compares them against the `#[repr(C)]`
 *    mirror. That is the check that matters: a hand-written ABI drifts when
 *    a field is reordered or a type is widened, and the symptom is a
 *    provider writing into a struct the framework reads at the wrong offsets,
 *    which is memory corruption rather than a compile error.
 *
 * 2. **Be a provider.** The same program fills a vtable and drives it, so the
 *    header is proved usable from C's side — a header that only compiles
 *    because Rust mirrors it is not a header anybody can implement.
 *
 * The output is `key=value` lines, one per fact, so the Rust side parses it
 * without needing a JSON parser in C.
 */

#include "floom_native_provider.h"

#include <stddef.h>
#include <stdio.h>
#include <string.h>

/* --- the layout probe -------------------------------------------------- */

#define REPORT_SIZE(type) printf("size.%s=%zu\n", #type, sizeof(type))

#define REPORT_OFFSET(type, field) \
    printf("offset.%s.%s=%zu\n", #type, #field, offsetof(type, field))

/* --- a provider, in C --------------------------------------------------- */

/* What the provider remembers, so the test can see that the calls arrived. */
typedef struct {
    int created;
    int destroyed;
    int focused;
    int blurred;
    int applied;
    FloomNativeState last_state;
    FloomNativeSlot last_slot;
    char last_kind_name[64];
} ProviderState;

static FloomNativeCapabilities probe_capabilities(void *self) {
    (void)self;
    FloomNativeCapabilities caps;
    memset(&caps, 0, sizeof(caps));
    /* Overlay first: the provider's own preference, and the order is part of
     * what it says. */
    caps.modes[0] = FLOOM_NATIVE_MODE_OVERLAY;
    caps.modes[1] = FLOOM_NATIVE_MODE_TEXTURE;
    caps.mode_count = 2;
    caps.snapshot = 0;
    caps.input_passthrough = 1;
    caps.input_capture = 0;
    caps.focusable = 1;
    caps.clip = FLOOM_NATIVE_CLIP_PROVIDER;
    return caps;
}

static int32_t probe_create(void *self, FloomNativeSlot slot, uint32_t kind,
                            const char *kind_name, char *out_error,
                            uint32_t out_capacity) {
    ProviderState *state = (ProviderState *)self;
    state->created += 1;
    state->last_slot = slot;
    if (kind_name != NULL) {
        strncpy(state->last_kind_name, kind_name,
                sizeof(state->last_kind_name) - 1);
        state->last_kind_name[sizeof(state->last_kind_name) - 1] = '\0';
    } else {
        state->last_kind_name[0] = '\0';
    }
    (void)kind;
    /* Refuse with a message, so the Rust side can prove it reads one. */
    if (out_error != NULL && out_capacity > 0) {
        const char *message = "the probe refuses on purpose";
        size_t len = strlen(message);
        if (len >= out_capacity) {
            len = out_capacity - 1;
        }
        memcpy(out_error, message, len);
        out_error[len] = '\0';
    }
    return 0;
}

static void probe_destroy(void *self, FloomNativeSlot slot) {
    ProviderState *state = (ProviderState *)self;
    state->destroyed += 1;
    state->last_slot = slot;
}

static void probe_apply(void *self, FloomNativeSlot slot,
                        const FloomNativeState *state_in) {
    ProviderState *state = (ProviderState *)self;
    state->applied += 1;
    state->last_slot = slot;
    if (state_in != NULL) {
        state->last_state = *state_in;
    }
}

static void probe_focus(void *self, FloomNativeSlot slot) {
    ProviderState *state = (ProviderState *)self;
    state->focused += 1;
    state->last_slot = slot;
}

static void probe_blur(void *self, FloomNativeSlot slot) {
    ProviderState *state = (ProviderState *)self;
    state->blurred += 1;
    state->last_slot = slot;
}

static FloomNativeProviderV1 probe_provider(ProviderState *state) {
    FloomNativeProviderV1 provider;
    memset(&provider, 0, sizeof(provider));
    provider.abi_version = FLOOM_NATIVE_PROVIDER_ABI_VERSION;
    provider.self = state;
    provider.capabilities = probe_capabilities;
    provider.create = probe_create;
    provider.destroy = probe_destroy;
    provider.apply = probe_apply;
    provider.focus = probe_focus;
    provider.blur = probe_blur;
    return provider;
}

/* --- the probe ---------------------------------------------------------- */

int main(void) {
    REPORT_SIZE(FloomNativeSlot);
    REPORT_OFFSET(FloomNativeSlot, index);
    REPORT_OFFSET(FloomNativeSlot, generation);

    REPORT_SIZE(FloomNativeDeviceRect);
    REPORT_OFFSET(FloomNativeDeviceRect, x);
    REPORT_OFFSET(FloomNativeDeviceRect, y);
    REPORT_OFFSET(FloomNativeDeviceRect, width);
    REPORT_OFFSET(FloomNativeDeviceRect, height);

    REPORT_SIZE(FloomNativeCapabilities);
    REPORT_OFFSET(FloomNativeCapabilities, modes);
    REPORT_OFFSET(FloomNativeCapabilities, mode_count);
    REPORT_OFFSET(FloomNativeCapabilities, snapshot);
    REPORT_OFFSET(FloomNativeCapabilities, input_passthrough);
    REPORT_OFFSET(FloomNativeCapabilities, input_capture);
    REPORT_OFFSET(FloomNativeCapabilities, focusable);
    REPORT_OFFSET(FloomNativeCapabilities, clip);

    REPORT_SIZE(FloomNativeState);
    REPORT_OFFSET(FloomNativeState, rect);
    REPORT_OFFSET(FloomNativeState, clip);
    REPORT_OFFSET(FloomNativeState, has_clip);
    REPORT_OFFSET(FloomNativeState, visible);
    REPORT_OFFSET(FloomNativeState, z);
    REPORT_OFFSET(FloomNativeState, input);
    REPORT_OFFSET(FloomNativeState, mode);

    REPORT_SIZE(FloomNativeProviderV1);
    REPORT_OFFSET(FloomNativeProviderV1, abi_version);
    REPORT_OFFSET(FloomNativeProviderV1, self);
    REPORT_OFFSET(FloomNativeProviderV1, capabilities);
    REPORT_OFFSET(FloomNativeProviderV1, create);
    REPORT_OFFSET(FloomNativeProviderV1, destroy);
    REPORT_OFFSET(FloomNativeProviderV1, apply);
    REPORT_OFFSET(FloomNativeProviderV1, focus);
    REPORT_OFFSET(FloomNativeProviderV1, blur);

    /* Drive a provider through the vtable, from C, so the header is proved
     * usable by the language it is written for. */
    ProviderState state;
    memset(&state, 0, sizeof(state));
    FloomNativeProviderV1 provider = probe_provider(&state);

    FloomNativeCapabilities caps = provider.capabilities(provider.self);
    printf("probe.mode_count=%u\n", (unsigned)caps.mode_count);
    printf("probe.first_mode=%u\n", (unsigned)caps.modes[0]);
    printf("probe.can_crop=%u\n", (unsigned)(caps.clip != FLOOM_NATIVE_CLIP_UNSUPPORTED));

    FloomNativeSlot slot;
    slot.index = 7;
    slot.generation = 3;

    char error[128];
    int32_t rc = provider.create(provider.self, slot, FLOOM_NATIVE_KIND_CUSTOM,
                                 "com.example.three-d", error, sizeof(error));
    printf("probe.create_rc=%d\n", (int)rc);
    printf("probe.create_message=%s\n", error);

    FloomNativeState apply_state;
    memset(&apply_state, 0, sizeof(apply_state));
    apply_state.rect.x = 10;
    apply_state.rect.y = 20;
    apply_state.rect.width = 300;
    apply_state.rect.height = 150;
    apply_state.visible = 1;
    apply_state.z = 5;
    apply_state.input = FLOOM_NATIVE_INPUT_CAPTURE;
    apply_state.mode = FLOOM_NATIVE_MODE_TEXTURE;
    provider.apply(provider.self, slot, &apply_state);

    provider.focus(provider.self, slot);
    provider.blur(provider.self, slot);
    provider.destroy(provider.self, slot);

    printf("probe.created=%d\n", state.created);
    printf("probe.destroyed=%d\n", state.destroyed);
    printf("probe.focused=%d\n", state.focused);
    printf("probe.blurred=%d\n", state.blurred);
    printf("probe.applied=%d\n", state.applied);
    printf("probe.kind_name=%s\n", state.last_kind_name);
    printf("probe.slot_index=%u\n", (unsigned)state.last_slot.index);
    printf("probe.slot_generation=%u\n", (unsigned)state.last_slot.generation);
    printf("probe.applied_rect=%d,%d,%u,%u\n", (int)state.last_state.rect.x,
           (int)state.last_state.rect.y, (unsigned)state.last_state.rect.width,
           (unsigned)state.last_state.rect.height);
    printf("probe.applied_mode=%u\n", (unsigned)state.last_state.mode);
    printf("probe.applied_input=%u\n", (unsigned)state.last_state.input);

    return 0;
}
