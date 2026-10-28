/* The native/Rust contract for the `nativeview` plugin.
 *
 * The native half implements these functions; the Rust half calls them
 * through `extern "C"`. There is no serialization boundary and no runtime
 * lookup — the symbols resolve when the crate is linked.
 */

#ifndef FLOOM_NATIVEVIEW_H
#define FLOOM_NATIVEVIEW_H

#include "floom_native_provider.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The plugin's version string, owned by the callee. Never NULL. */
const char *floom_nativeview_version(void);

/* Claims the plugin's platform state. Returns 0 on success. */
int floom_nativeview_enable(void);

/* Releases whatever `floom_nativeview_enable` claimed. */
void floom_nativeview_disable(void);

/* The plugin's capability id, stable across releases. */
int floom_nativeview_capability(void);

/* Hands the provider vtable to the Rust half.
 *
 * Exported by the Rust half, called by the native half's constructor: the
 * native code is the only party that knows when it is ready, and a
 * constructor runs before `main`.
 */
void floom_nativeview_register_provider(const void *provider);

/* --- the web view ------------------------------------------------------- */

/* Bridge: send a JSON message from Rust to the web view's JS.

 * Returns 0 when handed to WebKit, non-zero when the slot holds no web view.
 * The message is delivered to `window.floomNative.onMessage(json)` if defined.
 */
int floom_nativeview_post_message(FloomNativeSlot slot, const char *json,
                                  char *out_error, uint32_t out_capacity);

/* Bridge: register the Rust callback invoked when JS calls
 * `window.floomNative.postMessage(json)`. Pass NULL to unregister.
 * The callback receives the slot and a NUL-terminated JSON string owned by
 * the caller for the duration of the call.
 */
void floom_nativeview_set_message_callback(void (*cb)(FloomNativeSlot, const char *));

/* Loads `url` into the web participant bound to `slot`.
 *
 * Returns 0 when WebKit accepted the request, non-zero when the slot holds no
 * web view or the URL does not parse. `Ok` means the load *started*; whether
 * the page arrives is the page's own business and arrives asynchronously.
 *
 * A separate function rather than a provider verb because the provider ABI
 * (create / place / destroy) describes a composited rectangle, and what a web
 * view displays is not part of that contract.
 */
int floom_nativeview_load_url(FloomNativeSlot slot, const char *url,
                              char *out_error, uint32_t out_capacity);

/* The URL most recently handed to a web view, or NULL before the first. */
const char *floom_nativeview_last_url(void);

/* The message the most recent refusal carried, or NULL when none was refused. */
const char *floom_nativeview_last_refusal(void);

/* --- diagnostics -------------------------------------------------------- */

/* How many `create` calls were refused for a kind this plugin cannot build. */
int floom_nativeview_refused_count(void);

/* How many participants are alive right now. */
int floom_nativeview_live_count(void);

/* Whether the participant in `slot` holds a real web view. */
int floom_nativeview_slot_is_web(FloomNativeSlot slot);

/* Whether the participant in `slot` is really in the window's content view. */
int floom_nativeview_slot_is_attached(FloomNativeSlot slot);

#ifdef __cplusplus
}} /* extern "C" */
#endif

#endif /* FLOOM_NATIVEVIEW_H */
