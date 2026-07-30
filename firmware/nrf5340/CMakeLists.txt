cmake_minimum_required(VERSION 3.20.0)

find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})
project(rigtether_m1)

target_include_directories(app PRIVATE include)
target_sources(
  app
  PRIVATE
    src/audio_service.c
    src/ble_service.c
    src/diagnostics.c
    src/main.c
    src/monotonic.c
    src/operation_cache.c
    src/protocol_service.c
    src/safety_service.c
    src/usb_descriptors.c
)
