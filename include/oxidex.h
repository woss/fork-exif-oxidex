/**
 * OxiDex C FFI Header
 *
 * C-compatible bindings for the OxiDex library.
 *
 * For complete documentation, see docs/api/ffi_api.md
 *
 * @version 0.1.0
 * @date 2025-10-30
 */

#ifndef OXIDEX_H
#define OXIDEX_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

/* ============================================================================
 * Error Codes
 * ============================================================================ */

/** Success code */
#define OXIDEX_OK                      0
/** I/O error (file not found, permission denied, etc.) */
#define OXIDEX_ERR_IO                  1
/** Parse error (corrupted file, invalid format) */
#define OXIDEX_ERR_PARSE               2
/** Tag not found */
#define OXIDEX_ERR_TAG_NOT_FOUND       3
/** Invalid tag value (type mismatch, out of range) */
#define OXIDEX_ERR_INVALID_TAG_VALUE   4
/** Unsupported format */
#define OXIDEX_ERR_UNSUPPORTED_FORMAT  5
/** NULL pointer provided */
#define OXIDEX_ERR_NULL_POINTER        6
/** Internal error (panic caught) */
#define OXIDEX_ERR_INTERNAL            99

/* ============================================================================
 * Opaque Handle Type
 * ============================================================================ */

/**
 * Opaque handle for OxiDex operations.
 *
 * This is an opaque pointer - C code cannot access its internal structure.
 * Handles must be created with oxidex_create() and destroyed with
 * oxidex_destroy() to prevent memory leaks.
 */
typedef struct OxidexHandle OxidexHandle;

/* ============================================================================
 * Handle Lifecycle Functions
 * ============================================================================ */

/**
 * Creates a new OxiDex handle.
 *
 * @return Pointer to opaque handle on success, NULL if allocation fails
 *
 * @note The returned handle must be destroyed with oxidex_destroy()
 * @note This function is thread-safe
 *
 * @example
 * OxidexHandle* handle = oxidex_create();
 * if (!handle) {
 *     fprintf(stderr, "Out of memory\n");
 *     exit(1);
 * }
 */
OxidexHandle* oxidex_create(void);

/**
 * Destroys a handle and frees all associated resources.
 *
 * @param handle Handle to destroy (can be NULL, which is a no-op)
 *
 * @note After this call, the handle pointer is invalid
 * @note This function is NOT thread-safe for the same handle
 *
 * @example
 * oxidex_destroy(handle);
 * handle = NULL;  // Good practice
 */
void oxidex_destroy(OxidexHandle* handle);

/* ============================================================================
 * Metadata Reading Functions
 * ============================================================================ */

/**
 * Reads metadata from a file.
 *
 * @param handle Handle to store metadata in (must not be NULL)
 * @param filepath Path to file (null-terminated UTF-8, must not be NULL)
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_NULL_POINTER: handle or filepath is NULL
 * @error OXIDEX_ERR_IO: File not found, permission denied, or read error
 * @error OXIDEX_ERR_UNSUPPORTED_FORMAT: File format not recognized
 * @error OXIDEX_ERR_PARSE: File is corrupted or malformed
 *
 * @note This function is NOT thread-safe for the same handle
 *
 * @example
 * int result = oxidex_read_file(handle, "photo.jpg");
 * if (result != OXIDEX_OK) {
 *     fprintf(stderr, "Error: %s\n", oxidex_get_last_error());
 * }
 */
int oxidex_read_file(OxidexHandle* handle, const char* filepath);

/**
 * Returns the number of tags in the metadata.
 *
 * @param handle Handle to query (can be NULL)
 * @return Number of tags (0 if handle is NULL or no metadata loaded)
 *
 * @note This function is thread-safe for read-only access
 *
 * @example
 * size_t count = oxidex_get_tag_count(handle);
 * printf("Found %zu tags\n", count);
 */
size_t oxidex_get_tag_count(const OxidexHandle* handle);

/**
 * Retrieves tag name by index.
 *
 * @param handle Handle to query (must not be NULL)
 * @param index Zero-based index (must be less than tag count)
 * @return Pointer to null-terminated tag name string, NULL if out of bounds
 *
 * @note Returned string is valid until next API call or handle destruction
 * @note This function is thread-safe for read-only access
 *
 * @example
 * for (size_t i = 0; i < oxidex_get_tag_count(handle); i++) {
 *     const char* name = oxidex_get_tag_name_at(handle, i);
 *     printf("Tag %zu: %s\n", i, name);
 * }
 */
const char* oxidex_get_tag_name_at(const OxidexHandle* handle, size_t index);

/**
 * Checks if a tag exists.
 *
 * @param handle Handle to query (can be NULL)
 * @param tag_name Tag name to check (must not be NULL)
 * @return 1 if tag exists, 0 otherwise
 *
 * @note This function is thread-safe for read-only access
 *
 * @example
 * if (oxidex_has_tag(handle, "EXIF:Make")) {
 *     printf("Make tag is present\n");
 * }
 */
int oxidex_has_tag(const OxidexHandle* handle, const char* tag_name);

/* ============================================================================
 * Tag Access Functions
 * ============================================================================ */

/**
 * Retrieves tag value as a string.
 *
 * @param handle Handle to query (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @return Pointer to null-terminated UTF-8 string, NULL if not found or wrong type
 *
 * @note Returned string is valid until next API call or handle destruction
 * @note Returns NULL if the tag is not a String type
 * @note This function is thread-safe for read-only access
 *
 * @example
 * const char* make = oxidex_get_tag_string(handle, "EXIF:Make");
 * if (make) {
 *     printf("Camera: %s\n", make);
 * }
 */
