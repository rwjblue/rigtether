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

#include "rigtether/audio.h"
#include "rigtether/ble_service.h"
#include "rigtether/diagnostics.h"
#include "rigtether/monotonic.h"
#include "rigtether/protocol.h"
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
static uint16_t response_client_limit = UINT16_MAX;
static uint8_t device_id[16];
static uint8_t boot_id[16];
static char hello_value[640];
static char status_value[1024];
static uint64_t status_seq;
static rt_ble_logical_handler_t logical_handler;
static uint8_t logical_response[MAX_MESSAGE_BYTES];

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

static const char *release_cause_name(enum rt_release_cause cause)
{
	switch (cause) {
	case RT_RELEASE_BOOT_OR_UPDATE:
		return "boot_or_update";
	case RT_RELEASE_OPERATOR:
		return "operator_release";
	case RT_RELEASE_LEASE_EXPIRED:
		return "lease_expired";
	case RT_RELEASE_CONTINUOUS_CAP:
		return "continuous_cap";
	case RT_RELEASE_BLE_DISCONNECT:
		return "ble_disconnect";
	case RT_RELEASE_SESSION_REPLACED:
		return "session_replaced";
	case RT_RELEASE_PROTOCOL_FAULT:
		return "protocol_fault";
	case RT_RELEASE_HOST_ROUTE:
		return "host_route_unhealthy";
	case RT_RELEASE_DEVICE_AUDIO:
		return "device_audio_unhealthy";
	case RT_RELEASE_PROFILE:
		return "profile_not_ready";
	case RT_RELEASE_RADIO_CONTROL:
		return "radio_control_fault";
	case RT_RELEASE_INHIBIT:
		return "inhibit_open";
	case RT_RELEASE_OUTPUT_FAILED_ASSERT:
		return "output_failed_to_assert";
	case RT_RELEASE_OUTPUT_STUCK_ACTIVE:
		return "output_stuck_active";
	case RT_RELEASE_WATCHDOG:
		return "watchdog";
	}
	return "protocol_fault";
}

