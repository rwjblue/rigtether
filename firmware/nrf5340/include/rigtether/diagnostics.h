/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_DIAGNOSTICS_H
#define RIGTETHER_DIAGNOSTICS_H

#include <stdint.h>

enum rt_event_code {
	RT_EVENT_BOOT,
	RT_EVENT_RELEASE,
	RT_EVENT_FIRST_FAULT,
	RT_EVENT_BLE_CONNECTED,
	RT_EVENT_BLE_DISCONNECTED,
	RT_EVENT_PROTOCOL_FRAGMENT,
	RT_EVENT_PROTOCOL_FAULT,
	RT_EVENT_AUDIO_FAULT,
	RT_EVENT_WATCHDOG_FEED,
};

void rt_diag_record(enum rt_event_code code, uint32_t value);
uint32_t rt_diag_dropped(void);

#endif
