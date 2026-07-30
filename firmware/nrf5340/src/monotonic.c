/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <nrfx_timer.h>
#include <zephyr/kernel.h>
#include <zephyr/irq.h>
#include <zephyr/sys/atomic.h>

#include "rigtether/monotonic.h"

static const nrfx_timer_t lease_timer = NRFX_TIMER_INSTANCE(2);
static atomic_t timer_high;

static void timer_handler(nrf_timer_event_t event_type, void *context)
{
	ARG_UNUSED(context);
	if (event_type == NRF_TIMER_EVENT_COMPARE0) {
		atomic_inc(&timer_high);
	}
}

int rt_monotonic_init(void)
{
	nrfx_timer_config_t config = NRFX_TIMER_DEFAULT_CONFIG(1000000);
	config.bit_width = NRF_TIMER_BIT_WIDTH_32;
	int err = nrfx_timer_init(&lease_timer, &config, timer_handler);
	if (err != NRFX_SUCCESS) {
		return -EIO;
	}
	nrfx_timer_clear(&lease_timer);
	nrfx_timer_extended_compare(&lease_timer, NRF_TIMER_CC_CHANNEL0, UINT32_MAX,
				    NRF_TIMER_SHORT_COMPARE0_CLEAR_MASK, true);
	nrfx_timer_enable(&lease_timer);
	return 0;
}

uint64_t rt_monotonic_us(void)
{
	/*
	 * Synchronize with the wrap ISR and account for a compare event that became
	 * pending before the ISR could advance the software epoch. If the event
	 * arrived after capture, low is still in the upper half and belongs to the
	 * old epoch; otherwise the cleared counter belongs to the next epoch.
	 */
	unsigned int irq_key = irq_lock();
	uint32_t high = (uint32_t)atomic_get(&timer_high);
	uint32_t low = nrfx_timer_capture(&lease_timer, NRF_TIMER_CC_CHANNEL1);
	bool wrap_pending =
		nrf_timer_event_check(lease_timer.p_reg, NRF_TIMER_EVENT_COMPARE0);
	if (wrap_pending && low < (UINT32_MAX / 2U)) {
		high++;
	}
	irq_unlock(irq_key);

	return ((uint64_t)high << 32) | low;
}
