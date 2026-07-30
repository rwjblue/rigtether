/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_RADIO_H
#define RIGTETHER_RADIO_H

#include <stdint.h>

enum rt_radio_profile {
	RT_RADIO_PROFILE_KX2,
	RT_RADIO_PROFILE_KX3,
};

enum rt_radio_operation {
	RT_RADIO_NORMALIZE_SESSION,
	RT_RADIO_IDENTIFY,
	RT_RADIO_READ_FIRMWARE,
	RT_RADIO_READ_VFO_A,
	RT_RADIO_SET_VFO_A,
	RT_RADIO_READ_OPERATING_STATE,
	RT_RADIO_READ_MODE,
	RT_RADIO_READ_TX_STATE,
};

struct rt_radio_request {
	enum rt_radio_operation operation;
	uint64_t frequency_hz;
};

struct rt_radio_outcome {
	const char *error_code;
	const char *result_json;
};

void rt_radio_select_profile(enum rt_radio_profile profile);
int rt_radio_execute_typed(const struct rt_radio_request *request,
			   struct rt_radio_outcome *outcome);

#endif
