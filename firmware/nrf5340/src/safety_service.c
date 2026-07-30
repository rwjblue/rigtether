/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <string.h>
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
#define RELEASE_MAX_MS 100
#define ASSERTION_CHECK_MS (RELEASE_MAX_MS - SAFETY_PERIOD_MS)
#define CONTINUOUS_MAX_MS 60000
#define REARM_MIN_MS 1000
#define MAX_SESSION_OPERATIONS 512

static K_MUTEX_DEFINE(safety_lock);
static struct rt_safety_snapshot state;
static const struct device *watchdog;
static int watchdog_channel = -1;
static uint32_t next_fault_id = 1;
static char expired_lease_ids[MAX_SESSION_OPERATIONS][33];
static size_t expired_lease_count;
static bool assertion_pending;
static uint64_t assertion_deadline_ms;
static bool deassertion_pending;
static uint64_t deassertion_started_ms;
static bool capped_intent_present;
static char capped_intent_id[33];

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
	if (state.commanded_ptt && !deassertion_pending) {
		deassertion_pending = true;
		deassertion_started_ms = rt_monotonic_ms();
	}
	state.commanded_ptt = false;
	assertion_pending = false;
	assertion_deadline_ms = 0;
}

static bool ptt_output_active(void)
{
	/*
	 * This is only a logical fixture command until #13 supplies the selected
	 * output overlay. The default image cannot reach this path because the
	 * output option and physical health inputs are unavailable.
	 */
	if (!IS_ENABLED(CONFIG_RIGTETHER_PTT_OUTPUT_ENABLED)) {
		return false;
	}
	deassertion_pending = false;
	deassertion_started_ms = 0;
	state.commanded_ptt = true;
	return true;
}

static bool inputs_allow_receive_safe(void)
{
	return !state.commanded_ptt && state.inputs.ptt_out_known &&
	       !state.inputs.ptt_out_active && state.inputs.inhibit_known &&
	       state.inputs.inhibit_closed &&
	       state.inputs.host_route == RT_HEALTH_HEALTHY &&
	       state.inputs.device_audio == RT_HEALTH_HEALTHY &&
	       state.inputs.ble == RT_HEALTH_HEALTHY &&
	       state.inputs.protocol_session == RT_HEALTH_HEALTHY &&
	       state.inputs.radio_profile == RT_HEALTH_HEALTHY;
}

static void clear_intent_locked(void)
{
	state.intent_present = false;
	state.owner_boot_id[0] = '\0';
	state.owner_session_id[0] = '\0';
	state.intent_id[0] = '\0';
}

static void clear_owner_locked(void)
{
	state.owner_present = false;
	state.lease_id[0] = '\0';
	state.lease_deadline_ms = 0;
}

static void clear_expired_leases_locked(void)
{
	expired_lease_count = 0;
}

static bool lease_expired_locked(const char *lease_id)
{
	for (size_t index = 0; index < expired_lease_count; ++index) {
		if (strcmp(expired_lease_ids[index], lease_id) == 0) {
			return true;
		}
	}
	return false;
}

static void record_expired_lease_locked(const char *lease_id)
{
	if (lease_expired_locked(lease_id)) {
		return;
	}
	/*
	 * Every lease consumes at least one of the session's 512 operations, so
	 * this fixed history cannot fill before the protocol session is exhausted.
	 */
	if (expired_lease_count < ARRAY_SIZE(expired_lease_ids)) {
		strcpy(expired_lease_ids[expired_lease_count++], lease_id);
	}
}

static void refresh_rearm_locked(uint64_t now)
{
	if (state.state != RT_FAULT_LOCKOUT ||
	    state.first_fault != RT_RELEASE_CONTINUOUS_CAP ||
	    !state.cap_release_reported || !inputs_allow_receive_safe()) {
		state.rearm_started_present = false;
		state.rearm_started_ms = 0;
		return;
	}
	if (!state.rearm_started_present) {
		state.rearm_started_present = true;
		state.rearm_started_ms = now;
	}
}

