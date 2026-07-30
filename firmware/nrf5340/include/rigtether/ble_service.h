/* SPDX-License-Identifier: Apache-2.0 */
#ifndef RIGTETHER_BLE_SERVICE_H
#define RIGTETHER_BLE_SERVICE_H

#include <stdint.h>

int rt_ble_service_init(void);
uint16_t rt_ble_command_value_limit(void);
int rt_ble_publish_response(const uint8_t *value, uint16_t length);
int rt_ble_publish_status(const uint8_t *value, uint16_t length);

#endif
