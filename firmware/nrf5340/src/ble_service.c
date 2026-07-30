/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <string.h>
#include <zephyr/bluetooth/bluetooth.h>
#include <zephyr/bluetooth/conn.h>
#include <zephyr/bluetooth/gatt.h>
#include <zephyr/bluetooth/uuid.h>
#include <zephyr/drivers/hwinfo.h>
#include <zephyr/kernel.h>
#include <zephyr/random/random.h>
#include <zephyr/sys/byteorder.h>
#include <zephyr/sys/util.h>

#include "rigtether/ble_service.h"
#include "rigtether/diagnostics.h"
#include "rigtether/monotonic.h"
#include "rigtether/safety.h"

#define FRAME_HEADER 16
#define MAX_MESSAGE_BYTES 1024
#define START_FLAG BIT(0)
#define END_FLAG BIT(1)

#define RT_SERVICE_UUID BT_UUID_128_ENCODE(0x3dbbf179, 0x78cb, 0x4753, 0x9c4e, 0x092e2a4e1116)
#define RT_HELLO_UUID BT_UUID_128_ENCODE(0x686fd375, 0x482c, 0x407b, 0x856d, 0x3a5e757f9f03)
#define RT_COMMAND_UUID BT_UUID_128_ENCODE(0xf98fec49, 0xc274, 0x40d9, 0x9b4b, 0x38f5cef50e15)
#define RT_RESPONSE_UUID BT_UUID_128_ENCODE(0x7ba61ae1, 0xe6b0, 0x4e3f, 0xa718, 0xfd31315cd876)
#define RT_STATUS_UUID BT_UUID_128_ENCODE(0xf9ecbae1, 0x3204, 0x4870, 0xb338, 0x8f220e000585)

static struct bt_uuid_128 service_uuid = BT_UUID_INIT_128(RT_SERVICE_UUID);
static struct bt_uuid_128 hello_uuid = BT_UUID_INIT_128(RT_HELLO_UUID);
static struct bt_uuid_128 command_uuid = BT_UUID_INIT_128(RT_COMMAND_UUID);
static struct bt_uuid_128 response_uuid = BT_UUID_INIT_128(RT_RESPONSE_UUID);
static struct bt_uuid_128 status_uuid = BT_UUID_INIT_128(RT_STATUS_UUID);

static K_MUTEX_DEFINE(ble_lock);
static struct bt_conn *current_conn;
static uint16_t command_value_limit = 20;
static uint8_t device_id[16];
static uint8_t boot_id[16];
static char hello_value[640];
static char status_value[768];

struct transfer {
	bool active;
	uint32_t id;
	uint32_t total;
	uint32_t accepted;
	uint8_t bytes[MAX_MESSAGE_BYTES];
};

static struct transfer command_transfer;

static void render_hex(char out[33], const uint8_t value[16])
{
	static const char digits[] = "0123456789abcdef";
	for (size_t index = 0; index < 16; ++index) {
		out[index * 2] = digits[value[index] >> 4];
		out[index * 2 + 1] = digits[value[index] & 0x0f];
	}
	out[32] = '\0';
}

static void update_hello(void)
{
	char device_hex[33];
	char boot_hex[33];
	render_hex(device_hex, device_id);
	render_hex(boot_hex, boot_id);
	snprintk(hello_value, sizeof(hello_value),
		 "{\"type\":\"hello\",\"device_id\":\"%s\",\"boot_id\":\"%s\","
		 "\"versions\":[{\"major\":0,\"min_minor\":0,\"max_minor\":0}],"
		 "\"capabilities\":[\"first_cause_fault_v0\",\"independent_health_v0\","
		 "\"ordered_operations\",\"ptt_leases_v0\",\"typed_radio_v0\"],"
		 "\"profiles\":[\"kx2\",\"kx3\"],\"command_frame_limit\":%u,"
		 "\"max_message_bytes\":1024,\"max_session_operations\":512}",
		 device_hex, boot_hex, command_value_limit);
}

static ssize_t read_hello(struct bt_conn *conn, const struct bt_gatt_attr *attr,
			  void *buf, uint16_t len, uint16_t offset)
{
	ARG_UNUSED(conn);
	ARG_UNUSED(attr);
	k_mutex_lock(&ble_lock, K_FOREVER);
	update_hello();
	ssize_t result = bt_gatt_attr_read(conn, attr, buf, len, offset, hello_value,
					  strlen(hello_value));
	k_mutex_unlock(&ble_lock);
	return result;
}

