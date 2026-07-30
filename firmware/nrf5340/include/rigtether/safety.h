/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_SAFETY_H
#define RIGTETHER_SAFETY_H

#include <stdbool.h>
#include <stdint.h>

enum rt_health_state {
	RT_HEALTH_UNKNOWN,
	RT_HEALTH_HEALTHY,
	RT_HEALTH_UNHEALTHY,
	RT_HEALTH_VALIDATING,
};

enum rt_safety_state {
	RT_RECEIVE_SAFE,
	RT_TX_ACTIVE,
	RT_FAULT_LOCKOUT,
};

enum rt_release_cause {
	RT_RELEASE_BOOT_OR_UPDATE,
	RT_RELEASE_OPERATOR,
	RT_RELEASE_LEASE_EXPIRED,
	RT_RELEASE_CONTINUOUS_CAP,
	RT_RELEASE_BLE_DISCONNECT,
	RT_RELEASE_SESSION_REPLACED,
	RT_RELEASE_PROTOCOL_FAULT,
	RT_RELEASE_HOST_ROUTE,
	RT_RELEASE_DEVICE_AUDIO,
	RT_RELEASE_PROFILE,
	RT_RELEASE_RADIO_CONTROL,
	RT_RELEASE_INHIBIT,
	RT_RELEASE_OUTPUT_FAILED_ASSERT,
	RT_RELEASE_OUTPUT_STUCK_ACTIVE,
	RT_RELEASE_WATCHDOG,
};

enum rt_recovery_result {
	RT_RECOVERY_OK,
	RT_RECOVERY_NOT_LOCKED,
	RT_RECOVERY_WRONG_FAULT,
	RT_RECOVERY_INCOMPLETE,
};

struct rt_safety_inputs {
	enum rt_health_state host_route;
	enum rt_health_state device_audio;
	enum rt_health_state ble;
	enum rt_health_state protocol_session;
	enum rt_health_state radio_profile;
	bool inhibit_known;
	bool inhibit_closed;
	bool ptt_out_known;
	bool ptt_out_active;
};

struct rt_safety_snapshot {
	enum rt_safety_state state;
	struct rt_safety_inputs inputs;
	bool commanded_ptt;
	bool owner_present;
	uint64_t lease_deadline_ms;
	uint64_t continuous_started_ms;
	enum rt_release_cause last_release;
	uint64_t last_release_at_ms;
	bool first_fault_present;
	uint32_t first_fault_id;
	enum rt_release_cause first_fault;
	uint64_t first_fault_at_ms;
};

int rt_safety_init(void);
void rt_safety_release(enum rt_release_cause cause, bool lockout);
void rt_safety_ble_connected(void);
void rt_safety_ble_disconnected(void);
void rt_safety_session_replaced(void);
void rt_safety_protocol_session_active(void);
void rt_safety_protocol_fault(void);
void rt_safety_update_inputs(const struct rt_safety_inputs *inputs);
void rt_safety_snapshot(struct rt_safety_snapshot *snapshot);
enum rt_recovery_result rt_safety_recover(uint32_t fault_id);
bool rt_safety_complete_check_and_feed_watchdog(void);

#endif
