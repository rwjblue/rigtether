/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_AUDIO_H
#define RIGTETHER_AUDIO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

struct rt_audio_health {
	bool configured;
	bool tx_stream_active;
	bool clock_healthy;
	bool buffers_healthy;
	bool converter_healthy;
};

int rt_audio_init(void);
void rt_audio_mute_tx(void);
bool rt_audio_required_healthy(void);
void rt_audio_get_health(struct rt_audio_health *health);
void rt_audio_playback_convert(const int16_t *mono, int32_t *stereo, size_t samples);
void rt_audio_capture_convert(const int32_t *stereo, int16_t *mono, size_t samples);

extern const uint8_t rt_uac1_configuration_descriptor[192];

#endif
