#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct vt_virtual_display vt_virtual_display_t;

typedef struct {
    vt_virtual_display_t *handle;
    uint32_t display_id;
} vt_virtual_display_result_t;

vt_virtual_display_result_t vt_virtual_display_create(uint32_t width, uint32_t height, uint32_t ppi, bool hi_dpi, const char *name_utf8);
void vt_virtual_display_destroy(vt_virtual_display_t *display);
uint32_t vt_virtual_display_id(vt_virtual_display_t *display);

#ifdef __cplusplus
}
#endif
