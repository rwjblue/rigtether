/* SPDX-License-Identifier: Apache-2.0 */
#include <limits.h>
#include <zephyr/kernel.h>
#include <zephyr/sys/util.h>

#include "rigtether/audio.h"

static K_MUTEX_DEFINE(audio_lock);
static struct rt_audio_health audio_health;
static bool tx_muted = true;

int rt_audio_init(void)
{
	k_mutex_lock(&audio_lock, K_FOREVER);
	audio_health = (struct rt_audio_health){
		.configured = false,
		.tx_stream_active = false,
		.clock_healthy = false,
		.buffers_healthy = false,
		.converter_healthy = false,
	};
	tx_muted = true;
	k_mutex_unlock(&audio_lock);
	return 0;
}

void rt_audio_mute_tx(void)
{
	k_mutex_lock(&audio_lock, K_FOREVER);
	tx_muted = true;
	k_mutex_unlock(&audio_lock);
}

bool rt_audio_required_healthy(void)
{
	bool healthy;
	k_mutex_lock(&audio_lock, K_FOREVER);
	healthy = audio_health.configured && audio_health.tx_stream_active &&
		  audio_health.clock_healthy && audio_health.buffers_healthy &&
		  audio_health.converter_healthy;
	k_mutex_unlock(&audio_lock);
	return healthy;
}

void rt_audio_get_health(struct rt_audio_health *health)
{
	k_mutex_lock(&audio_lock, K_FOREVER);
	*health = audio_health;
	k_mutex_unlock(&audio_lock);
}

void rt_audio_playback_convert(const int16_t *mono, int32_t *stereo, size_t samples)
{
	for (size_t index = 0; index < samples; ++index) {
		int32_t widened = (int32_t)mono[index] << 8;
		stereo[index * 2] = widened;
		stereo[index * 2 + 1] = widened;
	}
}

void rt_audio_capture_convert(const int32_t *stereo, int16_t *mono, size_t samples)
{
	for (size_t index = 0; index < samples; ++index) {
		int32_t sample = CLAMP(stereo[index * 2], -8388608, 8388607);
		int32_t rounded =
			sample >= 0 ? (sample + 128) >> 8 : -((-sample + 128) >> 8);
		mono[index] = (int16_t)CLAMP(rounded, INT16_MIN, INT16_MAX);
	}
}
