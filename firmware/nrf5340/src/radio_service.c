/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <zephyr/sys/util.h>

#include "rigtether/radio.h"

static enum rt_radio_profile selected_profile = RT_RADIO_PROFILE_KX2;

void rt_radio_select_profile(enum rt_radio_profile profile)
{
	selected_profile = profile;
}

int rt_radio_execute_typed(const struct rt_radio_request *request,
			   struct rt_radio_outcome *outcome)
{
	ARG_UNUSED(request);
	ARG_UNUSED(selected_profile);

	/*
	 * Issue #13 has not selected or validated a radio-side CAT electrical
	 * interface. The nRF fixture therefore has no RadioIo backend and must not
	 * manufacture a successful query. The hardware-independent Rust firmware
	 * core binds this same typed operation set to the #9 transcript simulator.
	 * There is deliberately no raw-CAT entry point in this interface.
	 */
	outcome->error_code = "radio_disconnected";
	outcome->result_json = NULL;
	return -ENOTCONN;
}
