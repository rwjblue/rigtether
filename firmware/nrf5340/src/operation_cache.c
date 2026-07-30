/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <string.h>
#include <zephyr/storage/flash_map.h>
#include <zephyr/sys/crc.h>
#include <zephyr/sys/util.h>

#include "rigtether/operation_cache.h"

#define CACHE_MAGIC 0x52544330U
#define CACHE_LIMIT 512
#define RESPONSE_QUEUE_LIMIT 512
#define MAX_LOGICAL_BYTES 1024

struct cache_record_header {
	uint32_t magic;
	uint32_t crc;
	uint64_t seq;
	uint16_t request_length;
	uint16_t response_length;
	char op_id[32];
} __packed;

struct cache_index {
	uint32_t offset;
	uint64_t seq;
	uint16_t request_length;
	uint16_t response_length;
	char op_id[33];
};

struct queue_record_header {
	uint32_t magic;
	uint32_t crc;
	uint16_t response_length;
	uint16_t reserved;
} __packed;

static const struct flash_area *cache_area;
static const struct flash_area *response_queue_area;
static struct cache_index index_entries[CACHE_LIMIT];
static size_t index_count;
static uint32_t next_offset;
static uint8_t record_buffer[sizeof(struct cache_record_header) +
			     MAX_LOGICAL_BYTES * 2 + 4] __aligned(4);
static uint32_t queue_offsets[RESPONSE_QUEUE_LIMIT];
static uint16_t queue_lengths[RESPONSE_QUEUE_LIMIT];
static size_t queue_head;
static size_t queue_tail;
static size_t queue_count;
static uint32_t queue_next_offset;
static uint8_t queue_record_buffer[sizeof(struct queue_record_header) +
				   MAX_LOGICAL_BYTES + 4] __aligned(4);

static size_t aligned_record_size(size_t request_length, size_t response_length)
{
	return ROUND_UP(sizeof(struct cache_record_header) + request_length +
				response_length,
			4);
}

int rt_operation_cache_init(void)
{
	int err = flash_area_open(FIXED_PARTITION_ID(operation_cache_partition),
				  &cache_area);
	if (err != 0) {
		return err;
	}
	err = flash_area_open(FIXED_PARTITION_ID(response_queue_partition),
			      &response_queue_area);
	if (err != 0) {
		return err;
	}
	err = rt_operation_cache_reset();
	return err != 0 ? err : rt_response_queue_reset();
}

int rt_operation_cache_reset(void)
{
	if (cache_area == NULL) {
		return -ENODEV;
	}
	int err = flash_area_erase(cache_area, 0, flash_area_get_size(cache_area));
	if (err == 0) {
		index_count = 0;
		next_offset = 0;
	}
	return err;
}

int rt_operation_cache_lookup(const char *op_id, uint64_t seq,
			      const uint8_t *request, size_t request_length,
			      uint8_t *response, size_t response_capacity,
			      size_t *response_length,
			      enum rt_cache_lookup *lookup)
{
	*lookup = RT_CACHE_MISS;
	for (size_t entry_index = 0; entry_index < index_count; ++entry_index) {
		const struct cache_index *entry = &index_entries[entry_index];
		if (strcmp(entry->op_id, op_id) != 0) {
			continue;
		}
		if (entry->seq != seq || entry->request_length != request_length) {
			*lookup = RT_CACHE_ALTERED;
			return 0;
		}
		size_t record_size =
			aligned_record_size(entry->request_length, entry->response_length);
		int err = flash_area_read(cache_area, entry->offset, record_buffer,
					  record_size);
		if (err != 0) {
			return err;
		}
		struct cache_record_header *header =
			(struct cache_record_header *)record_buffer;
		if (header->magic != CACHE_MAGIC ||
		    header->crc != crc32_ieee(
					   &record_buffer[sizeof(*header)],
					   entry->request_length +
						   entry->response_length)) {
			return -EILSEQ;
		}
		const uint8_t *stored_request = &record_buffer[sizeof(*header)];
		if (memcmp(stored_request, request, request_length) != 0) {
			*lookup = RT_CACHE_ALTERED;
			return 0;
		}
		if (entry->response_length > response_capacity) {
			return -EMSGSIZE;
		}
		memcpy(response, &stored_request[entry->request_length],
		       entry->response_length);
		*response_length = entry->response_length;
		*lookup = RT_CACHE_EXACT;
		return 0;
	}
	return 0;
}

