/* SPDX-License-Identifier: Apache-2.0 */
#include <errno.h>
#include <string.h>
#include <zephyr/data/json.h>
#include <zephyr/kernel.h>
#include <zephyr/random/random.h>
#include <zephyr/sys/util.h>

#include "rigtether/ble_service.h"
#include "rigtether/monotonic.h"
#include "rigtether/operation_cache.h"
#include "rigtether/protocol.h"
#include "rigtether/safety.h"

#define MAX_LOGICAL_BYTES 1024
#define CAPABILITY_COUNT 5

struct version {
	uint64_t major;
	uint64_t minor;
};

struct session_start_message {
	char *type;
	char *boot_id;
	char *client_nonce;
	char *op_id;
	struct version select;
	char *required_capabilities[CAPABILITY_COUNT];
	size_t required_capabilities_len;
	uint64_t client_rx_frame_limit;
	uint64_t client_max_message_bytes;
};

struct request_message {
	char *type;
	struct version v;
	char *boot_id;
	char *session_id;
	char *op_id;
	uint64_t seq;
	struct json_obj_token command;
};

struct command_message {
	char *type;
};

static const struct json_obj_descr version_descr[] = {
	JSON_OBJ_DESCR_PRIM(struct version, major, JSON_TOK_UINT64),
	JSON_OBJ_DESCR_PRIM(struct version, minor, JSON_TOK_UINT64),
};

static const struct json_obj_descr session_start_descr[] = {
	JSON_OBJ_DESCR_PRIM(struct session_start_message, type, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct session_start_message, boot_id, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct session_start_message, client_nonce, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct session_start_message, op_id, JSON_TOK_STRING),
	JSON_OBJ_DESCR_OBJECT(struct session_start_message, select, version_descr),
	JSON_OBJ_DESCR_ARRAY(struct session_start_message, required_capabilities,
			     CAPABILITY_COUNT, required_capabilities_len,
			     JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct session_start_message, client_rx_frame_limit,
			    JSON_TOK_UINT64),
	JSON_OBJ_DESCR_PRIM(struct session_start_message, client_max_message_bytes,
			    JSON_TOK_UINT64),
};

static const struct json_obj_descr request_descr[] = {
	JSON_OBJ_DESCR_PRIM(struct request_message, type, JSON_TOK_STRING),
	JSON_OBJ_DESCR_OBJECT(struct request_message, v, version_descr),
	JSON_OBJ_DESCR_PRIM(struct request_message, boot_id, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct request_message, session_id, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct request_message, op_id, JSON_TOK_STRING),
	JSON_OBJ_DESCR_PRIM(struct request_message, seq, JSON_TOK_UINT64),
	JSON_OBJ_DESCR_PRIM(struct request_message, command, JSON_TOK_OPAQUE),
};

static const struct json_obj_descr command_descr[] = {
	JSON_OBJ_DESCR_PRIM(struct command_message, type, JSON_TOK_STRING),
};

static const char *const capabilities[CAPABILITY_COUNT] = {
	"first_cause_fault_v0",
	"independent_health_v0",
	"ordered_operations",
	"ptt_leases_v0",
	"typed_radio_v0",
};

static K_MUTEX_DEFINE(protocol_lock);
static char boot_identity[33];
static char session_identity[33];
static bool session_active;
static uint64_t next_seq = 1;
static uint8_t cached_start[MAX_LOGICAL_BYTES];
static size_t cached_start_length;
static uint8_t cached_start_response[MAX_LOGICAL_BYTES];
static size_t cached_start_response_length;
static char cached_client_nonce[33];
static char cached_start_op_id[33];

static void render_hex(char out[33], const uint8_t value[16])
{
	static const char digits[] = "0123456789abcdef";
	for (size_t index = 0; index < 16; ++index) {
		out[index * 2] = digits[value[index] >> 4];
		out[index * 2 + 1] = digits[value[index] & 0x0f];
	}
	out[32] = '\0';
}

static bool valid_id(const char *value)
{
	if (value == NULL || strlen(value) != 32) {
		return false;
	}
	for (size_t index = 0; index < 32; ++index) {
		if (!((value[index] >= '0' && value[index] <= '9') ||
		      (value[index] >= 'a' && value[index] <= 'f'))) {
			return false;
		}
	}
	return true;
}

static void skip_whitespace(const uint8_t *json, size_t length, size_t *offset)
{
	while (*offset < length &&
	       (json[*offset] == ' ' || json[*offset] == '\t' ||
		json[*offset] == '\r' || json[*offset] == '\n')) {
		(*offset)++;
	}
}

