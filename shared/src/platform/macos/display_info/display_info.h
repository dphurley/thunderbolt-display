#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t display_id;
    uint32_t width;
    uint32_t height;
    uint32_t is_main;
} vt_display_info_t;

// Returns number of displays written to out_displays (up to max_displays).
uint32_t vt_display_list(vt_display_info_t *out_displays, uint32_t max_displays);

#ifdef __cplusplus
}
#endif
