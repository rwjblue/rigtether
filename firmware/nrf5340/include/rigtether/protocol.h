/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_PROTOCOL_H
#define RIGTETHER_PROTOCOL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

void rt_protocol_init(const uint8_t device_id[16], const uint8_t boot_id[16]);
void rt_protocol_disconnect(void);
bool rt_protocol_session_id(char session_id[33]);
int rt_protocol_handle_logical(const uint8_t *request, size_t request_length,
			       uint8_t *response, size_t response_capacity,
			       size_t *response_length);

#endif
