#import "vtbridge.h"
#import <Foundation/Foundation.h>
#import <VideoToolbox/VideoToolbox.h>
#import <CoreVideo/CoreVideo.h>
#import <CoreMedia/CoreMedia.h>
#import <QuartzCore/QuartzCore.h>
#import <math.h>

struct vt_h264_encoder {
    VTCompressionSessionRef session;
    dispatch_semaphore_t semaphore;
    uint8_t *output;
    size_t output_size;
    bool is_keyframe;
    uint32_t width;
    uint32_t height;
    uint8_t *sps;
    size_t sps_size;
    uint8_t *pps;
    size_t pps_size;
};

struct vt_h264_decoder {
    VTDecompressionSessionRef session;
    CMVideoFormatDescriptionRef format;
    uint8_t *output;
    size_t output_size;
    uint32_t width;
    uint32_t height;
    uint8_t *sps;
    size_t sps_size;
    uint8_t *pps;
    size_t pps_size;
    dispatch_semaphore_t semaphore;
};

static void free_buffer(uint8_t **buffer, size_t *size) {
    if (*buffer) {
        free(*buffer);
        *buffer = NULL;
        *size = 0;
    }
}

static void set_buffer(uint8_t **buffer, size_t *size, const uint8_t *data, size_t data_size) {
    free_buffer(buffer, size);
    if (data_size == 0) {
        return;
    }
    *buffer = malloc(data_size);
    if (*buffer) {
        memcpy(*buffer, data, data_size);
        *size = data_size;
    }
}

static void append_bytes(uint8_t **buffer, size_t *size, const uint8_t *data, size_t data_size) {
    size_t new_size = *size + data_size;
    uint8_t *new_buffer = realloc(*buffer, new_size);
    if (!new_buffer) {
        return;
    }
    memcpy(new_buffer + *size, data, data_size);
    *buffer = new_buffer;
    *size = new_size;
}

static void vt_compression_callback(void *output_callback_ref_con,
                                   void *source_frame_ref_con,
                                   OSStatus status,
                                   VTEncodeInfoFlags info_flags,
                                   CMSampleBufferRef sample_buffer) {
    (void)source_frame_ref_con;
    (void)info_flags;
    vt_h264_encoder_t *encoder = (vt_h264_encoder_t *)output_callback_ref_con;
    if (status != noErr || !sample_buffer) {
        dispatch_semaphore_signal(encoder->semaphore);
        return;
    }

    bool is_keyframe = false;
    CFArrayRef attachments = CMSampleBufferGetSampleAttachmentsArray(sample_buffer, false);
    if (attachments && CFArrayGetCount(attachments) > 0) {
        CFDictionaryRef dict = (CFDictionaryRef)CFArrayGetValueAtIndex(attachments, 0);
        is_keyframe = !CFDictionaryContainsKey(dict, kCMSampleAttachmentKey_NotSync);
    }

    encoder->is_keyframe = is_keyframe;
    free_buffer(&encoder->output, &encoder->output_size);

    CMBlockBufferRef data_buffer = CMSampleBufferGetDataBuffer(sample_buffer);
    if (!data_buffer) {
        dispatch_semaphore_signal(encoder->semaphore);
        return;
    }

    size_t total_length = 0;
    char *data_pointer = NULL;
    if (CMBlockBufferGetDataPointer(data_buffer, 0, NULL, &total_length, &data_pointer) != kCMBlockBufferNoErr) {
        dispatch_semaphore_signal(encoder->semaphore);
        return;
    }

    const uint8_t start_code[4] = {0, 0, 0, 1};

    if (is_keyframe) {
        CMVideoFormatDescriptionRef format = CMSampleBufferGetFormatDescription(sample_buffer);
        if (format) {
            const uint8_t *sps = NULL;
            const uint8_t *pps = NULL;
            size_t sps_size = 0;
            size_t pps_size = 0;
            size_t sps_count = 0;
            size_t pps_count = 0;
            if (CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, 0, &sps, &sps_size, &sps_count, NULL) == noErr &&
                CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, 1, &pps, &pps_size, &pps_count, NULL) == noErr) {
                append_bytes(&encoder->output, &encoder->output_size, start_code, 4);
                append_bytes(&encoder->output, &encoder->output_size, sps, sps_size);
                append_bytes(&encoder->output, &encoder->output_size, start_code, 4);
                append_bytes(&encoder->output, &encoder->output_size, pps, pps_size);
                set_buffer(&encoder->sps, &encoder->sps_size, sps, sps_size);
                set_buffer(&encoder->pps, &encoder->pps_size, pps, pps_size);
            }
        }
    }

    size_t offset = 0;
    while (offset + 4 <= total_length) {
        uint32_t nalu_length = 0;
        memcpy(&nalu_length, data_pointer + offset, 4);
        nalu_length = CFSwapInt32BigToHost(nalu_length);
        offset += 4;
        if (offset + nalu_length > total_length) {
            break;
        }
        append_bytes(&encoder->output, &encoder->output_size, start_code, 4);
        append_bytes(&encoder->output, &encoder->output_size, (uint8_t *)data_pointer + offset, nalu_length);
        offset += nalu_length;
    }

    dispatch_semaphore_signal(encoder->semaphore);
}

