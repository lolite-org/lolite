cmake_minimum_required(VERSION 3.16)

project(c_usage C)

add_executable(c_usage c_usage.c)

set(SONATE_ROOT "${CMAKE_CURRENT_LIST_DIR}/../..")
set(SONATE_INCLUDE_DIR "${SONATE_ROOT}/crates/sonate_lib/include")
set(SONATE_LIB_DIR "${SONATE_ROOT}/target/release")

target_include_directories(c_usage PRIVATE
    "${SONATE_INCLUDE_DIR}"
)

if(WIN32)
    # Link against the import library produced by Cargo (MSVC).
    # This requires that you've built the DLL first (e.g. `cargo build --release` at repo root).
    target_link_libraries(c_usage PRIVATE "${SONATE_LIB_DIR}/sonate.dll.lib")

    # On Windows, ensure the runtime DLL is next to the built executable so it can run
    # directly from the build output directory.
    add_custom_command(TARGET c_usage POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${SONATE_LIB_DIR}/sonate.dll"
            "$<TARGET_FILE_DIR:c_usage>"
        VERBATIM
    )
elseif(APPLE)
    target_link_libraries(c_usage PRIVATE "${SONATE_LIB_DIR}/libsonate.dylib")
else()
    target_link_libraries(c_usage PRIVATE "${SONATE_LIB_DIR}/libsonate.so")
endif()
