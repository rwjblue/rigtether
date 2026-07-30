/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_PROTOCOL_H
#define RIGTETHER_PROTOCOL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

int rt_protocol_init(const uint8_t device_id[16], const uint8_t boot_id[16]);
void rt_protocol_disconnect(void);
void rt_protocol_att_limit_changed(void);
bool rt_protocol_session_id(char session_id[33]);
bool rt_protocol_session_active(void);
void rt_protocol_selected_profile(char profile[4]);
int rt_protocol_handle_logical(const uint8_t *request, size_t request_length,
			       uint8_t *response, size_t response_capacity,
			       size_t *response_length,
			       uint64_t status_sequence);

#endif