vt_h264_encoder_t *vt_h264_encoder_create(uint32_t width, uint32_t height, uint32_t bitrate, uint32_t fps) {
    vt_h264_encoder_t *encoder = calloc(1, sizeof(vt_h264_encoder_t));
    if (!encoder) {
        return NULL;
    }

    encoder->width = width;
    encoder->height = height;

    OSStatus status = VTCompressionSessionCreate(NULL, width, height, kCMVideoCodecType_H264, NULL, NULL, NULL,
                                                 vt_compression_callback, encoder, &encoder->session);
    if (status != noErr) {
        free(encoder);
        return NULL;
    }

    VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_RealTime, kCFBooleanTrue);
    VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanFalse);
    VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_H264_Baseline_AutoLevel);

    CFNumberRef bitrate_number = CFNumberCreate(NULL, kCFNumberSInt32Type, &bitrate);
    if (bitrate_number) {
        VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_AverageBitRate, bitrate_number);
        CFRelease(bitrate_number);
    }

    CFNumberRef fps_number = CFNumberCreate(NULL, kCFNumberSInt32Type, &fps);
    if (fps_number) {
        VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_ExpectedFrameRate, fps_number);
        CFRelease(fps_number);
    }

    uint32_t keyframe_interval = fps * 2;
    CFNumberRef keyframe_number = CFNumberCreate(NULL, kCFNumberSInt32Type, &keyframe_interval);
    if (keyframe_number) {
        VTSessionSetProperty(encoder->session, kVTCompressionPropertyKey_MaxKeyFrameInterval, keyframe_number);
        CFRelease(keyframe_number);
    }

    encoder->semaphore = dispatch_semaphore_create(0);
    VTCompressionSessionPrepareToEncodeFrames(encoder->session);

    return encoder;
}

void vt_h264_encoder_destroy(vt_h264_encoder_t *encoder) {
    if (!encoder) {
        return;
    }
    if (encoder->session) {
        VTCompressionSessionInvalidate(encoder->session);
        CFRelease(encoder->session);
    }
    free_buffer(&encoder->output, &encoder->output_size);
    free_buffer(&encoder->sps, &encoder->sps_size);
    free_buffer(&encoder->pps, &encoder->pps_size);
    if (encoder->semaphore) {
        encoder->semaphore = NULL;
    }
    free(encoder);
}

bool vt_h264_encoder_encode(vt_h264_encoder_t *encoder, const uint8_t *rgba_data, size_t rgba_size, vt_h264_encoded_frame_t *out_frame) {
    if (!encoder || !encoder->session || !rgba_data || rgba_size == 0) {
        return false;
    }

    uint32_t width = encoder->width;
    uint32_t height = encoder->height;
    size_t expected_size = (size_t)width * (size_t)height * 4;
    if (rgba_size < expected_size) {
        return false;
    }

    CVPixelBufferRef pixel_buffer = NULL;
    NSDictionary *attributes = @{(id)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_32BGRA)};
    CVReturn cv_status = CVPixelBufferCreate(kCFAllocatorDefault, width, height, kCVPixelFormatType_32BGRA,
                                             (__bridge CFDictionaryRef)attributes, &pixel_buffer);
    if (cv_status != kCVReturnSuccess) {
        return false;
    }

    CVPixelBufferLockBaseAddress(pixel_buffer, 0);
    uint8_t *dst = (uint8_t *)CVPixelBufferGetBaseAddress(pixel_buffer);
    size_t dst_bytes = CVPixelBufferGetDataSize(pixel_buffer);
    size_t copy_size = rgba_size < dst_bytes ? rgba_size : dst_bytes;

    for (size_t i = 0; i + 3 < copy_size; i += 4) {
        uint8_t r = rgba_data[i];
        uint8_t g = rgba_data[i + 1];
        uint8_t b = rgba_data[i + 2];
        uint8_t a = rgba_data[i + 3];
        dst[i] = b;
        dst[i + 1] = g;
        dst[i + 2] = r;
        dst[i + 3] = a;
    }

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

    CMTime pts = CMTimeMakeWithSeconds(CACurrentMediaTime(), 1000000000);
    OSStatus status = VTCompressionSessionEncodeFrame(encoder->session, pixel_buffer, pts, kCMTimeInvalid, NULL, NULL, NULL);
    CVPixelBufferRelease(pixel_buffer);

    if (status != noErr) {
        return false;
    }

    dispatch_semaphore_wait(encoder->semaphore, DISPATCH_TIME_FOREVER);

    if (!encoder->output || encoder->output_size == 0) {
        return false;
    }

    out_frame->data = encoder->output;
    out_frame->size = encoder->output_size;
    out_frame->is_keyframe = encoder->is_keyframe;
    return true;
}

