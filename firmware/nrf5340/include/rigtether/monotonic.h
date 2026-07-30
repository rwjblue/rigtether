/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_MONOTONIC_H
#define RIGTETHER_MONOTONIC_H

#include <stdint.h>

int rt_monotonic_init(void);
uint64_t rt_monotonic_us(void);
static inline uint64_t rt_monotonic_ms(void)
{
	return rt_monotonic_us() / 1000U;
}

#endif