static ssize_t read_status(struct bt_conn *conn, const struct bt_gatt_attr *attr,
			   void *buf, uint16_t len, uint16_t offset)
{
	struct rt_safety_snapshot snapshot;
	char device_hex[33];
	char boot_hex[33];
	rt_safety_snapshot(&snapshot);
	render_hex(device_hex, device_id);
	render_hex(boot_hex, boot_id);
	snprintk(status_value, sizeof(status_value),
		 "{\"type\":\"status\",\"v\":{\"major\":0,\"minor\":0},"
		 "\"device_id\":\"%s\",\"boot_id\":\"%s\",\"session_id\":null,"
		 "\"status_seq\":0,\"device_time_ms\":%llu,"
		 "\"health\":{\"ble_link\":\"connected\","
		 "\"protocol_session\":\"none\",\"host_usb_audio_route\":\"unknown\","
		 "\"device_usb_audio\":{\"aggregate\":\"unknown\",\"configured\":false,"
		 "\"tx_stream\":\"inactive\",\"clock\":\"unhealthy\","
		 "\"buffers\":\"unhealthy\",\"converter\":\"unhealthy\"},"
		 "\"radio_profile\":\"none\"},\"radio\":{\"profile\":null,"
		 "\"observed_tx\":\"receive\"},\"ptt\":{\"commanded\":\"inactive\","
		 "\"ptt_out\":\"unknown\",\"inhibit\":\"unknown\",\"owner\":null,"
		 "\"lease_deadline_ms\":null,\"continuous_started_ms\":null,"
		 "\"continuous_elapsed_ms\":0,\"safety_state\":\"receive_safe\","
		 "\"last_release\":{\"code\":\"boot_or_update\",\"at_ms\":0},"
		 "\"first_fault\":null}}",
		 device_hex, boot_hex, (unsigned long long)rt_monotonic_ms());
	return bt_gatt_attr_read(conn, attr, buf, len, offset, status_value,
				 strlen(status_value));
}

static int accept_fragment(const uint8_t *value, uint16_t length)
{
	if (length < FRAME_HEADER || value[0] != 0 || (value[1] & ~0x03U) != 0 ||
	    sys_get_be16(&value[2]) != 0) {
		return -EINVAL;
	}
	uint8_t flags = value[1];
	uint32_t transfer_id = sys_get_be32(&value[4]);
	uint32_t total = sys_get_be32(&value[8]);
	uint32_t offset = sys_get_be32(&value[12]);
	uint16_t payload = length - FRAME_HEADER;
	if (total == 0 || total > MAX_MESSAGE_BYTES || offset + payload > total) {
		return -EMSGSIZE;
	}
	if (!command_transfer.active) {
		if ((flags & START_FLAG) == 0 || offset != 0) {
			return -EINVAL;
		}
		command_transfer = (struct transfer){
			.active = true,
			.id = transfer_id,
			.total = total,
		};
	} else if ((flags & START_FLAG) != 0 || transfer_id != command_transfer.id ||
		   total != command_transfer.total || offset != command_transfer.accepted) {
		return -EINVAL;
	}
	memcpy(&command_transfer.bytes[offset], &value[FRAME_HEADER], payload);
	command_transfer.accepted += payload;
	if ((flags & END_FLAG) != 0) {
		if (command_transfer.accepted != command_transfer.total) {
			return -EINVAL;
		}
		/*
		 * The checked-in Rust firmware core consumes this exact logical payload
		 * in repository-vector tests. The hardware image deliberately has no
		 * radio/PTT assertion path until #13; an NCS/Rust ABI is a later build
		 * gate and cannot be replaced by raw CAT here.
		 */
		command_transfer.active = false;
	}
	rt_diag_record(RT_EVENT_PROTOCOL_FRAGMENT, payload);
	return 0;
}

static ssize_t write_command(struct bt_conn *conn, const struct bt_gatt_attr *attr,
			     const void *buf, uint16_t len, uint16_t offset,
			     uint8_t flags)
{
	ARG_UNUSED(conn);
	ARG_UNUSED(attr);
	ARG_UNUSED(flags);
	if (offset != 0 || len > command_value_limit) {
		rt_safety_protocol_fault();
		return BT_GATT_ERR(BT_ATT_ERR_INVALID_OFFSET);
	}
	if (accept_fragment(buf, len) != 0) {
		rt_diag_record(RT_EVENT_PROTOCOL_FAULT, 0);
		rt_safety_protocol_fault();
		return BT_GATT_ERR(BT_ATT_ERR_VALUE_NOT_ALLOWED);
	}
	return len;
}