static void vt_decompression_callback(void *decompression_output_ref_con,
                                     void *source_frame_ref_con,
                                     OSStatus status,
                                     VTDecodeInfoFlags info_flags,
                                     CVImageBufferRef image_buffer,
                                     CMTime presentation_time_stamp,
                                     CMTime presentation_duration) {
    (void)source_frame_ref_con;
    (void)info_flags;
    (void)presentation_time_stamp;
    (void)presentation_duration;

    vt_h264_decoder_t *decoder = (vt_h264_decoder_t *)decompression_output_ref_con;
    if (status != noErr || !image_buffer) {
        dispatch_semaphore_signal(decoder->semaphore);
        return;
    }

    CVPixelBufferRef pixel_buffer = (CVPixelBufferRef)image_buffer;
    CVPixelBufferLockBaseAddress(pixel_buffer, 0);
    size_t data_size = CVPixelBufferGetDataSize(pixel_buffer);
    uint8_t *base = (uint8_t *)CVPixelBufferGetBaseAddress(pixel_buffer);

    free_buffer(&decoder->output, &decoder->output_size);
    decoder->output = malloc(data_size);
    if (decoder->output) {
        memcpy(decoder->output, base, data_size);
        decoder->output_size = data_size;
        decoder->width = (uint32_t)CVPixelBufferGetWidth(pixel_buffer);
        decoder->height = (uint32_t)CVPixelBufferGetHeight(pixel_buffer);
    }

    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
    dispatch_semaphore_signal(decoder->semaphore);
}

vt_h264_decoder_t *vt_h264_decoder_create(void) {
    vt_h264_decoder_t *decoder = calloc(1, sizeof(vt_h264_decoder_t));
    if (!decoder) {
        return NULL;
    }
    decoder->semaphore = dispatch_semaphore_create(0);
    return decoder;
}

void vt_h264_decoder_destroy(vt_h264_decoder_t *decoder) {
    if (!decoder) {
        return;
    }
    if (decoder->session) {
        VTDecompressionSessionInvalidate(decoder->session);
        CFRelease(decoder->session);
    }
    if (decoder->format) {
        CFRelease(decoder->format);
    }
    free_buffer(&decoder->output, &decoder->output_size);
    free_buffer(&decoder->sps, &decoder->sps_size);
    free_buffer(&decoder->pps, &decoder->pps_size);
    free(decoder);
}

static bool parse_annexb(const uint8_t *data, size_t size, CFMutableArrayRef nalus) {
    size_t i = 0;
    while (i + 3 < size) {
        size_t start = i;
        size_t start_code_size = 0;
        if (data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1) {
            start_code_size = 3;
        } else if (data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1) {
            start_code_size = 4;
        }
        if (start_code_size == 0) {
            i++;
            continue;
        }
        start = i + start_code_size;
        i = start;
        while (i + 3 < size) {
            if (data[i] == 0 && data[i + 1] == 0 && (data[i + 2] == 1 || (data[i + 2] == 0 && data[i + 3] == 1))) {
                break;
            }
            i++;
        }
        size_t nalu_size = i - start;
        if (nalu_size > 0) {
            CFDataRef nalu = CFDataCreate(kCFAllocatorDefault, data + start, nalu_size);
            if (nalu) {
                CFArrayAppendValue(nalus, nalu);
                CFRelease(nalu);
            }
        }
    }
    return CFArrayGetCount(nalus) > 0;
}

static void update_parameter_sets(vt_h264_decoder_t *decoder, CFArrayRef nalus) {
    CFIndex count = CFArrayGetCount(nalus);
    for (CFIndex i = 0; i < count; i++) {
        CFDataRef nalu = (CFDataRef)CFArrayGetValueAtIndex(nalus, i);
        const uint8_t *bytes = CFDataGetBytePtr(nalu);
        size_t length = (size_t)CFDataGetLength(nalu);
        if (length == 0) {
            continue;
        }
        uint8_t nalu_type = bytes[0] & 0x1F;
        if (nalu_type == 7) {
            set_buffer(&decoder->sps, &decoder->sps_size, bytes, length);
        } else if (nalu_type == 8) {
            set_buffer(&decoder->pps, &decoder->pps_size, bytes, length);
        }
    }
}

