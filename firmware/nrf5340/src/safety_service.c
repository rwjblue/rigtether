/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <zephyr/device.h>
#include <zephyr/drivers/hwinfo.h>
#include <zephyr/drivers/watchdog.h>
#include <zephyr/kernel.h>
#include <zephyr/sys/util.h>

#include "rigtether/audio.h"
#include "rigtether/ble_service.h"
#include "rigtether/diagnostics.h"
#include "rigtether/monotonic.h"
#include "rigtether/safety.h"

#define SAFETY_PERIOD_MS 25
#define WATCHDOG_TIMEOUT_MS 250
#define LEASE_MAX_MS 500
#define CONTINUOUS_MAX_MS 60000

static K_MUTEX_DEFINE(safety_lock);
static struct rt_safety_snapshot state;
static const struct device *watchdog;
static int watchdog_channel = -1;
static uint32_t next_fault_id = 1;

static void status_notify_work_handler(struct k_work *work)
{
	ARG_UNUSED(work);
	rt_ble_notify_status();
}

K_WORK_DEFINE(status_notify_work, status_notify_work_handler);

static void status_changed(void)
{
	(void)k_work_submit(&status_notify_work);
}

static void ptt_output_inactive(void)
{
	/*
	 * No GPIO is selected until #13. The board image therefore has no physical
	 * assertion path. A future radio-disconnected overlay must keep this function
	 * as the sole writer and preserve passive inactive bias.
	 */
	state.commanded_ptt = false;
}

static bool inputs_allow_receive_safe(void)
{
	return !state.commanded_ptt && state.inputs.ptt_out_known &&
	       !state.inputs.ptt_out_active && state.inputs.inhibit_closed &&
	       state.inputs.host_route == RT_HEALTH_HEALTHY &&
	       state.inputs.device_audio == RT_HEALTH_HEALTHY &&
	       state.inputs.ble == RT_HEALTH_HEALTHY &&
	       state.inputs.protocol_session == RT_HEALTH_HEALTHY &&
	       state.inputs.radio_profile == RT_HEALTH_HEALTHY;
}