static void release_locked(enum rt_release_cause cause, bool lockout)
{
	if (cause == RT_RELEASE_CONTINUOUS_CAP && state.intent_present) {
		capped_intent_present = true;
		strcpy(capped_intent_id, state.intent_id);
	}
	ptt_output_inactive();
	rt_audio_mute_tx();
	if (cause == RT_RELEASE_LEASE_EXPIRED && state.owner_present) {
		record_expired_lease_locked(state.lease_id);
	}
	clear_owner_locked();
	state.continuous_started_ms = 0;
	state.last_release = cause;
	state.last_release_at_ms = rt_monotonic_ms();
	if (!lockout && cause != RT_RELEASE_CONTINUOUS_CAP) {
		clear_intent_locked();
	}
	if (lockout) {
		state.state = RT_FAULT_LOCKOUT;
		if (!state.first_fault_present) {
			state.first_fault_present = true;
			state.first_fault_id = next_fault_id++;
			state.first_fault = cause;
			state.first_fault_at_ms = state.last_release_at_ms;
			rt_diag_record(RT_EVENT_FIRST_FAULT, cause);
		}
	} else if (state.state != RT_FAULT_LOCKOUT &&
		   inputs_allow_receive_safe()) {
		state.state = RT_RECEIVE_SAFE;
	}
	if (cause == RT_RELEASE_CONTINUOUS_CAP) {
		state.cap_release_reported = false;
		state.rearm_started_present = false;
		state.rearm_started_ms = 0;
	}
	refresh_rearm_locked(state.last_release_at_ms);
	rt_diag_record(RT_EVENT_RELEASE, cause);
}