static bool ensure_decoder_session(vt_h264_decoder_t *decoder) {
    if (decoder->session && decoder->format) {
        return true;
    }
    if (!decoder->sps || !decoder->pps) {
        return false;
    }

    const uint8_t *parameter_sets[2] = {decoder->sps, decoder->pps};
    size_t parameter_set_sizes[2] = {decoder->sps_size, decoder->pps_size};
    CMVideoFormatDescriptionRef format = NULL;
    OSStatus status = CMVideoFormatDescriptionCreateFromH264ParameterSets(NULL, 2, parameter_sets,
                                                                          parameter_set_sizes, 4, &format);
    if (status != noErr) {
        return false;
    }

    VTDecompressionOutputCallbackRecord callback = {vt_decompression_callback, decoder};
    VTDecompressionSessionRef session = NULL;
    status = VTDecompressionSessionCreate(NULL, format, NULL, NULL, &callback, &session);
    if (status != noErr) {
        CFRelease(format);
        return false;
    }

    decoder->format = format;
    decoder->session = session;
    return true;
}

bool vt_h264_decoder_decode(vt_h264_decoder_t *decoder, const uint8_t *data, size_t size, vt_h264_decoded_frame_t *out_frame) {
    if (!decoder || !data || size == 0) {
        return false;
    }

    CFMutableArrayRef nalus = CFArrayCreateMutable(kCFAllocatorDefault, 0, &kCFTypeArrayCallBacks);
    if (!nalus) {
        return false;
    }

    bool parsed = parse_annexb(data, size, nalus);
    if (!parsed) {
        CFRelease(nalus);
        return false;
    }

    update_parameter_sets(decoder, nalus);
    if (!ensure_decoder_session(decoder)) {
        CFRelease(nalus);
        return false;
    }

    // Build AVCC buffer
    CFIndex nalu_count = CFArrayGetCount(nalus);
    size_t total_size = 0;
    for (CFIndex i = 0; i < nalu_count; i++) {
        CFDataRef nalu = (CFDataRef)CFArrayGetValueAtIndex(nalus, i);
        total_size += 4 + (size_t)CFDataGetLength(nalu);
    }

    uint8_t *avcc = malloc(total_size);
    if (!avcc) {
        CFRelease(nalus);
        return false;
    }

    size_t offset = 0;
    for (CFIndex i = 0; i < nalu_count; i++) {
        CFDataRef nalu = (CFDataRef)CFArrayGetValueAtIndex(nalus, i);
        size_t length = (size_t)CFDataGetLength(nalu);
        uint32_t be_length = CFSwapInt32HostToBig((uint32_t)length);
        memcpy(avcc + offset, &be_length, 4);
        offset += 4;
        memcpy(avcc + offset, CFDataGetBytePtr(nalu), length);
        offset += length;
    }

    CMBlockBufferRef block_buffer = NULL;
    OSStatus status = CMBlockBufferCreateWithMemoryBlock(NULL, avcc, total_size, kCFAllocatorDefault, NULL, 0, total_size, 0, &block_buffer);
    if (status != noErr) {
        free(avcc);
        CFRelease(nalus);
        return false;
    }

    CMSampleBufferRef sample_buffer = NULL;
    status = CMSampleBufferCreateReady(NULL, block_buffer, decoder->format, 1, 0, NULL, 1, NULL, &sample_buffer);
    if (status != noErr) {
        CFRelease(block_buffer);
        free(avcc);
        CFRelease(nalus);
        return false;
    }

    VTDecodeFrameFlags flags = 0;
    VTDecodeInfoFlags info_flags = 0;
    status = VTDecompressionSessionDecodeFrame(decoder->session, sample_buffer, flags, NULL, &info_flags);
    if (status != noErr) {
        CFRelease(sample_buffer);
        CFRelease(block_buffer);
        free(avcc);
        CFRelease(nalus);
        return false;
    }

    dispatch_semaphore_wait(decoder->semaphore, DISPATCH_TIME_FOREVER);

    if (!decoder->output || decoder->output_size == 0) {
        CFRelease(sample_buffer);
        CFRelease(block_buffer);
        free(avcc);
        CFRelease(nalus);
        return false;
    }

    out_frame->data = decoder->output;
    out_frame->size = decoder->output_size;
    out_frame->width = decoder->width;
    out_frame->height = decoder->height;

    CFRelease(sample_buffer);
    CFRelease(block_buffer);
    free(avcc);
    CFRelease(nalus);
    return true;
}
