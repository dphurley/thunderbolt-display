#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct vt_h264_encoder vt_h264_encoder_t;
typedef struct vt_h264_decoder vt_h264_decoder_t;

typedef struct {
    const uint8_t *data;
    size_t size;
    bool is_keyframe;
} vt_h264_encoded_frame_t;

typedef struct {
    uint8_t *data;
    size_t size;
    uint32_t width;
    uint32_t height;
} vt_h264_decoded_frame_t;

vt_h264_encoder_t *vt_h264_encoder_create(uint32_t width, uint32_t height, uint32_t bitrate, uint32_t fps);
void vt_h264_encoder_destroy(vt_h264_encoder_t *encoder);

// Encodes an RGBA frame (width*height*4 bytes). Returns true on success.
// Output frame data is owned by encoder until next encode call.
bool vt_h264_encoder_encode(vt_h264_encoder_t *encoder, const uint8_t *rgba_data, size_t rgba_size, vt_h264_encoded_frame_t *out_frame);

vt_h264_decoder_t *vt_h264_decoder_create(void);
void vt_h264_decoder_destroy(vt_h264_decoder_t *decoder);

// Decodes an H.264 Annex B frame. Output frame data is owned by decoder until next decode call.
bool vt_h264_decoder_decode(vt_h264_decoder_t *decoder, const uint8_t *data, size_t size, vt_h264_decoded_frame_t *out_frame);

#ifdef __cplusplus
}
#endif
