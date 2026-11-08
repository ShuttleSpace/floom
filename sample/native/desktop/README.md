# The desktop native half

This directory holds the desktop implementation of the C ABI in
`../common/floom_sample.h`.

**There is no CMake project here, and that is deliberate.** `build.rs` compiles
the `.c` files in this directory with the `cc` crate and links them into the
Rust crate, so the native code is in the binary from the first build. A second
build system over the same sources would compile them twice and produce
duplicate symbols at link time.

One directory serves Linux, macOS and Windows: the C ABI it implements is the
same on all three, and a per-OS copy would be three files that differ only in
an `#ifdef`.

Replace the body of `floom_sample_enable` with the real work. The header is
the contract, and nothing above it changes.
