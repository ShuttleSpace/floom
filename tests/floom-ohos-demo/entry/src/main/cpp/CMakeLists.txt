cmake_minimum_required(VERSION 3.5.0)
project(floom_entry)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# The N-API host module the ArkTS page imports as `libfloom_entry.so`. It is a
# thin seam: it owns the XComponent surface and the vsync loop, and calls the
# three `floom_harmony_*` functions in the prebuilt backend.
add_library(floom_entry SHARED bridge.cpp)

target_link_libraries(floom_entry PUBLIC
  libace_napi.z.so
  libace_ndk.z.so
  libnative_window.so
  libnative_vsync.so
  libhilog_ndk.z.so
  # The Floom backend, cross-compiled for OHOS and dropped in libs/arm64-v8a
  # so the HAP packages it beside this module (same dir → loader finds it).
  ${CMAKE_CURRENT_SOURCE_DIR}/libs/arm64-v8a/libfloom_platform_harmony.so)
