#import "display_info.h"
#import <CoreGraphics/CoreGraphics.h>

uint32_t vt_display_list(vt_display_info_t *out_displays, uint32_t max_displays) {
    if (!out_displays || max_displays == 0) {
        return 0;
    }

    CGDirectDisplayID display_ids[32];
    uint32_t display_count = 0;
    CGError error = CGGetActiveDisplayList(32, display_ids, &display_count);
    if (error != kCGErrorSuccess) {
        return 0;
    }

    uint32_t written = 0;
    for (uint32_t i = 0; i < display_count && written < max_displays; i++) {
        CGDirectDisplayID display_id = display_ids[i];
        CGRect bounds = CGDisplayBounds(display_id);
        out_displays[written].display_id = (uint32_t)display_id;
        out_displays[written].width = (uint32_t)bounds.size.width;
        out_displays[written].height = (uint32_t)bounds.size.height;
        out_displays[written].is_main = CGDisplayIsMain(display_id) ? 1 : 0;
        written++;
    }

    return written;
}
