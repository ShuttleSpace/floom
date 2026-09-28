cmake_minimum_required(VERSION 3.5.0)
project(ohos_app_bridge)

set(NATIVERENDER_ROOT_PATH ${CMAKE_CURRENT_SOURCE_DIR})

# The floem Rust runtime is linked as a *shared* library on purpose:
# `libohos.so` carries both the winit C-ABI entry (`ohos_winit_runtime_*`) and
# the floem `#[napi]` exports, and napi_init.cpp merges libohos.so's
# `napi_register_module_v1` into this module's exports (see its file header).
# Linking the staticlib here would duplicate that registration symbol.
add_library(ohos_runtime SHARED IMPORTED)
# IMPORTED_NO_SONAME is REQUIRED, not cosmetic: cargo/lld builds libohos.so
# without a SONAME, so a linker handed its absolute build-time path records that
# path verbatim as the DT_NEEDED entry of libentry.so. On device the loader then
# tries to open
#   .../entry/src/main/cpp/libs/arm64-v8a/libohos.so
# which does not exist, so libentry.so fails to load and the XComponent
# callbacks are never registered (no rendering, no touch). Declaring that the
# library has no SONAME makes CMake link it as `-L<dir> -lohos`, so the linker
# emits the plain `libohos.so`, which resolves next to libentry.so in
# /data/storage/el1/bundle/libs/arm64/.
set_target_properties(ohos_runtime PROPERTIES
    IMPORTED_LOCATION ${NATIVERENDER_ROOT_PATH}/libs/arm64-v8a/libohos.so
    IMPORTED_NO_SONAME TRUE
)

add_library(entry SHARED napi_init.cpp)
set_target_properties(entry PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/out
)
find_library(HILOG_LIB hilog_ndk.z)
# `libace_ndk.z.so` provides the OH_NativeXComponent_* API (surface size, touch
# events, callbacks); `libace_napi.z.so` alone does not export them.
find_library(ACE_NDK_LIB ace_ndk.z)
target_link_libraries(entry PUBLIC libace_napi.z.so ohos_runtime ${ACE_NDK_LIB} ${HILOG_LIB})
