#import "virtual_display.h"
#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>

// Private CoreGraphics interfaces (from Chromium's mac/virtual_display_util.mm)
@interface CGVirtualDisplayDescriptor : NSObject
@property(nonatomic, assign) CGSize sizeInPixels;
@property(nonatomic, assign) CGFloat pixelsPerInch;
@property(nonatomic, assign) BOOL hiDPI;
@property(nonatomic, copy) NSString *name;
@end

@interface CGVirtualDisplaySettings : NSObject
@property(nonatomic, assign) CGSize sizeInPixels;
@property(nonatomic, assign) CGFloat pixelsPerInch;
@end

typedef void (^CGVirtualDisplayChangeHandler)(CGVirtualDisplaySettings *settings, NSError *error);

@interface CGVirtualDisplay : NSObject
@property(nonatomic, readonly) CGDirectDisplayID displayID;
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)descriptor;
- (void)applySettings:(CGVirtualDisplaySettings *)settings;
- (void)setPreferredSettings:(CGVirtualDisplaySettings *)settings;
- (void)startWithQueue:(dispatch_queue_t)queue handler:(CGVirtualDisplayChangeHandler)handler;
- (void)stop;
@end

struct vt_virtual_display {
    CGVirtualDisplay *display;
};

vt_virtual_display_result_t vt_virtual_display_create(uint32_t width, uint32_t height, uint32_t ppi, bool hi_dpi, const char *name_utf8) {
    vt_virtual_display_result_t result;
    result.handle = NULL;
    result.display_id = 0;

    @autoreleasepool {
        CGVirtualDisplayDescriptor *descriptor = [CGVirtualDisplayDescriptor new];
        descriptor.sizeInPixels = CGSizeMake(width, height);
        descriptor.pixelsPerInch = (CGFloat)ppi;
        descriptor.hiDPI = hi_dpi ? YES : NO;
        if (name_utf8) {
            descriptor.name = [NSString stringWithUTF8String:name_utf8];
        }

        CGVirtualDisplay *display = [[CGVirtualDisplay alloc] initWithDescriptor:descriptor];
        if (!display) {
            return result;
        }

        CGVirtualDisplaySettings *settings = [CGVirtualDisplaySettings new];
        settings.sizeInPixels = CGSizeMake(width, height);
        settings.pixelsPerInch = (CGFloat)ppi;

        [display applySettings:settings];
        [display setPreferredSettings:settings];

        dispatch_queue_t queue = dispatch_get_main_queue();
        [display startWithQueue:queue handler:^(CGVirtualDisplaySettings *settings, NSError *error) {
            (void)settings;
            (void)error;
        }];

        vt_virtual_display_t *handle = calloc(1, sizeof(vt_virtual_display_t));
        if (!handle) {
            [display stop];
            return result;
        }

        handle->display = display;
        result.handle = handle;
        result.display_id = display.displayID;
    }

    return result;
}

void vt_virtual_display_destroy(vt_virtual_display_t *display) {
    if (!display) {
        return;
    }
    @autoreleasepool {
        if (display->display) {
            [display->display stop];
            display->display = nil;
        }
    }
    free(display);
}

uint32_t vt_virtual_display_id(vt_virtual_display_t *display) {
    if (!display || !display->display) {
        return 0;
    }
    return display->display.displayID;
}