BT_GATT_SERVICE_DEFINE(
	rt_service, BT_GATT_PRIMARY_SERVICE(&service_uuid),
	BT_GATT_CHARACTERISTIC(&hello_uuid.uuid, BT_GATT_CHRC_READ, BT_GATT_PERM_READ,
			       read_hello, NULL, NULL),
	BT_GATT_CHARACTERISTIC(&command_uuid.uuid, BT_GATT_CHRC_WRITE, BT_GATT_PERM_WRITE,
			       NULL, write_command, NULL),
	BT_GATT_CHARACTERISTIC(&response_uuid.uuid, BT_GATT_CHRC_INDICATE, BT_GATT_PERM_NONE,
			       NULL, NULL, NULL),
	BT_GATT_CCC(NULL, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE),
	BT_GATT_CHARACTERISTIC(&status_uuid.uuid, BT_GATT_CHRC_READ | BT_GATT_CHRC_NOTIFY,
			       BT_GATT_PERM_READ, read_status, NULL, NULL),
	BT_GATT_CCC(NULL, BT_GATT_PERM_READ | BT_GATT_PERM_WRITE));

static void connected(struct bt_conn *conn, uint8_t err)
{
	if (err != 0) {
		return;
	}
	k_mutex_lock(&ble_lock, K_FOREVER);
	current_conn = bt_conn_ref(conn);
	command_value_limit = MAX(20, bt_gatt_get_mtu(conn) - 3);
	update_hello();
	k_mutex_unlock(&ble_lock);
	rt_diag_record(RT_EVENT_BLE_CONNECTED, command_value_limit);
	rt_safety_ble_connected();
}

static void disconnected(struct bt_conn *conn, uint8_t reason)
{
	ARG_UNUSED(conn);
	k_mutex_lock(&ble_lock, K_FOREVER);
	if (current_conn != NULL) {
		bt_conn_unref(current_conn);
		current_conn = NULL;
	}
	command_transfer.active = false;
	command_value_limit = 20;
	k_mutex_unlock(&ble_lock);
	rt_diag_record(RT_EVENT_BLE_DISCONNECTED, reason);
	rt_safety_ble_disconnected();
}

BT_CONN_CB_DEFINE(connection_callbacks) = {
	.connected = connected,
	.disconnected = disconnected,
};

static void att_mtu_updated(struct bt_conn *conn, uint16_t tx, uint16_t rx)
{
	ARG_UNUSED(tx);
	ARG_UNUSED(rx);
	k_mutex_lock(&ble_lock, K_FOREVER);
	if (current_conn == conn) {
		command_value_limit = MAX(20, bt_gatt_get_mtu(conn) - 3);
		update_hello();
	}
	k_mutex_unlock(&ble_lock);
}

static struct bt_gatt_cb gatt_callbacks = {
	.att_mtu_updated = att_mtu_updated,
};

int rt_ble_service_init(void)
{
	ssize_t id_length = hwinfo_get_device_id(device_id, sizeof(device_id));
	if (id_length < 0) {
		return (int)id_length;
	}
	if (id_length < sizeof(device_id)) {
		memset(&device_id[id_length], 0, sizeof(device_id) - id_length);
	}
	int err = sys_csrand_get(boot_id, sizeof(boot_id));
	if (err != 0) {
		return err;
	}
	update_hello();
	err = bt_enable(NULL);
	if (err != 0) {
		return err;
	}
	bt_gatt_cb_register(&gatt_callbacks);
	const struct bt_data advertising[] = {
		BT_DATA_BYTES(BT_DATA_FLAGS, BT_LE_AD_GENERAL | BT_LE_AD_NO_BREDR),
		BT_DATA_BYTES(BT_DATA_UUID128_ALL, RT_SERVICE_UUID),
	};
	return bt_le_adv_start(BT_LE_ADV_CONN_NAME, advertising, ARRAY_SIZE(advertising),
			       NULL, 0);
}

uint16_t rt_ble_command_value_limit(void)
{
	k_mutex_lock(&ble_lock, K_FOREVER);
	uint16_t value = command_value_limit;
	k_mutex_unlock(&ble_lock);
	return value;
}

int rt_ble_publish_response(const uint8_t *value, uint16_t length)
{
	struct bt_gatt_indicate_params params = {
		.attr = &rt_service.attrs[6],
		.data = value,
		.len = length,
	};
	k_mutex_lock(&ble_lock, K_FOREVER);
	int result = current_conn == NULL ? -ENOTCONN :
					 bt_gatt_indicate(current_conn, &params);
	k_mutex_unlock(&ble_lock);
	return result;
}

int rt_ble_publish_status(const uint8_t *value, uint16_t length)
{
	k_mutex_lock(&ble_lock, K_FOREVER);
	int result = current_conn == NULL ? -ENOTCONN :
					 bt_gatt_notify(current_conn, &rt_service.attrs[9],
							value, length);
	k_mutex_unlock(&ble_lock);
	return result;
}
