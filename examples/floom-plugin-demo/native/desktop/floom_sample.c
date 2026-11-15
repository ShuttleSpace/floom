/* The desktop native half of the `sample` plugin.
 *
 * Implements the contract in `native/common/floom_sample.h`. This is the
 * reference implementation: it claims nothing and reports its version, so a
 * generated plugin compiles, links and passes its tests before there is any
 * real capability behind it.
 *
 * Replace the bodies with the real work. The ABI does not change.
 */

#include "floom_sample.h"

const char *floom_sample_version(void) {
    /* Owned by the callee: the next call into this plugin invalidates it. */
    static const char version[] = "sample 0.1.0";
    return version;
}

int floom_sample_enable(void) {
    /* Nothing to claim yet. Return 0 for success, non-zero to refuse. */
    return 0;
}

void floom_sample_disable(void) {
    /* Nothing to release. */
}

int floom_sample_capability(void) {
    return 0;
}

int floom_sample_abi_version(void) {
    return FLOOM_SAMPLE_ABI_VERSION;
}