static void update_hello(void)
{
	char device_hex[33];
	char boot_hex[33];
	char session_id[33];
	char session_json[36];
	render_hex(device_hex, device_id);
	render_hex(boot_hex, boot_id);
	if (rt_protocol_session_id(session_id)) {
		snprintk(session_json, sizeof(session_json), "\"%s\"", session_id);
	} else {
		strcpy(session_json, "null");
	}
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
	struct rt_audio_health audio;
	char device_hex[33];
	char boot_hex[33];
	char first_fault[192];
	const char *safety_state;
	const char *ble_health;
	const char *session_health;
	const char *host_health;
	const char *profile_health;
	const char *ptt_out;
	const char *observed_tx;
	const char *inhibit;
	const char *release_code;
	const char *fault_code;
	rt_safety_snapshot(&snapshot);
	rt_audio_get_health(&audio);
	render_hex(device_hex, device_id);
	render_hex(boot_hex, boot_id);
	safety_state = snapshot.state == RT_RECEIVE_SAFE	? "receive_safe" :
		       snapshot.state == RT_TX_ACTIVE	? "tx_active" :
							  "fault_lockout";
	ble_health = snapshot.inputs.ble == RT_HEALTH_HEALTHY	? "connected" :
		     snapshot.inputs.ble == RT_HEALTH_UNHEALTHY	? "disconnected" :
								      "unknown";
	session_health = snapshot.inputs.protocol_session == RT_HEALTH_HEALTHY ? "active" :
			 snapshot.inputs.protocol_session == RT_HEALTH_UNHEALTHY ?
				 "faulted" :
				 "none";
	host_health = snapshot.inputs.host_route == RT_HEALTH_HEALTHY	? "healthy" :
		      snapshot.inputs.host_route == RT_HEALTH_UNHEALTHY ? "unhealthy" :
									"unknown";
	profile_health = snapshot.inputs.radio_profile == RT_HEALTH_HEALTHY ? "ready" :
			 snapshot.inputs.radio_profile == RT_HEALTH_UNHEALTHY ?
				 "faulted" :
				 "none";
	ptt_out = !snapshot.inputs.ptt_out_known ? "unknown" :
		  snapshot.inputs.ptt_out_active  ? "active" :
						     "inactive";
	observed_tx = snapshot.inputs.ptt_out_known && snapshot.inputs.ptt_out_active ?
			      "transmit_or_pseudo_transmit" :
			      "receive";
	inhibit = snapshot.inputs.inhibit_closed ? "closed" : "open";
	release_code = release_cause_name(snapshot.last_release);
	if (snapshot.first_fault_present) {
		fault_code = release_cause_name(snapshot.first_fault);
		snprintk(first_fault, sizeof(first_fault),
			 "{\"fault_id\":\"%032x\",\"code\":\"%s\",\"at_ms\":%llu,"
			 "\"op_id\":null}",
			 snapshot.first_fault_id, fault_code,
			 (unsigned long long)snapshot.first_fault_at_ms);
	} else {
		strcpy(first_fault, "null");
	}
	k_mutex_lock(&ble_lock, K_FOREVER);
	status_seq++;
	snprintk(status_value, sizeof(status_value),
		 "{\"type\":\"status\",\"v\":{\"major\":0,\"minor\":0},"
		 "\"device_id\":\"%s\",\"boot_id\":\"%s\",\"session_id\":%s,"
		 "\"status_seq\":%llu,\"device_time_ms\":%llu,"
		 "\"health\":{\"ble_link\":\"%s\","
		 "\"protocol_session\":\"%s\",\"host_usb_audio_route\":\"%s\","
		 "\"device_usb_audio\":{\"aggregate\":\"%s\",\"configured\":%s,"
		 "\"tx_stream\":\"%s\",\"clock\":\"%s\","
		 "\"buffers\":\"%s\",\"converter\":\"%s\"},"
		 "\"radio_profile\":\"%s\"},\"radio\":{\"profile\":null,"
		 "\"observed_tx\":\"%s\"},\"ptt\":{\"commanded\":\"%s\","
		 "\"ptt_out\":\"%s\",\"inhibit\":\"%s\",\"owner\":null,"
		 "\"lease_deadline_ms\":null,\"continuous_started_ms\":null,"
		 "\"continuous_elapsed_ms\":0,\"safety_state\":\"%s\","
		 "\"last_release\":{\"code\":\"%s\",\"at_ms\":%llu},"
		 "\"first_fault\":%s}}",
		 device_hex, boot_hex, session_json, (unsigned long long)status_seq,
		 (unsigned long long)rt_monotonic_ms(), ble_health, session_health,
		 host_health,
		 audio.configured && audio.tx_stream_active && audio.clock_healthy &&
				 audio.buffers_healthy && audio.converter_healthy ?
			 "healthy" :
			 "unhealthy",
		 audio.configured ? "true" : "false",
		 audio.tx_stream_active ? "active" : "inactive",
		 audio.clock_healthy ? "healthy" : "unhealthy",
		 audio.buffers_healthy ? "healthy" : "unhealthy",
		 audio.converter_healthy ? "healthy" : "unhealthy", profile_health,
		 observed_tx, snapshot.commanded_ptt ? "active" : "inactive", ptt_out,
		 inhibit, safety_state, release_code,
		 (unsigned long long)snapshot.last_release_at_ms, first_fault);
	ssize_t result = bt_gatt_attr_read(conn, attr, buf, len, offset, status_value,
					  strlen(status_value));
	k_mutex_unlock(&ble_lock);
	return result;
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
		size_t response_length = 0;
		command_transfer.active = false;
		if (logical_handler == NULL) {
			return -ENOSYS;
		}
		int err = logical_handler(command_transfer.bytes, command_transfer.total,
					  logical_response, sizeof(logical_response),
					  &response_length);
		if (err != 0 || response_length == 0 ||
		    response_length > sizeof(logical_response)) {
			return err != 0 ? err : -EMSGSIZE;
		}
		err = rt_ble_publish_response(logical_response, response_length);
		if (err != 0) {
			rt_diag_record(RT_EVENT_PROTOCOL_FAULT, (uint32_t)-err);
		}
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

struct response_transfer {
	bool active;
	uint32_t id;
	uint32_t total;
	uint32_t offset;
	uint8_t logical[MAX_MESSAGE_BYTES];
	uint8_t frame[517];
	struct bt_gatt_indicate_params params;
};

static struct response_transfer response_transfer;
static uint32_t next_response_transfer_id = 1;

static int send_next_response_fragment(void);

static void response_indicated(struct bt_conn *conn,
			       struct bt_gatt_indicate_params *params, uint8_t err)
{
	ARG_UNUSED(conn);
	ARG_UNUSED(params);
	k_mutex_lock(&ble_lock, K_FOREVER);
	if (err != 0 || !response_transfer.active ||
	    response_transfer.offset == response_transfer.total) {
		response_transfer.active = false;
	} else if (send_next_response_fragment() != 0) {
		response_transfer.active = false;
	}
	k_mutex_unlock(&ble_lock);
}

static int send_next_response_fragment(void)
{
	uint16_t frame_limit =
		MIN(MIN(command_value_limit, response_client_limit),
		    sizeof(response_transfer.frame));
	if (frame_limit <= FRAME_HEADER || current_conn == NULL) {
		return -ENOTCONN;
	}
	uint16_t payload_limit = frame_limit - FRAME_HEADER;
	uint16_t payload =
		MIN(payload_limit, response_transfer.total - response_transfer.offset);
	uint8_t flags = response_transfer.offset == 0 ? START_FLAG : 0;
	if (response_transfer.offset + payload == response_transfer.total) {
		flags |= END_FLAG;
	}
	response_transfer.frame[0] = 0;
	response_transfer.frame[1] = flags;
	sys_put_be16(0, &response_transfer.frame[2]);
	sys_put_be32(response_transfer.id, &response_transfer.frame[4]);
	sys_put_be32(response_transfer.total, &response_transfer.frame[8]);
	sys_put_be32(response_transfer.offset, &response_transfer.frame[12]);
	memcpy(&response_transfer.frame[FRAME_HEADER],
	       &response_transfer.logical[response_transfer.offset], payload);
	response_transfer.params = (struct bt_gatt_indicate_params){
		.attr = &rt_service.attrs[6],
		.func = response_indicated,
		.data = response_transfer.frame,
		.len = FRAME_HEADER + payload,
	};
	int result = bt_gatt_indicate(current_conn, &response_transfer.params);
	if (result == 0) {
		response_transfer.offset += payload;
	}
	return result;
}

static void connected(struct bt_conn *conn, uint8_t err)
{
	if (err != 0) {
		return;
	}
	k_mutex_lock(&ble_lock, K_FOREVER);
	current_conn = bt_conn_ref(conn);
	command_value_limit = MAX(20, bt_gatt_get_mtu(conn) - 3);
	response_client_limit = UINT16_MAX;
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
	response_transfer.active = false;
	command_value_limit = 20;
	response_client_limit = UINT16_MAX;
	k_mutex_unlock(&ble_lock);
	rt_diag_record(RT_EVENT_BLE_DISCONNECTED, reason);
	rt_protocol_disconnect();
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
	rt_protocol_init(device_id, boot_id);
	rt_ble_register_logical_handler(rt_protocol_handle_logical);
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

void rt_ble_register_logical_handler(rt_ble_logical_handler_t handler)
{
	k_mutex_lock(&ble_lock, K_FOREVER);
	logical_handler = handler;
	k_mutex_unlock(&ble_lock);
}

uint16_t rt_ble_command_value_limit(void)
{
	k_mutex_lock(&ble_lock, K_FOREVER);
	uint16_t value = command_value_limit;
	k_mutex_unlock(&ble_lock);
	return value;
}

uint16_t rt_ble_set_response_frame_limit(uint16_t client_limit)
{
	k_mutex_lock(&ble_lock, K_FOREVER);
	response_client_limit = MAX(20, client_limit);
	uint16_t value = MIN(command_value_limit, response_client_limit);
	k_mutex_unlock(&ble_lock);
	return value;
}

int rt_ble_publish_response(const uint8_t *value, uint16_t length)
{
	if (length == 0 || length > MAX_MESSAGE_BYTES) {
		return -EMSGSIZE;
	}
	k_mutex_lock(&ble_lock, K_FOREVER);
	if (response_transfer.active) {
		k_mutex_unlock(&ble_lock);
		return -EBUSY;
	}
	response_transfer.active = true;
	response_transfer.id = next_response_transfer_id++;
	response_transfer.total = length;
	response_transfer.offset = 0;
	memcpy(response_transfer.logical, value, length);
	int result = send_next_response_fragment();
	if (result != 0) {
		response_transfer.active = false;
	}
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