void rt_safety_release(enum rt_release_cause cause, bool lockout)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	release_locked(cause, lockout);
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
	refresh_rearm_locked(rt_monotonic_ms());
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
	clear_expired_leases_locked();
	refresh_rearm_locked(rt_monotonic_ms());
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_session_replaced(void)
{
	rt_safety_release(RT_RELEASE_SESSION_REPLACED, false);
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_UNKNOWN;
	state.inputs.host_route = RT_HEALTH_UNKNOWN;
	clear_expired_leases_locked();
	refresh_rearm_locked(rt_monotonic_ms());
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_protocol_session_active(void)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_HEALTHY;
	refresh_rearm_locked(rt_monotonic_ms());
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_protocol_fault(void)
{
	rt_safety_release(RT_RELEASE_PROTOCOL_FAULT, true);
	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs.protocol_session = RT_HEALTH_UNHEALTHY;
	refresh_rearm_locked(rt_monotonic_ms());
	k_mutex_unlock(&safety_lock);
	status_changed();
}

void rt_safety_update_inputs(const struct rt_safety_inputs *inputs)
{
	bool release_audio = false;
	bool release_host_route = false;
	bool release_inhibit = false;
	bool release_profile = false;
	bool output_mismatch = false;
	uint64_t now = rt_monotonic_ms();

	k_mutex_lock(&safety_lock, K_FOREVER);
	state.inputs = *inputs;
	if (state.commanded_ptt && inputs->ptt_out_known &&
	    inputs->ptt_out_active) {
		assertion_pending = false;
		assertion_deadline_ms = 0;
	} else if (!state.commanded_ptt && inputs->ptt_out_known &&
		   !inputs->ptt_out_active) {
		deassertion_pending = false;
		deassertion_started_ms = 0;
	} else if (!state.commanded_ptt && inputs->ptt_out_known &&
		   inputs->ptt_out_active && !deassertion_pending) {
		deassertion_pending = true;
		deassertion_started_ms = now;
	}
	release_audio = state.commanded_ptt &&
			inputs->device_audio != RT_HEALTH_HEALTHY;
	release_host_route = state.commanded_ptt &&
			     inputs->host_route != RT_HEALTH_HEALTHY;
	release_inhibit = state.commanded_ptt &&
			  (!inputs->inhibit_known || !inputs->inhibit_closed);
	release_profile = state.commanded_ptt &&
			  inputs->radio_profile != RT_HEALTH_HEALTHY;
	output_mismatch = state.commanded_ptt &&
			  (!inputs->ptt_out_known ||
			   (!inputs->ptt_out_active &&
			    (!assertion_pending || now >= assertion_deadline_ms)));
	if (state.state == RT_TX_ACTIVE && inputs_allow_receive_safe()) {
		state.state = RT_RECEIVE_SAFE;
	}
	refresh_rearm_locked(now);
	k_mutex_unlock(&safety_lock);
	status_changed();

	if (release_inhibit && inputs->ptt_out_known &&
	    !inputs->ptt_out_active) {
		rt_safety_release(RT_RELEASE_INHIBIT, false);
	} else if (output_mismatch) {
		rt_safety_release(RT_RELEASE_OUTPUT_FAILED_ASSERT, true);
	} else if (release_inhibit) {
		rt_safety_release(RT_RELEASE_INHIBIT, false);
	} else if (release_host_route) {
		rt_safety_release(RT_RELEASE_HOST_ROUTE, false);
	} else if (release_audio) {
		rt_safety_release(RT_RELEASE_DEVICE_AUDIO, false);
	} else if (release_profile) {
		rt_safety_release(RT_RELEASE_PROFILE_FAULT, false);
	}
}

void rt_safety_snapshot(struct rt_safety_snapshot *snapshot)
{
	k_mutex_lock(&safety_lock, K_FOREVER);
	*snapshot = state;
	k_mutex_unlock(&safety_lock);
}

static enum rt_lease_result lease_precondition_locked(void)
{
	if (state.state == RT_FAULT_LOCKOUT) {
		return RT_LEASE_FAULT_LOCKOUT;
	}
	if (state.state != RT_RECEIVE_SAFE) {
		return RT_LEASE_PTT_OUT;
	}
	if (state.inputs.host_route != RT_HEALTH_HEALTHY) {
		return RT_LEASE_HOST_ROUTE;
	}
	if (state.inputs.device_audio != RT_HEALTH_HEALTHY) {
		return RT_LEASE_DEVICE_AUDIO;
	}
	if (state.inputs.ble != RT_HEALTH_HEALTHY ||
	    state.inputs.protocol_session != RT_HEALTH_HEALTHY) {
		return RT_LEASE_CONTROL;
	}
	if (state.inputs.radio_profile != RT_HEALTH_HEALTHY) {
		return RT_LEASE_PROFILE;
	}
	if (!state.inputs.inhibit_known || !state.inputs.inhibit_closed) {
		return RT_LEASE_INHIBIT;
	}
	if (!state.inputs.ptt_out_known || state.inputs.ptt_out_active) {
		return RT_LEASE_PTT_OUT;
	}
	return RT_LEASE_OK;
}

static enum rt_lease_result renew_precondition_locked(void)
{
	if (state.state == RT_FAULT_LOCKOUT) {
		return RT_LEASE_FAULT_LOCKOUT;
	}
	if (state.inputs.host_route != RT_HEALTH_HEALTHY) {
		return RT_LEASE_HOST_ROUTE;
	}
	if (state.inputs.device_audio != RT_HEALTH_HEALTHY) {
		return RT_LEASE_DEVICE_AUDIO;
	}
	if (state.inputs.ble != RT_HEALTH_HEALTHY ||
	    state.inputs.protocol_session != RT_HEALTH_HEALTHY) {
		return RT_LEASE_CONTROL;
	}
	if (state.inputs.radio_profile != RT_HEALTH_HEALTHY) {
		return RT_LEASE_PROFILE;
	}
	if (!state.inputs.inhibit_known || !state.inputs.inhibit_closed) {
		return RT_LEASE_INHIBIT;
	}
	if (!state.commanded_ptt || !state.inputs.ptt_out_known ||
	    !state.inputs.ptt_out_active) {
		return RT_LEASE_PTT_OUT;
	}
	return RT_LEASE_OK;
}

static bool intent_matches_locked(const char *boot_id, const char *session_id,
				  const char *intent_id)
{
	return state.intent_present &&
	       strcmp(state.owner_boot_id, boot_id) == 0 &&
	       strcmp(state.owner_session_id, session_id) == 0 &&
	       strcmp(state.intent_id, intent_id) == 0;
}

static bool owner_matches_locked(const char *boot_id, const char *session_id,
				 const char *intent_id, const char *lease_id)
{
	return state.owner_present &&
	       intent_matches_locked(boot_id, session_id, intent_id) &&
	       strcmp(state.lease_id, lease_id) == 0;
}

enum rt_intent_result rt_safety_intent_begin(const char *boot_id,
					     const char *session_id,
					     const char *intent_id)
{
	enum rt_intent_result result = RT_INTENT_OK;
	k_mutex_lock(&safety_lock, K_FOREVER);
	if (state.state == RT_FAULT_LOCKOUT) {
		result = RT_INTENT_FAULT_LOCKOUT;
	} else if (state.intent_present) {
		result = RT_INTENT_ACTIVE;
	} else {
		state.intent_present = true;
		strcpy(state.owner_boot_id, boot_id);
		strcpy(state.owner_session_id, session_id);
		strcpy(state.intent_id, intent_id);
	}
	k_mutex_unlock(&safety_lock);
	status_changed();
	return result;
}

enum rt_lease_result rt_safety_acquire(const char *boot_id,
				       const char *session_id,
				       const char *intent_id,
				       const char *lease_id,
				       uint32_t requested_ms,
				       uint64_t accepted_at_ms,
				       uint32_t *granted_ms,
				       uint64_t *deadline_ms)
{
	enum rt_lease_result result;
	k_mutex_lock(&safety_lock, K_FOREVER);
	if (!intent_matches_locked(boot_id, session_id, intent_id)) {
		result = RT_LEASE_INTENT_REQUIRED;
	} else if (state.owner_present) {
		result = RT_LEASE_ACTIVE;
	} else {
		result = lease_precondition_locked();
	}
	if (result == RT_LEASE_OK && !ptt_output_active()) {
		result = RT_LEASE_PTT_OUT;
	}
	if (result == RT_LEASE_OK) {
		state.owner_present = true;
		strcpy(state.lease_id, lease_id);
		state.continuous_started_ms = accepted_at_ms;
		state.lease_deadline_ms = accepted_at_ms + requested_ms;
		state.state = RT_TX_ACTIVE;
		assertion_pending = true;
		assertion_deadline_ms =
			rt_monotonic_ms() + ASSERTION_CHECK_MS;
		*granted_ms = requested_ms;
		*deadline_ms = state.lease_deadline_ms;
		rt_audio_unmute_tx();
	}
	k_mutex_unlock(&safety_lock);
	status_changed();
	return result;
}

enum rt_lease_result rt_safety_renew(const char *boot_id,
				     const char *session_id,
				     const char *intent_id,
				     const char *lease_id,
				     uint32_t requested_ms,
				     uint64_t accepted_at_ms,
				     uint32_t *granted_ms,
				     uint64_t *deadline_ms)
{
	enum rt_lease_result result = RT_LEASE_OK;
	uint64_t now = rt_monotonic_ms();
	k_mutex_lock(&safety_lock, K_FOREVER);
	if (lease_expired_locked(lease_id)) {
		result = RT_LEASE_EXPIRED;
	} else if (!owner_matches_locked(boot_id, session_id, intent_id, lease_id)) {
		result = RT_LEASE_NOT_FOUND;
	} else if (now >= state.lease_deadline_ms) {
		release_locked(RT_RELEASE_LEASE_EXPIRED, false);
		result = RT_LEASE_EXPIRED;
	} else if (now - state.continuous_started_ms >= CONTINUOUS_MAX_MS) {
		release_locked(RT_RELEASE_CONTINUOUS_CAP, true);
		result = RT_LEASE_CONTINUOUS_CAP;
	} else {
		result = renew_precondition_locked();
	}
	if (result == RT_LEASE_OK) {
		uint64_t cap_deadline =
			state.continuous_started_ms + CONTINUOUS_MAX_MS;
		uint64_t requested_deadline = accepted_at_ms + requested_ms;
		state.lease_deadline_ms = MIN(requested_deadline, cap_deadline);
		*granted_ms =
			(uint32_t)(state.lease_deadline_ms - accepted_at_ms);
		*deadline_ms = state.lease_deadline_ms;
	} else if (result == RT_LEASE_HOST_ROUTE) {
		release_locked(RT_RELEASE_HOST_ROUTE, false);
	} else if (result == RT_LEASE_DEVICE_AUDIO) {
		release_locked(RT_RELEASE_DEVICE_AUDIO, false);
	} else if (result == RT_LEASE_CONTROL) {
		release_locked(RT_RELEASE_PROTOCOL_FAULT, true);
	} else if (result == RT_LEASE_PROFILE) {
		release_locked(RT_RELEASE_PROFILE_FAULT, false);
	} else if (result == RT_LEASE_INHIBIT) {
		release_locked(RT_RELEASE_INHIBIT, false);
	} else if (result == RT_LEASE_PTT_OUT) {
		release_locked(RT_RELEASE_OUTPUT_FAILED_ASSERT, true);
	}
	k_mutex_unlock(&safety_lock);
	status_changed();
	return result;
}

bool rt_safety_operator_release(const char *intent_id)
{
	bool accepted;
	k_mutex_lock(&safety_lock, K_FOREVER);
	bool continuous_cap =
		state.state == RT_FAULT_LOCKOUT && state.first_fault_present &&
		state.first_fault == RT_RELEASE_CONTINUOUS_CAP;
	if (continuous_cap) {
		accepted = capped_intent_present &&
			   strcmp(capped_intent_id, intent_id) == 0;
	} else {
		accepted = !state.intent_present ||
			   strcmp(state.intent_id, intent_id) == 0;
	}
	if (accepted && continuous_cap) {
		ptt_output_inactive();
		rt_audio_mute_tx();
		clear_owner_locked();
		clear_intent_locked();
		capped_intent_present = false;
		capped_intent_id[0] = '\0';
		state.cap_release_reported = true;
		state.last_release = RT_RELEASE_OPERATOR;
		state.last_release_at_ms = rt_monotonic_ms();
		refresh_rearm_locked(state.last_release_at_ms);
	} else if (accepted) {
		release_locked(RT_RELEASE_OPERATOR, false);
	}
	k_mutex_unlock(&safety_lock);
	status_changed();
	return accepted;
}

enum rt_recovery_result rt_safety_recover(uint32_t fault_id)
{
	enum rt_recovery_result result;
	uint64_t now = rt_monotonic_ms();
	k_mutex_lock(&safety_lock, K_FOREVER);
	refresh_rearm_locked(now);
	if (state.state != RT_FAULT_LOCKOUT) {
		result = RT_RECOVERY_NOT_LOCKED;
	} else if (!state.first_fault_present || state.first_fault_id != fault_id) {
		result = RT_RECOVERY_WRONG_FAULT;
	} else if (!inputs_allow_receive_safe()) {
		result = RT_RECOVERY_INCOMPLETE;
	} else if (state.first_fault == RT_RELEASE_CONTINUOUS_CAP &&
		   (!state.cap_release_reported || !state.rearm_started_present ||
		    now - state.rearm_started_ms < REARM_MIN_MS)) {
		result = RT_RECOVERY_INCOMPLETE;
	} else {
		state.state = RT_RECEIVE_SAFE;
		state.first_fault_present = false;
		state.first_fault_id = 0;
		state.first_fault_at_ms = 0;
		clear_intent_locked();
		state.cap_release_reported = false;
		state.rearm_started_present = false;
		state.rearm_started_ms = 0;
		capped_intent_present = false;
		capped_intent_id[0] = '\0';
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
			now - state.continuous_started_ms >= CONTINUOUS_MAX_MS ?
				RT_RELEASE_CONTINUOUS_CAP :
				RT_RELEASE_LEASE_EXPIRED;
		k_mutex_unlock(&safety_lock);
		rt_safety_release(cause, cause == RT_RELEASE_CONTINUOUS_CAP);
		k_mutex_lock(&safety_lock, K_FOREVER);
		now = rt_monotonic_ms();
	}

	if (state.commanded_ptt && assertion_pending) {
		if (state.inputs.ptt_out_known &&
		    state.inputs.ptt_out_active) {
			assertion_pending = false;
			assertion_deadline_ms = 0;
		} else if (now >= assertion_deadline_ms) {
			k_mutex_unlock(&safety_lock);
			rt_safety_release(RT_RELEASE_OUTPUT_FAILED_ASSERT, true);
			return false;
		}
	}

	if (!state.commanded_ptt && deassertion_pending &&
	    state.inputs.ptt_out_known && state.inputs.ptt_out_active &&
	    now - deassertion_started_ms >= RELEASE_MAX_MS) {
		deassertion_pending = false;
		deassertion_started_ms = 0;
		k_mutex_unlock(&safety_lock);
		rt_safety_release(RT_RELEASE_OUTPUT_STUCK_ACTIVE, true);
		return false;
	}

	complete = !state.commanded_ptt ||
		   (state.owner_present && state.lease_deadline_ms > now &&
		    state.lease_deadline_ms - now <= LEASE_MAX_MS &&
		    state.inputs.ptt_out_known &&
		    state.inputs.ptt_out_active &&
		    state.inputs.inhibit_known && state.inputs.inhibit_closed &&
		    state.inputs.host_route == RT_HEALTH_HEALTHY &&
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
			.inhibit_known = false,
			.inhibit_closed = false,
			.ptt_out_known = false,
			.ptt_out_active = false,
		},
		.commanded_ptt = false,
		.last_release = initial_release,
		.last_release_at_ms = 0,
	};
	expired_lease_count = 0;
	deassertion_pending = false;
	deassertion_started_ms = 0;
	capped_intent_present = false;
	capped_intent_id[0] = '\0';
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