void rt_safety_release(enum rt_release_cause cause, bool lockout)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	ptt_output_inactive();
	rt_audio_mute_tx();
	state.owner_present = false;
	state.lease_deadline_ms = 0;
	state.continuous_started_ms = 0;
	state.last_release = cause;
	state.last_release_at_ms = rt_monotonic_ms();
	if (lockout) {
		state.state = RT_FAULT_LOCKOUT;
		if (!state.first_fault_present) {
			state.first_fault_present = true;
			state.first_fault_id = next_fault_id++;
			state.first_fault = cause;
			state.first_fault_at_ms = state.last_release_at_ms;
			rt_diag_record(RT_EVENT_FIRST_FAULT, cause);
		}
	} else if (inputs_allow_receive_safe()) {
		state.state = RT_RECEIVE_SAFE;
	}
	rt_diag_record(RT_EVENT_RELEASE, cause);
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_ble_connected(void)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.ble = RT_HEALTH_HEALTHY;
	/* Connection is not a session and cannot restore host-route health. */
	state.inputs.protocol_session = RT_HEALTH_UNKNOWN;
	state.inputs.host_route = RT_HEALTH_UNKNOWN;
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_ble_disconnected(void)
{
	rt_safety_release(RT_RELEASE_BLE_DISCONNECT, false);
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.ble = RT_HEALTH_UNHEALTHY;
	state.inputs.protocol_session = RT_HEALTH_UNHEALTHY;
	state.inputs.host_route = RT_HEALTH_UNKNOWN;
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_session_replaced(void)
{
	rt_safety_release(RT_RELEASE_SESSION_REPLACED, false);
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_UNKNOWN;
	state.inputs.host_route = RT_HEALTH_UNKNOWN;
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_protocol_session_active(void)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_HEALTHY;
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_protocol_fault(void)
{
	rt_safety_release(RT_RELEASE_PROTOCOL_FAULT, true);
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_UNHEALTHY;
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_update_inputs(const struct rt_safety_inputs *inputs)
{
	bool release_audio = false;
	bool release_inhibit = false;
	bool output_mismatch = false;

	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs = *inputs;
	release_audio = state.commanded_ptt &&
			inputs->device_audio != RT_HEALTH_HEALTHY;
	release_inhibit = state.commanded_ptt && !inputs->inhibit_closed;
	output_mismatch = state.commanded_ptt && inputs->ptt_out_known &&
			  !inputs->ptt_out_active;
	k_mutex_unlock(&safety_lock);
	status_changed();

	if (output_mismatch) {
		rt_safety_release(RT_RELEASE_OUTPUT_FAILED_ASSERT, true);
	} else if (release_inhibit) {
		rt_safety_release(RT_RELEASE_INHIBIT, false);
	} else if (release_audio) {
		rt_safety_release(RT_RELEASE_DEVICE_AUDIO, false);
	}
}

void rt_safety_snapshot(struct rt_safety_snapshot *snapshot)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	*snapshot = state;
	k_mutex_unlock(&safety_lock);
}

enum rt_recovery_result rt_safety_recover(uint32_t fault_id)
{
	enum rt_recovery_result result;
	k_mutex_lock(&safety_lock, K_FOREVER);
	if (state.state != RT_FAULT_LOCKOUT) {
		result = RT_RECOVERY_NOT_LOCKED;
	} else if (!state.first_fault_present || state.first_fault_id != fault_id) {
		result = RT_RECOVERY_WRONG_FAULT;
	} else if (!inputs_allow_receive_safe()) {
		result = RT_RECOVERY_INCOMPLETE;
	} else {
		state.state = RT_RECEIVE_SAFE;
		state.first_fault_present = false;
		state.first_fault_id = 0;
		state.first_fault_at_ms = 0;
		result = RT_RECOVERY_OK;
	}
	k_mutex_unlock(&safety_lock);
	status_changed();
	return result;
}

bool rt_safety_complete_check_and_feed_watchdog(void)
{
	bool complete = false;
	uint64_t now = rt_monotonic_ms();

	k_mutex_lock(&safety_lock, K_FOREVER);
	if (state.commanded_ptt && state.owner_present &&
	    (now >= state.lease_deadline_ms ||
	     now - state.continuous_started_ms >= CONTINUOUS_MAX_MS)) {
		enum rt_release_cause cause =
			now >= state.lease_deadline_ms ? RT_RELEASE_LEASE_EXPIRED :
							RT_RELEASE_CONTINUOUS_CAP;
		k_mutex_unlock(&safety_lock);
		rt_safety_release(cause, cause == RT_RELEASE_CONTINUOUS_CAP);
		k_mutex_lock(&safety_lock, K_FOREVER);
	}

	if (!state.commanded_ptt && state.inputs.ptt_out_known &&
	    state.inputs.ptt_out_active) {
		k_mutex_unlock(&safety_lock);
		rt_safety_release(RT_RELEASE_OUTPUT_STUCK_ACTIVE, true);
		return false;
	}

	complete = !state.commanded_ptt ||
		   (state.owner_present && state.lease_deadline_ms > now &&
		    state.lease_deadline_ms - now <= LEASE_MAX_MS &&
		    state.inputs.inhibit_closed &&
		    state.inputs.device_audio == RT_HEALTH_HEALTHY &&
		    state.inputs.ble == RT_HEALTH_HEALTHY &&
		    state.inputs.protocol_session == RT_HEALTH_HEALTHY &&
		    state.inputs.radio_profile == RT_HEALTH_HEALTHY);
	k_mutex_unlock(&safety_lock);

	if (complete && watchdog_channel >= 0 &&
	    wdt_feed(watchdog, watchdog_channel) == 0) {
		rt_diag_record(RT_EVENT_WATCHDOG_FEED, 0);
		return true;
	}
	return false;
}

static void safety_thread(void)
{
	while (true) {
		(void)rt_safety_complete_check_and_feed_watchdog();
		k_sleep(K_MSEC(SAFETY_PERIOD_MS));
	}
}

K_THREAD_DEFINE(safety_thread_id, 2048, safety_thread, NULL, NULL, NULL, -2, 0,
		K_FOREVER);

int rt_safety_init(void)
{
	uint32_t reset_cause = 0;
	enum rt_release_cause initial_release = RT_RELEASE_BOOT_OR_UPDATE;
	if (hwinfo_get_reset_cause(&reset_cause) == 0) {
		if ((reset_cause & RESET_WATCHDOG) != 0U) {
			initial_release = RT_RELEASE_WATCHDOG;
		}
		(void)hwinfo_clear_reset_cause();
	}
	state = (struct rt_safety_snapshot){
		.state = RT_RECEIVE_SAFE,
		.inputs = {
			.host_route = RT_HEALTH_UNKNOWN,
			.device_audio = RT_HEALTH_UNHEALTHY,
			.ble = RT_HEALTH_UNHEALTHY,
			.protocol_session = RT_HEALTH_UNHEALTHY,
			.radio_profile = RT_HEALTH_UNHEALTHY,
			.inhibit_closed = false,
			.ptt_out_known = false,
			.ptt_out_active = false,
		},
		.commanded_ptt = false,
		.last_release = initial_release,
		.last_release_at_ms = 0,
	};
	ptt_output_inactive();
	rt_audio_mute_tx();

	watchdog = DEVICE_DT_GET(DT_NODELABEL(wdt0));
	if (!device_is_ready(watchdog)) {
		return -ENODEV;
	}
	const struct wdt_timeout_cfg config = {
		.window = {.min = 0, .max = WATCHDOG_TIMEOUT_MS},
		.flags = WDT_FLAG_RESET_SOC,
	};
	watchdog_channel = wdt_install_timeout(watchdog, &config);
	if (watchdog_channel < 0) {
		return watchdog_channel;
	}
	/* Passing zero keeps the watchdog running in sleep and debug halt. */
	int err = wdt_setup(watchdog, 0);
	if (err == 0) {
		k_thread_start(safety_thread_id);
	}
	return err;
}