static int string_bounds(const uint8_t *json, size_t length, size_t *offset,
			 size_t *start, size_t *string_length)
{
	if (*offset >= length || json[*offset] != '"') {
		return -EINVAL;
	}
	(*offset)++;
	*start = *offset;
	bool escaped = false;
	while (*offset < length) {
		uint8_t character = json[(*offset)++];
		if (escaped) {
			escaped = false;
		} else if (character == '\\') {
			escaped = true;
		} else if (character == '"') {
			*string_length = *offset - *start - 1;
			return 0;
		}
	}
	return -EINVAL;
}

static int skip_value(const uint8_t *json, size_t length, size_t *offset)
{
	skip_whitespace(json, length, offset);
	if (*offset >= length) {
		return -EINVAL;
	}
	if (json[*offset] == '"') {
		size_t start;
		size_t value_length;
		return string_bounds(json, length, offset, &start, &value_length);
	}
	if (json[*offset] == '{' || json[*offset] == '[') {
		uint8_t stack[MAX_LOGICAL_BYTES];
		size_t depth = 0;
		stack[depth++] = json[(*offset)++];
		bool in_string = false;
		bool escaped = false;
		while (*offset < length && depth != 0) {
			uint8_t character = json[(*offset)++];
			if (in_string) {
				if (escaped) {
					escaped = false;
				} else if (character == '\\') {
					escaped = true;
				} else if (character == '"') {
					in_string = false;
				}
			} else if (character == '"') {
				in_string = true;
			} else if (character == '{' || character == '[') {
				stack[depth++] = character;
			} else if (character == '}' || character == ']') {
				uint8_t expected = character == '}' ? '{' : '[';
				if (depth == 0 || stack[depth - 1] != expected) {
					return -EINVAL;
				}
				depth--;
			}
		}
		return depth == 0 ? 0 : -EINVAL;
	}
	while (*offset < length && json[*offset] != ',' && json[*offset] != '}') {
		(*offset)++;
	}
	return 0;
}

static int extract_top_level_type(const uint8_t *json, size_t length,
				  char *type, size_t capacity)
{
	size_t offset = 0;
	skip_whitespace(json, length, &offset);
	if (offset >= length || json[offset++] != '{') {
		return -EINVAL;
	}
	while (offset < length) {
		skip_whitespace(json, length, &offset);
		if (offset < length && json[offset] == '}') {
			return -EINVAL;
		}
		size_t key_start;
		size_t key_length;
		if (string_bounds(json, length, &offset, &key_start, &key_length) != 0) {
			return -EINVAL;
		}
		skip_whitespace(json, length, &offset);
		if (offset >= length || json[offset++] != ':') {
			return -EINVAL;
		}
		skip_whitespace(json, length, &offset);
		if (key_length == 4 &&
		    memcmp(&json[key_start], "type", key_length) == 0) {
			size_t value_start;
			size_t value_length;
			if (string_bounds(json, length, &offset, &value_start,
					  &value_length) != 0 ||
			    value_length + 1 > capacity) {
				return -EINVAL;
			}
			memcpy(type, &json[value_start], value_length);
			type[value_length] = '\0';
			return 0;
		}
		if (skip_value(json, length, &offset) != 0) {
			return -EINVAL;
		}
		skip_whitespace(json, length, &offset);
		if (offset >= length || json[offset++] != ',') {
			return -EINVAL;
		}
	}
	return -EINVAL;
}

static bool exact_capability_set(const struct session_start_message *message)
{
	if (message->required_capabilities_len != CAPABILITY_COUNT) {
		return false;
	}
	for (size_t expected = 0; expected < CAPABILITY_COUNT; ++expected) {
		size_t matches = 0;
		for (size_t actual = 0; actual < CAPABILITY_COUNT; ++actual) {
			if (strcmp(capabilities[expected],
				   message->required_capabilities[actual]) == 0) {
				matches++;
			}
		}
		if (matches != 1) {
			return false;
		}
	}
	return true;
}

static int copy_response(const char *json, uint8_t *response,
			 size_t response_capacity, size_t *response_length)
{
	size_t length = strlen(json);
	if (length > response_capacity || length > MAX_LOGICAL_BYTES) {
		return -EMSGSIZE;
	}
	memcpy(response, json, length);
	*response_length = length;
	return 0;
}

