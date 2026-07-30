/* SPDX-License-Identifier: Apache-2.0 */
#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/sys/reboot.h>

#include "rigtether/audio.h"
#include "rigtether/ble_service.h"
#include "rigtether/diagnostics.h"
#include "rigtether/monotonic.h"
#include "rigtether/safety.h"

LOG_MODULE_REGISTER(rigtether, LOG_LEVEL_INF);

int main(void)
{
	int err;

	/*
	 * Initialize receive-safe ownership before transports. No BLE, CAT, USB, or
	 * response path can run before the sole PTT owner has forced inactivity.
	 */
	err = rt_monotonic_init();
	if (err != 0) {
		return err;
	}
	err = rt_audio_init();
	if (err != 0) {
		return err;
	}
	err = rt_safety_init();
	if (err != 0) {
		return err;
	}
	rt_diag_record(RT_EVENT_BOOT, 0);

	err = rt_ble_service_init();
	if (err != 0) {
		rt_safety_release(RT_RELEASE_PROTOCOL_FAULT, true);
		return err;
	}

	LOG_INF("RigTether M1 radio-disconnected fixture ready");
	LOG_WRN("PTT output and physical evidence remain disabled");
	return 0;
}
