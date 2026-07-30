/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_BLE_SERVICE_H
#define RIGTETHER_BLE_SERVICE_H

#include <stddef.h>
#include <stdint.h>

typedef int (*rt_ble_logical_handler_t)(const uint8_t *request, size_t request_length,
				       uint8_t *response, size_t response_capacity,
				       size_t *response_length,
				       uint64_t status_sequence);

int rt_ble_service_init(void);
void rt_ble_register_logical_handler(rt_ble_logical_handler_t handler);
uint16_t rt_ble_command_value_limit(void);
uint16_t rt_ble_set_response_frame_limit(uint16_t client_limit);
int rt_ble_publish_response(const uint8_t *value, uint16_t length);
int rt_ble_publish_status(const uint8_t *value, uint16_t length);
uint64_t rt_ble_notify_status(void);

#endif
