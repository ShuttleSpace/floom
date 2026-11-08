/* sample — the native/Rust contract.
 *
 * Everything here is a plain C ABI: no serialization, no runtime lookup, no
 * registry. The Rust half calls these symbols directly, and they resolve when
 * the crate is linked.
 *
 * The **native half implements** these functions; the **Rust half calls**
 * them. Every platform satisfies the same contract: a C file implements them
 * directly, Swift implements them with `@_cdecl`, and Kotlin is reached
 * through a C shim that forwards over JNI.
 *
 * Strings are returned as `const char *` owned by the callee. The caller must
 * not free them; the next call into the same plugin invalidates the previous
 * one. That constraint is the price of not serializing, and it is cheaper than
 * the copy it replaces.
 *
 * This file is valid C and valid C++ — headers in plugins tend to be included
 * from both.
 */

#ifndef FLOOM_SAMPLE_H
#define FLOOM_SAMPLE_H

#ifdef __cplusplus
extern "C" {
#endif

/* ABI version. The Rust half refuses to enable a plugin whose major differs. */
#define FLOOM_SAMPLE_ABI_VERSION 1

/* The plugin's version string, owned by the callee. Never NULL. */
const char *floom_sample_version(void);

/* Claims the plugin's capability. Returns 0 on success, non-zero on failure,
 * in which case nothing was claimed and the caller must not proceed. */
int floom_sample_enable(void);

/* Releases whatever `floom_sample_enable` claimed. Safe to call without a
 * matching enable. */
void floom_sample_disable(void);

/* The plugin's capability id, stable across releases. The Rust side matches
 * this against the enum it was compiled with. */
int floom_sample_capability(void);

/* The ABI version this half was actually compiled against.
 *
 * A function rather than only the macro above, because the Rust half has to be
 * able to ask a plugin it did not compile itself what it was built against —
 * a compile-time macro cannot answer that question. Returns
 * `FLOOM_SAMPLE_ABI_VERSION`.
 */
int floom_sample_abi_version(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* FLOOM_SAMPLE_H */