int rt_operation_cache_store(const char *op_id, uint64_t seq,
			     const uint8_t *request, size_t request_length,
			     const uint8_t *response, size_t response_length)
{
	if (index_count >= CACHE_LIMIT || request_length > MAX_LOGICAL_BYTES ||
	    response_length > MAX_LOGICAL_BYTES) {
		return -ENOSPC;
	}
	size_t record_size = aligned_record_size(request_length, response_length);
	if (next_offset + record_size > flash_area_get_size(cache_area)) {
		return -ENOSPC;
	}
	memset(record_buffer, 0xff, record_size);
	struct cache_record_header *header =
		(struct cache_record_header *)record_buffer;
	*header = (struct cache_record_header){
		.magic = CACHE_MAGIC,
		.seq = seq,
		.request_length = request_length,
		.response_length = response_length,
	};
	memcpy(header->op_id, op_id, 32);
	uint8_t *stored_request = &record_buffer[sizeof(*header)];
	memcpy(stored_request, request, request_length);
	memcpy(&stored_request[request_length], response, response_length);
	header->crc =
		crc32_ieee(stored_request, request_length + response_length);
	int err =
		flash_area_write(cache_area, next_offset, record_buffer, record_size);
	if (err != 0) {
		return err;
	}
	struct cache_index *entry = &index_entries[index_count++];
	*entry = (struct cache_index){
		.offset = next_offset,
		.seq = seq,
		.request_length = request_length,
		.response_length = response_length,
	};
	memcpy(entry->op_id, op_id, 32);
	entry->op_id[32] = '\0';
	next_offset += record_size;
	return 0;
}

size_t rt_operation_cache_count(void)
{
	return index_count;
}

int rt_response_queue_reset(void)
{
	if (response_queue_area == NULL) {
		return -ENODEV;
	}
	int err = flash_area_erase(response_queue_area, 0,
				   flash_area_get_size(response_queue_area));
	if (err == 0) {
		queue_head = 0;
		queue_tail = 0;
		queue_count = 0;
		queue_next_offset = 0;
	}
	return err;
}

int rt_response_queue_enqueue(const uint8_t *response, size_t response_length)
{
	if (response_length == 0 || response_length > MAX_LOGICAL_BYTES ||
	    queue_count >= RESPONSE_QUEUE_LIMIT) {
		return -ENOSPC;
	}
	size_t record_size =
		ROUND_UP(sizeof(struct queue_record_header) + response_length, 4);
	if (queue_next_offset + record_size >
	    flash_area_get_size(response_queue_area)) {
		if (queue_count != 0) {
			return -ENOSPC;
		}
		int err = rt_response_queue_reset();
		if (err != 0) {
			return err;
		}
	}
	memset(queue_record_buffer, 0xff, record_size);
	struct queue_record_header *header =
		(struct queue_record_header *)queue_record_buffer;
	*header = (struct queue_record_header){
		.magic = CACHE_MAGIC,
		.response_length = response_length,
	};
	uint8_t *stored_response = &queue_record_buffer[sizeof(*header)];
	memcpy(stored_response, response, response_length);
	header->crc = crc32_ieee(stored_response, response_length);
	int err = flash_area_write(response_queue_area, queue_next_offset,
				   queue_record_buffer, record_size);
	if (err != 0) {
		return err;
	}
	queue_offsets[queue_tail] = queue_next_offset;
	queue_lengths[queue_tail] = response_length;
	queue_tail = (queue_tail + 1) % RESPONSE_QUEUE_LIMIT;
	queue_count++;
	queue_next_offset += record_size;
	return 0;
}

int rt_response_queue_dequeue(uint8_t *response, size_t response_capacity,
			      size_t *response_length)
{
	if (queue_count == 0) {
		return 0;
	}
	uint16_t length = queue_lengths[queue_head];
	if (length > response_capacity) {
		return -EMSGSIZE;
	}
	size_t record_size =
		ROUND_UP(sizeof(struct queue_record_header) + length, 4);
	int err = flash_area_read(response_queue_area, queue_offsets[queue_head],
				  queue_record_buffer, record_size);
	if (err != 0) {
		return err;
	}
	struct queue_record_header *header =
		(struct queue_record_header *)queue_record_buffer;
	const uint8_t *stored_response = &queue_record_buffer[sizeof(*header)];
	if (header->magic != CACHE_MAGIC || header->response_length != length ||
	    header->crc != crc32_ieee(stored_response, length)) {
		return -EILSEQ;
	}
	memcpy(response, stored_response, length);
	*response_length = length;
	queue_head = (queue_head + 1) % RESPONSE_QUEUE_LIMIT;
	queue_count--;
	return 1;
}

size_t rt_response_queue_count(void)
{
	return queue_count;
}
