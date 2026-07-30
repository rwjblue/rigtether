/* SPDX-License-Identifier: Apache-2.0 */
#include <zephyr/kernel.h>
#include <zephyr/sys/atomic.h>

#include "rigtether/diagnostics.h"
#include "rigtether/monotonic.h"

#define EVENT_COUNT 128

struct event_record {
	uint64_t at_us;
	uint32_t value;
	uint16_t code;
};

static struct event_record events[EVENT_COUNT];
static atomic_t write_index;
static atomic_t dropped;

void rt_diag_record(enum rt_event_code code, uint32_t value)
{
	atomic_val_t index = atomic_inc(&write_index);
	if ((uint32_t)index >= EVENT_COUNT) {
		atomic_inc(&dropped);
	}
	struct event_record *record = &events[(uint32_t)index % EVENT_COUNT];
	record->at_us = rt_monotonic_us();
	record->value = value;
	record->code = (uint16_t)code;
}

uint32_t rt_diag_dropped(void)
{
	return (uint32_t)atomic_get(&dropped);
}
