#ifndef SONATE_H
#define SONATE_H

#include <stdbool.h> /* bool */
#include <stddef.h>  /* size_t */
#include <stdint.h>  /* uint64_t */
#include <stdlib.h>

#if defined(_WIN32) || defined(__CYGWIN__)
    #if defined(SONATE_LIB_EXPORTS)
        #define SONATE_API __declspec(dllexport)
    #else
        #define SONATE_API __declspec(dllimport)
    #endif
#else
    #define SONATE_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* Handle type for engine instances (matches Rust: pub type EngineHandle = usize) */
typedef size_t sonate_engine_handle_t;

/* ID type for nodes and other engine-owned objects. */
typedef uint64_t sonate_id_t;

typedef enum sonate_event_type_t {
    SONATE_EVENT_CLICK = 1,
    SONATE_EVENT_SCROLL = 2,
} sonate_event_type_t;

/*
 * Generic event payload.
 *
 * For SONATE_EVENT_CLICK:
 *   - x/y are cursor coordinates
 *   - element_ids/element_count describe hit-tested elements at cursor
 *
 * For SONATE_EVENT_SCROLL:
 *   - scroll_target_id identifies the scrolled node
 *   - scroll_dx/scroll_dy are the deltas
 *   - element_ids/element_count are unused
 *
 * element_ids points to memory owned by sonate and is only valid
 * for the duration of the callback invocation.
 */
typedef struct sonate_event_t {
    sonate_event_type_t event_type;
    double x;
    double y;
    sonate_id_t scroll_target_id;
    double scroll_dx;
    double scroll_dy;
    const sonate_id_t* element_ids;
    size_t element_count;
} sonate_event_t;

typedef void (*sonate_event_callback_t)(
    sonate_engine_handle_t handle,
    const sonate_event_t* event,
    void* user_data
);

/*
 * Initialize the sonate engine.
 *
 * use_same_process:
 *   - true  => run in same process
 *   - false => run in worker process
 *
 * Returns:
 *   engine handle on success, 0 on error
 */
SONATE_API sonate_engine_handle_t sonate_init(bool use_same_process);

/*
 * Add a CSS stylesheet to the engine.
 *
 * css_content: null-terminated UTF-8 string (must not be NULL)
 */
SONATE_API void sonate_add_stylesheet(sonate_engine_handle_t handle, const char* css_content);

/*
 * Create a new document node.
 *
 * node_id:
 *   caller-provided node id (must be non-zero; 0 is reserved for root)
 *
 * text_content:
 *   optional null-terminated UTF-8 string (may be NULL)
 *
 * Returns:
 *   node_id on success, 0 on error
 */
SONATE_API sonate_id_t sonate_create_node(sonate_engine_handle_t handle, sonate_id_t node_id, const char* text_content);

/*
 * Set parent-child relationship between nodes.
 */
SONATE_API void sonate_set_parent(sonate_engine_handle_t handle, sonate_id_t parent_id, sonate_id_t child_id);

/*
 * Set an attribute on a node.
 *
 * key/value: null-terminated UTF-8 strings (must not be NULL)
 */
SONATE_API void sonate_set_attribute(sonate_engine_handle_t handle, sonate_id_t node_id, const char* key, const char* value);

/*
 * Remove a node and all of its descendants from the document.
 *
 * node_id:
 *   target node id (must be non-zero; 0 is reserved for root)
 */
SONATE_API void sonate_destroy_node(sonate_engine_handle_t handle, sonate_id_t node_id);

/*
 * Get the root node ID of the document.
 *
 * Returns:
 *   root node id (0) or 0 if handle is invalid
 */
SONATE_API sonate_id_t sonate_root_id(sonate_engine_handle_t handle);

/*
 * Register a generic event callback.
 *
 * callback may be NULL to clear the current callback.
 *
 * Returns:
 *   0 on success, -1 on error
 */
SONATE_API int sonate_set_event_callback(
    sonate_engine_handle_t handle,
    sonate_event_callback_t callback,
    void* user_data
);

/*
 * Run the engine event loop (blocking).
 *
 * Returns:
 *   0 on success, -1 on error
 */
SONATE_API int sonate_run(sonate_engine_handle_t handle);

/*
 * Cleanup and destroy an engine instance.
 *
 * Returns:
 *   0 on success, -1 on error
 */
SONATE_API int sonate_destroy(sonate_engine_handle_t handle);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SONATE_H */