const char* oxidex_get_tag_string(const OxidexHandle* handle, const char* tag_name);

/**
 * Retrieves tag value as a 64-bit integer.
 *
 * @param handle Handle to query (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @param out_value Pointer to output variable (must not be NULL)
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_TAG_NOT_FOUND: Tag doesn't exist
 * @error OXIDEX_ERR_INVALID_TAG_VALUE: Tag is not an Integer type
 * @error OXIDEX_ERR_NULL_POINTER: Any parameter is NULL
 *
 * @note This function is thread-safe for read-only access
 *
 * @example
 * int64_t iso;
 * if (oxidex_get_tag_integer(handle, "EXIF:ISO", &iso) == OXIDEX_OK) {
 *     printf("ISO: %lld\n", (long long)iso);
 * }
 */
int oxidex_get_tag_integer(const OxidexHandle* handle, const char* tag_name, int64_t* out_value);

/**
 * Retrieves tag value as a double-precision float.
 *
 * @param handle Handle to query (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @param out_value Pointer to output variable (must not be NULL)
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_TAG_NOT_FOUND: Tag doesn't exist
 * @error OXIDEX_ERR_INVALID_TAG_VALUE: Tag is not a Float type
 * @error OXIDEX_ERR_NULL_POINTER: Any parameter is NULL
 *
 * @note This function is thread-safe for read-only access
 *
 * @example
 * double aperture;
 * if (oxidex_get_tag_float(handle, "EXIF:FNumber", &aperture) == OXIDEX_OK) {
 *     printf("Aperture: f/%.1f\n", aperture);
 * }
 */
int oxidex_get_tag_float(const OxidexHandle* handle, const char* tag_name, double* out_value);

/* ============================================================================
 * Metadata Writing Functions
 * ============================================================================ */

/**
 * Sets a tag value to a string.
 *
 * @param handle Handle to modify (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @param value String value (null-terminated UTF-8, must not be NULL)
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_NULL_POINTER: Any parameter is NULL
 *
 * @note This function is NOT thread-safe
 *
 * @example
 * oxidex_set_tag_string(handle, "EXIF:Artist", "John Doe");
 */
int oxidex_set_tag_string(OxidexHandle* handle, const char* tag_name, const char* value);

/**
 * Sets a tag value to an integer.
 *
 * @param handle Handle to modify (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @param value Integer value
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_NULL_POINTER: handle or tag_name is NULL
 *
 * @note This function is NOT thread-safe
 *
 * @example
 * oxidex_set_tag_integer(handle, "EXIF:ISO", 800);
 */
int oxidex_set_tag_integer(OxidexHandle* handle, const char* tag_name, int64_t value);

/**
 * Sets a tag value to a floating-point number.
 *
 * @param handle Handle to modify (must not be NULL)
 * @param tag_name Tag name (must not be NULL)
 * @param value Float value
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_NULL_POINTER: handle or tag_name is NULL
 * @error OXIDEX_ERR_INVALID_TAG_VALUE: value is NaN or infinity
 *
 * @note This function is NOT thread-safe
 *
 * @example
 * oxidex_set_tag_float(handle, "EXIF:FNumber", 2.8);
 */
int oxidex_set_tag_float(OxidexHandle* handle, const char* tag_name, double value);

/**
 * Removes a tag from the metadata.
 *
 * @param handle Handle to modify (must not be NULL)
 * @param tag_name Tag name to remove (must not be NULL)
 * @return OXIDEX_OK (always succeeds, even if tag didn't exist)
 *
 * @error OXIDEX_ERR_NULL_POINTER: handle or tag_name is NULL
 *
 * @note This function is NOT thread-safe
 *
 * @example
 * oxidex_remove_tag(handle, "EXIF:Thumbnail");
 */
int oxidex_remove_tag(OxidexHandle* handle, const char* tag_name);

/**
 * Writes metadata to a file.
 *
 * @param handle Handle containing metadata (must not be NULL)
 * @param filepath Path to file (null-terminated UTF-8, must not be NULL)
 * @return OXIDEX_OK on success, error code on failure
 *
 * @error OXIDEX_ERR_NULL_POINTER: handle or filepath is NULL
 * @error OXIDEX_ERR_IO: File not writable, disk full, permission denied
 * @error OXIDEX_ERR_UNSUPPORTED_FORMAT: Format doesn't support writing
 * @error OXIDEX_ERR_INVALID_TAG_VALUE: Metadata validation failed
 *
 * @note Write is atomic - original file is unchanged on error
 * @note This function is thread-safe for read-only handle access
 *
 * @example
 * if (oxidex_write_file(handle, "output.jpg") != OXIDEX_OK) {
 *     fprintf(stderr, "Write failed: %s\n", oxidex_get_last_error());
 * }
 */
int oxidex_write_file(const OxidexHandle* handle, const char* filepath);

/* ============================================================================
 * Error Handling Functions
 * ============================================================================ */

/**
 * Retrieves the last error message for the current thread.
 *
 * @return Pointer to null-terminated error message string (never NULL)
 *
 * @note Returns "No error" if no error has occurred
 * @note Each thread has its own error message (thread-local storage)
 * @note String is valid until next error on same thread or thread termination
 * @note This function is thread-safe
 *
 * @example
 * int result = oxidex_read_file(handle, "photo.jpg");
 * if (result != OXIDEX_OK) {
 *     fprintf(stderr, "Error: %s\n", oxidex_get_last_error());
 * }
 */
const char* oxidex_get_last_error(void);

#ifdef __cplusplus
}
#endif

#endif /* OXIDEX_H */
