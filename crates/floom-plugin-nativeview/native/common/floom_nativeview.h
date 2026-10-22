/* The native/Rust contract for the `nativeview` plugin.
 *
 * The native half implements these functions; the Rust half calls them
 * through `extern "C"`. There is no serialization boundary and no runtime
 * lookup — the symbols resolve when the crate is linked.
 */

#ifndef FLOOM_NATIVEVIEW_H
#define FLOOM_NATIVEVIEW_H

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

#ifdef __cplusplus
}} /* extern "C" */
#endif

#endif /* FLOOM_NATIVEVIEW_H */
