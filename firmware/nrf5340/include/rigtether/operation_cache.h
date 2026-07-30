/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_OPERATION_CACHE_H
#define RIGTETHER_OPERATION_CACHE_H

#include <stddef.h>
#include <stdint.h>

enum rt_cache_lookup {
	RT_CACHE_MISS,
	RT_CACHE_EXACT,
	RT_CACHE_ALTERED,
};

int rt_operation_cache_init(void);
int rt_operation_cache_reset(void);
int rt_operation_cache_lookup(const char *op_id, uint64_t seq,
			      const uint8_t *request, size_t request_length,
			      uint8_t *response, size_t response_capacity,
			      size_t *response_length,
			      enum rt_cache_lookup *lookup);
int rt_operation_cache_store(const char *op_id, uint64_t seq,
			     const uint8_t *request, size_t request_length,
			     const uint8_t *response, size_t response_length);
size_t rt_operation_cache_count(void);
int rt_response_queue_reset(void);
int rt_response_queue_enqueue(const uint8_t *response, size_t response_length);
int rt_response_queue_dequeue(uint8_t *response, size_t response_capacity,
			      size_t *response_length);
size_t rt_response_queue_count(void);

#endif
