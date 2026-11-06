/* The native/Rust contract for the `package-info` plugin.
 *
 * A native half implements the five functions below and the Rust half calls
 * them through `extern "C"` when `build.rs` reports one for the current target
 * (`cargo:rustc-cfg=floom_plugin_native`).
 *
 * This plugin's target arms are Rust modules (`src/<target>.rs`), so no C
 * source implements this header today: `build.rs` compiles nothing and
 * `src/lib.rs` never takes the `extern "C"` path. The header is the contract a
 * native half must implement, and `scripts/check-plugin-templates.sh` checks
 * that its symbol names match the plugin's crate name.
 */

#ifndef FLOOM_PACKAGE_INFO_H
#define FLOOM_PACKAGE_INFO_H

#ifdef __cplusplus
extern "C" {
#endif

/* The native half's version string, owned by the callee. Never NULL. */
const char *floom_package_info_version(void);

/* Claims the plugin's platform state. Returns 0 on success. */
int floom_package_info_enable(void);

/* Releases whatever `floom_package_info_enable` claimed. */
void floom_package_info_disable(void);

/* The plugin's capability id, stable across releases. */
int floom_package_info_capability(void);

/* The ABI version the native half was compiled against. */
int floom_package_info_abi_version(void);

#ifdef __cplusplus
}
#endif

#endif /* FLOOM_PACKAGE_INFO_H */