static int render_error(const char *code, const char *effect, char *response,
			size_t capacity)
{
	int length = snprintk(response, capacity,
			      "{\"ok\":false,\"error\":{\"code\":\"%s\","
			      "\"safety_effect\":\"%s\",\"radio_io_attempted\":false}}",
			      code, effect);
	return length < 0 || (size_t)length >= capacity ? -EMSGSIZE : length;
}

static int handle_session_start(char *json, size_t json_length,
				const uint8_t *exact_request,
				uint8_t *response, size_t response_capacity,
				size_t *response_length)
{
	struct session_start_message message = {0};
	char rendered[MAX_LOGICAL_BYTES];
	int64_t parsed = json_obj_parse(json, json_length, session_start_descr,
					ARRAY_SIZE(session_start_descr), &message);
	const int64_t required = BIT_MASK(ARRAY_SIZE(session_start_descr));
	if (parsed != required || strcmp(message.type, "session_start") != 0 ||
	    !valid_id(message.client_nonce) || !valid_id(message.op_id)) {
		int length = render_error("malformed", "none", rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if ((strcmp(message.client_nonce, cached_client_nonce) == 0 ||
	     strcmp(message.op_id, cached_start_op_id) == 0) &&
	    cached_start_length != 0) {
		if (cached_start_length == json_length &&
		    memcmp(cached_start, exact_request, json_length) == 0) {
			if (cached_start_response_length > response_capacity) {
				return -EMSGSIZE;
			}
			memcpy(response, cached_start_response,
			       cached_start_response_length);
			*response_length = cached_start_response_length;
			return 0;
		}
		int length =
			render_error("altered_duplicate", "none", rendered,
				     sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (strcmp(message.boot_id, boot_identity) != 0) {
		int length =
			render_error("stale_boot", "none", rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (message.select.major != 0 || message.select.minor != 0) {
		int length = render_error("unsupported_version", "none", rendered,
					  sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (!exact_capability_set(&message)) {
		int length = render_error("missing_capability", "none", rendered,
					  sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (message.client_rx_frame_limit < 20 ||
	    message.client_max_message_bytes < MAX_LOGICAL_BYTES) {
		int length = render_error("transport_limit_too_small", "none", rendered,
					  sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}

	rt_safety_session_replaced();
	uint8_t random_session[16];
	int err = sys_csrand_get(random_session, sizeof(random_session));
	if (err != 0) {
		return err;
	}
	render_hex(session_identity, random_session);
	err = rt_operation_cache_reset();
	if (err != 0) {
		rt_safety_protocol_fault();
		int length =
			render_error("protocol_fault", "lockout", rendered,
				     sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	session_active = true;
	next_seq = 1;
	rt_safety_protocol_session_active();
	uint16_t device_limit = rt_ble_command_value_limit();
	uint16_t tx_limit =
		rt_ble_set_response_frame_limit(message.client_rx_frame_limit);
	int length = snprintk(
		rendered, sizeof(rendered),
		"{\"ok\":true,\"result\":{\"type\":\"session_started\","
		"\"session_id\":\"%s\",\"selected\":{\"major\":0,\"minor\":0},"
		"\"capabilities\":[\"first_cause_fault_v0\","
		"\"independent_health_v0\",\"ordered_operations\","
		"\"ptt_leases_v0\",\"typed_radio_v0\"],"
		"\"device_rx_frame_limit\":%u,\"device_tx_frame_limit\":%u,"
		"\"max_message_bytes\":1024,\"max_session_operations\":512,"
		"\"next_seq\":1}}",
		session_identity, device_limit, tx_limit);
	if (length < 0 || (size_t)length >= sizeof(rendered)) {
		return -EMSGSIZE;
	}
	memcpy(cached_start, exact_request, json_length);
	cached_start_length = json_length;
	memcpy(cached_start_response, rendered, length);
	cached_start_response_length = length;
	strcpy(cached_client_nonce, message.client_nonce);
	strcpy(cached_start_op_id, message.op_id);
	return copy_response(rendered, response, response_capacity, response_length);
}

static const char *command_result(const char *type, char *body, size_t capacity)
{
	if (strcmp(type, "status_read") == 0) {
		snprintk(body, capacity,
			 "\"ok\":true,\"result\":{\"type\":\"status_read\"}");
	} else if (strcmp(type, "ptt_release") == 0) {
		rt_safety_release(RT_RELEASE_OPERATOR, false);
		snprintk(body, capacity,
			 "\"ok\":true,\"result\":{\"type\":\"ptt_release\","
			 "\"released\":true}");
	} else if (strcmp(type, "raw_cat") == 0 || strcmp(type, "TX") == 0 ||
		   strcmp(type, "SWT") == 0 || strcmp(type, "SWH") == 0 ||
		   strcmp(type, "KY") == 0 || strstr(type, "key") != NULL ||
		   strstr(type, "tune") != NULL || strstr(type, "xmit") != NULL) {
		snprintk(body, capacity,
			 "\"ok\":false,\"error\":{\"code\":"
			 "\"unsupported_radio_operation\",\"safety_effect\":\"none\","
			 "\"radio_io_attempted\":false}");
	} else if (strncmp(type, "radio_", 6) == 0) {
		snprintk(body, capacity,
			 "\"ok\":false,\"error\":{\"code\":\"radio_control_fault\","
			 "\"safety_effect\":\"none\",\"radio_io_attempted\":false}");
	} else if (strcmp(type, "ptt_acquire") == 0 ||
		   strcmp(type, "ptt_renew") == 0) {
		snprintk(body, capacity,
			 "\"ok\":false,\"error\":{\"code\":\"inhibit_open\","
			 "\"safety_effect\":\"none\",\"radio_io_attempted\":false}");
	} else {
		snprintk(body, capacity,
			 "\"ok\":false,\"error\":{\"code\":\"unsupported_command\","
			 "\"safety_effect\":\"none\",\"radio_io_attempted\":false}");
	}
	return body;
}

static int handle_request(char *json, size_t json_length,
			  const uint8_t *exact_request, uint8_t *response,
			  size_t response_capacity, size_t *response_length)
{
	struct request_message message = {0};
	char rendered[MAX_LOGICAL_BYTES];
	char command_json[MAX_LOGICAL_BYTES];
	char body[320];
	int64_t parsed = json_obj_parse(json, json_length, request_descr,
					ARRAY_SIZE(request_descr), &message);
	const int64_t required = BIT_MASK(ARRAY_SIZE(request_descr));
	if (parsed != required || strcmp(message.type, "request") != 0 ||
	    message.v.major != 0 || message.v.minor != 0 ||
	    !valid_id(message.boot_id) || !valid_id(message.session_id) ||
	    !valid_id(message.op_id)) {
		if (session_active) {
			rt_safety_protocol_fault();
		}
		int length = render_error("malformed",
					  session_active ? "lockout" : "none",
					  rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (!session_active || strcmp(message.session_id, session_identity) != 0 ||
	    strcmp(message.boot_id, boot_identity) != 0) {
		rt_safety_protocol_fault();
		int length =
			render_error(strcmp(message.boot_id, boot_identity) == 0 ?
					     "wrong_session" :
					     "stale_boot",
				     "lockout", rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	enum rt_cache_lookup cache_lookup;
	int cache_error = rt_operation_cache_lookup(
		message.op_id, message.seq, exact_request, json_length, response,
		response_capacity, response_length, &cache_lookup);
	if (cache_error != 0) {
		rt_safety_protocol_fault();
		session_active = false;
		int length =
			render_error("protocol_fault", "lockout", rendered,
				     sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (cache_lookup == RT_CACHE_EXACT) {
		return 0;
	}
	if (cache_lookup == RT_CACHE_ALTERED) {
		rt_safety_protocol_fault();
		int length = render_error("altered_duplicate", "lockout", rendered,
					  sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (next_seq > 512) {
		int length = render_error("session_exhausted", "none", rendered,
					  sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (message.seq != next_seq) {
		rt_safety_protocol_fault();
		int length = render_error(message.seq < next_seq ? "stale_operation" :
							       "out_of_order",
					  "lockout", rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}
	if (message.command.length >= sizeof(command_json)) {
		return -EMSGSIZE;
	}
	memcpy(command_json, message.command.start, message.command.length);
	command_json[message.command.length] = '\0';
	struct command_message command = {0};
	parsed = json_obj_parse(command_json, message.command.length, command_descr,
				ARRAY_SIZE(command_descr), &command);
	if (parsed != BIT(0) || command.type == NULL) {
		rt_safety_protocol_fault();
		int length =
			render_error("malformed", "lockout", rendered, sizeof(rendered));
		return length < 0 ? length :
				   copy_response(rendered, response, response_capacity,
						 response_length);
	}

	uint64_t accepted_at_ms = rt_monotonic_ms();
	command_result(command.type, body, sizeof(body));
	uint64_t response_next_seq = next_seq + 1;
	int length = snprintk(
		rendered, sizeof(rendered),
		"{\"type\":\"response\",\"v\":{\"major\":0,\"minor\":0},"
		"\"boot_id\":\"%s\",\"session_id\":\"%s\",\"op_id\":\"%s\","
		"\"seq\":%llu,\"accepted_at_ms\":%llu,\"next_seq\":%llu,%s}",
		boot_identity, session_identity, message.op_id,
		(unsigned long long)message.seq, (unsigned long long)accepted_at_ms,
		(unsigned long long)response_next_seq, body);
	if (length < 0 || (size_t)length >= sizeof(rendered)) {
		return -EMSGSIZE;
	}
	cache_error = rt_operation_cache_store(
		message.op_id, message.seq, exact_request, json_length,
		(const uint8_t *)rendered, length);
	if (cache_error != 0) {
		rt_safety_protocol_fault();
		session_active = false;
		int error_length =
			render_error("protocol_fault", "lockout", rendered,
				     sizeof(rendered));
		return error_length < 0 ?
			       error_length :
			       copy_response(rendered, response, response_capacity,
					     response_length);
	}
	next_seq = response_next_seq;
	return copy_response(rendered, response, response_capacity, response_length);
}

int rt_protocol_init(const uint8_t device_id[16], const uint8_t boot_id[16])
{
	ARG_UNUSED(device_id);
	k_mutex_lock(&protocol_lock, K_FOREVER);
	int err = rt_operation_cache_init();
	if (err != 0) {
		k_mutex_unlock(&protocol_lock);
		return err;
	}
	render_hex(boot_identity, boot_id);
	session_identity[0] = '\0';
	session_active = false;
	next_seq = 1;
	cached_start_length = 0;
	cached_start_response_length = 0;
	cached_client_nonce[0] = '\0';
	cached_start_op_id[0] = '\0';
	k_mutex_unlock(&protocol_lock);
	return 0;
}

void rt_protocol_disconnect(void)
{
	k_mutex_lock(&protocol_lock, K_FOREVER);
	session_active = false;
	session_identity[0] = '\0';
	next_seq = 1;
	k_mutex_unlock(&protocol_lock);
}

void rt_protocol_att_limit_changed(void)
{
	k_mutex_lock(&protocol_lock, K_FOREVER);
	bool replaced = session_active;
	if (replaced) {
		session_active = false;
		session_identity[0] = '\0';
		next_seq = 1;
	}
	k_mutex_unlock(&protocol_lock);
	if (replaced) {
		rt_safety_session_replaced();
		rt_ble_notify_status();
	}
}

bool rt_protocol_session_id(char session_id[33])
{
	k_mutex_lock(&protocol_lock, K_FOREVER);
	bool active = session_active;
	if (active) {
		strcpy(session_id, session_identity);
	}
	k_mutex_unlock(&protocol_lock);
	return active;
}

int rt_protocol_handle_logical(const uint8_t *request, size_t request_length,
			       uint8_t *response, size_t response_capacity,
			       size_t *response_length)
{
	if (request_length == 0 || request_length > MAX_LOGICAL_BYTES) {
		return -EMSGSIZE;
	}
	char json[MAX_LOGICAL_BYTES + 1];
	char envelope_type[16];
	memcpy(json, request, request_length);
	json[request_length] = '\0';
	int parsed =
		extract_top_level_type(request, request_length, envelope_type,
				       sizeof(envelope_type));
	k_mutex_lock(&protocol_lock, K_FOREVER);
	int result;
	if (parsed != 0) {
		if (session_active) {
			rt_safety_protocol_fault();
		}
		char rendered[160];
		int length = render_error("malformed",
					  session_active ? "lockout" : "none",
					  rendered, sizeof(rendered));
		result = length < 0 ?
				 length :
				 copy_response(rendered, response, response_capacity,
					       response_length);
	} else if (strcmp(envelope_type, "session_start") == 0) {
		result = handle_session_start(json, request_length, request, response,
					      response_capacity, response_length);
	} else if (strcmp(envelope_type, "request") == 0) {
		result = handle_request(json, request_length, request, response,
					response_capacity, response_length);
	} else {
		char rendered[160];
		int length = render_error("malformed",
					  session_active ? "lockout" : "none",
					  rendered, sizeof(rendered));
		if (session_active) {
			rt_safety_protocol_fault();
		}
		result = length < 0 ?
				 length :
				 copy_response(rendered, response, response_capacity,
					       response_length);
	}
	k_mutex_unlock(&protocol_lock);
	return result;
}
